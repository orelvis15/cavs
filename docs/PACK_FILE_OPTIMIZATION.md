# Pack-file optimization for patch-efficient builds (v0.9.0)

Pack files are where update efficiency is won or lost. This page lists
the failure modes `cavs analyze-packs` detects, what they cost under a
fixed-chunk updater, and the layout rules that fix them — each rule
validated by the measured cases in
[STEAMPIPE_COMPARISON.md](STEAMPIPE_COMPARISON.md).

## Running the analyzer

```sh
cavs analyze-packs ./Build_v1 ./Build_v2 --out pack-report.md
cavs analyze-packs old.pck new.pck --engine godot   # single packs work too
```

Output per pack: size, changed windows (64 KiB / 1 MiB / 8 MiB),
**scatteredness** (0 = one contiguous changed region, → 1 = every
changed window isolated), byte density inside changed windows, entropy,
fixed vs content-defined reuse, main issue, recommendation.

## Failure modes and fixes

### Scattered churn

One big pack changes in many distant regions. Fixed 1 MiB chunks cannot
reuse windows whose content moved or interleaves edits.
**Fix:** group assets by level/feature (update cadence), not by type;
split the pack so a change touches one part.

### Asset shuffling / offset cascades

Content similarity is high but fixed-chunk reuse is low: the bytes are
there, at different offsets. One grown asset shifts everything after
it; nondeterministic export orders shuffle assets between builds.
Measured cost: **32.88 MiB instead of 67 KiB** for identical content.
**Fix:** stable asset ordering, padding/alignment so unrelated assets
keep their offsets, never repack unchanged assets.

### Distributed TOC / absolute offsets

Thousands of tiny changed regions spread through the file — per-asset
headers or TOC entries rewritten every build. Measured: the same
64 KiB edit costs 32.00 MiB with distributed headers, 1.88 MiB with the
TOC centralized at the end.
**Fix:** move the TOC to the beginning or end; use relative offsets.

### Compression across asset boundaries

High entropy plus near-zero reuse under *both* chunk models: the pack
is one compressed (or encrypted) stream, so any source edit cascades to
the end of the file.
**Fix:** compress per asset; if alignment matters, pad compressed
assets into fixed slots (measured: 75% fixed reuse retained vs 0%).

### Timestamps / build IDs

Many files keep their size but change in one or two small windows.
Every release ships chunks that carry no content.
**Fix:** strip or pin timestamps and build IDs at export; make the
build deterministic.

### Oversized packs

| Size | Severity | Why |
|---|---|---|
| > 1 GiB | advisory | update I/O and CDN object size grow |
| > 2 GiB | warning | whole-pack rebuild on every touch |
| > 8 GiB | critical | local update I/O dominates any network saving |

**Fix:** split into 1–2 GiB parts aligned to depots/features. Measured
I/O effect: [IO_ESTIMATOR.md](IO_ESTIMATOR.md).

### New content inside an old pack

The pack grew by more than the build gained in new files: new content
was packed into a released pack, dirtying its layout.
**Fix:** ship new levels/features as new packs; keep released packs
immutable — the update then costs exactly the new content.

## The rule set

From `cavs optimize-layout` (advisory; `--write-plan` emits JSON):

- split oversized packs;
- group assets by level/feature;
- avoid asset order shuffling;
- add new content as new packs;
- avoid original filenames/timestamps inside packs;
- move the TOC to beginning/end; avoid absolute offsets;
- use per-asset compression instead of global compression;
- align Unreal-style pack padding to 1 MiB when targeting fixed-chunk
  updaters;
- separate platform-specific binaries from shared data.

## And if you cannot change the layout

CAVS's content-defined routes absorb most of these pathologies without
layout changes (shifted pack: 7.4 KiB vs 32.88 MiB), but a good layout
still wins: smaller plans, less local I/O, and efficiency on *every*
distribution channel — including fixed-chunk ones you don't control.
