#!/usr/bin/env python3
"""Export the learned mode's three parts for the browser.

    python export.py <weights.pth.tar> <pred.pt> <student.pt> <out_dir> <tag>
      <tag>.pred  predictor weights (our NCZP format, read by ncz::wonder::Predictor)
      <tag>.enc.onnx  the teacher's encoder (picture 0..1, sides multiples of 64 -> latents, not yet rounded)
      <tag>.dec.onnx  our student decoder (integer latents as float -> picture 0..1)
"""
import os, struct, sys
rest = sys.argv[1:]; sys.argv = sys.argv[:1]
exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "teacher.py")).read().split("wdir, kdir, out")[0])
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import torch, torch.nn as nn, torch.nn.functional as F
import student as stu
wpath, ppath, spath, od, tag = rest; os.makedirs(od, exist_ok=True)

# --- predictor -> NCZP
sd = torch.load(ppath, map_location="cpu"); order = sd["order"].tolist(); C = len(order)
import pred as pr
groups = pr.GROUPS[C]; wd = sd["nets.1.red.weight"].shape[0]
blob = bytearray(b"NCZP") + struct.pack("<HHH", C, len(groups), wd) + struct.pack(f"<{len(groups)}H", *groups) + struct.pack(f"<{C}H", *order)
def put(t): blob.extend(t.detach().float().contiguous().numpy().astype("<f4").tobytes())
for g in range(len(groups)):
    if g == 0: put(sd["nets.0.bias"].flatten())
    else:
        for k in ("red", "a", "b", "out"): put(sd[f"nets.{g}.{k}.weight"]); put(sd[f"nets.{g}.{k}.bias"])
open(f"{od}/{tag}.pred", "wb").write(blob); print("pred", len(blob), "bytes")

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
torch.onnx.export(T.g_a, x, f"{od}/{tag}.enc.onnx", input_names=["x"], output_names=["y"], dynamic_axes={"x": {2: "h", 3: "w"}, "y": {2: "lh", 3: "lw"}}, opset_version=17, dynamo=False)

# --- student decoder
ssd = torch.load(spath, map_location="cpu"); M = ssd["n.0.weight"].shape[1]; r = ssd["n.2.g"].shape[0] if "n.2.g" in ssd else 1
S = stu.Student(M, r=r); S.load_state_dict(ssd); S.eval()
class Dec(nn.Module):
    def __init__(s, S): super().__init__(); s.S = S
    def forward(s, y): return s.S(y).clamp(0, 1)
torch.onnx.export(Dec(S), torch.zeros(1, M, 8, 12), f"{od}/{tag}.dec.onnx", input_names=["y"], output_names=["x"], dynamic_axes={"y": {2: "lh", 3: "lw"}, "x": {2: "h", 3: "w"}}, opset_version=17, dynamo=False)
for f in ("enc", "dec"): print(f, os.path.getsize(f"{od}/{tag}.{f}.onnx"), "bytes")
