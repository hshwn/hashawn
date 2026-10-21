#!/usr/bin/env python3
"""Grain mode check: is the regenerated grain like the real grain?

    GRAIN_OUT=<dir> nczcli grain 2 <a.ppm> ...      # writes <name>.sep.ppm (clean before coding), .clean.ppm, .recon.ppm
    uv run --with numpy --with scipy --with pillow python mode_eval.py <raw_dir> <dir> <name> [<name> ...]

real grain  = original - clean picture (the encoder's split)
synth grain = decoded output - decoded clean picture (what the decoder grew)
Scored on FLAT pixels (lowest 30% local gradient of the clean picture), where grain is what you see. Same metrics as
plate_lab.py: strength per brightness band, radial spectrum (dB), value shape (W1 on normalised values), channel
coupling, clumping (spread of 6x6 local energy), blobs above +1 sigma. Writes <name>_crops.png: original | grain mode |
clean picture, a flat crop over a busy crop, 3x.
"""
import json, os, sys
import numpy as np
from PIL import Image
from scipy import ndimage as ndi
from scipy.stats import wasserstein_distance

OPP = np.array([[1, 1, 1], [1, 0, -1], [1, -2, 1]], dtype=np.float64) / np.array([[np.sqrt(3)], [np.sqrt(2)], [np.sqrt(6)]])
PS = 32; NB = 16

def load(p): return np.asarray(Image.open(p).convert("RGB"), dtype=np.float64)

def flat_of(clean):
    y = clean.mean(-1); g = np.abs(np.diff(y, axis=1, append=y[:, -1:])) + np.abs(np.diff(y, axis=0, append=y[-1:]))
    m = ndi.uniform_filter(g, 7); return m <= np.quantile(m, 0.3)

def spectrum(f, flat):
    win = np.outer(np.hanning(PS), np.hanning(PS)); win /= np.sqrt((win ** 2).mean()); s = np.zeros((PS, PS, 3)); n = 0
    for r in range(0, f.shape[0] - PS + 1, PS // 2):
        for c in range(0, f.shape[1] - PS + 1, PS // 2):
            if flat[r:r + PS, c:c + PS].mean() < 0.9: continue
            b = f[r:r + PS, c:c + PS] - f[r:r + PS, c:c + PS].mean((0, 1))
            s += np.abs(np.fft.fft2(b * win[..., None], axes=(0, 1))) ** 2 / PS ** 2; n += 1
    return s / max(n, 1), n

def radial(s2):
    f = np.fft.fftfreq(PS); fy, fx = np.meshgrid(f, f, indexing="ij"); r = np.hypot(fy, fx)
    b = np.linspace(0, 0.5, 9); return np.array([s2[(r >= b[i]) & (r < b[i + 1])].mean() for i in range(8)])

def stats(g, clean, flat):
    o = g @ OPP.T; yb = np.clip((clean.mean(-1) / 256 * NB).astype(int), 0, NB - 1)
    st = np.full((NB, 3), np.nan)
    for b in range(NB):
        m = flat & (yb == b)
        if m.sum() > 300: st[b] = np.sqrt((o[m] ** 2).mean(0))
    sd = np.sqrt((o[flat] ** 2).mean(0)); u = o / sd
    spec, n = spectrum(u, flat)
    e = ndi.uniform_filter(u[..., 0] ** 2, 6); clump = float(e[flat].std() / e[flat].mean())
    lab, _ = ndi.label((u[..., 0] > 1.0) & flat); sizes = np.bincount(lab.ravel())[1:]; sizes = sizes[sizes > 0]
    cov = np.cov(u[flat].T)
    return dict(st=st, sd=sd, u=u, spec=spec, n=n, clump=clump, blob=(float(sizes.mean()), float(np.quantile(sizes, 0.9))), cov=cov)

def crop(a, r, c, s=96): return np.repeat(np.repeat(np.clip(a[r:r + s, c:c + s], 0, 255).astype(np.uint8), 3, 0), 3, 1)

raw, d = sys.argv[1], sys.argv[2]; report = {}
for name in sys.argv[3:]:
    orig = load(f"{raw}/{name}.ppm"); sep = load(f"{d}/{name}.sep.ppm"); clean = load(f"{d}/{name}.clean.ppm"); recon = load(f"{d}/{name}.recon.ppm")
    if np.array_equal(recon, clean): print(name, "grain-free (no grain measured, coded as +-k only)"); report[name] = "grain-free"; continue
    flat = flat_of(sep)
    R = stats(orig - sep, sep, flat); S = stats(recon - clean, clean, flat)
    ok = ~np.isnan(R["st"][:, 0]) & ~np.isnan(S["st"][:, 0]) & (R["st"][:, 0] > 0.05)
    rs, ss = R["st"][ok], S["st"][ok]; m = rs > 0.05      # channels pinned at 0 or 255 have no grain to compare
    strength = float(np.mean(np.abs(ss[m] - rs[m]) / rs[m]))
    spec_db = float(np.mean([np.sqrt(np.mean((10 * np.log10(radial(S["spec"][..., k]) / radial(R["spec"][..., k]))) ** 2)) for k in range(3)]))
    shape = float(np.mean([wasserstein_distance(R["u"][..., k][flat][::5], S["u"][..., k][flat][::5]) for k in range(3)]))
    rep = dict(real_rms_flat=[round(float(x), 2) for x in R["sd"]], synth_rms_flat=[round(float(x), 2) for x in S["sd"]],
               real_rms_all=round(float((orig - sep).std()), 2), synth_rms_all=round(float((recon - clean).std()), 2),
               strength_rel_err=round(strength, 3), spectrum_db=round(spec_db, 2), shape_w1=round(shape, 3),
               coupling=round(float(np.abs(S["cov"] - R["cov"]).max()), 3),
               clumping=[round(R["clump"], 3), round(S["clump"], 3)], blob_mean=[round(R["blob"][0], 2), round(S["blob"][0], 2)],
               blob_p90=[R["blob"][1], S["blob"][1]], spectrum_patches=R["n"])
    report[name] = rep; print(name, json.dumps(rep))
    # crops: flattest and busiest 96x96 windows (by flat fraction)
    ff = ndi.uniform_filter(flat.astype(float), 96)[48:-48, 48:-48]
    (r1, c1) = np.unravel_index(np.argmax(ff), ff.shape); (r2, c2) = np.unravel_index(np.argmin(ff), ff.shape)
    rows = [np.concatenate([crop(x, r, c) for x in (orig, recon, clean)], 1) for (r, c) in ((r1, c1), (r2, c2))]
    Image.fromarray(np.concatenate(rows, 0)).save(f"{d}/{name}_crops.png")
json.dump(report, open(f"{d}/mode_report.json", "w"), indent=1)
