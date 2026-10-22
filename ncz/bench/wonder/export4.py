#!/usr/bin/env python3
"""Export a student decoder with 4-bit (or QBITS=8) weights for the browser (KEEP16=n.0,...: those layers stay fp16).

    python export4.py <student.pt> <out.onnx>        (then: nczcli compress out.onnx out.onnx.ncz)

Every QConv's weights go in as int8 holding the 16 levels -8..7, plus a per-output-channel scale, through
DequantizeLinear (ONNX Runtime folds it into float weights when the session loads). The levels are low-entropy bytes,
so ncz packs the file to ~4 bits a weight. The forward pass equals the QAT model's (same levels, same scales).
"""
import os, sys
import numpy as np, torch, torch.nn as nn, onnx
from onnx import numpy_helper, helper, TensorProto
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import student as stu

sd = torch.load(sys.argv[1], map_location="cpu"); M = sd["n.0.weight"].shape[1]
chans = (sd["n.0.weight"].shape[0],) + tuple(sd[f"n.{i}.0.weight"].shape[0] // 4 for i in (3, 4, 5, 6))
S = stu.Student(M, chans=chans); S.load_state_dict(sd); S.eval()
levels = {}
with torch.no_grad():
    for name, m in S.named_modules():
        if isinstance(m, stu.QConv) and name not in os.environ.get("KEEP16", "").split(","):     # KEEP16=n.0: layers left in fp16
            lo, hi = stu.QConv.levels(); sc = stu.QConv.scale(m.weight); k = torch.clamp(torch.round(m.weight / sc), lo, hi)
            m.weight.copy_(k * sc); levels["S." + name + ".weight"] = (k.to(torch.int8).numpy(), sc.flatten().numpy().astype(np.float32))
stu.QConv.qat = False
class Dec(nn.Module):
    def __init__(s, S): super().__init__(); s.S = S
    def forward(s, y): return s.S(y).clamp(0, 1)
tmp = sys.argv[2] + ".f32.onnx"
torch.onnx.export(Dec(S), torch.zeros(1, M, 8, 12), tmp, input_names=["y"], output_names=["x"], dynamic_axes={"y": {2: "lh", 3: "lw"}, "x": {2: "h", 3: "w"}}, opset_version=17, dynamo=False)
m = onnx.load(tmp); g = m.graph; inits, nodes, done = [], [], 0
for init in g.initializer:
    if init.name in levels:
        k, sc = levels[init.name]
        inits += [numpy_helper.from_array(k, init.name + "_q"), numpy_helper.from_array(sc, init.name + "_s"), numpy_helper.from_array(np.zeros(len(sc), np.int8), init.name + "_z")]
        nodes.append(helper.make_node("DequantizeLinear", [init.name + "_q", init.name + "_s", init.name + "_z"], [init.name], axis=0)); done += 1
    else:
        a = numpy_helper.to_array(init)
        if a.dtype == np.float32 and a.size > 16:          # biases and the last conv: fp16 storage + Cast
            inits.append(numpy_helper.from_array(a.astype(np.float16), init.name + "_h")); nodes.append(helper.make_node("Cast", [init.name + "_h"], [init.name], to=TensorProto.FLOAT))
        else: inits.append(init)
if done != len(levels): sys.exit(f"matched {done} of {len(levels)} quantised weights (initializer names differ)")
del g.initializer[:]; g.initializer.extend(inits); rest = list(g.node); del g.node[:]; g.node.extend(nodes + rest)
onnx.checker.check_model(m); onnx.save(m, sys.argv[2]); os.remove(tmp)
print(f"{done} {stu.QConv.bits}-bit convs, {sum(v[0].size for v in levels.values()) / 1e6:.2f}M levels, onnx {os.path.getsize(sys.argv[2])} bytes")
