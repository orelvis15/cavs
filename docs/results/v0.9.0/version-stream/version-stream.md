# Many-version release stream

10 versions of a 24.00 MiB build; ~3% of blocks change per release.

| Method | Storage | Adjacent updates | v1→vN jump | Any-pair coverage |
|---|---:|---:|---:|---|
| CAVS packfile store | 22.43 MiB (10 packfiles) | 9.75 MiB total | 6.58 MiB | every pair, same objects |
| bsdiff patches | 3.10 MiB (9 adjacent patches) + full artifacts | 3.10 MiB total | 2.78 MiB (dedicated patch) | needs 45 patches (O(N²)) or chain-apply |

CAVS jump v3→v10: 5.90 MiB. Adjacent per release (CAVS): 1.25 MiB, 1019.13 KiB, 1.06 MiB, 1.08 MiB, 978.11 KiB, 1.08 MiB, 1.16 MiB, 1.19 MiB, 996.07 KiB.
