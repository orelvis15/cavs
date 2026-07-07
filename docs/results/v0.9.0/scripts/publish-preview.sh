#!/usr/bin/env bash
# Phase-4 deliverable — publish-preview across every route on a
# deterministic directory pair (seed 7). butler and bsdiff/xdelta3 are
# probed and skipped with a warning when not installed.
set -euo pipefail
: "${CAVS:?set CAVS to the cavs binary}"
: "${RESULTS:?set RESULTS to the output directory}"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
cd "$WORK"

"$CAVS" bench gen-dir --out content --size 48MiB --seed 7
"$CAVS" publish-preview content/Build_v2 --previous content/Build_v1 \
  --routes all --out "$RESULTS/publish-preview"
rm -rf "$RESULTS/publish-preview/routes-work"
echo "== publish-preview written to $RESULTS/publish-preview"
