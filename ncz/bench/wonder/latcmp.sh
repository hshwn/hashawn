#!/bin/bash
# latcmp.sh <tout_dir> [qualities]: our latent coder on the 8 test photos, cold and primed on the other 16 Kodak photos
S=$1; QS=${2:-"1 2 3 4 5 6"}; B=$(dirname "$0")/../../target/release/nczcli
TEST=(01 03 05 08 13 19 21 23); TRAIN=(02 04 06 07 09 10 11 12 14 15 16 17 18 20 22 24)
for q in $QS; do
  P=""; for t in "${TRAIN[@]}"; do P="$P $S/kodak_kodim$t.q$q.lat"; done
  F=(); for t in "${TEST[@]}"; do F+=("$S/kodak_kodim$t.q$q.lat"); done
  cold=$("$B" lat "${F[@]}" | awk '{s+=$4; if($NF!="exact")bad=1} END{printf "%.4f%s", s/NR, bad?" MISMATCH":""}')
  prim=$(LAT_PRIME="$P" "$B" lat "${F[@]}" | awk '{s+=$4; if($NF!="exact")bad=1} END{printf "%.4f%s", s/NR, bad?" MISMATCH":""}')
  own=$(python3 -c "import json; r=json.load(open('$S/eval.json')); print(round(sum(r[n]['$q']['own_bpp'] for n in r)/len(r),4))")
  echo "q$q  teacher $own  cold $cold  primed $prim"
done
