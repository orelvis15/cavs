# Publish Preview

> SteamPipe-style estimate based on public documentation. This is not Valve's exact SteamPipe implementation.

`content/Build_v1` → `content/Build_v2`

| Route | Network | Build/Diff | Apply | Peak RSS | Verified | Notes |
|---|---:|---:|---:|---:|---|---|
| SteamPipe-style (fixed 1 MiB) | 5.77 MiB | — | — | — | model | 15 chunks; ~59.96 MiB local rebuild I/O |
| full download (raw) | 49.26 MiB | — | 0 ms | — | yes | no old-version reuse |
| full zstd-19 (CAVS bootstrap) | 23.28 MiB | 1221 ms | 8 ms | — | yes | cache-less first install |
| CAVS chunk / hybrid (wire) | 2.08 MiB | 102 ms | — | — | yes | 37 of 437 chunks new; same bytes for warm cache or cold cache + previous install (hybrid) |
| CAVS offline plan (.cavsplan) | 1.01 MiB | 175 ms | 125 ms | — | yes | portable patch: signature diff + zstd-19 payload, journaled apply |
| pairwise proxy: bsdiff+zstd-19 | 1.20 MiB | 10355 ms | 3303 ms | 556 MiB | yes | one exact old→new pair only (proxy) |
| pairwise proxy: bsdiff+brotli-9 | 1.20 MiB | 10355 ms | 3303 ms | 556 MiB | yes | one exact old→new pair only (proxy) |
| pairwise proxy: xdelta3+zstd-19 | 1.19 MiB | 3780 ms | 3644 ms | 233 MiB | yes | one exact old→new pair only (proxy) |
| pairwise proxy: xdelta3+brotli-9 | 1.19 MiB | 3780 ms | 3644 ms | 233 MiB | yes | one exact old→new pair only (proxy) |

## Decision summary

Recommended route:
  **CAVS offline plan (.cavsplan)**

Why:
  lowest verified network payload (1.01 MiB) · streaming apply with bounded memory · no pairwise O(N²) patch explosion · byte-identical output verified

## Release-readiness warnings

- route skipped: butler offline: CAVS-E-BUTLER-NOT-FOUND: cannot execute "butler"; pass --butler-bin or install butler
- route skipped: pairwise patches serve exactly one old→new pair; storage and generation cost grow with every published pair
- [critical] Assets shifted or reordered inside the file (game.pck) — Keep a stable asset order, pad or align entries so unrelated assets keep their offsets, and avoid full repacks for small changes.
