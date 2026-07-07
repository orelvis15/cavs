#!/usr/bin/env bash
# Benchmarks A (model validation), B (pack pathology), C (directory vs
# blob) and G (Godot PCK): 12 deterministic layouts under the
# SteamPipe-style model, real .cavsplans, and bsdiff/xdelta3/butler
# when installed. Seed 9 reproduces the published tables exactly.
set -euo pipefail
: "${CAVS:?set CAVS to the cavs binary}"
: "${RESULTS:?set RESULTS to the output directory}"

ARGS=(--out "$RESULTS/steampipe-cases" --assets 32 --seed 9 --include-pairwise)
if [ -n "${BUTLER:-}" ]; then
  ARGS+=(--butler-bin "$BUTLER")
fi
"$CAVS" bench steampipe-cases "${ARGS[@]}"
