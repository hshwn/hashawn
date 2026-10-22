#!/usr/bin/env python3
"""Store an ONNX model's convolution weights as 8-bit (per output channel, symmetric, through DequantizeLinear) and its
other float weights as fp16 (+ Cast). For the encoder: GDN parameters (names with gamma/beta) stay fp16.

    uv run --with onnx --with numpy python quant_onnx.py <in.onnx> <out.onnx> [bits]
"""
import sys, numpy as np, onnx
from onnx import numpy_helper, helper, TensorProto
src, dst = sys.argv[1], sys.argv[2]; bits = int(sys.argv[3]) if len(sys.argv) > 3 else 8; L = 2 ** (bits - 1) - 1
m = onnx.load(src); g = m.graph
conv_w = {n.input[1] for n in g.node if n.op_type == "Conv" and len(n.input) > 1}
inits, nodes, nq = [], [], 0
for init in g.initializer:
    a = numpy_helper.to_array(init)
    if init.name in conv_w and a.ndim == 4 and a.dtype == np.float32 and "gamma" not in init.name and "beta" not in init.name and a.size > 64:
        sc = np.maximum(np.abs(a).reshape(a.shape[0], -1).max(1), 1e-8) / L
        k = np.clip(np.round(a / sc[:, None, None, None]), -L - 1, L).astype(np.int8)
        inits += [numpy_helper.from_array(k, init.name + "_q"), numpy_helper.from_array(sc.astype(np.float32), init.name + "_s"), numpy_helper.from_array(np.zeros(len(sc), np.int8), init.name + "_z")]
        nodes.append(helper.make_node("DequantizeLinear", [init.name + "_q", init.name + "_s", init.name + "_z"], [init.name], axis=0)); nq += 1
    elif a.dtype == np.float32 and a.size > 16:
        inits.append(numpy_helper.from_array(a.astype(np.float16), init.name + "_h")); nodes.append(helper.make_node("Cast", [init.name + "_h"], [init.name], to=TensorProto.FLOAT))
    else: inits.append(init)
del g.initializer[:]; g.initializer.extend(inits); rest = list(g.node); del g.node[:]; g.node.extend(nodes + rest)
onnx.checker.check_model(m); onnx.save(m, dst); print(f"{nq} convs at {bits} bits")
