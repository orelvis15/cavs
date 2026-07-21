//! `cavs-fetch` — an embeddable serverless/CDN fetch engine for CAVS.
//!
//! A launcher or game links this library to install and update a build
//! **in-process**, straight from a static export produced by
//! `cavs store export --static-plans` (S3 / R2 / GitHub Pages / nginx / a
//! local folder) — no `cavs-server` and no shelling out to the CLI. It:
//!
//! 1. reads the per-asset `manifest.json` (reconstruction structure) and
//!    `chunk-map.json` (each chunk's pack + absolute byte range),
//! 2. computes the missing set against a persistent content-addressable
//!    cache (so an update downloads only what changed),
//! 3. HTTP-Range-GETs (or slice-reads, for a local folder) the missing
//!    chunks concurrently, verifying every one by BLAKE3, and
//! 4. reconstructs the output files from the cache — byte-identical or it
//!    fails.
//!
//! It reports progress through a callback and supports cooperative
//! cancellation, so a UI can show a progress bar and a Cancel button. The
//! same engine is exposed through the CAVS SDKs (`fetchStatic`) and the C
//! ABI, which is what the Unity and Unreal plugins call.

mod cache;
mod reconstruct;
mod source;

pub use cache::ChunkCache;
pub use source::StaticSource;

use anyhow::{bail, Context, Result};
use cavs_hash::{from_hex, hash_chunk, ChunkHash};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

const CHUNK_FLAG_ZSTD: u32 = 1;

/// BG4 pretransform chunk flag (mirrors `cavs_format::CHUNK_FLAG_BG4`).
const CHUNK_FLAG_BG4: u32 = 1 << 1;

/// Inverse of the BG4 byte-grouping pretransform (mirrors
/// `cavs_format::bg4_ungroup`; duplicated to keep this crate embeddable
/// without a cavs-format dependency).
fn bg4_ungroup(grouped: &[u8]) -> Vec<u8> {
    let len = grouped.len();
    let mut out = vec![0u8; len];
    let mut it = grouped.iter();
    for lane in 0..4 {
        let mut i = lane;
        while i < len {
            out[i] = *it.next().unwrap();
            i += 4;
        }
    }
    out
}

/// Options for a serverless fetch.
pub struct FetchOptions<'a> {
    /// Concurrent range requests (>=1).
    pub connections: usize,
    /// Optional Ed25519 public key (64 hex) to enforce the content signature.
    pub pubkey: Option<String>,
    /// Progress callback: invoked with cumulative `(done_bytes, total_bytes)`
    /// as chunks land. `total_bytes` is the wire size of the missing set.
    pub progress: Option<&'a (dyn Fn(u64, u64) + Send + Sync)>,
    /// Cooperative cancellation: when set to `true`, an in-flight fetch stops
    /// and returns [`FetchError::Cancelled`].
    pub cancel: Option<&'a AtomicBool>,
}

impl Default for FetchOptions<'_> {
    fn default() -> Self {
        Self {
            connections: 8,
            pubkey: None,
            progress: None,
            cancel: None,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, serde::Serialize)]
pub struct FetchStats {
    /// Bytes pulled over the wire (stored, possibly compressed).
    pub wire_bytes: u64,
    /// Decompressed bytes written to the cache.
    pub raw_bytes: u64,
    /// Chunks downloaded.
    pub fetched: u64,
    /// Chunks already present in the cache (an update's reuse).
    pub reused: u64,
    /// Total logical size of the reconstructed asset.
    pub logical_bytes: u64,
}

/// A fetch failure with a stable reason, so an embedder can decide
/// retry/repair/cancel without parsing prose.
#[derive(Debug)]
pub enum FetchError {
    Cancelled,
    Other(anyhow::Error),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Cancelled => write!(f, "CAVS-E-CANCELLED: fetch cancelled"),
            FetchError::Other(e) => write!(f, "{e:#}"),
        }
    }
}
impl std::error::Error for FetchError {}
impl From<anyhow::Error> for FetchError {
    fn from(e: anyhow::Error) -> Self {
        FetchError::Other(e)
    }
}

#[derive(Debug, Deserialize)]
struct ChunkMapFile {
    #[allow(dead_code)]
    asset: String,
    chunks: Vec<ChunkMapEntry>,
}

#[derive(Debug, Deserialize, Clone)]
struct ChunkMapEntry {
    hash: String,
    len_raw: u32,
    len_stored: u32,
    flags: u32,
    pack: String,
    pack_offset_abs: u64,
}

