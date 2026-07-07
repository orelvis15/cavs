#!/usr/bin/env bash
# Benchmark D — depot sharing and install plans by ownership.
#
# Builds a workspace whose depots share content deterministically:
# windows/linux share the same generated build plus a small platform
# binary each; demo is a subset of the build; lang-es and hd-textures
# are unique. All content comes from `cavs bench gen-dir` (seeded) and
# fixed byte patterns — no /dev/urandom, so reruns are identical.
set -euo pipefail
: "${CAVS:?set CAVS to the cavs binary}"
: "${RESULTS:?set RESULTS to the output directory}"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
OUT="$RESULTS/depot-sharing"
mkdir -p "$OUT"
cd "$WORK"

# Deterministic base content (seed 7).
"$CAVS" bench gen-dir --out content --size 48MiB --seed 7

mkdir -p depots/windows depots/linux depots/demo depots/lang-es depots/hd
cp -R content/Build_v1/. depots/windows/
cp -R content/Build_v1/. depots/linux/
# Small deterministic platform binaries.
head -c 300000 /dev/zero | tr '\0' 'W' > depots/windows/win64.exe
head -c 260000 /dev/zero | tr '\0' 'L' > depots/linux/game.x86_64
# Demo = every third file of the build.
i=0
find content/Build_v1 -type f | sort | while read -r f; do
  i=$((i+1))
  if [ $((i % 3)) = 0 ]; then
    rel="${f#content/Build_v1/}"
    mkdir -p "depots/demo/$(dirname "$rel")"
    cp "$f" "depots/demo/$rel"
  fi
done
# Unique depots: deterministic patterned payloads.
head -c 4000000 /dev/zero | tr '\0' 'E' > depots/lang-es/ui_text_es.bin
head -c 9000000 /dev/zero | tr '\0' 'H' > depots/hd/textures_hd.bin

"$CAVS" workspace init ws --app my-game
"$CAVS" depot add windows --workspace ws --platform windows
"$CAVS" depot add linux --workspace ws --platform linux
"$CAVS" depot add demo --workspace ws --optional
"$CAVS" depot add lang-es --workspace ws --language es
"$CAVS" depot add hd-textures --workspace ws --optional
"$CAVS" branch add public --workspace ws
"$CAVS" build create --workspace ws --branch public \
  --depot windows=depots/windows --depot linux=depots/linux \
  --depot demo=depots/demo --depot lang-es=depots/lang-es \
  --depot hd-textures=depots/hd --label v1

"$CAVS" depot analyze-sharing --workspace ws --out "$OUT/sharing.md"
"$CAVS" depot analyze-sharing --workspace ws --json > "$OUT/sharing.json"
"$CAVS" install-plan --workspace ws --branch public --platform windows \
  --json > "$OUT/install-windows-base.json"
"$CAVS" install-plan --workspace ws --branch public --platform windows \
  --language es --owned windows,lang-es,hd-textures \
  --json > "$OUT/install-windows-full.json"
"$CAVS" install-plan --workspace ws --branch public --platform linux \
  --json > "$OUT/install-linux.json"
"$CAVS" install-plan --workspace ws --branch public --platform windows \
  --owned windows,demo --from build_1001 \
  --json > "$OUT/install-demo-after-full.json"
echo "== benchmark D written to $OUT"
