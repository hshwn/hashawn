#!/usr/bin/env python3
"""Swatch ceiling test: instead of inventing texture, the file carries a few small samples ("swatches") of the photo's
own textures, and the decoder lays their fine detail onto the smooth picture. Here the swatches are perfect (cut from
the original, uncompressed), so this is the best the idea can look; their byte cost is the next question.

    python swatch_ceiling.py <original.ppm> <smooth.png> <out.png> [K swatches] [swatch px]

Encoder (sees the original):
  - every 16 x 16 block gets a texture descriptor (fine-detail energy in three bands, orientation, colour) and the
    blocks are clustered into K kinds; flat blocks (little fine detail) are left alone
  - each kind's swatch is the swatch-sized window holding the most blocks of that kind
  - side info: per block, which swatch (or none), and per 32 x 32 block the fine-detail level (as the detail map)
Decoder (sees the smooth picture, the swatches and the side info), blocks in raster order:
  - candidate patches: every position in the swatch (and its mirror image); cost = how badly its coarse shape matches
    the smooth picture there + how badly its fine detail joins the already placed neighbours in the 4 px overlap
  - the fine detail (picture minus its blur, sigma 2) of the cheapest patch, scaled to the block's level, replaces the
    smooth picture's own fine detail in the block; overlaps are cross-faded
"""
import sys
import numpy as np
from scipy import ndimage as ndi
from PIL import Image
from numpy.lib.stride_tricks import sliding_window_view as win

B, OV, SIG = 16, 4, 2.0
LUMA = np.array([0.299, 0.587, 0.114])

def band(img, s=SIG): return img - ndi.gaussian_filter(img, (s, s, 0)[: img.ndim], mode="reflect")
def low(img, s=SIG): return ndi.gaussian_filter(img, (s, s, 0)[: img.ndim], mode="reflect")

def descriptors(o):
    y = o @ LUMA; H, W = y.shape; bh, bw = H // B, W // B
    def brms(h): return np.sqrt((h[: bh * B, : bw * B] ** 2).reshape(bh, B, bw, B).mean((1, 3)))
    e = [np.log(brms(y - ndi.gaussian_filter(y, s)) + 0.5) for s in (1, 2, 4)]
    gy, gx = np.gradient(ndi.gaussian_filter(y, 1))
    def bm(a): return a[: bh * B, : bw * B].reshape(bh, B, bw, B).mean((1, 3))
    jxx, jyy, jxy = bm(gx * gx), bm(gy * gy), bm(gx * gy); tr = jxx + jyy + 1e-6
    c2, s2 = (jxx - jyy) / tr, 2 * jxy / tr                                  # orientation x coherence
    cb, cr = o[..., 2] - y, o[..., 0] - y
    f = np.stack(e + [c2, s2, bm(y) / 32, bm(cb) / 16, bm(cr) / 16], -1)
    return f, e[1]

def kmeans(x, k, it=30, seed=0):
    r = np.random.default_rng(seed); c = x[r.choice(len(x), k, replace=False)]
    for _ in range(it):
        lab = ((x[:, None] - c[None]) ** 2).sum(-1).argmin(1)
        c = np.stack([x[lab == j].mean(0) if (lab == j).any() else c[j] for j in range(k)])
    return lab

def brms16(h):
    H, W = h.shape[:2]; bh, bw = H // B, W // B; return np.sqrt((h[: bh * B, : bw * B] ** 2).reshape(bh, B, bw, B).mean((1, 3)))
