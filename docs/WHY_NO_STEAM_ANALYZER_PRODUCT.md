# Why there is no separate steam-analyzer product

CAVS will not ship a separate `steam-analyzer` product.

Earlier iterations of this repository carried a standalone
`steam-analyzer/` crate with its own binary (`cavs-steam`). As of
v0.9.0 that crate is gone, and its capabilities — grown considerably —
live inside the `cavs` CLI as a command family:

```sh
cavs analyze steampipe ./Build_v1 ./Build_v2
cavs bench steampipe-style ./Build_v1 ./Build_v2
cavs publish-preview ./Build_v2 --previous ./Build_v1 --routes all
cavs analyze-packs ./Build_v1 ./Build_v2
cavs io-estimate ./Build_v1 ./Build_v2
```

## Reasons

- **avoid branding/trademark confusion** — a product named after Steam
  risks looking like an official Valve tool. A CAVS command family
  cannot be mistaken for one;
- **avoid splitting the project** — a separate product means separate
  branding, documentation, repo maintenance and marketing burden, and a
  weaker connection to CAVS;
- **keep all benchmarking and analysis under CAVS** — the analyzer
  shares its chunking, hashing, plan and benchmark infrastructure with
  the rest of the toolkit; duplicating that logic in a second product
  would fork it;
- **make SteamPipe-style analysis one feature of a broader build-update
  toolkit** — update-cost estimation, pack diagnostics, route planning,
  I/O estimation and the workspace model are one story, not five tools.

## Positioning

The SteamPipe-style analyzer is:

> A CAVS command family for estimating and diagnosing SteamPipe-like
> update behavior.

It is **not** a Steam product, a Valve-compatible tool, an official
SteamPipe implementation, or a SteamPipe replacement. Allowed naming:
*SteamPipe-style*, *SteamPipe-inspired*,
*SteamPipe update model estimate* — always as an estimated model,
never as real SteamPipe compatibility. See
[STEAMPIPE_STYLE_MODEL.md](STEAMPIPE_STYLE_MODEL.md) for what the model
does and does not claim.
