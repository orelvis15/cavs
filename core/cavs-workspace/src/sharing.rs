//! Shared depot/content reuse analysis: how many bytes two depots have in
//! common, computed over their content-addressed chunk indices.

use crate::DepotIndex;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize, Clone)]
pub struct PairSharing {
    pub depot_a: String,
    pub depot_b: String,
    pub shared_bytes: u64,
    pub unique_a_bytes: u64,
    pub unique_b_bytes: u64,
    /// shared / (shared + unique_a + unique_b)
    pub reuse_ratio: f64,
}

/// Unique chunk set of a depot: hash → chunk length (dedup'd inside the
/// depot itself).
fn chunk_set(index: &DepotIndex) -> HashMap<&str, u64> {
    let mut set = HashMap::new();
    for chunks in index.files.values() {
        for (hash, len) in chunks {
            set.entry(hash.as_str()).or_insert(*len);
        }
    }
    set
}

/// Sharing between two depots.
pub fn pair(a: &DepotIndex, b: &DepotIndex) -> PairSharing {
    let sa = chunk_set(a);
    let sb = chunk_set(b);
    let mut shared = 0u64;
    let mut unique_a = 0u64;
    for (hash, len) in &sa {
        if sb.contains_key(hash) {
            shared += len;
        } else {
            unique_a += len;
        }
    }
    let unique_b: u64 = sb
        .iter()
        .filter(|(h, _)| !sa.contains_key(*h))
        .map(|(_, len)| len)
        .sum();
    let total = shared + unique_a + unique_b;
    PairSharing {
        depot_a: a.depot_id.clone(),
        depot_b: b.depot_id.clone(),
        shared_bytes: shared,
        unique_a_bytes: unique_a,
        unique_b_bytes: unique_b,
        reuse_ratio: if total == 0 {
            0.0
        } else {
            shared as f64 / total as f64
        },
    }
}

/// Sharing across every pair of the given depots.
pub fn matrix(indices: &[DepotIndex]) -> Vec<PairSharing> {
    let mut out = Vec::new();
    for i in 0..indices.len() {
        for j in (i + 1)..indices.len() {
            out.push(pair(&indices[i], &indices[j]));
        }
    }
    out.sort_by_key(|p| std::cmp::Reverse(p.shared_bytes));
    out
}

/// Bytes a client must fetch for `wanted` when it already holds every
/// chunk of `have` (cross-depot install reuse).
pub fn fetch_bytes(wanted: &DepotIndex, have: &[&DepotIndex]) -> u64 {
    let mut held: HashMap<&str, u64> = HashMap::new();
    for h in have {
        held.extend(chunk_set(h));
    }
    let mut fetch = 0u64;
    let mut counted: HashMap<&str, ()> = HashMap::new();
    for chunks in wanted.files.values() {
        for (hash, len) in chunks {
            if !held.contains_key(hash.as_str()) && counted.insert(hash.as_str(), ()).is_none() {
                fetch += len;
            }
        }
    }
    fetch
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn index(id: &str, files: &[(&str, &[(&str, u64)])]) -> DepotIndex {
        let mut map = BTreeMap::new();
        let mut total = 0;
        for (path, chunks) in files {
            total += chunks.iter().map(|(_, l)| l).sum::<u64>();
            map.insert(
                path.to_string(),
                chunks
                    .iter()
                    .map(|(h, l)| (h.to_string(), *l))
                    .collect::<Vec<_>>(),
            );
        }
        DepotIndex {
            depot_id: id.into(),
            total_bytes: total,
            files: map,
        }
    }

    #[test]
    fn pair_sharing_and_reuse_ratio() {
        let a = index(
            "windows",
            &[
                ("data.bin", &[("h1", 100), ("h2", 200)]),
                ("win.exe", &[("h3", 50)]),
            ],
        );
        let b = index(
            "linux",
            &[
                ("data.bin", &[("h1", 100), ("h2", 200)]),
                ("linux.bin", &[("h4", 30)]),
            ],
        );
        let s = pair(&a, &b);
        assert_eq!(s.shared_bytes, 300);
        assert_eq!(s.unique_a_bytes, 50);
        assert_eq!(s.unique_b_bytes, 30);
        assert!((s.reuse_ratio - 300.0 / 380.0).abs() < 1e-9);
    }

    #[test]
    fn fetch_reuses_held_depots() {
        let base = index("base", &[("a", &[("h1", 100), ("h2", 200)])]);
        let dlc = index("dlc", &[("b", &[("h2", 200), ("h9", 500)])]);
        assert_eq!(fetch_bytes(&dlc, &[&base]), 500);
        assert_eq!(fetch_bytes(&dlc, &[]), 700);
    }

    #[test]
    fn matrix_covers_every_pair() {
        let a = index("a", &[("x", &[("h1", 1)])]);
        let b = index("b", &[("x", &[("h1", 1)])]);
        let c = index("c", &[("x", &[("h2", 1)])]);
        let m = matrix(&[a, b, c]);
        assert_eq!(m.len(), 3);
        assert_eq!(m[0].shared_bytes, 1); // a↔b first (most shared)
    }
}
