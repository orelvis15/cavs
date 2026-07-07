#!/usr/bin/env bash
# Benchmark F — local disk I/O: a ~3-byte change in a 256 MiB pack,
# monolithic vs split into 8 parts. The pack comes from the seeded
# generator (`cavs bench gen --seed 5`), and the "new" version flips
# three fixed bytes — fully deterministic.
set -euo pipefail
: "${CAVS:?set CAVS to the cavs binary}"
: "${RESULTS:?set RESULTS to the output directory}"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
OUT="$RESULTS/io-estimate"
mkdir -p "$OUT"
cd "$WORK"

"$CAVS" bench gen --out dataset --size 256MiB --seed 5
mkdir -p old new old-split new-split
cp dataset/v1.bin old/world.pak
cp dataset/v1.bin new/world.pak
python3 - <<'EOF'
with open('new/world.pak', 'r+b') as f:
    for off in (100_000_000, 100_000_001, 200_000_000):
        f.seek(off); b = f.read(1)
        f.seek(off); f.write(bytes([b[0] ^ 0xFF]))
EOF
python3 - <<'EOF'
old = open('old/world.pak','rb').read()
new = open('new/world.pak','rb').read()
n = 8; size = len(old) // n
for i in range(n):
    open(f'old-split/level_{i:02}.pak','wb').write(old[i*size:(i+1)*size])
    open(f'new-split/level_{i:02}.pak','wb').write(new[i*size:(i+1)*size])
EOF

"$CAVS" io-estimate old new --out "$OUT/big-pack-tiny-change.md"
"$CAVS" io-estimate old-split new-split --out "$OUT/split-packs.md"
echo "== benchmark F written to $OUT"
