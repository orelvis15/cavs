#!/usr/bin/env bash
# Benchmark H — route planner policies across client states, on the
# split-pack pair from benchmark F (regenerated here deterministically).
set -euo pipefail
: "${CAVS:?set CAVS to the cavs binary}"
: "${RESULTS:?set RESULTS to the output directory}"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
OUT="$RESULTS/route-planner"
mkdir -p "$OUT"
cd "$WORK"

"$CAVS" bench gen --out dataset --size 256MiB --seed 5
mkdir -p old-split new-split
python3 - <<'EOF'
data = bytearray(open('dataset/v1.bin','rb').read())
new = bytearray(data)
for off in (100_000_000, 100_000_001, 200_000_000):
    new[off] ^= 0xFF
n = 8; size = len(data) // n
for i in range(n):
    open(f'old-split/level_{i:02}.pak','wb').write(data[i*size:(i+1)*size])
    open(f'new-split/level_{i:02}.pak','wb').write(new[i*size:(i+1)*size])
EOF

for state in "cold-install" "has-previous-install" \
             "warm-cache,has-previous-install" "has-previous-install,low-ram" \
             "has-previous-install,slow-hdd" "has-previous-install,low-disk"; do
  if [ "$state" = "cold-install" ]; then
    "$CAVS" plan-update --to new-split --client-state "$state" \
      --policy balanced --json > "$OUT/state-${state//,/+}.json"
  else
    "$CAVS" plan-update --from old-split --to new-split --client-state "$state" \
      --policy balanced --json > "$OUT/state-${state//,/+}.json"
  fi
done

python3 - "$OUT" <<'EOF'
import json, glob, sys
out = sys.argv[1]
rows = []
for f in sorted(glob.glob(out + '/state-*.json')):
    d = json.load(open(f))
    best = next(r for r in d['routes'] if r['route'] == d['chosen'])
    rows.append((d['client_state'] or 'cold-install', d['chosen'],
                 best['network_bytes'] / 1048576, d['reason']))
lines = ['# Route planner by client state (Benchmark H)', '',
         '| Client state | Recommended route | Network | Reason |',
         '|---|---|---:|---|']
for s, c, n, r in rows:
    lines.append(f'| {s} | {c} | {n:.2f} MiB | {r} |')
open(out + '/policies.md', 'w').write('\n'.join(lines) + '\n')
EOF
echo "== benchmark H written to $OUT"
