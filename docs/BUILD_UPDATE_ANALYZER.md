# The build/update analyzer (v0.9.0)

The v0.9.0 command family answers, before you publish a build:

```text
Why is this update large?
Which files caused the update cost?
How would a SteamPipe-style fixed-1MiB chunk model behave?
How does CAVS compare with full download, butler, bsdiff, xdelta3?
Are pack files causing churn? Are assets shuffled inside a pack?
Is there distributed TOC or absolute-offset churn?
Is compression crossing asset boundaries?
How much local disk I/O will the update require?
Which route should a client use?
```

All estimates labeled SteamPipe-style come from a public fixed-1MiB
model, never Valve's implementation
([STEAMPIPE_STYLE_MODEL.md](STEAMPIPE_STYLE_MODEL.md)).

## The three altitude levels

| Command | Question it answers |
|---|---|
| `cavs bench steampipe-style` | *numbers* — how many chunks change, how many bytes ship |
| `cavs analyze steampipe` | *diagnosis* — why it costs that much, what to fix |
| `cavs publish-preview` | *decision* — every route measured, one recommended |

### `cavs bench steampipe-style ./old ./new`

The raw model run: chunk counts, estimated download, per-file ranking,
local rebuild I/O. Options: `--chunk-size`, `--compression none|zstd-N`,
`--scope per-file|global`, `--ignore`, `--json`, `--markdown`, `--out`.
`cavs analyze update-cost` is an alias.

### `cavs analyze steampipe ./old ./new [--engine godot|unity|unreal]`

Runs the model **plus** the content-defined contrast model, per-file
change heatmaps, entropy sampling and the detectors:

| Finding kind | Signal | Typical cause |
|---|---|---|
| `scattered_pack_churn` | many changed 1 MiB windows across many runs | assets grouped by type instead of update cadence |
| `asset_shuffling` | CDC reuse ≫ fixed reuse | reordered assets, grown asset shifting the rest, nondeterministic packing |
| `toc_churn` | many tiny isolated dirty regions spanning the file | distributed TOC entries / absolute offsets rewritten each build |
| `compressed_blob` | entropy ≥ 7.5 bits/byte and no reuse under either model | pack compressed/encrypted as one stream |
| `metadata_churn` | ≥5 same-size files with ≤2 dirty 64 KiB windows | timestamps, build IDs, generated names |
| `oversized_pack` | pack > 1 GiB (info) / 2 GiB (warning) / 8 GiB (critical) | monolithic packs |
| `new_content_in_old_pack` | pack grew ≫ new files added | new content packed into a released pack |

Each finding reports severity, the affected file, estimated wasted
bytes, why it happens, the recommended fix and the expected
improvement. Engine hints add Unreal/Unity/Godot-specific advice.

### `cavs publish-preview ./new --previous ./old --routes all`

Measures the real routes (full raw, full zstd, CAVS chunk/hybrid, CAVS
`.cavsplan` — built and applied, output verified byte-identical — plus
butler and bsdiff/xdelta3 proxies when those tools are installed), adds
the SteamPipe-style estimate row, lists release-readiness warnings from
the analyzer, and recommends a route with the reason. Missing external
tools are skipped with a warning, never fatal. Workspace mode:
`--workspace ./ws --from build_1001 --to build_1002`.

## Companion commands

- `cavs analyze-packs` — the pack-file table: size, changed windows,
  scatteredness, entropy, main issue, one-line fix
  ([PACK_FILE_OPTIMIZATION.md](PACK_FILE_OPTIMIZATION.md));
- `cavs analyze godot-pck` — Godot-specific PCK analysis
  ([GODOT_PCK_ANALYZER.md](GODOT_PCK_ANALYZER.md));
- `cavs optimize-layout` — the advisory restructuring plan
  (`--write-plan plan.json` for automation);
- `cavs io-estimate` — local disk I/O per route
  ([IO_ESTIMATOR.md](IO_ESTIMATOR.md));
- `cavs plan-update` — policy-scored route choice
  ([ROUTE_PLANNER.md](ROUTE_PLANNER.md)).

## Exit signals for CI

`analyze steampipe --json` exposes `findings[].severity`; gate a
pipeline by parsing it (e.g. fail on any `critical`). Measured
behavior across the pathological layouts:
[STEAMPIPE_COMPARISON.md](STEAMPIPE_COMPARISON.md).
