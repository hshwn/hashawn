#!/usr/bin/env python3
"""Tiny mode against JPEG XL, WebP and AVIF at the same file size, Kodak 8.

    uv run --with numpy --with pillow --with ssimulacra2 --with imagecodecs python scoreboard.py <kodak_dir> <tiny_dir> <q> <out.png>

<tiny_dir> holds what the browser tool wrote: <nn>.q<q>.nczw (the file), <nn>.q<q>.png (unpacked smooth) and
<nn>.q<q>sw.png (unpacked textured). Each other codec gets the largest quality whose file is at most the tiny file's
size (binary search on its quality knob). Scores: SSIMULACRA2 (higher is better, ~70 = high quality, ~30 = low) and PSNR.
"""
import os, sys, tempfile
import numpy as np, ssimulacra2, imagecodecs
from PIL import Image

kd, td, q, outp = sys.argv[1:5]; tmp = tempfile.mkdtemp()
NAMES = ["01", "03", "05", "08", "13", "19", "21", "23"]
def s2(a, b):
    Image.fromarray(a).save(f"{tmp}/r.png"); Image.fromarray(b).save(f"{tmp}/d.png")
    return float(ssimulacra2.compute_ssimulacra2(f"{tmp}/r.png", f"{tmp}/d.png"))
def psnr(a, b): return 10 * np.log10(255 ** 2 / ((a.astype(float) - b) ** 2).mean())

def fit(enc, dec, lo, hi, budget, higher_is_better=True, iters=14, integer=False):
    """Largest-quality setting whose output fits the budget: (bytes, decoded) or None."""
    best = None
    for _ in range(iters):
        mid = (lo + hi) / 2; mid = round(mid) if integer else mid
        b = enc(mid)
        if len(b) <= budget:
            best = (len(b), dec(b))
            if higher_is_better: lo = mid
            else: hi = mid
        else:
            if higher_is_better: hi = mid
            else: lo = mid
        if integer and hi - lo <= 1: break
    return best

codecs = {
    "JPEG XL": lambda o, n: fit(lambda d: imagecodecs.jpegxl_encode(o, distance=d, effort=7), imagecodecs.jpegxl_decode, 0.1, 25.0, n, higher_is_better=False),
    "WebP": lambda o, n: fit(lambda k: imagecodecs.webp_encode(o, level=k, method=6), imagecodecs.webp_decode, 0, 100, n, integer=True),
    "AVIF": lambda o, n: fit(lambda k: imagecodecs.avif_encode(o, level=k, speed=4), imagecodecs.avif_decode, 0, 100, n, integer=True),
}
cols = ["tiny textured", "tiny smooth"] + list(codecs)
tot = {c: np.zeros(3) for c in cols}; rows = []
print(f"{'photo':8s} {'bytes':>7s} " + "".join(f"{c:>24s}" for c in cols) + "   (SSIMULACRA2 / PSNR)")
for n in NAMES:
    o = np.asarray(Image.open(f"{kd}/kodak_kodim{n}.ppm").convert("RGB")); budget = os.path.getsize(f"{td}/{n}.q{q}.nczw")
    ims = {"tiny textured": (budget, np.asarray(Image.open(f"{td}/{n}.q{q}sw.png").convert("RGB"))),
           "tiny smooth": (budget, np.asarray(Image.open(f"{td}/{n}.q{q}.png").convert("RGB")))}
    for c, f in codecs.items():
        r = f(o, budget); ims[c] = (r[0], r[1][..., :3]) if r else None
    line = f"{n:8s} {budget:7d} "
    for c in cols:
        if ims[c] is None: line += f"{'(does not fit)':>24s}"; continue
        v = np.array([s2(o, ims[c][1]), psnr(o, ims[c][1]), ims[c][0]]); tot[c] += v; line += f"{v[0]:9.1f} /{v[1]:6.2f} ({v[2]:6.0f})"
    print(line, flush=True)
    if n in ("13", "08", "05"):
        H, W = o.shape[:2]; y0, x0 = (40, 60) if n == "13" else (H // 2 - 100, W // 2 - 150)
        # columns: original, tiny textured, JPEG XL, AVIF (grey where a codec cannot get that small)
        rows.append(np.concatenate([a[y0:y0 + 200, x0:x0 + 300] for a in [o] + [ims[c][1] if ims[c] is not None else np.full_like(o, 128) for c in ("tiny textured", "JPEG XL", "AVIF")]], 1))
print(f"{'mean':8s} {'':7s} " + "".join(f"{tot[c][0] / len(NAMES):9.1f} /{tot[c][1] / len(NAMES):6.2f} ({tot[c][2] / len(NAMES):6.0f})" for c in cols))
g = np.concatenate(rows, 0); Image.fromarray(g).resize((g.shape[1] * 2, g.shape[0] * 2), Image.NEAREST).save(outp)
