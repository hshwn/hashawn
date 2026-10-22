#!/usr/bin/env python3
"""A teacher's rounded latents of pictures (.lat for nczcli wonder) and its own decoder's reconstruction of them.
    TEACHER=cheng|hpcm python teacher_lat.py <weights> <kodak_dir> <out_dir> <names...>   (prints PSNR per picture)"""
import os, sys
import numpy as np, torch
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
rest = sys.argv[1:]; sys.argv = sys.argv[:1]
import pred
from PIL import Image
wpath, kd, od, names = rest[0], rest[1], rest[2], rest[3:]; os.makedirs(od, exist_ok=True)
T = pred.teacher(wpath); ps = []
for n in names:
    o = np.asarray(Image.open(f"{kd}/{n}.ppm").convert("RGB"))
    x = torch.from_numpy(o.astype(np.float32) / 255).permute(2, 0, 1)[None].to(pred.dev)
    with torch.no_grad():
        y = torch.round(T.g_a(x)); xh = T.g_s(y).clamp(0, 1)
    lat = y[0].to(torch.int16).cpu().numpy()
    open(f"{od}/{n}.lat", "wb").write(np.array(lat.shape, np.uint16).tobytes() + lat.tobytes())
    r = (xh[0].permute(1, 2, 0).cpu().numpy() * 255 + 0.5).astype(np.uint8); Image.fromarray(r).save(f"{od}/{n}.png")
    p = 10 * np.log10(255 ** 2 / ((r.astype(float) - o) ** 2).mean()); ps.append(p); print(n, lat.shape, f"{p:.2f} dB", flush=True)
print(f"mean {np.mean(ps):.2f} dB")
