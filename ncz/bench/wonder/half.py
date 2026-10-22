#!/usr/bin/env python3
"""Store an ONNX model's float weights as fp16 (half the download); a Cast back to fp32 follows each one, so every
operation still runs in fp32 on every backend.   uv run --with onnx --with numpy python half.py <in.onnx> <out.onnx>"""
import sys, onnx, numpy as np
from onnx import numpy_helper, helper, TensorProto
m = onnx.load(sys.argv[1]); g = m.graph; new_inits = []; casts = []
for init in g.initializer:
    a = numpy_helper.to_array(init)
    if a.dtype == np.float32 and a.size > 16:
        h = numpy_helper.from_array(a.astype(np.float16), init.name + "_f16"); new_inits.append(h)
        casts.append(helper.make_node("Cast", [h.name], [init.name], to=TensorProto.FLOAT))
    else: new_inits.append(init)
del g.initializer[:]; g.initializer.extend(new_inits)
nodes = list(g.node); del g.node[:]; g.node.extend(casts + nodes)
onnx.checker.check_model(m); onnx.save(m, sys.argv[2])
