//! Deterministic build walking and mmap helpers shared by the analyzers.

use anyhow::Result;
use memmap2::Mmap;
use std::fs::File;
use std::path::{Path, PathBuf};

/// Recursively list files under `root` as sorted (relative-path,
/// absolute-path) pairs. When `root` is a single file, that file is the
/// only entry (its name as the relative path), so every analyzer works
/// for artifacts and directories alike.
pub fn walk(root: &Path) -> Result<Vec<(String, PathBuf)>> {
    if root.is_file() {
        let name = root
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "artifact".into());
        return Ok(vec![(name, root.to_path_buf())]);
    }
    let mut out = Vec::new();
    fn rec(base: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let ft = entry.file_type()?;
            if ft.is_dir() {
                rec(base, &path, out)?;
            } else if ft.is_file() {
                let rel = path
                    .strip_prefix(base)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.push((rel, path));
            }
        }
        Ok(())
    }
    rec(root, root, &mut out)?;
    out.sort();
    Ok(out)
}

/// mmap a file for read; `None` for empty files (mapping them is undefined).
pub fn mmap(path: &Path) -> Result<Option<Mmap>> {
    let file = File::open(path)?;
    if file.metadata()?.len() == 0 {
        return Ok(None);
    }
    // SAFETY: analyzers read a build tree they were pointed at; files are
    // not concurrently mutated during a run.
    Ok(Some(unsafe { Mmap::map(&file)? }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_is_sorted_and_handles_single_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("b")).unwrap();
        std::fs::write(dir.path().join("b/z.bin"), b"z").unwrap();
        std::fs::write(dir.path().join("a.bin"), b"a").unwrap();
        let files = walk(dir.path()).unwrap();
        let rels: Vec<&str> = files.iter().map(|(r, _)| r.as_str()).collect();
        assert_eq!(rels, vec!["a.bin", "b/z.bin"]);

        let single = walk(&dir.path().join("a.bin")).unwrap();
        assert_eq!(single.len(), 1);
        assert_eq!(single[0].0, "a.bin");
    }
}
