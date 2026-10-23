#!/bin/bash
# Build the browser files for one tiny-mode quality from rig checkpoints:
#   ship_q.sh <q> <teacher.pth.tar> <student.pt> <pred.pt> <out_dir>
# encoder: teacher g_a to ONNX, convs 8-bit (GDN fp16); decoder: student, 8-bit with the first conv fp16; predictor:
# NCZQ 8-bit. Each packed with ncz -> <out_dir>/q<q>.{enc.onnx,dec.onnx}.ncz, q<q>.pred.ncz (what worker.js fetches).
set -e
q=$1; T=$2; S=$3; PR=$4; O=$5; W=$(cd "$(dirname "$0")" && pwd); CLI=$W/../../target/release/nczcli; tmp=$(mktemp -d)
UV="uv run -q --with torch --with onnx --with numpy --with pillow --with compressai python -I"
$UV $W/enc_export.py $T $tmp/enc.f32.onnx
uv run -q --with onnx --with numpy python -I $W/quant_onnx.py $tmp/enc.f32.onnx $tmp/enc.onnx
KEEP16=n.0 QBITS=8 uv run -q --with torch --with onnx --with numpy --with pillow python -I $W/export4.py $S $tmp/dec.onnx
$UV $W/pred_export.py $PR $tmp/f32.pred
uv run -q --with numpy python -I $W/pred_q8.py $tmp/f32.pred $tmp/q.pred
mkdir -p $O
$CLI compress $tmp/enc.onnx $O/q$q.enc.onnx.ncz; $CLI compress $tmp/dec.onnx $O/q$q.dec.onnx.ncz; $CLI compress $tmp/q.pred $O/q$q.pred.ncz
ls -la $O/q$q.*
