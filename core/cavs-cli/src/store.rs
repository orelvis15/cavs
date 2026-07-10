//! `cavs store` — manage a global content-addressable store: ingest `.cavs`
//! releases (deduplicating chunks across all of them), unpublish, garbage
//! collect zero-ref chunks, and report storage savings.

use crate::report::human_bytes;
use crate::StorageArg;
use anyhow::{Context, Result};
use cavs_format::{Reader, SEGMENT_FLAG_RANDOM_ACCESS};
use cavs_hash::to_hex;
use cavs_store::{AssetRecord, GlobalStore, StoreLayout, StoreSegment, StoreTrack};
use std::path::Path;

/// Ingest a `.cavs` file into the store under `asset_name`, deduplicating
/// its chunks against everything already stored. `storage` selects the
/// physical layout when the store is newly created.
pub fn add(
    store_dir: &Path,
    asset_name: &str,
    cavs_path: &Path,
    storage: Option<StorageArg>,
) -> Result<()> {
    let mut reader =
        Reader::open(cavs_path).with_context(|| format!("cannot open {}", cavs_path.display()))?;
    // Refuse to ingest content whose embedded signature is invalid.
    let signature = match reader.verify_signature()? {
        cavs_format::SignatureStatus::Valid(_) => reader.embedded_signature(),
        cavs_format::SignatureStatus::Unsigned => None,
    };

    let layout = storage.map(|s| match s {
        StorageArg::Loose => StoreLayout::Loose,
        StorageArg::Packfiles => StoreLayout::Packfiles,
    });
    let mut store = GlobalStore::open_with_layout(store_dir, layout)?;
    let chunks = reader.chunks().to_vec();

    // Store every unique chunk (in stored/compressed form) into the CAS.
    let mut new_chunks = 0u64;
    let mut new_bytes = 0u64;
    for i in 0..chunks.len() as u32 {
        let (stored, flags, len_raw) = reader.read_chunk_stored(i)?;
        let hash = chunks[i as usize].hash;
        if store.put_chunk(&hash, &stored, flags, len_raw)? {
            new_chunks += 1;
            new_bytes += stored.len() as u64;
        }
    }

    let hex = |idx: u32| to_hex(&chunks[idx as usize].hash);
    let tracks: Vec<StoreTrack> = reader
        .tracks()
        .iter()
        .map(|t| StoreTrack {
            track_id: t.track_id,
            kind: t.kind as u8,
            codec: t.codec.clone(),
            name: t.name.clone(),
            timescale: t.timescale,
            init_chunks: t.init_chunks.iter().map(|&i| hex(i)).collect(),
        })
        .collect();
    let segments: Vec<StoreSegment> = reader
        .segments()
        .iter()
        .map(|s| StoreSegment {
            segment_id: s.segment_id,
            track_id: s.track_id,
            pts_start: s.pts_start,
            duration: s.duration,
            random_access: s.flags & SEGMENT_FLAG_RANDOM_ACCESS != 0,
            chunks: s.chunks.iter().map(|&i| hex(i)).collect(),
        })
        .collect();

    let record = AssetRecord {
        name: asset_name.to_string(),
        asset_uuid: reader
            .superblock()
            .asset_uuid
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        tracks,
        segments,
        dict: reader.dict().iter().map(|&i| hex(i)).collect(),
        chunk_table: chunks.iter().map(|c| to_hex(&c.hash)).collect(),
        merkle_root: to_hex(&reader.integrity().merkle_root),
        signature: signature.map(|(sig, _)| {
            to_hex(&sig[..32].try_into().unwrap()) + &to_hex(&sig[32..].try_into().unwrap())
        }),
        signer_pubkey: signature.map(|(_, pk)| to_hex(&pk)),
        meta: reader.meta().to_vec(),
    };
    store.publish_asset(&record)?;

    let total = chunks.len() as u64;
    println!(
        "added   : {asset_name} ({} chunks, {} new / {} deduplicated)",
        total,
        new_chunks,
        total - new_chunks
    );
    println!("new data: {} written to store", human_bytes(new_bytes));
    print_stats(&store);
    Ok(())
}

pub fn remove(store_dir: &Path, asset_name: &str) -> Result<()> {
    let mut store = GlobalStore::open(store_dir)?;
    if store.unpublish_asset(asset_name)? {
        println!(
            "removed : {asset_name} (run `cavs store {} gc` to reclaim space)",
            store_dir.display()
        );
        print_stats(&store);
    } else {
        println!("not found: {asset_name}");
    }
    Ok(())
}

pub fn gc(store_dir: &Path, grace_secs: u64) -> Result<()> {
    let mut store = GlobalStore::open(store_dir)?;
    let (removed, bytes) = store.gc(grace_secs)?;
    println!(
        "gc      : removed {removed} zero-ref chunks, reclaimed {}",
        human_bytes(bytes)
    );
    print_stats(&store);
    Ok(())
}

pub fn stat(store_dir: &Path) -> Result<()> {
    let store = GlobalStore::open(store_dir)?;
    println!("store   : {}", store_dir.display());
    for name in store.asset_names() {
        println!("  asset : {name}");
    }
    print_stats(&store);
    Ok(())
}

/// Re-hash every chunk and check every referenced pack's integrity.
pub fn verify(store_dir: &Path) -> Result<()> {
    let store = GlobalStore::open(store_dir)?;
    let checked = store.verify()?;
    println!("verify  : OK — {checked} chunks re-hashed, packs intact");
    Ok(())
}

