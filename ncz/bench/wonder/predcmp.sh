#!/bin/bash
# Latent bytes on Kodak 8 with two predictors, through the real coder (8-bit predictor, as the page ships it):
#   predcmp.sh <a_pred.pt> <b_pred.pt> <lat_dir> <q>
# <lat_dir>/kodak_kodimNN.q<q>.lat: the teacher's integer latents (teacher.py). Prints bytes per photo and totals.
set -e
W=$(cd "$(dirname "$0")" && pwd); CLI=$W/../../target/release/nczcli; tmp=$(mktemp -d)
for k in a b; do
  pt=$1; [ $k = b ] && pt=$2
  uv run -q --with torch --with numpy python -I $W/pred_export.py $pt $tmp/$k.f32.pred >/dev/null
  uv run -q --with numpy python -I $W/pred_q8.py $tmp/$k.f32.pred $tmp/$k.pred >/dev/null
  $CLI wonder $tmp/$k.pred $3/kodak_kodim{01,03,05,08,13,19,21,23}.q$4.lat > $tmp/$k.txt
done
paste <(awk '{print $1, $2, $NF}' $tmp/a.txt) <(awk '{print $2, $NF}' $tmp/b.txt) | awk '{a+=$2; b+=$4; printf "%-22s %7d %7d  (%+.1f%%)  %s %s\n", $1, $2, $4, ($4/$2-1)*100, $3, $5} END {printf "total %22d %7d  (%+.1f%%)\n", a, b, (b/a-1)*100}'
