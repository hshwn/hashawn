#!/usr/bin/env python3
"""Flat colours, compressed: albedo -> K repeated colours (palette + label map) -> guide = palette colour x shading.

    python palette.py <npz_dir> <K> <smooth> <f> <out_dir> <a.ppm> ...

albedo (gamma, Marigold) -> k-means to K colours -> label map, optionally mode-filtered (smooth x smooth) so regions
are solid -> palette re-fitted as each region's mean PHOTO colour / shading (what the photo actually shows there).
shading: Marigold shading, gamma, 1/f resolution, 8 bit, upsampled bilinearly.
guide = round(palette[label] * shading_up / 256), per channel gain fitted (3 numbers).
Writes <name>.labels (u8 raw), .palette (u8 RGB), .shade.ppm (low-res), .guide.ppm. Everything the guide uses is
quantised and stored, so a decoder could rebuild it exactly from the side information.
"""
import os, sys, numpy as np
from PIL import Image
from scipy import ndimage as ndi
from scipy.cluster.vq import kmeans2

npz, K, sm, f, out = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4]), sys.argv[5]; os.makedirs(out, exist_ok=True)

def area_down(a, f):
    H, W, _ = a.shape; h, w = -(-H // f), -(-W // f)
    p = np.pad(a, ((0, h * f - H), (0, w * f - W), (0, 0)), mode="edge")
    return p.reshape(h, f, w, f, 3).mean((1, 3))

def up(lo, H, W, f):
    h, w, _ = lo.shape
    fy = np.clip((np.arange(H) + 0.5) / f - 0.5, 0, h - 1); fx = np.clip((np.arange(W) + 0.5) / f - 0.5, 0, w - 1)
    y0 = np.floor(fy).astype(int); x0 = np.floor(fx).astype(int); y1 = np.minimum(y0 + 1, h - 1); x1 = np.minimum(x0 + 1, w - 1)
    ty = (fy - y0)[:, None, None]; tx = (fx - x0)[None, :, None]
    top = lo[y0][:, x0] * (1 - tx) + lo[y0][:, x1] * tx; bot = lo[y1][:, x0] * (1 - tx) + lo[y1][:, x1] * tx
    return top * (1 - ty) + bot * ty

def mode_filter(lab, K, s):
    if s <= 1: return lab
    votes = np.stack([ndi.uniform_filter((lab == k).astype(np.float32), s) for k in range(K)], 0)
    return votes.argmax(0).astype(lab.dtype)

for p in sys.argv[6:]:
    name = os.path.basename(p).rsplit(".", 1)[0]; img = np.asarray(Image.open(p), dtype=np.float64); H, W, _ = img.shape
    z = np.load(f"{npz}/{name}.npz")
    A = np.clip(z["albedo"].astype(np.float64), 0, 1) ** (1 / 2.2)
    S = np.clip(z["shading"].astype(np.float64), 1e-4, None) ** (1 / 2.2); S = S / np.quantile(S, 0.995)
    s8 = np.clip(np.round(area_down(S * 250, f)), 1, 255); su = up(s8, H, W, f)
    rng = np.random.default_rng(0); sample = A.reshape(-1, 3)[rng.choice(H * W, 20000, replace=False)]
    cent, _ = kmeans2(sample, K, minit="++", seed=1, iter=20)
    lab = ((A.reshape(-1, 1, 3) - cent[None]) ** 2).sum(-1).argmin(1).reshape(H, W).astype(np.uint8)
    lab = mode_filter(lab, K, sm)
    # palette = what the photo shows per region once the light is divided out (scale 256 = shading 1.0)
    pal = np.zeros((K, 3))
    for k in range(K):
        m = lab == k
        if m.any(): pal[k] = (img[m] * su[m]).sum(0) / np.maximum((su[m] ** 2).sum(0), 1e-9) * 256
    pal8 = np.clip(np.round(pal), 0, 255)
    guide = np.clip(np.round(pal8[lab] * su / 256), 0, 255)
    err = img - guide
    lab.tofile(f"{out}/{name}.labels"); pal8.astype(np.uint8).tofile(f"{out}/{name}.palette")
    Image.fromarray(s8.astype(np.uint8)).save(f"{out}/{name}.shade.ppm"); Image.fromarray(guide.astype(np.uint8)).save(f"{out}/{name}.guide.ppm")
    used = len(np.unique(lab))
    print(f"{name:16s} K {K} smooth {sm} (used {used})  guide error rms {err.std():6.2f} (photo rms {img.std():6.2f})", flush=True)
