#!/usr/bin/env python3
"""Grain lab: measure a photo's grain, regenerate it two ways, score both against the real grain.

    uv run --python 3.12 --with "numpy<2" --with scipy --with "scikit-image<0.25" --with pillow --with PyWavelets --with bm3d \
        python grain_lab.py <out_dir> <png> [<png> ...]

1. Separate: BM3D driven by the grain's own spectrum (see separate.py) -> clean picture + real grain. Cached.
2. Fingerprint (opponent colour: brightness Y, red-blue C1, green-magenta C2):
     strength: grain std per channel in 16 brightness bands of the clean picture
     spectrum: 2D power spectrum (32x32) of the brightness-normalised grain, per channel (clump size + direction)
     coupling: 3x3 covariance of the normalised grain across channels
     shape: 33 quantiles of each channel's normalised grain (film grain is not Gaussian)
3. Regenerate:
     av1:  AR model (lag 3, like AV1's), 64x64 grain template, 32x32 blocks at random offsets, strength by brightness
     ours: fresh grain for the whole image (no tiles), filtered to the measured 2D spectrum, coupled across channels,
           reshaped to the measured quantiles, strength per channel by brightness
4. Score vs the real grain: spectrum error (dB, radial), strength-curve error, coupling error, shape error
   (Wasserstein on normalised values), clumping (spread of local grain energy), and repetition (largest
   off-centre peak of the grain's autocorrelation; tiling shows up here). Writes 3x crops for viewing.
"""
import json, os, sys, time
import numpy as np
from PIL import Image
from scipy import ndimage as ndi
from scipy.stats import wasserstein_distance

out_dir = sys.argv[1]; paths = sys.argv[2:]
os.makedirs(out_dir, exist_ok=True)
rng = np.random.default_rng(1)
OPP = np.array([[1, 1, 1], [1, 0, -1], [1, -2, 1]], dtype=np.float64) / np.array([[np.sqrt(3)], [np.sqrt(2)], [np.sqrt(6)]])
OPPI = np.linalg.inv(OPP)
NB = 16                                      # brightness bands
FLOOR = 0.25 / 255                           # strength floor: bands with ~no grain must not blow up the normalised grain
CLIP = 6.0                                   # normalised grain is clipped to +-6 (beyond is not grain)
PS = 32                                      # spectrum patch size
NQ = 1025                                    # quantiles describing the grain's value distribution (tails need many)

def load(p): return np.asarray(Image.open(p).convert("RGB"), dtype=np.float64) / 255.0

def separate(img, cache):
    if os.path.exists(cache): return np.load(cache)
    import bm3d
    from skimage.restoration import denoise_nl_means, estimate_sigma
    sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
    from separate import grain_psd
    sig = estimate_sigma(img, channel_axis=-1, average_sigmas=True)
    pilot = denoise_nl_means(img, h=1.2 * sig, sigma=sig, patch_size=5, patch_distance=6, fast_mode=True, channel_axis=-1)
    full, _ = grain_psd(img, pilot)
    clean = bm3d.bm3d(img @ OPP.T, full) @ OPPI.T
    np.save(cache, clean); return clean

def bands(y): return np.clip((y * NB).astype(int), 0, NB - 1)

