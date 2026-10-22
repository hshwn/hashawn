#!/usr/bin/env python3
"""Shading side-information for `nczcli shade-bench`: 8-bit gamma-space shading at 1/f resolution.

    python shading_ppm.py retinex <sigma> <f> <out_dir> <a.ppm> ...       # classical stand-in: blurred photo
    python shading_ppm.py gray    <sigma> <f> <out_dir> <a.ppm> ...       # same, one grey shading for all channels
    python shading_ppm.py marigold <npz_dir> <f> <out_dir> <a.ppm> ...    # Marigold-IID Lighting shading (linear -> gamma)
"""
import os, sys, numpy as np
from PIL import Image
from scipy import ndimage as ndi

mode, arg, f, out = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]; os.makedirs(out, exist_ok=True)

def area_down(a, f):
    H, W, _ = a.shape; h, w = -(-H // f), -(-W // f)
    p = np.pad(a, ((0, h * f - H), (0, w * f - W), (0, 0)), mode="edge")
    return p.reshape(h, f, w, f, 3).mean((1, 3))

for p in sys.argv[5:]:
    name = os.path.basename(p).rsplit(".", 1)[0]; img = np.asarray(Image.open(p), dtype=np.float64)
    if mode in ("retinex", "gray"):
        s = float(arg); src = img if mode == "retinex" else np.repeat(img.mean(-1, keepdims=True), 3, -1)
        sh = np.stack([ndi.gaussian_filter(src[..., c], s) for c in range(3)], -1)
    else:
        z = np.load(f"{arg}/{name}.npz"); S = np.clip(z["shading"].astype(np.float64), 1e-4, None)
        sh = S ** (1 / 2.2); sh = sh / np.quantile(sh, 0.995) * 250          # gamma space, scaled into 8 bits
    lo = np.clip(np.round(area_down(sh, f)), 1, 255).astype(np.uint8)
    Image.fromarray(lo).save(f"{out}/{name}.ppm")
