# v0.9.0 benchmark results — raw outputs & reproduction

Raw outputs of the v0.9.0 SteamPipe-class analysis benchmark suite
(plan benchmarks A–H). The tables in
[BENCHMARKS.md](../../BENCHMARKS.md) and
[STEAMPIPE_COMPARISON.md](../../STEAMPIPE_COMPARISON.md) derive from the
files in this directory; the exact commands to regenerate them are below.

> Every SteamPipe-style figure is an estimate from a public fixed-1MiB
> model — not Valve's exact SteamPipe implementation
> (see [STEAMPIPE_STYLE_MODEL.md](../../STEAMPIPE_STYLE_MODEL.md)).

## Environment

| | |
|---|---|
| CPU | Apple M3 Pro (12 cores: 6P + 6E) |
| RAM | 36 GiB |
| OS | macOS 26.5.1 (Darwin 25.5.0) |
| Filesystem | APFS (internal NVMe) |
| rustc | 1.96.1 (release build, `lto = "thin"`, `codegen-units = 1`) |
| bsdiff / bspatch | 4.3 (Homebrew) |
| xdelta3 | 3.2.0 |
| butler | not installed for this run (routes report it as skipped) |
| Date | 2026-07-07 |

## What is here

| Directory | Plan benchmark | Content |
|---|---|---|
| `steampipe-cases/` | A (model validation), B (pack pathology), C (directory vs blob), G (Godot PCK) | 12 deterministic layouts measured under the SteamPipe-style model, real `.cavsplan`s, bsdiff and xdelta3 |
| `depot-sharing/` | D (depot sharing) | Sharing matrix across windows/linux/demo/lang-es/hd-textures depots, plus install plans by platform/language/ownership |
| `version-stream/` | E (many-version branch stream) | 10-release stream: content-addressed store vs pairwise patch storage |
| `io-estimate/` | F (local disk I/O) | 256 MiB pack with a ~3-byte change, monolithic vs split by level |
| `route-planner/` | H (route planner policies) | `plan-update` decisions across client states |
| `publish-preview/` | Phase-4 deliverable | Full route table + SteamPipe-style row + recommendation for a real directory pair |

## Headline numbers

From `steampipe-cases/steampipe-cases.md` (32 × ~1 MiB assets per pack,
seed 9):

| Case | SteamPipe-style estimate | CAVS `.cavsplan` | Diagnosis |
|---|---:|---:|---|
| pack-localized-small | 1.00 MiB | 131 KiB | localized / OK |
| pack-shifted | 32.85 MiB | 7.4 KiB | asset_shuffling |
| pack-shuffled | 32.88 MiB | 67 KiB | asset_shuffling |
| pack-toc-distributed | 32.00 MiB | 2.13 MiB | toc_churn |
| pack-toc-end (fix applied) | 1.88 MiB | 132 KiB | localized / OK |
| pack-global-compressed | 194 KiB (whole blob) | 130 KiB | 0% fixed reuse |
| pack-per-asset-compressed | 97 KiB | 68 KiB | 75% fixed reuse |
| new-content-new-pack | 4.00 MiB (= the new content) | 4.00 MiB | localized / OK |

From `io-estimate/`: a ~3-byte change in a 256 MiB pack downloads
2 MiB under the fixed-1MiB model but still costs **512 MiB of local
disk I/O** (read + rebuild); splitting the pack into 8 parts cuts that
to 128 MiB.

From `depot-sharing/`: windows ↔ linux depots share 98.9% of their
bytes; a demo owner who already installed the full build downloads
**0 B**.

## Reproduction — exact commands

All commands from the repository root after
`cargo build --release -p cavs-cli`, with `CAVS=target/release/cavs`.

```sh
# A, B, C, G — pathology cases (add --butler-bin when butler is installed)
$CAVS bench steampipe-cases --out results/steampipe-cases --include-pairwise

# D — depot sharing: build a workspace with shared depots, then
$CAVS depot analyze-sharing --workspace ws --out sharing.md
$CAVS install-plan --workspace ws --branch public --platform windows --json
$CAVS install-plan --workspace ws --branch public --platform windows \
  --owned windows,demo --from build_1001 --json

# E — many-version stream
$CAVS bench version-stream --out results/version-stream --size 24MiB --versions 10

# F — local disk I/O (dataset: 256 MiB pack, ~3 bytes flipped)
$CAVS io-estimate ./old ./new --out io-report.md

# H — route planner policies
$CAVS plan-update --from ./old --to ./new \
  --client-state has-previous-install,slow-hdd --policy balanced --json

# Publish preview (all routes; butler/pairwise skipped when missing)
$CAVS publish-preview ./Build_v2 --previous ./Build_v1 --routes all --out preview
```

The `version-stream/store/` payload directory is deleted after the run
(only the `.md`/`.json` summaries are kept here).
