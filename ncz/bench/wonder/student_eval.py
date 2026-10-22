#!/usr/bin/env python3
"""Teacher vs student decoder on the same latents (same file size): SSIMULACRA2 and PSNR, plus side-by-side crops.

    uv run --with numpy --with pillow --with ssimulacra2 python student_eval.py <kodak_dir> <teacher_dir> <student_dir> <q> <out_dir>
"""
import os, sys, tempfile
import numpy as np, ssimulacra2
from PIL import Image

kd, td, sd, q, od = sys.argv[1:6]; os.makedirs(od, exist_ok=True); tmp = tempfile.mkdtemp()
names = ["kodak_kodim" + t for t in ["01", "03", "05", "08", "13", "19", "21", "23"]]
def s2(a, b):
    Image.fromarray(a).save(f"{tmp}/r.png"); Image.fromarray(b).save(f"{tmp}/d.png")
    return float(ssimulacra2.compute_ssimulacra2(f"{tmp}/r.png", f"{tmp}/d.png"))
def psnr(a, b): return 10 * np.log10(255 ** 2 / ((a.astype(float) - b) ** 2).mean())
rows = []; tot = np.zeros(4)
print(f"{'photo':14s} {'teacher s2':>10s} {'student s2':>10s} {'teacher dB':>10s} {'student dB':>10s}")
for n in names:
    o = np.asarray(Image.open(f"{kd}/{n}.ppm").convert("RGB"))
    t = np.asarray(Image.open(f"{td}/{n}.q{q}.png").convert("RGB")); s = np.asarray(Image.open(f"{sd}/{n}.q{q}.png").convert("RGB"))
    v = np.array([s2(o, t), s2(o, s), psnr(o, t), psnr(o, s)]); tot += v
    print(f"{n:14s} {v[0]:10.2f} {v[1]:10.2f} {v[2]:10.2f} {v[3]:10.2f}")
    if n in ("kodak_kodim05", "kodak_kodim23", "kodak_kodim13"):
        H, W = o.shape[:2]; y0, x0 = H // 2 - 128, W // 2 - 192
        rows.append(np.concatenate([a[y0:y0 + 256, x0:x0 + 384] for a in (o, t, s)], 1))
v = tot / len(names)
print(f"{'mean':14s} {v[0]:10.2f} {v[1]:10.2f} {v[2]:10.2f} {v[3]:10.2f}")
Image.fromarray(np.concatenate(rows, 0)).resize((384 * 3 * 2, 256 * len(rows) * 2), Image.NEAREST).save(f"{od}/student_q{q}.png")