def fingerprint(clean, grain):
    g = grain @ OPP.T; y = clean @ np.array([0.299, 0.587, 0.114]); b = bands(y)
    strength = np.zeros((NB, 3)); cnt = np.bincount(b.ravel(), minlength=NB)
    for c in range(3):
        s2 = np.bincount(b.ravel(), weights=(g[..., c] ** 2).ravel(), minlength=NB)
        strength[:, c] = np.sqrt(s2 / np.maximum(cnt, 1))
    # fill empty / thin bands from neighbours
    for c in range(3):
        ok = cnt > 500
        strength[:, c] = np.interp(np.arange(NB), np.arange(NB)[ok], strength[ok, c]) if ok.any() else strength[:, c]
    strength = np.maximum(strength, FLOOR)
    unit = np.clip(g / strength[b], -CLIP, CLIP)
    H, W, _ = g.shape
    spec = np.zeros((PS, PS, 3)); n = 0
    win = np.outer(np.hanning(PS), np.hanning(PS)); win /= np.sqrt((win ** 2).mean())
    for r in range(0, H - PS, PS // 2):
        for c0 in range(0, W - PS, PS // 2):
            blk = unit[r:r + PS, c0:c0 + PS]; blk = blk - blk.mean((0, 1))
            spec += np.abs(np.fft.fft2(blk * win[..., None], axes=(0, 1))) ** 2 / (PS * PS); n += 1
    spec /= n
    flat = unit.reshape(-1, 3)
    cov = np.cov(flat.T)
    quant = np.quantile(flat, np.linspace(0, 1, NQ), axis=0)
    return dict(strength=strength, spec=spec, cov=cov, quant=quant)

def to_unit_shape(x, quant):
    """Rank-map a field to the measured quantiles (keeps its spatial structure, changes its value distribution)."""
    r = np.argsort(np.argsort(x.ravel())) / (x.size - 1)
    return np.interp(r, np.linspace(0, 1, NQ), quant).reshape(x.shape)

def synth_ours(clean, fp):
    H, W, _ = clean.shape
    # white noise -> per-channel filter by sqrt of the measured spectrum (upsampled from 32x32 by its kernel)
    fields = []
    for c in range(3):
        k = np.real(np.fft.ifft2(np.sqrt(np.maximum(fp["spec"][..., c], 0))))     # amplitude kernel (periodic, 32x32)
        k = np.fft.fftshift(k)
        big = np.zeros((H, W)); big[:PS, :PS] = k; big = np.roll(big, (-(PS // 2), -(PS // 2)), (0, 1))
        wn = rng.standard_normal((H, W))
        f = np.real(np.fft.ifft2(np.fft.fft2(wn) * np.fft.fft2(big)))
        fields.append(f / f.std())
    f = np.stack(fields, -1)
    # couple channels to the measured covariance (decorrelate, then colour)
    L = np.linalg.cholesky(fp["cov"] + 1e-9 * np.eye(3))
    C0 = np.cov(f.reshape(-1, 3).T); L0 = np.linalg.cholesky(C0 + 1e-12 * np.eye(3))
    f = (f.reshape(-1, 3) @ np.linalg.inv(L0).T @ L.T).reshape(H, W, 3)
    for c in range(3): f[..., c] = to_unit_shape(f[..., c], fp["quant"][:, c])
    y = clean @ np.array([0.299, 0.587, 0.114]); b = bands(y)
    return (f * fp["strength"][b]) @ OPPI.T

def film_field(H, W, r0, spread, S=4):
    """Physical grain: discs (lognormal radii, mean r0 px) scattered so half the area is covered, rendered at Sx
    resolution, averaged to pixels. Returns a zero-mean, unit-std field."""
    r = r0 * np.exp(spread * rng.standard_normal(8)); r = np.sort(r)                 # 8 size classes
    lam = np.log(2) / (np.pi * np.mean(r ** 2))                                       # grains per pixel for 50% cover
    n = rng.poisson(lam * H * W)
    cls = rng.integers(0, 8, n)
    ys = rng.random(n) * H * S; xs = rng.random(n) * W * S
    canvas = np.zeros((H * S, W * S), dtype=np.uint8)
    for k in range(8):
        rad = max(r[k] * S, 0.5); R = int(np.ceil(rad))
        dy, dx = np.mgrid[-R:R + 1, -R:R + 1]; m = dy ** 2 + dx ** 2 <= rad ** 2
        oy, ox = dy[m].ravel(), dx[m].ravel()
        sel = cls == k; cy = ys[sel].astype(int); cx = xs[sel].astype(int)
        yy = (cy[:, None] + oy[None, :]) % (H * S); xx = (cx[:, None] + ox[None, :]) % (W * S)
        canvas[yy.ravel(), xx.ravel()] = 1
    f = canvas.reshape(H, S, W, S).mean((1, 3))
    f = f - f.mean(); return f / max(f.std(), 1e-9)

def radial_of(field):
    H, W = field.shape; spec = np.zeros((PS, PS)); n = 0
    win = np.outer(np.hanning(PS), np.hanning(PS)); win /= np.sqrt((win ** 2).mean())
    for r in range(0, H - PS, PS):
        for c in range(0, W - PS, PS):
            b = field[r:r + PS, c:c + PS]; b = b - b.mean(); spec += np.abs(np.fft.fft2(b * win)) ** 2 / (PS * PS); n += 1
    return radial(spec / n)

def fit_film(fp, H=256, W=256):
    """Grain size and size spread whose spectrum best matches the measured brightness-grain spectrum."""
    target = radial(fp["spec"][..., 0]); best = None
    for r0 in (0.25, 0.35, 0.5, 0.7, 1.0, 1.4):
        for spread in (0.1, 0.35, 0.7):
            e = np.sqrt(np.mean((10 * np.log10(radial_of(film_field(H, W, r0, spread)) / target)) ** 2))
            if best is None or e < best[0]: best = (e, r0, spread)
    return best

def synth_film(clean, fp, r0, spread):
    """Physical grain per colour layer, coupled across channels, reshaped and scaled like `ours`."""
    H, W, _ = clean.shape
    f = np.stack([film_field(H, W, r0, spread) for _ in range(3)], -1)
    L = np.linalg.cholesky(fp["cov"] + 1e-9 * np.eye(3))
    C0 = np.cov(f.reshape(-1, 3).T); L0 = np.linalg.cholesky(C0 + 1e-12 * np.eye(3))
    f = (f.reshape(-1, 3) @ np.linalg.inv(L0).T @ L.T).reshape(H, W, 3)
    for c in range(3): f[..., c] = to_unit_shape(f[..., c], fp["quant"][:, c])
    y = clean @ np.array([0.299, 0.587, 0.114]); b = bands(y)
    return (f * fp["strength"][b]) @ OPPI.T

def synth_av1(clean, unit_real, fp):
    """AV1-style: AR(lag 3) fitted on the real normalised grain, 64x64 template, 32x32 blocks, strength by brightness."""
    H, W, _ = clean.shape; lag = 3
    offs = [(dy, dx) for dy in range(-lag, 1) for dx in range(-lag, lag + 1) if (dy < 0 or dx < 0)]
    tmpl = np.zeros((64 + lag, 64 + 2 * lag, 3))
    for c in range(3):
        u = unit_real[..., c]
        Y = u[lag:, lag:-lag].ravel()
        X = np.stack([u[lag + dy:u.shape[0] + dy, lag + dx:u.shape[1] - lag + dx].ravel() for dy, dx in offs], 1)
        coef, *_ = np.linalg.lstsq(X, Y, rcond=None)
        resid = np.std(Y - X @ coef)
        t = np.zeros((64 + 2 * lag, 64 + 2 * lag))
        for r in range(lag, t.shape[0]):
            for cc in range(lag, t.shape[1] - lag):
                t[r, cc] = sum(coef[i] * t[r + dy, cc + dx] for i, (dy, dx) in enumerate(offs)) + resid * rng.standard_normal()
        t = t[2 * lag:2 * lag + 64, lag:lag + 64]
        tmpl[:64, :64, c] = t / max(t.std(), 1e-9)
    g = np.zeros((H, W, 3))
    for r in range(0, H, 32):
        for c0 in range(0, W, 32):
            oy, ox = rng.integers(0, 32), rng.integers(0, 32)
            blk = tmpl[oy:oy + 32, ox:ox + 32]
            g[r:r + 32, c0:c0 + 32] = blk[: min(32, H - r), : min(32, W - c0)]
    y = clean @ np.array([0.299, 0.587, 0.114]); b = bands(y)
    return (g * fp["strength"][b]) @ OPPI.T

def radial(spec2d):
    f = np.fft.fftfreq(PS); fy, fx = np.meshgrid(f, f, indexing="ij"); r = np.hypot(fy, fx)
    bins = np.linspace(0, 0.5, 9); out = []
    for i in range(8):
        m = (r >= bins[i]) & (r < bins[i + 1]); out.append(spec2d[m].mean())
    return np.array(out)

def clumping(unit):
    """Spread of local grain energy: clumpy grain has bursts of energy, smooth noise doesn't."""
    e = ndi.uniform_filter(unit[..., 0] ** 2, 4)
    return float(e.std() / max(e.mean(), 1e-9))

def repetition(g):
    """Largest autocorrelation peak away from the centre (tiling leaves periodic peaks)."""
    y = g[..., 0] - g[..., 0].mean()
    ac = np.real(np.fft.ifft2(np.abs(np.fft.fft2(y)) ** 2)); ac /= ac[0, 0]
    ac = np.fft.fftshift(ac); cy, cx = np.array(ac.shape) // 2
    ac[cy - 4:cy + 5, cx - 4:cx + 5] = 0
    return float(ac.max())

def score(clean, real, synth, fp_real):
    fp_s = fingerprint(clean, synth)
    spec_db = float(np.mean([np.sqrt(np.mean((10 * np.log10(radial(fp_s["spec"][..., c]) / radial(fp_real["spec"][..., c]))) ** 2)) for c in range(3)]))
    strength = float(np.mean(np.abs(fp_s["strength"] - fp_real["strength"]) / np.maximum(fp_real["strength"], 1e-9)))
    coupling = float(np.abs(fp_s["cov"] - fp_real["cov"]).max())
    yb = bands(clean @ np.array([0.299, 0.587, 0.114]))
    ur = np.clip((real @ OPP.T) / fp_real["strength"][yb], -CLIP, CLIP); us = np.clip((synth @ OPP.T) / fp_s["strength"][yb], -CLIP, CLIP)
    shape = float(np.mean([wasserstein_distance(ur[..., c].ravel()[::7], us[..., c].ravel()[::7]) for c in range(3)]))
    return dict(spectrum_db=spec_db, strength_rel=strength, coupling=coupling, shape_w1=shape, clumping=clumping(us), repetition=repetition(synth)), ur

def crop3(a, r, c, s=96):
    x = np.clip(a[r:r + s, c:c + s], 0, 1); x = (x * 255 + 0.5).astype(np.uint8)
    return np.asarray(Image.fromarray(x).resize((s * 3, s * 3), Image.NEAREST))

report = {}
for p in paths:
    name = os.path.basename(p)[:-4]; t0 = time.time()
    img = load(p); clean = separate(img, os.path.join(out_dir, f"{name}_clean.npy"))
    real = img - clean
    fp = fingerprint(clean, real)
    yb = bands(clean @ np.array([0.299, 0.587, 0.114]))
    unit_real = np.clip((real @ OPP.T) / fp["strength"][yb], -CLIP, CLIP)
    s_av1 = synth_av1(clean, unit_real, fp); s_ours = synth_ours(clean, fp)
    ferr, r0, spread = fit_film(fp); s_film = synth_film(clean, fp, r0, spread)
    real_scores, _ = score(clean, real, real, fp)
    r_av1, _ = score(clean, real, s_av1, fp); r_ours, _ = score(clean, real, s_ours, fp); r_film, _ = score(clean, real, s_film, fp)
    report[name] = {"grain_rms_levels": float(real.std() * 255), "film_fit": {"r0_px": r0, "spread": spread, "fit_db": ferr}, "real": real_scores, "av1": r_av1, "ours": r_ours, "film": r_film}
    print(f"{name}: grain rms {real.std() * 255:.2f} levels  ({time.time() - t0:.0f}s)")
    print(f"   film grain fit: mean grain radius {r0} px, size spread {spread} (fit {ferr:.2f} dB)")
    for k, v in [("real", real_scores), ("av1-style", r_av1), ("ours", r_ours), ("film", r_film)]:
        print(f"   {k:10s} spectrum {v['spectrum_db']:5.2f} dB  strength {v['strength_rel']:5.3f}  coupling {v['coupling']:5.3f}  shape {v['shape_w1']:5.3f}  clumping {v['clumping']:5.3f}  repetition {v['repetition']:5.3f}")
    # crops: the flattest-but-mid-brightness region, where grain shows
    y = clean @ np.array([0.299, 0.587, 0.114]); gm = ndi.uniform_filter(np.hypot(ndi.sobel(y, 0), ndi.sobel(y, 1)), 32)
    score_map = gm + 2 * np.abs(y - 0.45)
    r, c = np.unravel_index(np.argmin(score_map[48:-144, 48:-144]), score_map[48:-144, 48:-144].shape); r += 48; c += 48
    row = np.concatenate([crop3(img, r, c), crop3(clean, r, c), crop3(clean + s_av1, r, c), crop3(clean + s_ours, r, c), crop3(clean + s_film, r, c)], 1)
    Image.fromarray(row).save(os.path.join(out_dir, f"{name}_crops.png"))
json.dump(report, open(os.path.join(out_dir, "grain_report.json"), "w"), indent=1)
print("crops: original | clean | clean + AV1-style grain | clean + our grain | clean + film-model grain")
