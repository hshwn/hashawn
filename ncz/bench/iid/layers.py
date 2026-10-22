#!/usr/bin/env python3
"""Test (b): code the photo as layers instead of pixels. Writes PPMs for `nczcli img-bench 0` to size.

    python layers.py <npz_dir> <f> <out_dir> <a.ppm> ...

albedo:   Marigold albedo -> gamma -> 8 bit, full resolution (coded exact)
shading:  Marigold shading -> gamma -> 8 bit at 1/f resolution (coded exact), upsampled bilinearly
residual: photo - round(c * albedo * shading)  (+128, coded exact); c per channel by least squares (3 numbers, free)
The decoder would rebuild the photo exactly: albedo x upsampled shading + residual. Prints the residual's spread
and how often it would not fit in 8 bits (those samples would need an escape; counted, not coded here).
"""
import os, sys, numpy as np
from PIL import Image

npz, f, out = sys.argv[1], int(sys.argv[2]), sys.argv[3]; os.makedirs(out, exist_ok=True)

def area_down(a, f):
    H, W, _ = a.shape; h, w = -(-H // f), -(-W // f)
    p = np.pad(a, ((0, h * f - H), (0, w * f - W), (0, 0)), mode="edge")
    return p.reshape(h, f, w, f, 3).mean((1, 3))

def up(lo, H, W, f):      # bilinear, sample centres aligned (same as nczcli shade-bench)
    h, w, _ = lo.shape
    fy = np.clip((np.arange(H) + 0.5) / f - 0.5, 0, h - 1); fx = np.clip((np.arange(W) + 0.5) / f - 0.5, 0, w - 1)
    y0 = np.floor(fy).astype(int); x0 = np.floor(fx).astype(int); y1 = np.minimum(y0 + 1, h - 1); x1 = np.minimum(x0 + 1, w - 1)
    ty = (fy - y0)[:, None, None]; tx = (fx - x0)[None, :, None]
    top = lo[y0][:, x0] * (1 - tx) + lo[y0][:, x1] * tx; bot = lo[y1][:, x0] * (1 - tx) + lo[y1][:, x1] * tx
    return top * (1 - ty) + bot * ty

for p in sys.argv[4:]:
    name = os.path.basename(p).rsplit(".", 1)[0]; img = np.asarray(Image.open(p), dtype=np.float64); H, W, _ = img.shape
    z = np.load(f"{npz}/{name}.npz")
    A = np.clip(z["albedo"].astype(np.float64), 0, 1) ** (1 / 2.2)
    S = np.clip(z["shading"].astype(np.float64), 1e-4, None) ** (1 / 2.2); S = S / np.quantile(S, 0.995)
    a8 = np.clip(np.round(A * 255), 0, 255); s8 = np.clip(np.round(area_down(S * 250, f)), 1, 255)
    su = up(s8, H, W, f)
    prod = a8 * su
    c = np.array([(prod[..., k] * img[..., k]).sum() / max((prod[..., k] ** 2).sum(), 1e-9) for k in range(3)])
    P = np.round(prod * c)
    R = img - P
    esc = float((np.abs(R) > 127).mean())
    Image.fromarray(a8.astype(np.uint8)).save(f"{out}/{name}.albedo.ppm")
    Image.fromarray(s8.astype(np.uint8)).save(f"{out}/{name}.shading.ppm")
    Image.fromarray(np.clip(R + 128, 0, 255).astype(np.uint8)).save(f"{out}/{name}.residual.ppm")
    print(f"{name:16s} residual rms {R.std():6.2f} (photo rms {img.std():6.2f})  |r|>127: {100 * esc:.3f}%  c {np.round(c, 4)}")
