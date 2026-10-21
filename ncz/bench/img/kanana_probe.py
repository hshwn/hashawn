#!/usr/bin/env python3
"""Does a pretrained vision encoder help code the fill-in pixels of a coarse-first image coder?

Coarse-first: the quarter image Q = I[0::2, 0::2] (exact pixels) is coded first; the other 3/4 of the
pixels are then predicted from it. For each fill-in sample (pixel x channel) the baseline guess is the mean
of its nearest quarter-image pixels; a small head predicts the residual's mean and Laplace scale. The head
is trained four ways, same seed, steps and size, differing only in its extra input:
  A  local: the 4x4 quarter-image neighbourhood of that channel relative to the guess, position type, channel
  B  A + Kanana-2 vision encoder tiny features of Q (512-d, one per 32x32 block of Q)
  C  A + the same architecture with RANDOM weights (is it the pretraining, or just more inputs?)
  D  A + a raw 4x4x3 thumbnail of the same 32x32 block (is it more than "more pixels"?)
Score: Laplace bits per sample on Kodak fill-in samples (never trained on). Train: COCO photos downsampled 2x.

    python kanana_probe.py <coco_dir> <kodak_ppm_dir> <model_id_or_dir>
"""
import glob, json, os, sys, time
import numpy as np
import torch, torch.nn as nn
from PIL import Image
from transformers import AutoModel, AutoImageProcessor, AutoConfig

coco_dir, kodak_dir, mid = sys.argv[1:4]
dev = "cuda"
torch.manual_seed(0); np.random.seed(0)

