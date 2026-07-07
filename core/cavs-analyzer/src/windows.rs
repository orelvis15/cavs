//! Change heatmaps over fixed windows and the scatteredness score.
//!
//! Positional comparison: window `i` of the old bytes against window `i`
//! of the new bytes. This is deliberately *not* the chunk-reuse model —
//! it answers "where inside the file did bytes change", which is what the
//! pack diagnostics (TOC churn, scattered edits) need.

use serde::Serialize;

/// The window sizes `cavs analyze-packs` reports on.
pub const REPORT_WINDOWS: &[usize] = &[64 * 1024, 1024 * 1024, 8 * 1024 * 1024];

#[derive(Serialize, Clone)]
pub struct Heatmap {
    pub window_size: u64,
    pub total_windows: u64,
    pub changed_windows: u64,
    /// Contiguous runs of changed windows.
    pub runs: u64,
    /// (runs − 1) / (changed − 1): 0 = one contiguous block of changes,
    /// → 1 = every changed window is isolated.
    pub scatteredness: f64,
    /// Fraction of the file's windows between the first and last change.
    pub span_ratio: f64,
    /// Differing bytes inside changed windows / bytes of changed windows.
    /// Low density + many runs = tiny scattered edits (TOC-like churn).
    pub changed_byte_density: f64,
    /// Largest changed ranges as (start_window, windows) pairs, biggest
    /// first, at most 5.
    pub largest_ranges: Vec<(u64, u64)>,
    /// Average changed-run length in windows.
    pub mean_run_len: f64,
}

/// Compare `old` and `new` positionally over fixed windows of
/// `window_size` bytes. Length differences count as changed windows.
pub fn heatmap(old: &[u8], new: &[u8], window_size: usize) -> Heatmap {
    let w = window_size.max(1);
    let total = new.len().div_ceil(w).max(old.len().div_ceil(w));
    let mut changed = vec![false; total];
    let mut changed_bytes = 0u64;
    let mut changed_window_bytes = 0u64;

    for (i, slot) in changed.iter_mut().enumerate() {
        let start = i * w;
        let o = old.get(start..old.len().min(start + w)).unwrap_or(&[]);
        let n = new.get(start..new.len().min(start + w)).unwrap_or(&[]);
        if o == n {
            continue;
        }
        *slot = true;
        let len = o.len().max(n.len());
        changed_window_bytes += len as u64;
        let common = o.len().min(n.len());
        let diff_in_common = o[..common]
            .iter()
            .zip(&n[..common])
            .filter(|(a, b)| a != b)
            .count();
        changed_bytes += (diff_in_common + (len - common)) as u64;
    }

    let mut runs = 0u64;
    let mut ranges: Vec<(u64, u64)> = Vec::new();
    let mut i = 0usize;
    while i < total {
        if changed[i] {
            let start = i;
            while i < total && changed[i] {
                i += 1;
            }
            runs += 1;
            ranges.push((start as u64, (i - start) as u64));
        } else {
            i += 1;
        }
    }
    ranges.sort_by_key(|(_, len)| std::cmp::Reverse(*len));
    ranges.truncate(5);

    let changed_count = changed.iter().filter(|c| **c).count() as u64;
    let scatteredness = if changed_count <= 1 {
        0.0
    } else {
        (runs - 1) as f64 / (changed_count - 1) as f64
    };
    let span_ratio = match (
        changed.iter().position(|c| *c),
        changed.iter().rposition(|c| *c),
    ) {
        (Some(first), Some(last)) if total > 0 => (last - first + 1) as f64 / total as f64,
        _ => 0.0,
    };

    Heatmap {
        window_size: w as u64,
        total_windows: total as u64,
        changed_windows: changed_count,
        runs,
        scatteredness,
        span_ratio,
        changed_byte_density: if changed_window_bytes == 0 {
            0.0
        } else {
            changed_bytes as f64 / changed_window_bytes as f64
        },
        largest_ranges: ranges,
        mean_run_len: if runs == 0 {
            0.0
        } else {
            changed_count as f64 / runs as f64
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_data_has_no_changes() {
        let data = vec![7u8; 1 << 20];
        let h = heatmap(&data, &data, 64 * 1024);
        assert_eq!(h.changed_windows, 0);
        assert_eq!(h.scatteredness, 0.0);
        assert_eq!(h.runs, 0);
    }

    #[test]
    fn localized_change_is_one_run_zero_scatteredness() {
        let old = vec![7u8; 1 << 20];
        let mut new = old.clone();
        // 3 consecutive 64 KiB windows changed
        for b in &mut new[128 * 1024..320 * 1024] {
            *b = 9;
        }
        let h = heatmap(&old, &new, 64 * 1024);
        assert_eq!(h.changed_windows, 3);
        assert_eq!(h.runs, 1);
        assert_eq!(h.scatteredness, 0.0);
        assert_eq!(h.largest_ranges[0], (2, 3));
    }

    #[test]
    fn scattered_tiny_changes_score_high_with_low_density() {
        let old = vec![7u8; 4 << 20];
        let mut new = old.clone();
        // One byte flipped in every fourth 64 KiB window: isolated runs.
        let w = 64 * 1024;
        for i in (0..(new.len() / w)).step_by(4) {
            new[i * w] ^= 0xff;
        }
        let h = heatmap(&old, &new, w);
        assert!(h.changed_windows >= 15);
        assert_eq!(h.runs, h.changed_windows); // all isolated
        assert!(h.scatteredness > 0.9, "scatteredness {}", h.scatteredness);
        assert!(h.changed_byte_density < 0.001);
        assert!(h.span_ratio > 0.9);
        assert!((h.mean_run_len - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn growth_counts_as_changed_tail() {
        let old = vec![1u8; 100 * 1024];
        let mut new = old.clone();
        new.extend(vec![2u8; 100 * 1024]);
        let h = heatmap(&old, &new, 64 * 1024);
        assert!(h.changed_windows >= 2);
    }
}
