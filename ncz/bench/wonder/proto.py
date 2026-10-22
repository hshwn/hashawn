#!/usr/bin/env python3
"""Wonder format, prototype 0: structure kept, texture described.

    uv run --with numpy --with scipy --with pillow --with imagecodecs --with ssimulacra2 python proto.py <out_dir> <a.ppm> ...

Picture -> YCoCg -> Laplacian pyramid, coded closed-loop from coarse to fine (each level predicted from the coarser
reconstruction). The coarse base and coarse levels are quantised and kept. In the two finest levels, every 16x16 block
(at that level's resolution) gets one of:
  keep    quantised detail (edges, text, eyes: structured detail, high orientation coherence)
  regrow  no detail stored, only its energy (one byte per block); the decoder grows band-limited noise shaped to
          the level's measured spectrum, scaled to that energy (grass, foliage, water, sand, grain)
  drop    nothing (smooth areas)
Regrown detail does not enter the prediction chain (finer levels are coded against the clean reconstruction).
Bytes are REAL: every stored array goes through ncz's byte coder (nczcli compress). Matched control: the same codec
with regrowth off (blocks either kept or dropped). JPEG XL (effort 7) matched by size. Quality: SSIMULACRA2.
Floating point here: the prototype only has to prove the bet; the real format will be integer and deterministic.
"""
import json, os, subprocess, sys, tempfile
import numpy as np, imagecodecs as ic, ssimulacra2
from PIL import Image
from scipy import ndimage as ndi

NCZ = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../target/release/nczcli")
TMP = tempfile.mkdtemp(); BS = 16; NLEV = 4

def nbytes(arr):
    """Real compressed size of an int array (zigzag, 1 or 2 bytes per value) through ncz's byte coder."""
    a = np.asarray(arr, dtype=np.int64).ravel()
    if a.size == 0: return 0
    z = np.where(a >= 0, 2 * a, -2 * a - 1)
    raw = z.astype(np.uint8).tobytes() if z.max() < 256 else z.astype(np.uint16).tobytes()
    open(f"{TMP}/a", "wb").write(raw); subprocess.run([NCZ, "compress", f"{TMP}/a", f"{TMP}/b"], check=True)
    return os.path.getsize(f"{TMP}/b")

def to_ycc(rgb):
    r, g, b = rgb[..., 0], rgb[..., 1], rgb[..., 2]
    co = r - b; t = b + co / 2; cg = g - t; y = t + cg / 2
    return np.stack([y, co, cg], -1)
def from_ycc(y3):
    y, co, cg = y3[..., 0], y3[..., 1], y3[..., 2]
    t = y - cg / 2; g = cg + t; b = t - co / 2; r = b + co
    return np.stack([r, g, b], -1)

