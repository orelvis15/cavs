# The local disk I/O estimator — `cavs io-estimate` (v0.9.0)

Download size is not the only update cost. A fixed-chunk updater builds
each touched file alongside the old one and commits at the end — so a
25 GiB pack with 10 changed bytes still costs ~50 GiB of local reads
and writes. `cavs io-estimate` prices that per route, per device.

```sh
cavs io-estimate ./Build_v1 ./Build_v2 --out io-report.md
cavs io-estimate ./old ./new --device-profiles devices.toml --json
```

## Metrics per route

For full download, SteamPipe-style (fixed 1 MiB), CAVS chunks/hybrid
and CAVS `.cavsplan`:

- bytes downloaded, bytes read from the old build, bytes written;
- **temporary disk required** (the `.cavsplan` route stages per file,
  so its peak is the largest touched file, not the sum);
- file creates / renames / deletes, estimated seeks;
- estimated wall time per device profile;
- an `io_dominates_network` flag when local I/O exceeds the whole
  build — the signal that a small download will still feel slow.

## Device profiles

Defaults match the plan; override any set with `--device-profiles`:

```toml
[hdd]
sequential_read_mb_s = 120
sequential_write_mb_s = 100
seek_ms = 8

[sata_ssd]
sequential_read_mb_s = 500
sequential_write_mb_s = 450
seek_ms = 0.1

[nvme]
sequential_read_mb_s = 3500
sequential_write_mb_s = 2500
seek_ms = 0.02
```

Time model: reads/seq_read + (writes + download)/seq_write +
seeks × seek_ms. These are estimates for comparing layouts and routes,
not filesystem simulations.

## Measured example (benchmark F)

A ~3-byte change in a 256 MiB pack
([results/v0.9.0/io-estimate/](results/v0.9.0/io-estimate/)):

| Layout | Download | Read old | Write | HDD est. | NVMe est. |
|---|---:|---:|---:|---:|---:|
| one 256 MiB pack | 2.00 MiB | 256 MiB | 256 MiB | 4.7 s | 176 ms |
| split into 8 × 32 MiB | 2.00 MiB | 64 MiB | 64 MiB | 1.2 s | 45 ms |
| full download (reference) | 256 MiB | 0 | 256 MiB | 2.6 s | 102 ms |

Two lessons the flag makes explicit:

1. **The delta routes dominate on I/O, not network**: on an HDD, the
   2 MiB "smart" update of the monolithic pack (4.7 s) is *slower* than
   the dumb 256 MiB full download (2.6 s).
2. **Splitting the pack fixes it**: same download, 4× less I/O — the
   layout advice from
   [PACK_FILE_OPTIMIZATION.md](PACK_FILE_OPTIMIZATION.md) measured.
