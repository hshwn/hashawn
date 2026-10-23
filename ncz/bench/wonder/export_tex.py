#!/usr/bin/env python3
"""Export a texture decoder (texture2.py's MapStudent: latents + detail map) for the browser.

    python export_tex.py <student_map.pt> <out.onnx>      (then: nczcli compress out.onnx out.onnx.ncz)

Inputs: y (latents, 1 x M x h x w) and m (detail map, 1 x 1 x h/2 x w/2: log2 high-pass rms per 32 x 32 block, half
steps); a decoder trained with noise (texture2.py NOISE=2) also takes z (1 x 9 x 16h x 16w, standard normal per pixel:
channel 0 added after every upsampling stage, 1-8 mixed into the last two). Weights: 8-bit per output channel (DequantizeLinear), the first conv fp16 (8-bit hurts it), other floats fp16.
"""
import os, sys
import numpy as np, torch, torch.nn as nn, torch.nn.functional as F, onnx
from onnx import numpy_helper, helper, TensorProto
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
os.environ.setdefault("QBITS", "8")
import student as stu
sd = torch.load(sys.argv[1], map_location="cpu"); M = sd["S.n.0.weight"].shape[1] - 1
chans = (sd["S.n.0.weight"].shape[0],) + tuple(sd[f"S.n.{i}.0.weight"].shape[0] // 4 for i in (3, 4, 5, 6))
S = stu.Student(M, chans=chans)
S.n[0] = stu.QConv(M + 1, chans[0], 3, padding=1)
S.load_state_dict({k[2:]: v for k, v in sd.items() if k.startswith("S.")}); S.eval()
noise = "nmix.0.weight" in sd
if noise:
    ng = [sd[f"ng.{i}"] for i in range(4)]
    nmix = nn.ModuleList([nn.Conv2d(8, sd[f"nmix.{i}.weight"].shape[0], 1) for i in range(2)])
    nmix.load_state_dict({k[5:]: v for k, v in sd.items() if k.startswith("nmix.")})
levels = {}
with torch.no_grad():
    for name, m in S.named_modules():
        if isinstance(m, stu.QConv) and name != "n.0":
            lo, hi = stu.QConv.levels(); sc = stu.QConv.scale(m.weight); k = torch.clamp(torch.round(m.weight / sc), lo, hi)
            m.weight.copy_(k * sc); levels["S." + name + ".weight"] = (k.to(torch.int8).numpy(), sc.flatten().numpy().astype(np.float32))
stu.QConv.qat = False
class Dec(nn.Module):
    def __init__(s, S): super().__init__(); s.S = S
    def forward(s, y, m):
        mm = F.interpolate(m / 8, scale_factor=2.0, mode="nearest")
        return (s.S.n(torch.cat([y * s.S.inv, mm], 1)) + 0.5).clamp(0, 1)
class NoiseDec(nn.Module):
    """texture2.py MapStudent.forward with NOISE=2, the noise given as input z."""
    def __init__(s, S): super().__init__(); s.S = S; s.nmix = nmix; s.ng = nn.ParameterList([nn.Parameter(g) for g in ng])
    def forward(s, y, m, z):
        h = torch.cat([y * s.S.inv, F.interpolate(m / 8, scale_factor=2.0, mode="nearest")], 1)
        for i, layer in enumerate(s.S.n):
            if i in (5, 6):
                h = layer[2](layer[1](layer[0](h)))
                k = 2 ** (6 - i); n8 = z[:, 1:9]; n8 = F.avg_pool2d(n8, k) * k if k > 1 else n8
                h = layer[3](h + s.nmix[i - 5](n8))
            else: h = layer(h)
            if 3 <= i <= 6:
                k = 2 ** (6 - i); n = F.avg_pool2d(z[:, :1], k) * k if k > 1 else z[:, :1]
                h = h + s.ng[i - 3] * n
        return (h + 0.5).clamp(0, 1)
tmp = sys.argv[2] + ".f32.onnx"
if noise:
    torch.onnx.export(NoiseDec(S), (torch.zeros(1, M, 8, 12), torch.zeros(1, 1, 4, 6), torch.zeros(1, 9, 128, 192)), tmp, input_names=["y", "m", "z"], output_names=["x"],
                      dynamic_axes={"y": {2: "lh", 3: "lw"}, "m": {2: "mh", 3: "mw"}, "z": {2: "h", 3: "w"}, "x": {2: "h", 3: "w"}}, opset_version=17, dynamo=False)
else:
    torch.onnx.export(Dec(S), (torch.zeros(1, M, 8, 12), torch.zeros(1, 1, 4, 6)), tmp, input_names=["y", "m"], output_names=["x"],
                      dynamic_axes={"y": {2: "lh", 3: "lw"}, "m": {2: "mh", 3: "mw"}, "x": {2: "h", 3: "w"}}, opset_version=17, dynamo=False)
mdl = onnx.load(tmp); g = mdl.graph; inits, nodes, done = [], [], 0
for init in g.initializer:
    if init.name in levels:
        k, sc = levels[init.name]
        inits += [numpy_helper.from_array(k, init.name + "_q"), numpy_helper.from_array(sc, init.name + "_s"), numpy_helper.from_array(np.zeros(len(sc), np.int8), init.name + "_z")]
        nodes.append(helper.make_node("DequantizeLinear", [init.name + "_q", init.name + "_s", init.name + "_z"], [init.name], axis=0)); done += 1
    else:
        a = numpy_helper.to_array(init)
        if a.dtype == np.float32 and a.size > 16:
            inits.append(numpy_helper.from_array(a.astype(np.float16), init.name + "_h")); nodes.append(helper.make_node("Cast", [init.name + "_h"], [init.name], to=TensorProto.FLOAT))
        else: inits.append(init)
if done != len(levels): sys.exit(f"matched {done} of {len(levels)} quantised weights")
del g.initializer[:]; g.initializer.extend(inits); rest = list(g.node); del g.node[:]; g.node.extend(nodes + rest)
onnx.checker.check_model(mdl); onnx.save(mdl, sys.argv[2]); os.remove(tmp)
print(f"{done} 8-bit convs, onnx {os.path.getsize(sys.argv[2])} bytes")
