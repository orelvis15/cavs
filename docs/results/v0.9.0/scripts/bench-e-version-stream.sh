#!/usr/bin/env bash
# Benchmark E — many-version branch stream: 10 releases with ~3% drift
# (seed 5), CAVS content-addressed store vs pairwise patch storage.
set -euo pipefail
: "${CAVS:?set CAVS to the cavs binary}"
: "${RESULTS:?set RESULTS to the output directory}"

"$CAVS" bench version-stream --out "$RESULTS/version-stream" \
  --size 24MiB --versions 10 --seed 5
# Keep the summaries; the chunk store payload is reproducible on demand.
rm -rf "$RESULTS/version-stream/store"
echo "== benchmark E written to $RESULTS/version-stream"
