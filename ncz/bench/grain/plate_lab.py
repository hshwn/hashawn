#!/usr/bin/env python3
"""Plate lab: real 35mm grain from uniformly exposed negatives (no picture to separate), regenerated three ways.

    uv run --python 3.12 --with "numpy<2" --with scipy --with tifffile --with pillow python plate_lab.py <out_dir> <scan> [<scan> ...]

Scans: Wikimedia Commons CC0 grain scans by Wiley.andrew.r (Nikon Coolscan V ED, ~4000 dpi, 16-bit): film exposed to a
white card, so every frame is pure grain at one density. A 2048x2048 centre crop; grain = scan - heavy blur (removes
vignetting/unevenness); dust specks (beyond 5 sigma, dilated) are masked out of the statistics.

Generators (all fitted to the real grain, same strength):
  av1:  AR lag 3 fitted per channel, 64x64 template, 32x32 tiles at random offsets (AV1's design)
  ours: whole-field noise shaped to the measured 2D spectrum, channel coupling, 1025-quantile value shape
  film: discs with lognormal radii scattered for 50% cover at 4x resolution, fitted (size, spread) to the spectrum,
        then the same coupling and value shape as ours
Scores vs real: spectrum error (dB), shape (Wasserstein), clumping (spread of local energy), repetition (autocorr
peak), and blobs: threshold at +1 sigma, connected components -> mean blob area and 90th-percentile area (px).
"""
import json, os, sys
import numpy as np
import tifffile
from PIL import Image
from scipy import ndimage as ndi
from scipy.stats import wasserstein_distance

out_dir = sys.argv[1]; paths = sys.argv[2:]; os.makedirs(out_dir, exist_ok=True)
rng = np.random.default_rng(7)
N = 2048; PS = 64; NQ = 1025; CLIP = 6.0

def load(p):
    a = tifffile.imread(p) if p.lower().endswith((".tif", ".tiff")) else np.asarray(Image.open(p))
    a = a.astype(np.float64) / 65535.0
    if a.ndim == 3: a = a[..., :3]                     # drop the scanner's infrared dust channel
    else: a = a[..., None]
    H, W, _ = a.shape; r, c = (H - N) // 2, (W - N) // 2
    return a[r:r + N, c:c + N]

def grain_of(a):
    g = a - np.stack([ndi.gaussian_filter(a[..., k], 24) for k in range(a.shape[-1])], -1)
    s = g.std((0, 1)); dust = (np.abs(g) > 5 * s).any(-1)
    dust = ndi.binary_dilation(dust, iterations=3)
    return g, ~dust

def spectrum(f, mask=None):
    """Mean 2D power spectrum of PSxPS patches (fully unmasked patches only), per channel."""
    H, W, C = f.shape; spec = np.zeros((PS, PS, C)); n = 0
    win = np.outer(np.hanning(PS), np.hanning(PS)); win /= np.sqrt((win ** 2).mean())
    for r in range(0, H - PS + 1, PS):
        for c in range(0, W - PS + 1, PS):
            if mask is not None and not mask[r:r + PS, c:c + PS].all(): continue
            b = f[r:r + PS, c:c + PS] - f[r:r + PS, c:c + PS].mean((0, 1))
            spec += np.abs(np.fft.fft2(b * win[..., None], axes=(0, 1))) ** 2 / (PS * PS); n += 1
    return spec / max(n, 1)

def radial(s2):
    f = np.fft.fftfreq(PS); fy, fx = np.meshgrid(f, f, indexing="ij"); r = np.hypot(fy, fx)
    b = np.linspace(0, 0.5, 13); return np.array([s2[(r >= b[i]) & (r < b[i + 1])].mean() for i in range(12)])

def unit_shape(x, quant):
    r = np.argsort(np.argsort(x.ravel())) / (x.size - 1)
    return np.interp(r, np.linspace(0, 1, NQ), quant).reshape(x.shape)

def couple_and_shape(f, fp):
    C = f.shape[-1]
    if C > 1:
        L = np.linalg.cholesky(fp["cov"] + 1e-12 * np.eye(C)); C0 = np.cov(f.reshape(-1, C).T); L0 = np.linalg.cholesky(C0 + 1e-12 * np.eye(C))
        f = (f.reshape(-1, C) @ np.linalg.inv(L0).T @ L.T).reshape(f.shape)
    for k in range(C): f[..., k] = unit_shape(f[..., k], fp["quant"][:, k])
    return f * fp["std"]

