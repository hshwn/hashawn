#!/usr/bin/env python3
"""Decode cost of the student with 1, 2, 3 passes per block: parameters, operations per pixel, and time to decode a
768x512 picture on this machine's CPU (PyTorch, fp32, all cores; relative numbers matter, the browser build will differ).
    uv run --with torch python decode_speed.py"""
import os, sys, time, torch
sys.argv = sys.argv[:1]
import importlib.util
src = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "student.py")).read()
src = src[src.index("class Res(nn.Module):"):src.index("def setup(")]
import torch.nn as nn, torch.nn.functional as F
exec(src)
torch.set_num_threads(os.cpu_count())
def macs(net, M):
    tot = 0
    def hook(m, i, o):
        nonlocal tot
        if isinstance(m, nn.Conv2d): tot += o.shape[2] * o.shape[3] * m.in_channels * m.out_channels * m.kernel_size[0] * m.kernel_size[1]
    hs = [m.register_forward_hook(hook) for m in net.modules()]
    with torch.no_grad(): net(torch.zeros(1, M, 8, 8))
    for h in hs: h.remove()
    return tot / (128 * 128)
for r in (1, 2, 3):
    S = Student(192, r=r).eval(); y = torch.randn(1, 192, 32, 48)
    with torch.no_grad():
        for _ in range(2): S(y)
        t = time.time()
        for _ in range(5): S(y)
    print(f"passes {r}: {sum(p.numel() for p in S.parameters()) / 1e6:.3f}M params, {macs(S, 192) / 1e3:.0f}k MAC per pixel, {(time.time() - t) / 5 * 1000:.0f} ms per 768x512 picture (CPU)")
