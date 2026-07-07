# Local Disk I/O Estimate

> I/O figures are estimates from the update model; device times assume sequential throughput plus per-seek latency.

`benchF/old` → `benchF/new`

| Route | Download | Read old | Write | Temp required | Creates | Renames | Deletes | hdd est. | nvme est. | sata_ssd est. |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| full download (raw) | 256.00 MiB | 0 B | 0 B | 256.00 MiB | 1 | 1 | 0 | 2.6s | 102ms | 569ms |
| SteamPipe-style (fixed 1 MiB) | 2.00 MiB | 256.00 MiB | 256.00 MiB | 256.00 MiB | 0 | 1 | 0 | 4.7s | 176ms | 1.1s |
| CAVS chunks / hybrid | 193.76 KiB | 256.00 MiB | 256.00 MiB | 256.00 MiB | 0 | 1 | 0 | 4.7s | 176ms | 1.1s |
| CAVS .cavsplan | 193.76 KiB | 256.00 MiB | 256.00 MiB | 256.00 MiB | 0 | 1 | 0 | 4.7s | 176ms | 1.1s |

> **SteamPipe-style (fixed 1 MiB)**: local I/O (256.00 MiB read + 256.00 MiB write) exceeds the whole build — the network saving does not translate into a faster update on slow disks.

> **CAVS chunks / hybrid**: local I/O (256.00 MiB read + 256.00 MiB write) exceeds the whole build — the network saving does not translate into a faster update on slow disks.

> **CAVS .cavsplan**: local I/O (256.00 MiB read + 256.00 MiB write) exceeds the whole build — the network saving does not translate into a faster update on slow disks.