def encode(o, s, K, SW):
    f, e2 = descriptors(o); bh, bw = e2.shape
    # swatches only where the smooth picture LOST the fine detail (texture); where it kept most of it (edges, letters,
    # window frames) the base already has the structure and pasting would wreck it
    ro, rs = brms16(band(o) @ LUMA), brms16(band(s) @ LUMA)
    tex = (ro > 3.0) & (rs < 0.6 * ro)
    lab = np.full((bh, bw), -1); x = f[tex]; x = (x - x.mean(0)) / (x.std(0) + 1e-6)
    lab[tex] = kmeans(x, K)
    # each kind's swatch: the window with the most blocks of that kind, every block of it texture (an edge or a window
    # frame inside a swatch gets pasted onto bare walls); kinds with no such window take the next-nearest kind
    n = SW // B; sw = []; keep = []
    pure = win(tex.astype(int), (n, n)).sum((-2, -1)) == n * n
    for k in range(K):
        cnt = win((lab == k).astype(int), (n, n)).sum((-2, -1)) * pure
        if cnt.max() < 2: continue
        by, bx = np.unravel_index(cnt.argmax(), cnt.shape); keep.append(k)
        sw.append(o[by * B: by * B + SW, bx * B: bx * B + SW].copy())
    cent = np.stack([x[lab[tex] == k].mean(0) for k in keep]); rem = -np.ones(K + 1, int); rem[keep] = np.arange(len(keep))
    for k in range(K):
        if k not in keep and (lab == k).any():
            rem[k] = ((x[lab[tex] == k].mean(0) - cent) ** 2).sum(-1).argmin()
    lab = np.where(lab >= 0, rem[lab], -1)
    # level per 32 x 32 block: log2 fine-detail rms in half steps (as the detail map), measured on this band
    yb = band(o @ LUMA); H, W = yb.shape
    lvl = np.round(2 * np.log2(np.sqrt((yb[: H // 32 * 32, : W // 32 * 32] ** 2).reshape(H // 32, 32, W // 32, 32).mean((1, 3))).clip(0.25)))
    return lab, sw, lvl

def decode(s, lab, sw, lvl, wstruct=1.0):
    sb = band(s); rs = brms16(sb @ LUMA)
    H, W, _ = s.shape; P = B + OV; pad = OV
    acc = np.zeros((H + pad, W + pad, 3)); wsum = np.zeros((H + pad, W + pad))
    sl = low(s) @ LUMA; sl_p = np.pad(sl, ((0, pad), (0, pad)), mode="edge")
    srcs = []
    for k, w in enumerate(sw):
        for flip in (False, True):
            ww = w[:, ::-1] if flip else w                                     # a grey swatch: its detail goes on all three channels
            hb = np.repeat(band(ww)[..., None], 3, -1); ll = low(ww)
            srcs.append((win(hb, (P, P), axis=(0, 1)).reshape(-1, 3, P, P).transpose(0, 2, 3, 1),       # (n, P, P, 3)
                         win(ll, (P, P)).reshape(-1, P, P)))
    groups = {}
    for k in range(len(sw)): groups[k] = (np.concatenate([srcs[2 * k][0], srcs[2 * k + 1][0]]), np.concatenate([srcs[2 * k][1], srcs[2 * k + 1][1]]))
    ramp = np.ones(P); ramp[:OV] = (np.arange(OV) + 0.5) / OV; ramp[-OV:] = ramp[:OV][::-1]; fw = np.outer(ramp, ramp)
    for by in range(lab.shape[0]):
        for bx in range(lab.shape[1]):
            k = lab[by, bx]
            if k < 0: continue
            hb, ll = groups[k]; y0, x0 = by * B, bx * B
            tgt = sl_p[y0: y0 + P, x0: x0 + P]
            a = ll - ll.mean((1, 2), keepdims=True); t = tgt - tgt.mean()
            cost = wstruct * ((a - t) ** 2).mean((1, 2)) / (t.var() + 4)
            placed = wsum[y0: y0 + P, x0: x0 + P] > 0                        # overlap with neighbours already laid
            if placed.any():
                cur = acc[y0: y0 + P, x0: x0 + P] / np.maximum(wsum[y0: y0 + P, x0: x0 + P], 1e-6)[..., None]
                cost = cost + ((hb - cur[None]) ** 2)[:, placed].mean((1, 2)) / (cur[placed].var() + 1)
            j = cost.argmin(); patch = hb[j]
            target = 2 ** (lvl[min(by // 2, lvl.shape[0] - 1), min(bx // 2, lvl.shape[1] - 1)] / 2)
            miss = np.sqrt(max(target ** 2 - rs[by, bx] ** 2, 0.0))             # only the detail the smooth picture lacks
            g = np.clip(miss / (np.sqrt(((patch @ LUMA) ** 2).mean()) + 1e-3), 0.0, 2.0)
            acc[y0: y0 + P, x0: x0 + P] += patch * g * fw[..., None]; wsum[y0: y0 + P, x0: x0 + P] += fw
    m = wsum[:H, :W]; new = acc[:H, :W] / np.maximum(m, 1e-6)[..., None]
    mask = np.clip(m / fw.max(), 0, 1)[..., None]                               # 1 inside laid blocks, fades at their border
    return np.clip(s + mask * new, 0, 255)                                     # added on top: the base keeps its edges

def dct_code(atlas, step):
    """Swatches transform-coded: 8 x 8 DCT, uniform step growing with frequency, rounding toward zero (dead zone).
    Bytes estimated as the adaptive cost of the integer levels (per-frequency-band counts, Laplace smoothed), which a
    context-mixing coder matches or beats. Returns (decoded atlas, bytes)."""
    from scipy.fft import dctn, idctn
    H, W = atlas.shape; a = atlas - 128.0
    bl = a.reshape(H // 8, 8, W // 8, 8).transpose(0, 2, 1, 3); c = dctn(bl, axes=(-2, -1), norm="ortho")
    u, v = np.meshgrid(np.arange(8), np.arange(8), indexing="ij"); qm = step * (1 + 0.15 * (u + v)); qm[0, 0] = 4
    lv = np.trunc(c / qm + np.sign(c) * 0.3)
    rec = idctn(lv * qm, axes=(-2, -1), norm="ortho").transpose(0, 2, 1, 3).reshape(H, W) + 128
    bits = 0.0; band = (u + v).ravel(); L = lv.reshape(-1, 64)
    for b in range(15):
        vals = L[:, band == b].ravel().astype(int); cnt = {}
        for x in vals: p = (cnt.get(x, 0) + 0.4) / (sum(cnt.values()) + 0.4 * 64); bits -= np.log2(p); cnt[x] = cnt.get(x, 0) + 1
    return rec.clip(0, 255), int(bits / 8) + 8
def code(sw, lab, lvl, near, tmp, cli):
    """The swatches as one grey atlas coded by ncz's PNG coder (near +-K, or exact), labels and levels packed by ncz.
    Returns (decoded grey swatches, bytes of swatches, bytes of labels + levels)."""
    import subprocess, os
    SW = sw[0].shape[0]; atlas = np.concatenate([np.round(w @ LUMA).clip(0, 255) for w in sw], 1).astype(np.uint8)
    Image.fromarray(atlas, "L").save(f"{tmp}.atlas.png")
    if near >= 100: back, nb = dct_code(atlas.astype(float), near - 100)      # near 100 + step: transform coded
    else:
        mode = "0" if near == 0 else f"n{near}"
        subprocess.run([cli, "png-mode", mode, f"{tmp}.atlas.png", f"{tmp}.atlas.ncz", f"{tmp}.atlas.back.png"], check=True, capture_output=True)
        back = np.asarray(Image.open(f"{tmp}.atlas.back.png").convert("L"), dtype=float); nb = os.path.getsize(f"{tmp}.atlas.ncz")
    side = np.concatenate([(lab + 1).astype(np.uint8).ravel(), (lvl + 4).clip(0, 255).astype(np.uint8).ravel()]).tobytes()
    open(f"{tmp}.side", "wb").write(side); subprocess.run([cli, "compress", f"{tmp}.side", f"{tmp}.side.ncz"], check=True, capture_output=True)
    return [back[:, i * SW:(i + 1) * SW] for i in range(len(sw))], nb, os.path.getsize(f"{tmp}.side.ncz")

if __name__ == "__main__":
    # swatch_ceiling.py <original.ppm> <smooth.png> <out.png> [K] [swatch px] [near: -1 perfect grey, 0 exact, K +-K] [nczcli]
    o = np.asarray(Image.open(sys.argv[1]).convert("RGB"), dtype=float); s = np.asarray(Image.open(sys.argv[2]).convert("RGB"), dtype=float)
    K = int(sys.argv[4]) if len(sys.argv) > 4 else 8; SW = int(sys.argv[5]) if len(sys.argv) > 5 else 48
    near = int(sys.argv[6]) if len(sys.argv) > 6 else -1
    lab, sw, lvl = encode(o, s, K, SW)
    if near < 0: swg, nb, ns = [w @ LUMA for w in sw], 0, 0
    else: swg, nb, ns = code(sw, lab, lvl, near, sys.argv[3], sys.argv[7])
    out = decode(s, lab, swg, lvl)
    Image.fromarray(np.round(out).astype(np.uint8)).save(sys.argv[3])
    psnr = lambda a: 10 * np.log10(255 ** 2 / ((a - o) ** 2).mean())
    print(f"{sys.argv[1].split('/')[-1]} {len(sw)} swatches, {(lab >= 0).mean() * 100:.0f}% of blocks; smooth {psnr(s):.2f} dB, swatches {psnr(out):.2f} dB; "
          f"bytes: swatches {nb} + labels/levels {ns}")
