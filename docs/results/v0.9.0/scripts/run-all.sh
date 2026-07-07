#!/usr/bin/env bash
# Run the whole v0.9.0 benchmark suite (plan benchmarks A-H).
#
# Usage:   ./run-all.sh [RESULTS_DIR]
# Env:     CAVS=path/to/cavs (default: target/release/cavs from repo root)
#          BUTLER=path/to/butler (optional; skipped when absent)
#
# Every dataset is deterministic (fixed seeds); reports embed the
# environment (OS, CPU, RAM, disk, tool versions, command, seed).
set -euo pipefail
cd "$(dirname "$0")"

export RESULTS="${1:-$(pwd)/out}"
export CAVS="${CAVS:-$(cd ../../../.. && pwd)/target/release/cavs}"
mkdir -p "$RESULTS"

echo "== cavs: $CAVS"
echo "== results: $RESULTS"
"$CAVS" --version

./bench-abcg-steampipe-cases.sh
./bench-d-depot-sharing.sh
./bench-e-version-stream.sh
./bench-f-io-estimate.sh
./bench-h-route-planner.sh
./publish-preview.sh

echo "== done: $RESULTS"
