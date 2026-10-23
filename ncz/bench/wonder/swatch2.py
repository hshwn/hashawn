#!/usr/bin/env python3
"""Swatches v2: the encoder chooses, per block, what fills in the lost texture: one of the swatches, grain, or
nothing, by trying each and keeping the one whose result looks most like the original there (texture statistics, not
pixels: a texture can look right without matching pixel for pixel). Grain is for fine random texture (plaster, paint,
film grain), where a coded swatch turns into stripes; swatches are for structured texture (leaves, tiles, tread).

    python swatch2.py <original.ppm> <smooth.png> <out.png> [K] [swatch px] [dct step] [labels.png]

The decoder repeats the same raster walk with the chosen labels (every choice depends only on the smooth picture, the
coded swatches, the levels and what is already laid), so it gets the same picture. Unused swatches are dropped and the
blocks re-chosen, so only swatches that earn their bytes are sent.
"""
import sys, os, subprocess
import numpy as np
from scipy import ndimage as ndi
from PIL import Image
from numpy.lib.stride_tricks import sliding_window_view as win
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from swatch_ceiling import encode, dct_code, band, low, brms16, LUMA, B, OV

P = B + OV; NONE, GRAIN = -1, -2
DEBUG = {tuple(map(int, t.split(","))) for t in os.environ.get("DEBUG", "").split(";") if t}

def hnoise(H, W, soft=0.6):
    """Grain field: position-hashed noise, blurred by `soft` (the photo's grain size, chosen by the encoder: 1 byte),
    unit rms in the band."""
    y, x = np.mgrid[:H, :W].astype(np.uint32)
    h = (x * np.uint32(0x9E3779B1)) ^ (y * np.uint32(0x85EBCA77))
    h ^= h >> np.uint32(16); h *= np.uint32(0x7feb352d); h ^= h >> np.uint32(15); h *= np.uint32(0x846ca68b); h ^= h >> np.uint32(16)
    n = ((h & np.uint32(255)) + ((h >> np.uint32(8)) & np.uint32(255)) + ((h >> np.uint32(16)) & np.uint32(255)) + (h >> np.uint32(24))).astype(float)
    n = (n - 510) / 147.8
    if soft > 0: n = ndi.gaussian_filter(n, soft)
    n = band(n); return n / n.std()
SOFTS = (0.0, 0.3, 0.45, 0.6, 0.75, 0.9)
def fine_share(h): f = h - ndi.gaussian_filter(h, 0.7); return (f * f).mean() / ((h * h).mean() + 1e-6)
def grain_soft(o, cand):
    """The grain size whose fine / whole-band energy matches the original's in its texture blocks."""
    ob = band(o) @ LUMA; H, W = ob.shape; m = np.kron(cand, np.ones((B, B)))[:H, :W] > 0
    f = ob - ndi.gaussian_filter(ob, 0.7); r = (f[m] ** 2).mean() / ((ob[m] ** 2).mean() + 1e-6)
    return min(SOFTS, key=lambda s: abs(fine_share(hnoise(128, 128, s)) - r))

def feats(h):
    """Texture statistics of a patch of fine detail (grey): energy in two bands, energy by direction, peakiness."""
    f = h - ndi.gaussian_filter(h, 0.7); m = h - f
    gy, gx = np.gradient(h); e = [(gx * gx).mean(), (gy * gy).mean(), ((gx + gy) ** 2).mean() / 2, ((gx - gy) ** 2).mean() / 2]
    tot = sum(e) + 1e-3; k = (h ** 4).mean() / ((h ** 2).mean() ** 2 + 1e-3)
    return np.array([np.log(np.sqrt((f * f).mean()) + 0.5), np.log(np.sqrt((m * m).mean()) + 0.5)] + [ei / tot * 4 for ei in e] + [np.log(k + 0.1)])

