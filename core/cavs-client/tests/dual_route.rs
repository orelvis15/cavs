//! End-to-end tests of the v2 dual delivery route: real cavs + cavs-server +
//! cavs-client binaries over HTTP.
//!
//! Covers the four routing outcomes: bootstrap when it is cheaper for a cold
//! cache (and that it seeds the cache so the next update is incremental),
//! chunks when the bootstrap is not worth it, chunks when no bootstrap
//! exists, and chunks when the sidecar is tampered with.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

fn bin(name: &str) -> PathBuf {
    let mut path = std::env::current_exe().unwrap();
    path.pop(); // deps/
    path.pop(); // debug/
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

struct ServerGuard(Child);

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn spawn_server(cavs_files: &[&Path]) -> (ServerGuard, String) {
    let mut cmd = Command::new(bin("cavs-server"));
    for f in cavs_files {
        cmd.arg(f);
    }
    let mut child = cmd
        .args(["--listen", "127.0.0.1:0"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn cavs-server");
    let stdout = child.stdout.take().unwrap();
    let mut line = String::new();
    BufReader::new(stdout)
        .read_line(&mut line)
        .expect("server did not print its address");
    let url = line
        .trim()
        .strip_prefix("listening on ")
        .expect("unexpected server banner")
        .to_string();
    (ServerGuard(child), url)
}

fn stats(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("stats json missing"))
        .expect("bad stats json")
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

/// Build-like payload with *long-range approximate* redundancy: the second
/// half repeats the first half with a sparse byte flipped every 97 bytes.
/// Content-defined chunking cannot dedup it (every chunk differs by a few
/// bytes -> different hash), but whole-file zstd matches it almost entirely —
/// exactly the asymmetry that makes the bootstrap route cheaper for a cold
/// client, mirroring how real packs compress better as one stream.
fn build_like(len: usize, seed: u32) -> Vec<u8> {
    let half = len / 2;
    let mut out = pseudo_random(half, seed);
    let mut echo = out.clone();
    for i in (0..echo.len()).step_by(97) {
        echo[i] ^= 0x5A;
    }
    out.extend_from_slice(&echo);
    out.truncate(len);
    out
}

#[test]
fn cold_bootstrap_then_incremental_update() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();

    // v1 and v2: v2 rewrites a slice in the middle (a localized update).
    let v1 = build_like(6 * 1024 * 1024, 100);
    let mut v2 = v1.clone();
    let at = 3 * 1024 * 1024;
    v2[at..at + 256 * 1024].copy_from_slice(&pseudo_random(256 * 1024, 999));
    std::fs::write(d.join("game_v1.pck"), &v1).unwrap();
    std::fs::write(d.join("game_v2.pck"), &v2).unwrap();

    let (ok, out) = run(
        "cavs",
        &[
            "pack",
            "--raw",
            d.join("game_v1.pck").to_str().unwrap(),
            "--profile",
            "auto",
            "--bootstrap",
            "-o",
            d.join("game_v1.cavs").to_str().unwrap(),
        ],
    );
    assert!(ok, "pack v1 failed:\n{out}");
    assert!(d.join("game_v1.cavs.bootstrap.zst").is_file());
    let (ok, out) = run(
        "cavs",
        &[
            "pack",
            "--raw",
            d.join("game_v2.pck").to_str().unwrap(),
            "--profile",
            "auto",
            "--prev",
            d.join("game_v1.cavs").to_str().unwrap(),
            "--bootstrap",
            "-o",
            d.join("game_v2.cavs").to_str().unwrap(),
        ],
    );
    assert!(ok, "pack v2 failed:\n{out}");

    let (_guard, url) = spawn_server(&[&d.join("game_v1.cavs"), &d.join("game_v2.cavs")]);

    // Cold fetch v1: bootstrap route, byte-identical output, cache seeded.
    let (ok, out) = run(
        "cavs-client",
        &[
            "fetch",
            &url,
            "game_v1",
            "-o",
            d.join("out1").to_str().unwrap(),
            "--cache",
            d.join("cache").to_str().unwrap(),
            "--stats-json",
            d.join("s1.json").to_str().unwrap(),
        ],
    );
    assert!(ok, "cold fetch failed:\n{out}");
    let s1 = stats(&d.join("s1.json"));
    assert_eq!(s1["delivery_mode"], "bootstrap", "stats: {s1}");
    assert!(s1["seeded_chunks"].as_u64().unwrap() > 0);
    assert_eq!(std::fs::read(d.join("out1/game_v1.pck")).unwrap(), v1);
    // The wire cost must actually beat the chunk-path estimate the server
    // reported (that is the whole point of the route).
    assert!(s1["inline_bytes"].as_u64().unwrap() < v1.len() as u64);

    // Update to v2 with the seeded cache: chunk route, small payload.
    let (ok, out) = run(
        "cavs-client",
        &[
            "fetch",
            &url,
            "game_v2",
            "-o",
            d.join("out2").to_str().unwrap(),
            "--cache",
            d.join("cache").to_str().unwrap(),
            "--stats-json",
            d.join("s2.json").to_str().unwrap(),
        ],
    );
    assert!(ok, "update fetch failed:\n{out}");
    let s2 = stats(&d.join("s2.json"));
    assert_eq!(s2["delivery_mode"], "chunks", "stats: {s2}");
    assert!(s2["refs"].as_u64().unwrap() > 0, "no cache reuse: {s2}");
    // The update must cost a fraction of the full build: the bootstrap
    // seeding is what makes this possible on a cold-installed client.
    let update_wire = s2["inline_bytes"].as_u64().unwrap();
    assert!(
        update_wire < v2.len() as u64 / 3,
        "update too big: {update_wire} of {}",
        v2.len()
    );
    assert_eq!(std::fs::read(d.join("out2/game_v2.pck")).unwrap(), v2);

    // Warm re-fetch: references only, zero wire.
    let (ok, out) = run(
        "cavs-client",
        &[
            "fetch",
            &url,
            "game_v2",
            "-o",
            d.join("out3").to_str().unwrap(),
            "--cache",
            d.join("cache").to_str().unwrap(),
            "--stats-json",
            d.join("s3.json").to_str().unwrap(),
        ],
    );
    assert!(ok, "warm fetch failed:\n{out}");
    let s3 = stats(&d.join("s3.json"));
    assert_eq!(s3["delivery_mode"], "references");
    assert_eq!(s3["inline_bytes"].as_u64().unwrap(), 0);
}

#[test]
fn incompressible_payload_stays_on_chunk_route() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();

    // Pure random bytes: the whole-file zstd bootstrap cannot be >=2%
    // cheaper than the chunk path, so the server must not offer it.
    let v1 = pseudo_random(4 * 1024 * 1024, 55);
    std::fs::write(d.join("blob.bin"), &v1).unwrap();
    let (ok, out) = run(
        "cavs",
        &[
            "pack",
            "--raw",
            d.join("blob.bin").to_str().unwrap(),
            "--bootstrap",
            "-o",
            d.join("blob.cavs").to_str().unwrap(),
        ],
    );
    assert!(ok, "pack failed:\n{out}");

    let (_guard, url) = spawn_server(&[&d.join("blob.cavs")]);
    let (ok, out) = run(
        "cavs-client",
        &[
            "fetch",
            &url,
            "blob",
            "-o",
            d.join("out").to_str().unwrap(),
            "--cache",
            d.join("cache").to_str().unwrap(),
            "--stats-json",
            d.join("s.json").to_str().unwrap(),
        ],
    );
    assert!(ok, "fetch failed:\n{out}");
    let s = stats(&d.join("s.json"));
    assert_eq!(s["delivery_mode"], "chunks", "stats: {s}");
    assert_eq!(std::fs::read(d.join("out/blob.bin")).unwrap(), v1);
}

#[test]
fn asset_without_bootstrap_serves_chunks() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let v1 = build_like(2 * 1024 * 1024, 7);
    std::fs::write(d.join("asset.pck"), &v1).unwrap();
    // No --bootstrap at pack time.
    let (ok, out) = run(
        "cavs",
        &[
            "pack",
            "--raw",
            d.join("asset.pck").to_str().unwrap(),
            "-o",
            d.join("asset.cavs").to_str().unwrap(),
        ],
    );
    assert!(ok, "pack failed:\n{out}");

    let (_guard, url) = spawn_server(&[&d.join("asset.cavs")]);
    let (ok, out) = run(
        "cavs-client",
        &[
            "fetch",
            &url,
            "asset",
            "-o",
            d.join("out").to_str().unwrap(),
            "--cache",
            d.join("cache").to_str().unwrap(),
            "--stats-json",
            d.join("s.json").to_str().unwrap(),
        ],
    );
    assert!(ok, "fetch failed:\n{out}");
    let s = stats(&d.join("s.json"));
    assert_eq!(s["delivery_mode"], "chunks");
    assert_eq!(std::fs::read(d.join("out/asset.pck")).unwrap(), v1);
}