def load_coco(p):
    im = Image.open(p).convert("RGB"); w, h = im.size
    return np.asarray(im.resize((w // 2, h // 2), Image.BOX), dtype=np.int32)
def load_ppm(p):
    b = open(p, "rb").read(); parts = b.split(maxsplit=4); w, h = int(parts[1]), int(parts[2])
    return np.frombuffer(parts[4][: w * h * 3], np.uint8).reshape(h, w, 3).astype(np.int32)

proc = AutoImageProcessor.from_pretrained(mid, trust_remote_code=True)
enc_pre = AutoModel.from_pretrained(mid, trust_remote_code=True, dtype=torch.bfloat16).to(dev).eval()
cfg = AutoConfig.from_pretrained(mid, trust_remote_code=True)
torch.manual_seed(1)
enc_rnd = AutoModel.from_config(cfg, trust_remote_code=True).to(torch.bfloat16).to(dev).eval()
torch.manual_seed(0)

@torch.no_grad()
def encode(enc, q):
    """Feature grid (gh, gw, 512) for quarter image q, plus the resized size it saw."""
    inputs = proc([Image.fromarray(q.astype(np.uint8))], return_tensors="pt").to(dev)
    out = enc(**inputs).last_hidden_state.float()
    thw = inputs["image_grid_thw"][0].tolist(); gh, gw = thw[1], thw[2]
    if out.shape[0] == gh * gw: pass
    elif out.shape[0] == (gh // 2) * (gw // 2): gh, gw = gh // 2, gw // 2
    else: raise SystemExit(f"unexpected token count {out.shape} for grid {thw}")
    return out.reshape(gh, gw, -1).cpu().numpy().astype(np.float16)

def rows(img, frac):
    """Fill-in samples of one image: local features, target residual, (gy, gx) block coords in Q-space."""
    h, w, _ = img.shape; h -= h % 2; w -= w % 2; img = img[:h, :w]
    q = img[0::2, 0::2]; qh, qw, _ = q.shape
    ys, xs = np.mgrid[0:h, 0:w]; m = ((ys % 2) | (xs % 2)).astype(bool)
    ys, xs = ys[m], xs[m]
    if frac < 1: k = np.random.rand(len(ys)) < frac; ys, xs = ys[k], xs[k]
    typ = (ys % 2) * 2 + (xs % 2)                        # 1 = right of a Q pixel, 2 = below, 3 = diagonal centre
    qy0, qx0 = ys // 2, xs // 2
    qp = np.pad(q, ((2, 3), (2, 3), (0, 0)), mode="edge")
    feats, targ, chans = [], [], []
    for c in range(3):
        nb = np.stack([qp[qy0 + 2 + dy, qx0 + 2 + dx, c] for dy in (-1, 0, 1, 2) for dx in (-1, 0, 1, 2)], 1)   # 4x4 around
        a, b_, cc, d = qp[qy0 + 2, qx0 + 2, c], qp[qy0 + 2, qx0 + 3, c], qp[qy0 + 3, qx0 + 2, c], qp[qy0 + 3, qx0 + 3, c]
        guess = np.where(typ == 1, (a + b_) / 2, np.where(typ == 2, (a + cc) / 2, (a + b_ + cc + d) / 4))
        f = np.concatenate([(nb - guess[:, None]) / 64.0, np.eye(3)[typ - 1], np.eye(3)[[c] * len(ys)]], 1)
        feats.append(f.astype(np.float32)); targ.append((img[ys, xs, c] - guess).astype(np.float32)); chans.append(np.full(len(ys), c))
    return np.concatenate(feats), np.concatenate(targ), np.tile(qy0, 3), np.tile(qx0, 3), q

def thumb_grid(q, gh, gw):
    """Raw 4x4x3 thumbnail per block, block grid matching the encoder's (gh, gw)."""
    qh, qw, _ = q.shape
    out = np.zeros((gh, gw, 48), np.float16)
    for by in range(gh):
        for bx in range(gw):
            y0, y1 = by * qh // gh, max(by * qh // gh + 1, (by + 1) * qh // gh); x0, x1 = bx * qw // gw, max(bx * qw // gw + 1, (bx + 1) * qw // gw)
            blk = q[y0:y1, x0:x1].astype(np.float32)
            t = np.asarray(Image.fromarray(blk.astype(np.uint8)).resize((4, 4), Image.BOX), np.float32)
            out[by, bx] = ((t - t.mean()) / 64.0).ravel()
    return out

def build(paths, loader, frac):
    X, Y, BI, banks = [], [], [], {"B": [], "C": [], "D": []}; off = 0
    for p in paths:
        img = loader(p); f, t, qy, qx, q = rows(img, frac)
        gp = encode(enc_pre, q); gr = encode(enc_rnd, q); gh, gw, _ = gp.shape
        gd = thumb_grid(q, gh, gw)
        by = np.minimum(qy * gh // q.shape[0], gh - 1); bx = np.minimum(qx * gw // q.shape[1], gw - 1)
        BI.append(off + by * gw + bx); off += gh * gw
        banks["B"].append(gp.reshape(-1, gp.shape[-1])); banks["C"].append(gr.reshape(-1, gr.shape[-1])); banks["D"].append(gd.reshape(-1, 48))
        X.append(f); Y.append(t)
    return (torch.from_numpy(np.concatenate(X)).to(dev), torch.from_numpy(np.concatenate(Y)).to(dev), torch.from_numpy(np.concatenate(BI)).to(dev),
            {k: torch.from_numpy(np.concatenate(v)).float().to(dev) for k, v in banks.items()})

class Head(nn.Module):
    def __init__(self, nloc, nextra):
        super().__init__()
        self.proj = nn.Linear(nextra, 32) if nextra else None
        self.f = nn.Sequential(nn.Linear(nloc + (32 if nextra else 0), 64), nn.ReLU(), nn.Linear(64, 64), nn.ReLU(), nn.Linear(64, 2))
    def forward(self, x, e=None):
        if self.proj is not None: x = torch.cat([x, self.proj(e)], 1)
        o = self.f(x); return o[:, 0] * 64.0, o[:, 1]

def bits(y, mu, logb):
    b = torch.exp(logb).clamp(min=0.05)
    cdf = lambda t: torch.where(t < mu, 0.5 * torch.exp((t - mu) / b), 1 - 0.5 * torch.exp(-(t - mu) / b))
    return -torch.log2((cdf(y + 0.5) - cdf(y - 0.5)).clamp(min=1e-9))

t0 = time.time()
coco = sorted(glob.glob(os.path.join(coco_dir, "*.jpg"))); kodak = sorted(glob.glob(os.path.join(kodak_dir, "kodak_*.ppm")))
Xtr, Ytr, Btr, banks_tr = build(coco, load_coco, 0.05)
Xte, Yte, Bte, banks_te = build(kodak, load_ppm, 1.0)
print(f"rows: train {len(Ytr)/1e6:.1f}M, test {len(Yte)/1e6:.1f}M; encoder feature dim {banks_tr['B'].shape[1]}; built in {time.time()-t0:.0f}s", flush=True)

res = {}
for name in ["A", "B", "C", "D"]:
    torch.manual_seed(0)
    nextra = 0 if name == "A" else banks_tr[name].shape[1]
    net = Head(Xtr.shape[1], nextra).to(dev); opt = torch.optim.Adam(net.parameters(), lr=2e-3)
    steps, bs = 20000, 8192; sched = torch.optim.lr_scheduler.CosineAnnealingLR(opt, steps)
    for s in range(steps):
        i = torch.randint(0, len(Ytr), (bs,), device=dev)
        e = None if name == "A" else banks_tr[name][Btr[i]]
        mu, lb = net(Xtr[i], e); b = torch.exp(lb).clamp(min=0.05)
        loss = ((Ytr[i] - mu).abs() / b + torch.log(b)).mean()
        opt.zero_grad(); loss.backward(); opt.step(); sched.step()
    with torch.no_grad():
        tot = []
        for j in range(0, len(Yte), 1 << 18):
            sl = slice(j, j + (1 << 18)); e = None if name == "A" else banks_te[name][Bte[sl]]
            mu, lb = net(Xte[sl], e); tot.append(bits(Yte[sl], mu, lb))
        r = torch.cat(tot).mean().item()
    res[name] = r
    print(f"{name}: {r:.4f} bits/sample on Kodak fill-in samples", flush=True)
for k in "BCD": print(f"{k} vs A: {100 * (res[k] / res['A'] - 1):+.2f}%")
json.dump(res, open("kanana_probe.json", "w"), indent=1)
