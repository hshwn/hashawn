#!/usr/bin/env python3
"""Export only the teacher's encoder to ONNX (GDN parameters precomputed).   python enc_export.py <weights.pth.tar> <out.onnx>"""
import os, sys
rest = sys.argv[1:]; sys.argv = sys.argv[:1]
exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "teacher.py")).read().split("wdir, kdir, out")[0])
import torch, torch.nn as nn, torch.nn.functional as F
wpath, outp = rest
# --- encoder: GDN with its parameters precomputed (the training-time reparametrisation does not export)
T = Cheng2020Anchor.from_state_dict(load_state_dict(torch.load(wpath, map_location="cpu"))).eval()
from compressai.layers import GDN
class PlainGDN(nn.Module):
    def __init__(s, g):
        super().__init__(); beta = g.beta_reparam(g.beta).detach(); gamma = g.gamma_reparam(g.gamma).detach()
        s.inverse = g.inverse; s.register_buffer("beta", beta); s.register_buffer("gamma", gamma[:, :, None, None])
    def forward(s, x):
        n = F.conv2d(x * x, s.gamma, s.beta)
        return x * torch.sqrt(n) if s.inverse else x * torch.rsqrt(n)
def swap(m):
    for name, ch in m.named_children():
        if isinstance(ch, GDN): setattr(m, name, PlainGDN(ch))
        else: swap(ch)
swap(T.g_a)
x = torch.rand(1, 3, 128, 192)
with torch.no_grad(): ref = Cheng2020Anchor.from_state_dict(load_state_dict(torch.load(wpath, map_location="cpu"))).eval().g_a(x); got = T.g_a(x)
print("encoder plain-GDN max diff", float((ref - got).abs().max()))
torch.onnx.export(T.g_a, x, outp, input_names=["x"], output_names=["y"], dynamic_axes={"x": {2: "h", 3: "w"}, "y": {2: "lh", 3: "lw"}}, opset_version=17, dynamo=False)

print("enc", os.path.getsize(outp), "bytes")
