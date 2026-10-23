#!/usr/bin/env python3
"""Export a texture decoder (texture2.py's MapStudent: latents + detail map) for the browser.

    python export_tex.py <student_map.pt> <out.onnx>      (then: nczcli compress out.onnx out.onnx.ncz)

Inputs: y (latents, 1 x M x h x w) and m (detail map, 1 x 1 x h/2 x w/2: log2 high-pass rms per 32 x 32 block, half
steps). Weights: 8-bit per output channel (DequantizeLinear), the first conv fp16 (8-bit hurts it), other floats fp16.
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
S.load_state_dict({k[2:]: v for k, v in sd.items()}); S.eval()
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
tmp = sys.argv[2] + ".f32.onnx"
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
