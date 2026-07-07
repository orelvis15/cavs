# Route planner by client state (Benchmark H)

| Client state | Recommended route | Network | Reason |
|---|---|---:|---|
| cold-install | full download | 256.00 MiB | 256.00 MiB over the wire · ~16.00 MiB peak RAM · 0 B temp disk · policy balanced |
| has-previous-install,low-disk | .cavsplan | 0.13 MiB | 128.81 KiB over the wire · ~40.00 MiB peak RAM · 64.00 MiB temp disk · policy balanced |
| has-previous-install,low-ram | .cavsplan | 0.13 MiB | 128.81 KiB over the wire · ~40.00 MiB peak RAM · 64.00 MiB temp disk · policy balanced |
| has-previous-install,slow-hdd | .cavsplan | 0.13 MiB | 128.81 KiB over the wire · ~40.00 MiB peak RAM · 64.00 MiB temp disk · policy balanced |
| has-previous-install | .cavsplan | 0.13 MiB | 128.81 KiB over the wire · ~40.00 MiB peak RAM · 64.00 MiB temp disk · policy balanced |
| warm-cache,has-previous-install | .cavsplan | 0.13 MiB | 128.81 KiB over the wire · ~40.00 MiB peak RAM · 64.00 MiB temp disk · policy balanced |