#[test]
fn tampered_bootstrap_sidecar_is_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let d = dir.path();
    let v1 = build_like(2 * 1024 * 1024, 21);
    std::fs::write(d.join("asset.pck"), &v1).unwrap();
    let (ok, out) = run(
        "cavs",
        &[
            "pack",
            "--raw",
            d.join("asset.pck").to_str().unwrap(),
            "--profile",
            "auto",
            "--bootstrap",
            "-o",
            d.join("asset.cavs").to_str().unwrap(),
        ],
    );
    assert!(ok, "pack failed:\n{out}");

    // Corrupt the sidecar: the server must refuse it at load and fall back
    // to the chunk route (never serving unverifiable bytes).
    let sidecar = d.join("asset.cavs.bootstrap.zst");
    let mut bytes = std::fs::read(&sidecar).unwrap();
    let mid = bytes.len() / 2;
    bytes[mid] ^= 0xFF;
    std::fs::write(&sidecar, &bytes).unwrap();

    let (_guard, url) = spawn_server(&[&d.join("asset.cavs")]);
    let (ok, out) = run(
        "cavs-client",
        &[
            "fetch",
            &url,
            "asset",
            "-o",
            d.join("out").to_str().unwrap(),
            "--cache",
            d.join("cache").to_str().unwrap(),
            "--stats-json",
            d.join("s.json").to_str().unwrap(),
        ],
    );
    assert!(ok, "fetch failed:\n{out}");
    let s = stats(&d.join("s.json"));
    assert_eq!(s["delivery_mode"], "chunks", "stats: {s}");
    assert_eq!(std::fs::read(d.join("out/asset.pck")).unwrap(), v1);
}
