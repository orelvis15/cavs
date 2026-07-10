//! End-to-end test for the v1.4.0 serverless / CDN-only fetch: pack two
//! versions, ingest into a packfile store, `store export --static-plans`,
//! then `cavs-client fetch-static` straight from the exported directory —
//! no cavs-server anywhere — and assert byte-identical reconstruction plus
//! incremental updates (the second version reuses the first's cache).

use std::path::PathBuf;
use std::process::Command;

fn bin(name: &str) -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop();
    path.pop();
    path.push(name);
    path
}

fn run(binary: &str, args: &[&str]) -> (bool, String) {
    let out = Command::new(bin(binary))
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run {binary}: {e}"));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

fn pseudo_random(len: usize, seed: u32) -> Vec<u8> {
    let mut out = vec![0u8; len];
    let mut state = seed;
    for b in out.iter_mut() {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        *b = (state >> 24) as u8;
    }
    out
}

fn parse_wire_bytes(output: &str) -> f64 {
    let line = output
        .lines()
        .find(|l| l.starts_with("egress"))
        .unwrap_or_else(|| panic!("no egress line in:\n{output}"));
    let rest = line.split(':').nth(1).unwrap().trim();
    let mut parts = rest.split_whitespace();
    let value: f64 = parts.next().unwrap().parse().unwrap();
    let unit = parts.next().unwrap();
    let mult = match unit {
        "B" => 1.0,
        "KiB" => 1024.0,
        "MiB" => 1024.0 * 1024.0,
        other => panic!("unexpected unit {other}"),
    };
    value * mult
}

#[test]
fn serverless_fetch_from_static_export() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();

    // Two versions of a build; v2 rewrites a slice in the middle.
    let v1 = pseudo_random(3_000_000, 21);
    let mut v2 = v1.clone();
    v2[1_400_000..1_460_000].copy_from_slice(&pseudo_random(60_000, 99));
    std::fs::write(d.join("game_v1.bin"), &v1).unwrap();
    std::fs::write(d.join("game_v2.bin"), &v2).unwrap();

    // Pack both (fastcdc so the update is boundary-stable).
    for v in ["v1", "v2"] {
        let src = d.join(format!("game_{v}.bin"));
        let cavs = d.join(format!("game_{v}.cavs"));
        let mut args = vec![
            "pack".to_string(),
            "--raw".to_string(),
            src.to_str().unwrap().to_string(),
            "--profile".to_string(),
            "fastcdc-16k".to_string(),
            "--zstd-level".to_string(),
            "9".to_string(),
            "-o".to_string(),
            cavs.to_str().unwrap().to_string(),
        ];
        if v == "v2" {
            args.push("--prev".to_string());
            args.push(d.join("game_v1.cavs").to_str().unwrap().to_string());
        }
        let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let (ok, out) = run("cavs", &refs);
        assert!(ok, "pack {v} failed:\n{out}");
    }

    // Ingest into a packfile store (export requires the packfile layout).
    let store = d.join("store");
    let (ok, out) = run(
        "cavs",
        &[
            "store",
            store.to_str().unwrap(),
            "add",
            "game_v1",
            d.join("game_v1.cavs").to_str().unwrap(),
            "--storage",
            "packfiles",
        ],
    );
    assert!(ok, "store add v1 failed:\n{out}");
    let (ok, out) = run(
        "cavs",
        &[
            "store",
            store.to_str().unwrap(),
            "add",
            "game_v2",
            d.join("game_v2.cavs").to_str().unwrap(),
        ],
    );
    assert!(ok, "store add v2 failed:\n{out}");

    // Export the static tree (packs + manifest.json + chunk-map.json).
    let dist = d.join("dist");
    let (ok, out) = run(
        "cavs",
        &[
            "store",
            store.to_str().unwrap(),
            "export",
            "--out",
            dist.to_str().unwrap(),
            "--static-plans",
        ],
    );
    assert!(ok, "export failed:\n{out}");
    assert!(dist.join("assets/game_v1/manifest.json").is_file());
    assert!(dist.join("assets/game_v1/chunk-map.json").is_file());

    // Serverless cold install of v1 straight from the directory.
    let cache = d.join("cache");
    let out1 = d.join("install1");
    let (ok, log1) = run(
        "cavs-client",
        &[
            "fetch-static",
            dist.to_str().unwrap(),
            "game_v1",
            "-o",
            out1.to_str().unwrap(),
            "--cache",
            cache.to_str().unwrap(),
        ],
    );
    assert!(ok, "serverless fetch v1 failed:\n{log1}");
    assert_eq!(std::fs::read(out1.join("game_v1.bin")).unwrap(), v1);
    let cold_wire = parse_wire_bytes(&log1);
    assert!(
        cold_wire > 0.0,
        "cold install should download bytes:\n{log1}"
    );

    // Serverless update to v2 reusing the same cache: only changed chunks.
    let out2 = d.join("install2");
    let (ok, log2) = run(
        "cavs-client",
        &[
            "fetch-static",
            dist.to_str().unwrap(),
            "game_v2",
            "-o",
            out2.to_str().unwrap(),
            "--cache",
            cache.to_str().unwrap(),
        ],
    );
    assert!(ok, "serverless fetch v2 failed:\n{log2}");
    assert_eq!(std::fs::read(out2.join("game_v2.bin")).unwrap(), v2);
    let update_wire = parse_wire_bytes(&log2);
    // The update must cost a small fraction of a cold install of v2.
    assert!(
        update_wire < cold_wire * 0.5,
        "update wire {update_wire} should be far below cold {cold_wire}:\n{log2}"
    );
}
