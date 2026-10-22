#!/usr/bin/env python3
"""Wonder format, prototype 1: the finest detail band generated from the coarse picture by small networks.

    uv run --with torch --with numpy --with scipy --with pillow --with imagecodecs --with ssimulacra2 \
        python proto1.py <P.pt> <T.pt> <out_dir> <a.ppm> ...

Same pyramid coder as proto.py (real bytes through ncz's byte coder), but the finest brightness band is:
  keep    residual (true detail - P's prediction), quantised          -> P makes kept blocks cheaper
  drop    P's prediction, nothing stored
  regrow  T's texture (coarse picture + noise + stored block energy)  -> 1 byte per block
Configurations at each q: "plain" (no networks: keep/drop, like proto.py without regrowth), "P" (predictor, no
regrowth), "PT" (predictor + texture regrowth in low-coherence blocks). JPEG XL matched by size; SSIMULACRA2.
"""
import json, os, sys
import numpy as np, torch
from PIL import Image
import proto
from proto import nbytes, to_ycc, from_ycc, up, gauss_pyr, block_stats, BS, NLEV, ssim2, jxl_at

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import train   # network definitions (CPU here)
torch.set_num_threads(os.cpu_count())

def load(path, cin):
    n = train.Net(cin); n.load_state_dict(torch.load(path, map_location="cpu")); n.eval(); return n

def run_net(net, x, pad=16):
    # mirror-pad: the networks were trained on interior crops and draw garbage at picture borders otherwise
    with torch.no_grad():
        H, W = x.shape[-2:]; ph, pw = H % 2, W % 2
        t = torch.from_numpy(np.pad(x, ((0, 0), (pad, pad + ph), (pad, pad + pw)), mode="reflect")).float()[None]
        return net(t)[0, 0].numpy()[pad:pad + H, pad:pad + W]

def encode_decode(rgb, q, P=None, T=None, tau=0.55, seed=1, texture_min=1.0):
    ycc = to_ycc(rgb.astype(float)); H, W, _ = ycc.shape
    total = 16; st = {"keep": 0, "regrow": 0, "drop": 0}
    pyr = [gauss_pyr(ycc[..., c], NLEV) for c in range(3)]
    recs, preds = [], []
    # coarse levels (>= 1) of all channels first: the networks need the whole coarse picture
    for c in range(3):
        cw = 1.0 if c == 0 else 1.8; g = pyr[c]
        base = np.round(g[NLEV] / (q * 0.5 * cw)); total += nbytes(base); rec = base * q * 0.5 * cw
        for k in range(NLEV - 1, 0, -1):
            pred = up(rec, g[k].shape); d = g[k] - pred; step = q * cw * [1.0, 0.75, 0.6, 0.5][k]
            if k >= 2: qd = np.round(d / step); total += nbytes(qd); rec = pred + qd * step; continue
            e, coh = block_stats(d, g[k]); floor = step * 0.35
            mode = (e >= floor).astype(np.int8)
            mask = np.kron(mode == 1, np.ones((BS, BS)))[: d.shape[0], : d.shape[1]].astype(bool)
            qd = np.round(d / step) * mask; total += nbytes(qd) + nbytes(mode); rec = pred + qd * step
        recs.append(rec)
    coarse = np.stack([up(recs[c], pyr[c][0].shape) for c in range(3)], 0)           # 3,H,W
    out = coarse.copy()
    # finest brightness band
    g0 = pyr[0][0]; d = g0 - coarse[0]; step = q
    pnet = run_net(P, coarse / 64.0) if P is not None else np.zeros_like(d)
    e, coh = block_stats(d, g0); r_e, _ = block_stats(d - pnet, g0)
    nby, nbx = e.shape; mode = np.zeros((nby, nbx), np.int8)                          # 0 drop (use P), 1 keep, 2 regrow
    floor = step * 0.35
    for by in range(nby):
        for bx in range(nbx):
            if T is not None and coh[by, bx] < tau and e[by, bx] >= texture_min * step: mode[by, bx] = 2    # strong, unstructured detail only
            elif r_e[by, bx] < floor: mode[by, bx] = 0
            else: mode[by, bx] = 1
    keep = np.kron(mode == 1, np.ones((BS, BS)))[:H, :W].astype(bool)
    qd = np.round((d - pnet) / step) * keep; total += nbytes(qd) + nbytes(mode)
    fine = pnet + qd * step
    if (mode == 2).any():
        eq = np.clip(np.round(np.log2(np.maximum(e, 1e-4)) * 8), -64, 191)
        total += nbytes(eq * (mode == 2))
        rng = np.random.default_rng(seed); noise = rng.standard_normal((H, W))
        emap = np.kron(eq / 8.0, np.ones((BS, BS)))[:H, :W]
        tex = run_net(T, np.concatenate([coarse / 64.0, noise[None], emap[None] / 4.0], 0))
        rg = np.kron(mode == 2, np.ones((BS, BS)))[:H, :W].astype(bool)
        fine = np.where(rg, tex, fine)
    out[0] = coarse[0] + fine
    for m, kk in ((0, "drop"), (1, "keep"), (2, "regrow")): st[kk] = int((mode == m).sum())
    return np.clip(np.round(from_ycc(np.moveaxis(out, 0, -1))), 0, 255), total, st

if __name__ == "__main__":
    P, T = load(sys.argv[1], 3), load(sys.argv[2], 5)
    out = sys.argv[3]; os.makedirs(out, exist_ok=True); report = {}
    for p in sys.argv[4:]:
        name = os.path.basename(p).rsplit(".", 1)[0]; img = np.asarray(Image.open(p).convert("RGB"), dtype=float); n = img.shape[0] * img.shape[1]
        r = {}
        for q in (6, 10, 16, 24):
            for tag, nets in (("plain", (None, None)), ("P", (P, None)), ("PT", (P, T))):
                rec, nb, st = encode_decode(img, q, *nets)
                jb, jd = jxl_at(img.astype(np.uint8), nb)
                key = f"q{q}{tag}"
                r[key] = dict(bpp=round(8 * nb / n, 3), s2=round(ssim2(img, rec), 2), jxl_bpp=round(8 * jb / n, 3), jxl_s2=round(ssim2(img, jd), 2), blocks=st)
                if q in (10, 16): Image.fromarray(rec.astype(np.uint8)).save(f"{out}/{name}.{key}.png"); Image.fromarray(jd.astype(np.uint8)).save(f"{out}/{name}.jxl_{key}.png")
                print(name, key, json.dumps(r[key]), flush=True)
        report[name] = r
        json.dump(report, open(f"{out}/report.json", "w"), indent=1)