def fingerprint(g, ok):
    std = g[ok].std(0); u = np.clip(g / std, -CLIP, CLIP)
    flat = u[ok]
    return dict(std=std, spec=spectrum(u, ok), cov=np.cov(flat.T).reshape(g.shape[-1], g.shape[-1]),
                quant=np.quantile(flat, np.linspace(0, 1, NQ), axis=0)), u

def clump_of(f):
    e = ndi.uniform_filter(f ** 2, 6); return float(e.std() / e.mean())

def gen_ours_env(fp, C, target_clump):
    """ours + a slow random envelope (grain locally denser in places), envelope strength fitted to the real clumping."""
    base = gen_ours(fp, C) / fp["std"]
    m = ndi.gaussian_filter(rng.standard_normal((N, N)), 8); m /= m.std()
    best = None
    for a in (0.0, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.4, 0.5, 0.6):
        f = base * np.exp(a * m)[..., None]
        f0 = unit_shape(f[..., 0].copy(), fp["quant"][:, 0])          # judge clumping after the final reshaping
        c = clump_of(f0 / f0.std())
        if best is None or abs(c - target_clump) < abs(best[1] - target_clump): best = (a, c, f)
    f = best[2]
    for k in range(C): f[..., k] = unit_shape(f[..., k], fp["quant"][:, k])
    return f * fp["std"], best[0]