/// Fetch `asset` from the static tree at `source` into `output`, caching in
/// `cache_dir`. Returns egress/reuse stats. Byte-identical reconstruction or
/// an error — a partially written output file is never promoted.
pub fn fetch_static(
    source: &StaticSource,
    asset: &str,
    output: &Path,
    cache_dir: &Path,
    opts: &FetchOptions,
) -> std::result::Result<FetchStats, FetchError> {
    fetch_static_inner(source, asset, output, cache_dir, opts).map_err(|e| {
        // Preserve an explicit cancellation as such.
        if e.downcast_ref::<Cancelled>().is_some() {
            FetchError::Cancelled
        } else {
            FetchError::Other(e)
        }
    })
}

struct Cancelled;
impl std::fmt::Debug for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cancelled")
    }
}
impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cancelled")
    }
}
impl std::error::Error for Cancelled {}

fn fetch_static_inner(
    source: &StaticSource,
    asset: &str,
    output: &Path,
    cache_dir: &Path,
    opts: &FetchOptions,
) -> Result<FetchStats> {
    let cache = ChunkCache::open(cache_dir)?;

    // 1. Manifest + chunk-map.
    let manifest_bytes = source
        .get_all(&format!("assets/{asset}/manifest.json"))
        .with_context(|| format!("asset {asset}: no manifest.json in the static tree"))?;
    let manifest: cavs_proto::Manifest =
        serde_json::from_slice(&manifest_bytes).context("parsing manifest.json")?;

    if let Some(pk) = &opts.pubkey {
        verify_signature(&manifest, pk)?;
    }

    let map_bytes = source
        .get_all(&format!("assets/{asset}/chunk-map.json"))
        .with_context(|| format!("asset {asset}: no chunk-map.json in the static tree"))?;
    let map: ChunkMapFile = serde_json::from_slice(&map_bytes).context("parsing chunk-map.json")?;
    let locations: HashMap<String, ChunkMapEntry> = map
        .chunks
        .into_iter()
        .map(|c| (c.hash.clone(), c))
        .collect();

    // 2. Missing set.
    let mut seen = std::collections::HashSet::new();
    let mut missing: Vec<ChunkMapEntry> = Vec::new();
    let mut reused = 0u64;
    for hex in manifest_chunk_hashes(&manifest) {
        if !seen.insert(hex.clone()) {
            continue;
        }
        if cache.contains(&hex) {
            reused += 1;
            continue;
        }
        let loc = locations.get(&hex).with_context(|| {
            format!("chunk {hex} referenced by manifest but absent from chunk-map")
        })?;
        missing.push(loc.clone());
    }

    // 3. Concurrent range fetch, coalesced: adjacent missing chunks of the
    //    same pack travel in one Range GET instead of one request per chunk.
    let groups = plan_range_groups(missing);
    let total_wire: u64 = groups.iter().map(|g| g.span).sum();
    let stats = fetch_missing_parallel(source, &groups, &cache, opts, total_wire)?;

    // 4. Reconstruct from cache.
    reconstruct::reconstruct(&manifest, &cache, output)?;

    Ok(FetchStats {
        reused,
        logical_bytes: reconstruct::logical_bytes(&manifest),
        ..stats
    })
}

/// Tolerated gap between two missing chunks fetched in one range: the extra
/// bytes cost less than another round-trip (mirrors the store's read
/// coalescing).
const MAX_COALESCE_GAP: u64 = 64 * 1024;

/// Upper bound of one coalesced range: keeps per-request memory bounded and
/// requests parallelizable across connections.
const MAX_COALESCED_RANGE: u64 = 8 * 1024 * 1024;

/// One Range GET covering a run of missing chunks in the same pack.
struct RangeGroup {
    pack: String,
    /// Absolute offset of the first chunk.
    start: u64,
    /// Bytes to request (last chunk end − start, gaps included).
    span: u64,
    chunks: Vec<ChunkMapEntry>,
}

/// Group the missing set into coalesced ranges: sort by (pack, offset), then
/// extend the current run while the gap to the next chunk is at most
/// [`MAX_COALESCE_GAP`] and the total span stays within
/// [`MAX_COALESCED_RANGE`]. A push writes related chunks contiguously, so a
/// cold or update fetch typically collapses thousands of per-chunk requests
/// into a few dozen ranges.
fn plan_range_groups(mut missing: Vec<ChunkMapEntry>) -> Vec<RangeGroup> {
    missing.sort_by(|a, b| {
        (a.pack.as_str(), a.pack_offset_abs).cmp(&(b.pack.as_str(), b.pack_offset_abs))
    });
    let mut groups: Vec<RangeGroup> = Vec::new();
    for entry in missing {
        let end = entry.pack_offset_abs + entry.len_stored as u64;
        if let Some(g) = groups.last_mut() {
            let g_end = g.start + g.span;
            if g.pack == entry.pack
                && entry.pack_offset_abs >= g_end
                && entry.pack_offset_abs - g_end <= MAX_COALESCE_GAP
                && end - g.start <= MAX_COALESCED_RANGE
            {
                g.span = end - g.start;
                g.chunks.push(entry);
                continue;
            }
        }
        groups.push(RangeGroup {
            pack: entry.pack.clone(),
            start: entry.pack_offset_abs,
            span: entry.len_stored as u64,
            chunks: vec![entry],
        });
    }
    groups
}