/// Build the runtime `Manifest` for a stored asset (the reconstruction
/// structure a client needs: ordered chunks per track/segment, with each
/// chunk's raw length pulled from the store ledger). Mirrors the server's
/// `AppState::manifest`, but reads from an `AssetRecord` + the chunk ledger
/// so a *serverless* client can plan a fetch from a static export.
fn record_to_manifest(store: &GlobalStore, record: &AssetRecord) -> cavs_proto::Manifest {
    let chunk_ref = |hex: &str| {
        let len = cavs_hash::from_hex(hex)
            .and_then(|h| store.chunk_info(&h))
            .map(|i| i.len_raw)
            .unwrap_or(0);
        cavs_proto::ChunkRef {
            hash: hex.to_string(),
            len,
        }
    };
    cavs_proto::Manifest {
        asset: record.name.clone(),
        asset_uuid: record.asset_uuid.clone(),
        tracks: record
            .tracks
            .iter()
            .map(|t| cavs_proto::ManifestTrack {
                track_id: t.track_id,
                kind: cavs_format::TrackKind::from_u8(t.kind)
                    .map(|k| k.label().to_string())
                    .unwrap_or_else(|| "data".to_string()),
                codec: t.codec.clone(),
                name: t.name.clone(),
                timescale: t.timescale,
                init_chunks: t.init_chunks.iter().map(|h| chunk_ref(h)).collect(),
            })
            .collect(),
        segments: record
            .segments
            .iter()
            .map(|s| cavs_proto::ManifestSegment {
                segment_id: s.segment_id,
                track_id: s.track_id,
                pts_start: s.pts_start,
                duration: s.duration,
                random_access: s.random_access,
                chunks: s.chunks.iter().map(|h| chunk_ref(h)).collect(),
            })
            .collect(),
        dict: record.dict.clone(),
        chunk_table: record.chunk_table.clone(),
        merkle_root: record.merkle_root.clone(),
        signature: record.signature.clone(),
        signer_pubkey: record.signer_pubkey.clone(),
        meta: record.meta.clone(),
    }
}

/// Write `assets/<name>/manifest.json` for every asset into the export tree,
/// so a serverless client can read the reconstruction structure with no
/// running server. Returns the relative paths written.
fn write_static_manifests(store: &GlobalStore, out: &Path) -> Result<Vec<String>> {
    let mut written = Vec::new();
    for name in store.asset_names() {
        let record = store.get_asset(&name)?;
        let manifest = record_to_manifest(store, &record);
        let rel = format!("assets/{name}/manifest.json");
        let dst = out.join(&rel);
        std::fs::create_dir_all(dst.parent().unwrap())?;
        std::fs::write(&dst, serde_json::to_vec_pretty(&manifest)?)
            .with_context(|| format!("writing {}", dst.display()))?;
        written.push(rel);
    }
    Ok(written)
}

/// Export as a deterministic immutable object tree for object storage/CDN.
pub fn export(store_dir: &Path, out: &Path, static_plans: bool) -> Result<()> {
    let store = GlobalStore::open(store_dir)?;
    let mut written = store.export_object_store(out)?;
    if static_plans {
        let plans = store.export_static_plans(out)?;
        let manifests = write_static_manifests(&store, out)?;
        println!(
            "plans   : {} chunk-map.json + {} manifest.json (serverless clients)",
            plans.len(),
            manifests.len()
        );
        written.extend(plans);
        written.extend(manifests);
    }
    let packs = written
        .iter()
        .filter(|p| p.starts_with("chunks/packs/"))
        .count();
    println!(
        "exported: {} objects ({packs} packs) -> {}",
        written.len(),
        out.display()
    );
    println!("layout  :");
    for rel in written.iter().take(6) {
        println!("  {rel}");
    }
    if written.len() > 6 {
        println!("  … {} more", written.len() - 6);
    }
    println!(
        "headers : packs/indexes are content-addressed — serve them with\n          \
         Cache-Control: public, max-age=31536000, immutable\n          \
         ETag: \"blake3-<filename stem>\"\n          \
         assets/<name>/record.json is mutable — Cache-Control: no-cache"
    );
    Ok(())
}

fn print_stats(store: &GlobalStore) {
    let s = store.stats();
    // stored_bytes can briefly exceed the logical (referenced) total when
    // zero-ref orphans await gc, so saturate to avoid an underflow.
    let saved = if s.logical_stored_bytes == 0 {
        0.0
    } else {
        s.logical_stored_bytes.saturating_sub(s.stored_bytes) as f64 * 100.0
            / s.logical_stored_bytes as f64
    };
    println!(
        "totals  : {} assets · {} unique chunks · {} stored ({} zero-ref)",
        s.assets,
        s.unique_chunks,
        human_bytes(s.stored_bytes),
        s.zero_ref_chunks
    );
    println!(
        "dedup   : {} logical -> {} unique = {:.1}% storage saved across versions/titles",
        human_bytes(s.logical_stored_bytes),
        human_bytes(s.stored_bytes),
        saved
    );
    if s.layout == StoreLayout::Packfiles {
        let live_pct = if s.pack_disk_bytes == 0 {
            100.0
        } else {
            s.pack_live_bytes as f64 * 100.0 / s.pack_disk_bytes as f64
        };
        println!(
            "packs   : {} packfiles · {} on disk · {:.1}% live",
            s.pack_count,
            human_bytes(s.pack_disk_bytes),
            live_pct
        );
    }
}
