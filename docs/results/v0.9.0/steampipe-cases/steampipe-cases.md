# SteamPipe-style Model & Pack Pathology Benchmark

> SteamPipe-style estimate based on public documentation. This is not Valve's exact SteamPipe implementation.

Deterministic datasets (seed 9, 32 × 1 MiB assets per pack).

## Environment

| | |
|---|---|
| OS | macOS 26.5.1 (Darwin 25.5.0) (aarch64) |
| CPU | Apple M3 Pro |
| RAM | 36 GiB |
| Disk | APFS (internal NVMe SSD) |
| CAVS | cavs 0.9.0 |
| bsdiff | present: bsdiff oldfile newfile patchfile |
| xdelta3 | Xdelta version 3.2.0, Copyright (C) Joshua MacDonald |
| zstd (linked library) | 1.5.7 |
| Command | `/Users/l41777/Documents/repositories/bitlakelab/cavs-oss/target/release/cavs bench steampipe-cases --out /Users/l41777/Documents/repositories/bitlakelab/cavs-oss/docs/results/v0.9.0/steampipe-cases --assets 32 --seed 9 --include-pairwise` |
| Dataset seed | 9 |

## Results

| Case | New size | SteamPipe-style | Changed chunks | Fixed reuse | CDC reuse | CAVS .cavsplan | butler | bsdiff | xdelta3 | Diagnosis |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| pack-localized-small | 32.88 MiB | 1.00 MiB | 1 of 33 | 97.0% | 99.6% | 131.48 KiB | — | 64.82 KiB | 64.30 KiB | localized / OK |
| pack-localized-medium | 32.88 MiB | 4.00 MiB | 4 of 33 | 87.8% | 95.7% | 1.07 MiB | — | 804.67 KiB | 800.36 KiB | localized / OK |
| pack-shifted | 32.88 MiB | 32.88 MiB | 33 of 33 | 0.0% | 99.8% | 7.44 KiB | — | 4.57 KiB | 4.25 KiB | asset_shuffling |
| pack-shuffled | 32.88 MiB | 32.88 MiB | 33 of 33 | 0.0% | 98.8% | 67.48 KiB | — | 151 B | 237 B | asset_shuffling |
| pack-toc-distributed | 32.88 MiB | 32.00 MiB | 32 of 33 | 2.7% | 91.4% | 2.13 MiB | — | 65.00 KiB | 64.57 KiB | toc_churn, asset_shuffling |
| pack-toc-end | 32.88 MiB | 1.88 MiB | 2 of 33 | 94.3% | 99.4% | 131.56 KiB | — | 64.80 KiB | 64.31 KiB | localized / OK |
| pack-global-compressed | 195.49 KiB | 194.07 KiB | 1 of 1 | 0.0% | 18.0% | 130.38 KiB | — | 64.84 KiB | 64.25 KiB | localized / OK |
| pack-per-asset-compressed | 4.00 MiB | 96.58 KiB | 1 of 4 | 75.0% | 97.4% | 68.35 KiB | — | 64.84 KiB | 64.23 KiB | localized / OK |
| new-content-new-pack | 36.88 MiB | 4.00 MiB | 4 of 37 | 89.2% | 89.1% | 4.00 MiB | — | 4.00 MiB | 4.00 MiB | localized / OK |
| directory-build | 32.88 MiB | 1.00 MiB | 1 of 64 | 97.0% | 99.6% | 1.01 MiB | — | 68.88 KiB | 67.80 KiB | localized / OK |
| godot-pck-localized | 2.50 MiB | 1.00 MiB | 1 of 3 | 60.0% | 97.2% | 128.46 KiB | — | 32.65 KiB | 32.19 KiB | localized / OK |
| godot-pck-shifted | 3.50 MiB | 3.50 MiB | 4 of 4 | 0.0% | 62.3% | 1.06 MiB | — | 1.01 MiB | 1.00 MiB | localized / OK |

## Case descriptions

- **pack-localized-small** — one 64 KiB edit inside a big pack
- **pack-localized-medium** — four 200 KiB edits spread over the pack
- **pack-shifted** — 4 KiB inserted at the front; every byte after shifts
- **pack-shuffled** — same assets, new order
- **pack-toc-distributed** — per-asset headers rewritten every build (build id); one 64 KiB real edit
- **pack-toc-end** — same edit and build id bump with the TOC at the end only
- **pack-global-compressed** — whole pack zstd-3 as one stream; one 64 KiB source edit
- **pack-per-asset-compressed** — each asset compressed into a padded 128 KiB slot; same 64 KiB source edit
- **new-content-new-pack** — 4 new assets ship as a new pack; the old pack stays identical
- **directory-build** — same assets as individual files; one 64 KiB edit
- **godot-pck-localized** — Godot PCK with one edited resource
- **godot-pck-shifted** — Godot PCK with a new resource packed first (offset shift)

> skipped: pairwise patches serve exactly one old→new pair; storage and generation cost grow with every published pair
