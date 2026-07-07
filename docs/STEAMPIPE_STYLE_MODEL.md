# The SteamPipe-style update model (v0.9.0)

CAVS v0.9.0 includes an update-cost model inspired by the publicly
documented behavior of SteamPipe, Valve's content system. This page
defines exactly what that model is — and, more importantly, what it is
not.

## What the model is not

- The model is based on **public SteamPipe documentation**
  ([Steamworks: Uploading to Steam](https://partner.steamgames.com/doc/sdk/uploading)).
- **It is not Valve's implementation.** CAVS does not reproduce Steam's
  code, formats or servers.
- It uses **fixed 1 MiB chunks as a public approximation** of the
  documented "roughly 1 MB" split.
- It does **not model Steam encryption internals**.
- It does **not model private Valve alternate build algorithms** — if
  Steam applies smarter matching than the documented chunk search, real
  updates may be smaller than this estimate.

Every report produced by the model carries this labeling:

```text
SteamPipe-style estimate based on public documentation.
This is not Valve's exact SteamPipe implementation.
```

Naming follows the same rule everywhere: *SteamPipe-style* or
*SteamPipe-inspired*, always presented as an **estimated model** —
never "SteamPipe compatible", "SteamPipe replacement" or any claim of
real SteamPipe compatibility. There is also deliberately no separate
`steam-analyzer` product; see
[WHY_NO_STEAM_ANALYZER_PRODUCT.md](WHY_NO_STEAM_ANALYZER_PRODUCT.md).

## What the model computes

The documented behavior being approximated:

> SteamPipe splits files into roughly 1 MB chunks. Chunks are compressed
> and encrypted. During updates, SteamPipe searches for matching chunks
> from previous builds — ideally only new or modified file portions
> become new chunks.

The CAVS model:

```text
1. Walk the old build deterministically (sorted relative paths).
2. Split each file into fixed 1 MiB chunks (from the start of the file).
3. Hash each chunk with BLAKE3-256.
4. Walk the new build the same way.
5. Split new files into fixed 1 MiB chunks.
6. Count chunks whose hash does not exist in the old index.
7. Estimate transfer as zstd-3 of each new chunk (configurable).
8. Report unchanged / modified / new / deleted files.
9. Report the local rebuild I/O: every touched file is re-read and
   re-written in full, because a fixed-chunk updater builds the new
   file alongside the old one and commits at the end.
```

### Matching scope

- `--scope per-file` (default): a new chunk only matches chunks the
  *same path* had in the old build. This is the conservative,
  documented reading.
- `--scope global`: a new chunk matches any old chunk anywhere in the
  build — an optimistic upper bound that also credits moved files.

### Options

```sh
cavs bench steampipe-style ./Build_v1 ./Build_v2 \
  --chunk-size 1MiB \            # the documented size; any size accepted
  --hash blake3 \
  --compression zstd-3 \         # none | zstd-N
  --scope per-file \             # per-file | global
  --ignore-from .cavsignore \
  --json --markdown report.md --out results/
```

Both single artifacts and directories work; two single files are treated
as the same logical artifact even when their names differ
(`old.pck` vs `new.pck`).

## Why fixed chunks are sensitive to layout

A fixed-size chunker cuts at absolute offsets. Any of these turns
unchanged content into "new" chunks:

| Layout event | Effect under fixed 1 MiB chunks |
|---|---|
| 4 KiB inserted at the front | every later chunk boundary slides → ~0% reuse |
| assets reordered inside a pack | chunk contents shift → ~0% reuse |
| distributed TOC / absolute offsets rewritten | one dirty region per chunk → ~0% reuse |
| pack compressed as one stream | edits cascade beyond the edit point |
| 1 MiB-aligned assets, stable order | reuse survives edits perfectly |

This is exactly why Steam's own documentation recommends limiting pack
sizes, grouping assets by update cadence, avoiding timestamps and using
per-asset compression. `cavs analyze steampipe` detects these failure
modes and recommends the fix
([BUILD_UPDATE_ANALYZER.md](BUILD_UPDATE_ANALYZER.md)).

## Validation

The model's behavior across the classic update patterns is measured in
[docs/results/v0.9.0/steampipe-cases/](results/v0.9.0/steampipe-cases/)
(deterministic datasets, reproducible with
`cavs bench steampipe-cases`). Summary:
[STEAMPIPE_COMPARISON.md](STEAMPIPE_COMPARISON.md).