def lay(s, sw, lvl, cand, lab=None, o=None, soft=0.6):
    """Lay fine detail on the smooth picture block by block. With the original `o`, choose each block's label (encoder);
    else use `lab` (decoder). Returns (picture, labels)."""
    H, W, _ = s.shape; bh, bw = H // B, W // B
    acc = np.zeros((H + OV, W + OV)); wsum = np.zeros((H + OV, W + OV))
    sb = band(s) @ LUMA; rs = brms16(sb); sl = np.pad(low(s) @ LUMA, ((0, OV), (0, OV)), mode="edge")
    sbp = np.pad(sb, ((0, OV), (0, OV)), mode="edge"); ob = np.pad(band(o) @ LUMA, ((0, OV), (0, OV)), mode="edge") if o is not None else None
    grain = hnoise(H + OV, W + OV, soft)
    src = []
    for w in sw:
        hb, ll = [], []
        for ww in (w, w[:, ::-1]): hb.append(win(band(ww), (P, P)).reshape(-1, P, P)); ll.append(win(low(ww), (P, P)).reshape(-1, P, P))
        src.append((np.concatenate(hb), np.concatenate(ll)))
    ramp = np.ones(P); ramp[:OV] = (np.arange(OV) + 0.5) / OV; ramp[-OV:] = ramp[:OV][::-1]; fw = np.outer(ramp, ramp)
    out_lab = np.full((bh, bw), NONE)
    for by in range(bh):
        for bx in range(bw):
            if not cand[by, bx]: continue
            y0, x0 = by * B, bx * B; reg = np.s_[y0: y0 + P, x0: x0 + P]
            target = 2 ** (lvl[min(by // 2, lvl.shape[0] - 1), min(bx // 2, lvl.shape[1] - 1)] / 2)
            miss = np.sqrt(max(target ** 2 - rs[by, bx] ** 2, 0.0))
            placed = wsum[reg] > 0; cur = acc[reg] / np.maximum(wsum[reg], 1e-6)
            def patch(k):
                if k == GRAIN: return grain[reg] * miss          # unit-rms field: straight to the missing level
                else:
                    hb, ll = src[k]; t = sl[reg] - sl[reg].mean(); a = ll - ll.mean((1, 2), keepdims=True)
                    cost = ((a - t) ** 2).mean((1, 2)) / (t.var() + 4)
                    if placed.any(): cost = cost + ((hb - cur[None]) ** 2)[:, placed].mean(1) / (cur[placed].var() + 1)
                    p = hb[cost.argmin()]
                return p * np.clip(miss / (np.sqrt((p ** 2).mean()) + 1e-3), 0.0, 2.0)
            if o is not None:              # encoder: try every option, keep the one that looks most like the original
                fo = feats(ob[reg]); best, bk = None, NONE
                for k in [NONE, GRAIN] + list(range(len(sw))):
                    add = 0 if k == NONE else patch(k)
                    blend = np.where(placed, (cur * wsum[reg] + add * fw) / (wsum[reg] + fw), add) if k != NONE else np.where(placed, cur, 0)
                    d = ((feats(sbp[reg] + blend) - fo) ** 2).sum() + (0.05 if k >= 0 else 0.0)   # a swatch block must clearly win
                    if (by, bx) in DEBUG: print(by, bx, k, np.round(feats(sbp[reg] + blend) - fo, 2), round(float(d), 3))
                    if best is None or d < best: best, bk = d, k
                k = bk
            else: k = lab[by, bx]
            out_lab[by, bx] = k
            if k == NONE: continue
            acc[reg] += patch(k) * fw; wsum[reg] += fw
    m = wsum[:H, :W]; new = acc[:H, :W] / np.maximum(m, 1e-6); mask = np.clip(m / fw.max(), 0, 1)
    return np.clip(s + (mask * new)[..., None], 0, 255), out_lab

def run(o, s, K, SW, step, tmp, cli):
    lab0, sw, lvl = encode(o, s, K, SW); cand = lab0 >= 0
    atlas = np.concatenate([np.round(w @ LUMA).clip(0, 255) for w in sw], 1)
    back, _ = dct_code(atlas, step); swc = [back[:, i * SW:(i + 1) * SW] for i in range(len(sw))]
    soft = grain_soft(o, cand); print("grain size", soft)
    out, lab = lay(s, swc, lvl, cand, o=o, soft=soft)
    used = [k for k in range(len(swc)) if (lab == k).sum() >= 4]
    if len(used) < len(swc):          # drop swatches that did not earn their bytes, choose again
        swc = [swc[k] for k in used]; out, lab = lay(s, swc, lvl, cand, o=o, soft=soft)
    nb = dct_code(np.concatenate(swc, 1), step)[1] if swc else 0
    dec, _ = lay(s, swc, lvl, cand, lab=lab, soft=soft)  # what the decoder makes from the labels alone
    assert np.array_equal(dec, out), "decoder disagrees with the encoder"
    side = np.concatenate([(lab + 2).astype(np.uint8).ravel(), (lvl + 4).clip(0, 255).astype(np.uint8).ravel()]).tobytes()
    open(f"{tmp}.side", "wb").write(side); subprocess.run([cli, "compress", f"{tmp}.side", f"{tmp}.side.ncz"], check=True, capture_output=True)
    return out, lab, len(swc), nb, os.path.getsize(f"{tmp}.side.ncz")

if __name__ == "__main__":
    o = np.asarray(Image.open(sys.argv[1]).convert("RGB"), dtype=float); s = np.asarray(Image.open(sys.argv[2]).convert("RGB"), dtype=float)
    K = int(sys.argv[4]) if len(sys.argv) > 4 else 8; SW = int(sys.argv[5]) if len(sys.argv) > 5 else 48
    step = float(sys.argv[6]) if len(sys.argv) > 6 else 10; cli = sys.argv[7]
    out, lab, ns, nb, nside = run(o, s, K, SW, step, sys.argv[3], cli)
    Image.fromarray(np.round(out).astype(np.uint8)).save(sys.argv[3])
    if len(sys.argv) > 8:              # labels as a picture: grey none, white grain, colours swatches
        pal = np.array([[60, 60, 60], [255, 255, 255]] + [[230, 25, 75], [60, 180, 75], [255, 225, 25], [0, 130, 200], [245, 130, 48], [145, 30, 180], [70, 240, 240], [240, 50, 230]], np.uint8)
        Image.fromarray(pal[np.where(lab == NONE, 0, np.where(lab == GRAIN, 1, lab + 2))]).resize((lab.shape[1] * 8, lab.shape[0] * 8), Image.NEAREST).save(sys.argv[8])
    psnr = lambda a: 10 * np.log10(255 ** 2 / ((a - o) ** 2).mean())
    print(f"{sys.argv[1].split('/')[-1]}: blocks none {(lab == NONE).mean() * 100:.0f}% grain {(lab == GRAIN).mean() * 100:.0f}% swatch {(lab >= 0).mean() * 100:.0f}% "
          f"({ns} swatches); {psnr(out):.2f} dB; bytes swatches {nb} + labels/levels {nside} = {nb + nside}")