def gen_ours(fp, C):
    out = []
    for k in range(C):
        amp = np.sqrt(np.maximum(fp["spec"][..., k], 0)); ker = np.fft.fftshift(np.real(np.fft.ifft2(amp)))
        big = np.zeros((N, N)); big[:PS, :PS] = ker; big = np.roll(big, (-(PS // 2), -(PS // 2)), (0, 1))
        f = np.real(np.fft.ifft2(np.fft.fft2(rng.standard_normal((N, N))) * np.fft.fft2(big))); out.append(f / f.std())
    return couple_and_shape(np.stack(out, -1), fp)

def disc_field(r0, spread, S=4, n_side=N):
    rr = np.sort(r0 * np.exp(spread * rng.standard_normal(8)))
    lam = np.log(2) / (np.pi * np.mean(rr ** 2)); n = rng.poisson(lam * n_side * n_side)
    cls = rng.integers(0, 8, n); ys = (rng.random(n) * n_side * S).astype(int); xs = (rng.random(n) * n_side * S).astype(int)
    canvas = np.zeros((n_side * S, n_side * S), dtype=np.uint8)
    for k in range(8):
        rad = max(rr[k] * S, 0.5); R = int(np.ceil(rad)); dy, dx = np.mgrid[-R:R + 1, -R:R + 1]; m = dy ** 2 + dx ** 2 <= rad ** 2
        sel = cls == k
        for oy, ox in zip(dy[m], dx[m]): canvas[(ys[sel] + oy) % (n_side * S), (xs[sel] + ox) % (n_side * S)] = 1
    f = canvas.reshape(n_side, S, n_side, S).mean((1, 3)); f -= f.mean(); return f / max(f.std(), 1e-9)

def fit_film(fp):
    target = radial(fp["spec"][..., 0]); best = None
    for r0 in (0.4, 0.6, 0.9, 1.3, 1.8, 2.5, 3.5):
        for spread in (0.15, 0.4, 0.8):
            f = disc_field(r0, spread, n_side=512)[..., None]
            e = float(np.sqrt(np.mean((10 * np.log10(radial(spectrum(f)[..., 0]) / target)) ** 2)))
            if best is None or e < best[0]: best = (e, r0, spread)
    return best

def gen_film(fp, C, r0, spread):
    return couple_and_shape(np.stack([disc_field(r0, spread) for _ in range(C)], -1), fp)

def gen_av1(u_real, fp, C):
    lag = 3; offs = [(dy, dx) for dy in range(-lag, 1) for dx in range(-lag, lag + 1) if (dy < 0 or dx < 0)]
    tmpl = np.zeros((64, 64, C))
    for k in range(C):
        u = u_real[:512, :512, k]
        Y = u[lag:, lag:-lag].ravel()
        X = np.stack([u[lag + dy:u.shape[0] + dy, lag + dx:u.shape[1] - lag + dx].ravel() for dy, dx in offs], 1)
        coef, *_ = np.linalg.lstsq(X, Y, rcond=None); sd = np.std(Y - X @ coef)
        t = np.zeros((64 + 2 * lag, 64 + 2 * lag))
        for r in range(lag, t.shape[0]):
            for cc in range(lag, t.shape[1] - lag):
                t[r, cc] = sum(coef[i] * t[r + dy, cc + dx] for i, (dy, dx) in enumerate(offs)) + sd * rng.standard_normal()
        t = t[2 * lag:2 * lag + 64, lag:lag + 64]; tmpl[..., k] = (t - t.mean()) / t.std()
    g = np.zeros((N, N, C))
    for r in range(0, N, 32):
        for c in range(0, N, 32):
            oy, ox = rng.integers(0, 32), rng.integers(0, 32); g[r:r + 32, c:c + 32] = tmpl[oy:oy + 32, ox:ox + 32]
    return g * fp["std"]

def blobs(u0):
    lab, n = ndi.label(u0 > 1.0); sizes = np.bincount(lab.ravel())[1:]
    return float(sizes.mean()), float(np.quantile(sizes, 0.9))

def score(g, ok, fp_real):
    fp, u = fingerprint(g, ok); C = g.shape[-1]
    spec_db = float(np.mean([np.sqrt(np.mean((10 * np.log10(radial(fp["spec"][..., k]) / radial(fp_real["spec"][..., k]))) ** 2)) for k in range(C)]))
    ur = fp_real["u"]
    shape = float(np.mean([wasserstein_distance(ur[..., k][ok][::11], u[..., k][ok][::11]) for k in range(C)]))
    e = ndi.uniform_filter(u[..., 0] ** 2, 6); clump = float(e[ok].std() / e[ok].mean())
    y = u[..., 0] - u[..., 0].mean(); ac = np.fft.fftshift(np.real(np.fft.ifft2(np.abs(np.fft.fft2(y)) ** 2))); ac /= ac.max()
    cy = N // 2; ac[cy - 6:cy + 7, cy - 6:cy + 7] = 0
    bm, b90 = blobs(u[..., 0])
    coup = float(np.abs(fp["cov"] - fp_real["cov"]).max()) if C > 1 else 0.0
    return dict(spectrum_db=spec_db, shape_w1=shape, coupling=coup, clumping=clump, repetition=float(ac.max()), blob_mean_px=bm, blob_p90_px=b90)

def crop(g, std, s=160, z=2):
    x = g[:s, :s, :] / (6 * std) + 0.5; x = np.clip(x, 0, 1)
    x = (x * 255).astype(np.uint8); x = x[..., 0] if x.shape[-1] == 1 else x
    im = Image.fromarray(x); return np.asarray(im.resize((s * z, s * z), Image.NEAREST).convert("RGB"))

report = {}
for p in paths:
    name = os.path.basename(p).split(".")[0]
    a = load(p); g, ok = grain_of(a); C = g.shape[-1]
    fp, u = fingerprint(g, ok); fp["u"] = u
    ferr, r0, spread = fit_film(fp)
    real_score = score(g, ok, fp)
    env, env_a = gen_ours_env(fp, C, real_score["clumping"])         # target = the dust-masked real clumping
    gens = {"av1": gen_av1(u, fp, C), "ours": gen_ours(fp, C), "ours+env": env, "film": gen_film(fp, C, r0, spread)}
    ok_all = np.ones((N, N), bool)
    rep = {"real": real_score}
    for k, v in gens.items(): rep[k] = score(v, ok_all, fp)
    rep["film_fit"] = {"r0_px": r0, "spread": spread, "fit_db": ferr}; rep["envelope_strength"] = env_a
    rep["grain_rms_8bit_levels"] = [float(x * 255) for x in fp["std"]]
    report[name] = rep
    print(f"{name}: {C} channel(s), grain rms {', '.join(f'{x * 255:.1f}' for x in fp['std'])} (8-bit levels), dust masked {100 * (1 - ok.mean()):.1f}%")
    print(f"   film fit: grain radius {r0} px, spread {spread} ({ferr:.2f} dB); envelope strength {env_a}")
    for k in ["real", "av1", "ours", "ours+env", "film"]:
        s = rep[k]; print(f"   {k:5s} spectrum {s['spectrum_db']:5.2f} dB  shape {s['shape_w1']:.3f}  coupling {s['coupling']:.3f}  clumping {s['clumping']:.3f}  repetition {s['repetition']:.3f}  blobs mean {s['blob_mean_px']:5.1f} px  p90 {s['blob_p90_px']:5.1f} px")
    std = fp["std"]
    row = np.concatenate([crop(g[300:], std), crop(gens['av1'], std), crop(gens['ours'], std), crop(gens['ours+env'], std)], 1)
    Image.fromarray(row).save(os.path.join(out_dir, f"{name}_plate_crops.png"))
json.dump(report, open(os.path.join(out_dir, "plate_report.json"), "w"), indent=1, default=float)
print("crops: real | AV1-style | ours | ours + envelope  (grain only, 2x)")