fn fetch_missing_parallel(
    source: &StaticSource,
    missing: &[RangeGroup],
    cache: &ChunkCache,
    opts: &FetchOptions,
    total_wire: u64,
) -> Result<FetchStats> {
    if missing.is_empty() {
        if let Some(p) = opts.progress {
            p(0, 0);
        }
        return Ok(FetchStats::default());
    }
    let workers = opts.connections.max(1).min(missing.len());
    let next = AtomicUsize::new(0);
    let failed = AtomicBool::new(false);
    let first_error: Mutex<Option<anyhow::Error>> = Mutex::new(None);
    let wire = AtomicUsize::new(0);
    let raw = AtomicUsize::new(0);
    let fetched = AtomicUsize::new(0);

    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                if failed.load(Ordering::Relaxed) {
                    return;
                }
                if opts.cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
                    let mut g = first_error.lock().unwrap();
                    if g.is_none() {
                        *g = Some(anyhow::Error::new(Cancelled));
                    }
                    failed.store(true, Ordering::Relaxed);
                    return;
                }
                let idx = next.fetch_add(1, Ordering::Relaxed);
                if idx >= missing.len() {
                    return;
                }
                match fetch_group(source, &missing[idx], cache) {
                    Ok((raw_len, wire_len, chunk_count)) => {
                        wire.fetch_add(wire_len, Ordering::Relaxed);
                        raw.fetch_add(raw_len, Ordering::Relaxed);
                        fetched.fetch_add(chunk_count, Ordering::Relaxed);
                        if let Some(p) = opts.progress {
                            p(wire.load(Ordering::Relaxed) as u64, total_wire);
                        }
                    }
                    Err(e) => {
                        let mut g = first_error.lock().unwrap();
                        if g.is_none() {
                            *g = Some(e);
                        }
                        failed.store(true, Ordering::Relaxed);
                        return;
                    }
                }
            });
        }
    });

    if let Some(e) = first_error.into_inner().unwrap() {
        return Err(e);
    }
    Ok(FetchStats {
        wire_bytes: wire.load(Ordering::Relaxed) as u64,
        raw_bytes: raw.load(Ordering::Relaxed) as u64,
        fetched: fetched.load(Ordering::Relaxed) as u64,
        ..FetchStats::default()
    })
}

/// Fetch one coalesced range and land every chunk it covers: slice each
/// chunk out of the response, decode, BLAKE3-verify and cache it. The
/// coalescing never weakens verification — every chunk is still checked
/// against its own hash. Returns `(raw_bytes, wire_bytes, chunks)`.
fn fetch_group(
    source: &StaticSource,
    group: &RangeGroup,
    cache: &ChunkCache,
) -> Result<(usize, usize, usize)> {
    let wire = source.get_range(&group.pack, group.start, group.span)?;
    if (wire.len() as u64) < group.span {
        bail!(
            "short range read from {}: got {} of {} bytes",
            group.pack,
            wire.len(),
            group.span
        );
    }
    let mut raw_total = 0usize;
    for entry in &group.chunks {
        let hash: ChunkHash = from_hex(&entry.hash)
            .with_context(|| format!("bad hash {} in chunk-map", entry.hash))?;
        let at = (entry.pack_offset_abs - group.start) as usize;
        let stored = &wire[at..at + entry.len_stored as usize];
        let mut raw = if entry.flags & CHUNK_FLAG_ZSTD != 0 {
            zstd::bulk::decompress(stored, entry.len_raw as usize)
                .map_err(|e| anyhow::anyhow!("decompressing chunk {}: {e}", entry.hash))?
        } else {
            stored.to_vec()
        };
        if entry.flags & CHUNK_FLAG_BG4 != 0 {
            raw = bg4_ungroup(&raw);
        }
        if raw.len() != entry.len_raw as usize || hash_chunk(&raw) != hash {
            bail!(
                "CAVS-E-CHUNK-HASH-MISMATCH: chunk {} failed verification",
                entry.hash
            );
        }
        raw_total += raw.len();
        cache.put(&hash, &raw)?;
    }
    Ok((raw_total, wire.len(), group.chunks.len()))
}

