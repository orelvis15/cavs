//! Keep the remote's static export tree in sync with its GlobalStore, and
//! serialize store writers across processes.

use anyhow::{Context, Result};
use cavs_store::GlobalStore;
use std::path::Path;

/// Exclusive advisory lock on `<tree>/.store.lock`. Held for the whole
/// upload (ingest + export); released when dropped (the OS also releases it
/// if the process dies). Note: advisory file locks are unreliable on some
/// network filesystems (NFS) — see the crate README.
pub struct StoreLock {
    _file: std::fs::File,
}

impl StoreLock {
    pub fn acquire(tree: &Path) -> Result<Self> {
        let path = tree.join(".store.lock");
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&path)
            .with_context(|| format!("cannot open lock file {}", path.display()))?;
        file.lock()
            .with_context(|| format!("cannot lock {}", path.display()))?;
        Ok(Self { _file: file })
    }
}

/// Refresh the static export tree read by `cavs-fetch`: immutable packs and
/// indexes (skipped when already present), per-asset `record.json`,
/// `chunk-map.json` and `manifest.json`.
pub fn export_remote(store: &GlobalStore, tree: &Path) -> Result<Vec<String>> {
    let mut written = store.export_object_store(tree)?;
    written.extend(store.export_static_plans(tree)?);
    written.extend(store.export_static_manifests(tree)?);
    Ok(written)
}
