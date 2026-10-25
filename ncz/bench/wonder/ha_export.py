#!/usr/bin/env python3
"""The teacher's hyperprior encoder h_a (latents y -> z) to ONNX, for tiny mode's ported entropy model (ncz tm.rs).

    python ha_export.py <teacher.pth.tar> <out.onnx>      (then quant_onnx.py for 8-bit weights, nczcli compress)

Input y (1 x N x h x w, float, h and w multiples of 4), output z (1 x N x h/4 x w/4, float). Encoder side only: z is
rounded around the medians in wasm, so small numeric differences between machines only change which file is written.
"""
import os, sys
rest = sys.argv[1:]; sys.argv = sys.argv[:1]
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pred
pred.dev = "cpu"
import torch
from pred import teacher

T = teacher(rest[0]); N = T.h_a[0].in_channels
torch.onnx.export(T.h_a, (torch.zeros(1, N, 8, 12),), rest[1], input_names=["y"], output_names=["z"],
                  dynamic_axes={"y": {2: "h", 3: "w"}, "z": {2: "hz", 3: "wz"}}, opset_version=17, dynamo=False)
print(rest[1], os.path.getsize(rest[1]), "bytes")
