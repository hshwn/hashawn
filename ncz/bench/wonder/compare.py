#!/usr/bin/env python3
"""Several decoders of the same latents (same file size) against the original: SSIMULACRA2, PSNR, side-by-side crops.

    uv run --with numpy --with pillow --with ssimulacra2 python compare.py <kodak_dir> <q> <out.png> <label>=<dir> ...
"""
import os, sys, tempfile
import numpy as np, ssimulacra2
from PIL import Image

kd, q, outp = sys.argv[1:4]; cols = [a.split("=", 1) for a in sys.argv[4:]]; tmp = tempfile.mkdtemp()
names = ["kodak_kodim" + t for t in ["01", "03", "05", "08", "13", "19", "21", "23"]]
def s2(a, b):
    Image.fromarray(a).save(f"{tmp}/r.png"); Image.fromarray(b).save(f"{tmp}/d.png")
    return float(ssimulacra2.compute_ssimulacra2(f"{tmp}/r.png", f"{tmp}/d.png"))
def psnr(a, b): return 10 * np.log10(255 ** 2 / ((a.astype(float) - b) ** 2).mean())
tot = {l: np.zeros(2) for l, _ in cols}; rows = []
print("photo          " + "".join(f"{l:>22s}" for l, _ in cols))
for n in names:
    o = np.asarray(Image.open(f"{kd}/{n}.ppm").convert("RGB")); ims = []; line = f"{n:14s} "
    for l, d in cols:
        a = np.asarray(Image.open(f"{d}/{n}.q{q}.png").convert("RGB")); ims.append(a)
        v = np.array([s2(o, a), psnr(o, a)]); tot[l] += v; line += f"{v[0]:12.2f} {v[1]:7.2f} dB"
    print(line)
    if n in ("kodak_kodim05", "kodak_kodim23", "kodak_kodim13"):
        H, W = o.shape[:2]; y0, x0 = H // 2 - 128, W // 2 - 144
        rows.append(np.concatenate([a[y0:y0 + 256, x0:x0 + 288] for a in [o] + ims], 1))
print("mean           " + "".join(f"{tot[l][0] / len(names):12.2f} {tot[l][1] / len(names):7.2f} dB" for l, _ in cols))
g = np.concatenate(rows, 0); Image.fromarray(g).resize((g.shape[1] * 2, g.shape[0] * 2), Image.NEAREST).save(outp)
