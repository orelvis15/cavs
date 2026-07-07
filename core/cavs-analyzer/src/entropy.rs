//! Shannon entropy estimation, used to spot compressed/encrypted blobs
//! whose shape defeats block-level patching.

/// Shannon entropy in bits/byte of a byte slice (0.0 for empty input).
pub fn shannon(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut counts = [0u64; 256];
    for b in data {
        counts[*b as usize] += 1;
    }
    let len = data.len() as f64;
    counts
        .iter()
        .filter(|c| **c > 0)
        .map(|c| {
            let p = *c as f64 / len;
            -p * p.log2()
        })
        .sum()
}

/// Entropy of a large payload without reading it all: samples up to
/// `samples` evenly spaced 64 KiB windows. Small payloads are measured
/// exactly.
pub fn sampled(data: &[u8], samples: usize) -> f64 {
    const WINDOW: usize = 64 * 1024;
    let samples = samples.max(1);
    if data.len() <= samples * WINDOW {
        return shannon(data);
    }
    let stride = data.len() / samples;
    let mut acc = 0.0;
    for i in 0..samples {
        let start = i * stride;
        let end = data.len().min(start + WINDOW);
        acc += shannon(&data[start..end]);
    }
    acc / samples as f64
}

/// Above this bits/byte the payload behaves like compressed/encrypted
/// data: nearly every source edit cascades through the container.
pub const HIGH_ENTROPY: f64 = 7.5;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_data_has_zero_entropy() {
        assert_eq!(shannon(&[7u8; 4096]), 0.0);
    }

    #[test]
    fn compressed_data_is_high_entropy() {
        // Compressible but not trivial: random 1 KiB blocks, each written
        // twice, so the compressed stream stays large and near-random.
        let mut state = 42u32;
        let mut source = Vec::new();
        for _ in 0..256 {
            let block: Vec<u8> = (0..1024)
                .map(|_| {
                    state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                    (state >> 24) as u8
                })
                .collect();
            source.extend_from_slice(&block);
            source.extend_from_slice(&block);
        }
        let compressed = zstd::bulk::compress(&source, 19).unwrap();
        assert!(compressed.len() > 32 * 1024);
        assert!(shannon(&compressed) > HIGH_ENTROPY);
        assert!(shannon(&source) > shannon(&compressed) - 1.0);
    }

    #[test]
    fn sampling_approximates_full_measurement() {
        let mut data = Vec::new();
        let mut state = 1234u32;
        for _ in 0..(8 << 20) {
            state = state.wrapping_mul(1664525).wrapping_add(1013904223);
            data.push((state >> 24) as u8);
        }
        let full = shannon(&data);
        let quick = sampled(&data, 16);
        assert!((full - quick).abs() < 0.2, "full {full} vs sampled {quick}");
    }
}