/// Every unique chunk hash the manifest references (init + segment chunks).
fn manifest_chunk_hashes(manifest: &cavs_proto::Manifest) -> Vec<String> {
    let mut set = std::collections::HashSet::new();
    for t in &manifest.tracks {
        for c in &t.init_chunks {
            set.insert(c.hash.clone());
        }
    }
    for s in &manifest.segments {
        for c in &s.chunks {
            set.insert(c.hash.clone());
        }
    }
    set.into_iter().collect()
}

/// Enforce the manifest's Ed25519 content signature against a trusted key.
fn verify_signature(manifest: &cavs_proto::Manifest, trusted_hex: &str) -> Result<()> {
    use ed25519_dalek::Verifier;
    let sig_hex = manifest
        .signature
        .as_deref()
        .context("asset is not signed but a pubkey was given")?;
    let signer_hex = manifest
        .signer_pubkey
        .as_deref()
        .context("asset signature has no public key")?;
    if !signer_hex.eq_ignore_ascii_case(trusted_hex) {
        bail!("asset is signed by an untrusted key {signer_hex}");
    }
    let leaves: Vec<ChunkHash> = manifest
        .chunk_table
        .iter()
        .map(|h| from_hex(h).context("bad hash in chunk_table"))
        .collect::<Result<_>>()?;
    let root = cavs_hash::merkle_root(&leaves);
    if !manifest
        .merkle_root
        .eq_ignore_ascii_case(&cavs_hash::to_hex(&root))
    {
        bail!("manifest merkle_root does not match its chunk_table");
    }
    let pk: [u8; 32] = decode_hex(signer_hex, 32)?.try_into().unwrap();
    let sig: [u8; 64] = decode_hex(sig_hex, 64)?.try_into().unwrap();
    let key = ed25519_dalek::VerifyingKey::from_bytes(&pk).context("invalid signer key")?;
    let message = cavs_hash::content_signature_message(&root, leaves.len() as u64);
    key.verify(&message, &ed25519_dalek::Signature::from_bytes(&sig))
        .map_err(|_| anyhow::anyhow!("content signature is INVALID"))?;
    // Every referenced chunk must be covered by the signed table.
    let table: std::collections::HashSet<&str> =
        manifest.chunk_table.iter().map(|s| s.as_str()).collect();
    for h in manifest_chunk_hashes(manifest) {
        if !table.contains(h.as_str()) {
            bail!("chunk {h} referenced but not covered by the signed table");
        }
    }
    Ok(())
}

fn decode_hex(s: &str, len: usize) -> Result<Vec<u8>> {
    if s.len() != len * 2 {
        bail!("expected {} hex chars, got {}", len * 2, s.len());
    }
    (0..len)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).context("bad hex"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(pack: &str, offset: u64, len: u32) -> ChunkMapEntry {
        ChunkMapEntry {
            hash: format!("{pack}-{offset}"),
            len_raw: len,
            len_stored: len,
            flags: 0,
            pack: pack.to_string(),
            pack_offset_abs: offset,
        }
    }

    #[test]
    fn adjacent_chunks_coalesce_into_one_range() {
        let groups = plan_range_groups(vec![
            entry("p", 100, 50),
            entry("p", 150, 50),
            // 64 KiB gap is still tolerated
            entry("p", 200 + MAX_COALESCE_GAP, 10),
        ]);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].start, 100);
        assert_eq!(groups[0].span, 100 + MAX_COALESCE_GAP + 10);
        assert_eq!(groups[0].chunks.len(), 3);
    }

    #[test]
    fn gap_pack_and_span_limits_split_groups() {
        let groups = plan_range_groups(vec![
            entry("p", 0, 10),
            entry("p", 10 + MAX_COALESCE_GAP + 1, 10), // gap too large
            entry("q", 0, 10),                         // different pack
        ]);
        assert_eq!(groups.len(), 3);

        // Span cap: two large chunks that would exceed the max stay apart.
        let half = (MAX_COALESCED_RANGE / 2 + 1) as u32;
        let groups = plan_range_groups(vec![entry("p", 0, half), entry("p", half as u64, half)]);
        assert_eq!(groups.len(), 2);
    }

    #[test]
    fn unsorted_input_is_sorted_before_grouping() {
        let groups = plan_range_groups(vec![entry("p", 60, 40), entry("p", 0, 60)]);
        assert_eq!(groups.len(), 1);
        assert_eq!((groups[0].start, groups[0].span), (0, 100));
    }
}
