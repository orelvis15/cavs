# Local Disk I/O Estimate

> I/O figures are estimates from the update model; device times assume sequential throughput plus per-seek latency.

`benchF/old-split` → `benchF/new-split`

| Route | Download | Read old | Write | Temp required | Creates | Renames | Deletes | hdd est. | nvme est. | sata_ssd est. |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| full download (raw) | 256.00 MiB | 0 B | 0 B | 256.00 MiB | 8 | 8 | 0 | 2.6s | 103ms | 570ms |
| SteamPipe-style (fixed 1 MiB) | 2.00 MiB | 64.00 MiB | 64.00 MiB | 64.00 MiB | 0 | 2 | 0 | 1.2s | 45ms | 275ms |
| CAVS chunks / hybrid | 193.76 KiB | 64.00 MiB | 64.00 MiB | 64.00 MiB | 0 | 2 | 0 | 1.2s | 44ms | 271ms |
| CAVS .cavsplan | 193.76 KiB | 64.00 MiB | 64.00 MiB | 32.00 MiB | 0 | 2 | 0 | 1.2s | 44ms | 271ms |