K5 = np.array([1, 4, 6, 4, 1], float) / 16
def blur(x): return ndi.convolve1d(ndi.convolve1d(x, K5, axis=0, mode="reflect"), K5, axis=1, mode="reflect")
def down(x): return blur(x)[::2, ::2]
def up(x, shape):
    u = np.zeros(shape); u[::2, ::2] = x[: (shape[0] + 1) // 2, : (shape[1] + 1) // 2]
    return blur(u) * 4

def gauss_pyr(x, n):
    g = [x]
    for _ in range(n): g.append(down(g[-1]))
    return g

def band_noise(shape, rng):
    """White noise, Laplacian-band-limited like a level's detail, unit variance."""
    n = rng.standard_normal(shape); b = n - up(down(n), shape)
    return b / b.std()

def shape_noise(noise, psd):
    """Colour the noise with a measured 2D power spectrum (whole-plane FFT filter)."""
    H, W = noise.shape; f = np.fft.rfft2(noise)
    ps = np.fft.fftshift(psd); fy = np.fft.fftfreq(H); fx = np.fft.rfftfreq(W)
    iy = np.clip(np.round(np.fft.fftshift(np.fft.fftfreq(psd.shape[0])).searchsorted(fy)), 0, psd.shape[0] - 1).astype(int)
    ix = np.clip(np.round(np.fft.fftshift(np.fft.fftfreq(psd.shape[1])).searchsorted(fx)), 0, psd.shape[1] - 1).astype(int)
    amp = np.sqrt(np.maximum(ps[iy[:, None], ix[None, :]], 1e-12))
    out = np.fft.irfft2(f * amp, s=(H, W)); return out / max(out.std(), 1e-9)

def block_stats(d, x0, mask_fn=None):
    """Per 16x16 block: rms energy and orientation coherence (structure tensor of the level's reconstruction input)."""
    H, W = d.shape; nby, nbx = -(-H // BS), -(-W // BS)
    gy, gx = np.gradient(x0)
    jxx, jyy, jxy = gx * gx, gy * gy, gx * gy
    e = np.zeros((nby, nbx)); coh = np.zeros((nby, nbx))
    for by in range(nby):
        for bx in range(nbx):
            s = (slice(by * BS, (by + 1) * BS), slice(bx * BS, (bx + 1) * BS))
            e[by, bx] = np.sqrt((d[s] ** 2).mean())
            a, b, c = jxx[s].sum(), jyy[s].sum(), jxy[s].sum()
            tr = a + b; det = np.sqrt((a - b) ** 2 + 4 * c * c)
            coh[by, bx] = det / tr if tr > 1e-9 else 0.0
    return e, coh

def block_up(m, shape):
    """Block map -> per-pixel map, bilinear (smooth energy, no block edges)."""
    nby, nbx = m.shape
    yy = (np.arange(shape[0]) + 0.5) / BS - 0.5; xx = (np.arange(shape[1]) + 0.5) / BS - 0.5
    return ndi.map_coordinates(m, np.meshgrid(np.clip(yy, 0, nby - 1), np.clip(xx, 0, nbx - 1), indexing="ij"), order=1, mode="nearest")

def encode_decode(rgb, q, regrow, tau=0.55, seed=1):
    """Returns (reconstructed rgb, total bytes, stats)."""
    rng = np.random.default_rng(seed)
    ycc = to_ycc(rgb.astype(float)); H, W, _ = ycc.shape
    total = 16; st = {"keep": 0, "regrow": 0, "drop": 0}
    out = np.zeros_like(ycc)
    for c in range(3):
        cw = 1.0 if c == 0 else 1.8                                   # colour detail: coarser
        g = gauss_pyr(ycc[..., c], NLEV)
        base = np.round(g[NLEV] / (q * 0.5 * cw)); total += nbytes(base)
        rec = base * q * 0.5 * cw; synth_total = None
        synth_levels = []
        for k in range(NLEV - 1, -1, -1):
            pred = up(rec, g[k].shape); d = g[k] - pred
            step = q * cw * [1.0, 0.75, 0.6, 0.5][k]
            if k >= 2:                                                 # coarse levels: always kept
                qd = np.round(d / step); total += nbytes(qd); rec = pred + qd * step; continue
            if c != 0 and k == 0:                                      # finest colour level: dropped (eyes do not see it)
                rec = pred; continue
            e, coh = block_stats(d, g[k])
            nby, nbx = e.shape
            mode = np.zeros((nby, nbx), np.int8)                       # 0 drop, 1 keep, 2 regrow
            floor = step * 0.35
            for by in range(nby):
                for bx in range(nbx):
                    if e[by, bx] < floor: mode[by, bx] = 0
                    elif regrow and k == 0 and coh[by, bx] < tau and c == 0: mode[by, bx] = 2     # finest band only: coarser bands are visible structure
                    else: mode[by, bx] = 1
            keepmask = np.kron(mode == 1, np.ones((BS, BS)))[: d.shape[0], : d.shape[1]].astype(bool)
            qd = np.round(d / step) * keepmask
            total += nbytes(qd) + nbytes(mode)
            rec = pred + qd * step
            st["keep"] += int((mode == 1).sum()); st["drop"] += int((mode == 0).sum()); st["regrow"] += int((mode == 2).sum())
            if (mode == 2).any():
                # fingerprint: energy per regrown block (log, 1 byte) + this level's 32x32 spectrum (from regrown blocks)
                eq = np.clip(np.round(np.log2(np.maximum(e, 1e-3)) * 8) + 64, 0, 255) * (mode == 2)
                total += nbytes(eq)
                pats = []
                for by in range(nby):
                    for bx in range(nbx):
                        if mode[by, bx] == 2 and (by + 2) * BS <= d.shape[0] and (bx + 2) * BS <= d.shape[1]:
                            p = d[by * BS:by * BS + 32, bx * BS:bx * BS + 32]; p = p - p.mean()
                            pats.append(np.abs(np.fft.fft2(p * np.outer(np.hanning(32), np.hanning(32)))) ** 2)
                psd = np.mean(pats, 0) if pats else np.ones((32, 32))
                total += 32 * 17                                        # spectrum, 1 byte per half-plane bin
                eng = 2.0 ** ((eq - 64) / 8) * (mode == 2)
                n = shape_noise(band_noise(d.shape, rng), psd)
                synth_levels.append((k, n * block_up(eng, d.shape) * block_up((mode == 2).astype(float), d.shape)))
        # assemble: clean chain + regrown detail upsampled from its level
        img = rec
        for k, s in synth_levels:
            for _ in range(k): s = up(s, (s.shape[0] * 2, s.shape[1] * 2))
            img = img + s[: H, : W]
        out[..., c] = img
    return np.clip(np.round(from_ycc(out)), 0, 255), total, st

def ssim2(a, b):
    Image.fromarray(a.astype(np.uint8)).save(f"{TMP}/r.png"); Image.fromarray(b.astype(np.uint8)).save(f"{TMP}/d.png")
    return float(ssimulacra2.compute_ssimulacra2(f"{TMP}/r.png", f"{TMP}/d.png"))

def jxl_at(a8, nb):
    lo, hi = 0.05, 25.0
    for _ in range(14):
        d = (lo * hi) ** 0.5
        if len(ic.jpegxl_encode(a8, distance=d, effort=7)) <= nb: hi = d
        else: lo = d
    b = ic.jpegxl_encode(a8, distance=hi, effort=7); return len(b), ic.jpegxl_decode(b).astype(float)

if __name__ == "__main__":
    out = sys.argv[1]; os.makedirs(out, exist_ok=True); report = {}
    for p in sys.argv[2:]:
        name = os.path.basename(p).rsplit(".", 1)[0]; img = np.asarray(Image.open(p).convert("RGB"), dtype=float); n = img.shape[0] * img.shape[1]
        r = {}
        for q in (6, 10, 16, 24):
            for rg in (False, True):
                rec, nb, st = encode_decode(img, q, rg)
                jb, jd = jxl_at(img.astype(np.uint8), nb)
                tag = f"q{q}{'R' if rg else ''}"
                r[tag] = dict(bpp=round(8 * nb / n, 3), s2=round(ssim2(img, rec), 2), jxl_bpp=round(8 * jb / n, 3), jxl_s2=round(ssim2(img, jd), 2), blocks=st)
                if q == 16: Image.fromarray(rec.astype(np.uint8)).save(f"{out}/{name}.{tag}.png"); Image.fromarray(jd.astype(np.uint8)).save(f"{out}/{name}.jxl_{tag}.png")
                print(name, tag, json.dumps(r[tag]), flush=True)
        report[name] = r
        json.dump(report, open(f"{out}/report.json", "w"), indent=1)
