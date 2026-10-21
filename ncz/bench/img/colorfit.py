#!/usr/bin/env python3
"""Learned colour-from-brightness for ncz's pixel mode (planar order: the whole green plane is known).

For red (then blue) at each pixel the network sees:
  - green in the 5x5 window around the pixel, relative to this pixel's green (ahead of it too: green is
    coded first), 24 values
  - this channel's colour offset (value - green) at the 6 already-coded neighbours W N NW NE WW NN
  - blue only: red's offset at this pixel and at W and N (red is coded before blue)
  - variant "thumb": the global hint, this channel's offset averaged over 16x16 blocks (what the encoder
    would send), bilinearly read at the pixel, 1 value
and predicts this pixel's offset plus its own uncertainty (Laplace scale), trained by Laplace NLL.

Train on COCO photos downsampled 2x (removes most JPEG artefacts); test on Kodak only (never trained on).
Reports bits per sample of the learned Laplace model on Kodak (an estimate of coding cost) next to the same
estimate for the hand-made colour predictor (fit colour = a*green + b over the neighbours, as in img.rs), and
exports weights (f32 LE) for the Rust coder.

    python colorfit.py <coco_dir> <kodak_ppm_dir> <out_dir>
"""
import glob, json, math, os, sys, time
import numpy as np
import torch, torch.nn as nn
from PIL import Image

coco_dir, kodak_dir, out_dir = sys.argv[1:4]
SPECS = [tuple(x.split(":")) for x in (sys.argv[4] if len(sys.argv) > 4 else "red:32:6000,blue:32:6000").split(",")]
os.makedirs(out_dir, exist_ok=True)
dev = "cuda" if torch.cuda.is_available() else "cpu"
torch.manual_seed(0); np.random.seed(0)
R = 2  # window radius

def load_coco(p):
    im = Image.open(p).convert("RGB")
    w, h = im.size
    im = im.resize((w // 2, h // 2), Image.BOX)
    return np.asarray(im, dtype=np.int32)

def load_ppm(p):
    b = open(p, "rb").read(); parts = b.split(maxsplit=4)
    w, h = int(parts[1]), int(parts[2])
    return np.frombuffer(parts[4][: w * h * 3], np.uint8).reshape(h, w, 3).astype(np.int32)

def thumb(off, bs=16):
    """Block means of an offset plane (int), bilinear upsampled back to pixel resolution."""
    h, w = off.shape; th, tw = (h + bs - 1) // bs, (w + bs - 1) // bs
    pad = np.pad(off, ((0, th * bs - h), (0, tw * bs - w)), mode="edge").astype(np.float64)
    t = np.round(pad.reshape(th, bs, tw, bs).mean((1, 3)))
    ys = (np.arange(h) + 0.5) / bs - 0.5; xs = (np.arange(w) + 0.5) / bs - 0.5
    y0 = np.clip(np.floor(ys).astype(int), 0, th - 1); x0 = np.clip(np.floor(xs).astype(int), 0, tw - 1)
    y1 = np.clip(y0 + 1, 0, th - 1); x1 = np.clip(x0 + 1, 0, tw - 1)
    fy = np.clip(ys - y0, 0, 1)[:, None]; fx = np.clip(xs - x0, 0, 1)[None, :]
    a = t[y0][:, x0]; b_ = t[y0][:, x1]; c = t[y1][:, x0]; d = t[y1][:, x1]
    return a * (1 - fy) * (1 - fx) + b_ * (1 - fy) * fx + c * fy * (1 - fx) + d * fy * fx

GREEN_NB = [(0, -1), (0, -2), (-1, -2), (-1, -1), (-1, 0), (-1, 1), (-1, 2), (-2, -2), (-2, -1), (-2, 0), (-2, 1), (-2, 2)]
def green_features(img, frac=1.0):
    """Causal: green at the 12 already-coded neighbours (two rows above, two to the left), relative to the
    anchor (W + N) / 2; target = green - anchor. Hand baseline = MED (median edge detector) prediction."""
    g = img[..., 1]; h, w = g.shape
    ys, xs = np.mgrid[R:h, R:w - R]; ys, xs = ys.ravel(), xs.ravel()
    if frac < 1.0:
        keep = np.random.rand(len(ys)) < frac; ys, xs = ys[keep], xs[keep]
    wv, nv, nw = g[ys, xs - 1], g[ys - 1, xs], g[ys - 1, xs - 1]
    anchor = (wv + nv) // 2
    X = np.stack([g[ys + dy, xs + dx] - anchor for dy, dx in GREEN_NB], 1).astype(np.float32) / 64.0
    y = (g[ys, xs] - anchor).astype(np.float32)
    med = np.where(nw >= np.maximum(wv, nv), np.minimum(wv, nv), np.where(nw <= np.minimum(wv, nv), np.maximum(wv, nv), wv + nv - nw))
    return X, y, (med - anchor).astype(np.float32)

def features(img, ch, with_thumb, frac=1.0):
    """Rows for every interior pixel (radius 2 border skipped) of channel ch (0 = red, 2 = blue)."""
    g = img[..., 1]; c = img[..., ch]; off = c - g
    h, w = g.shape
    ys, xs = np.mgrid[R:h - R, R:w - R]
    ys, xs = ys.ravel(), xs.ravel()
    if frac < 1.0:
        keep = np.random.rand(len(ys)) < frac; ys, xs = ys[keep], xs[keep]
    gc = g[ys, xs]
    cols = []
    for dy in range(-R, R + 1):
        for dx in range(-R, R + 1):
            if dy == 0 and dx == 0: continue
            cols.append(g[ys + dy, xs + dx] - gc)
    for dy, dx in [(0, -1), (-1, 0), (-1, -1), (-1, 1), (0, -2), (-2, 0)]:
        cols.append(off[ys + dy, xs + dx])
    if ch == 2:
        ro = img[..., 0] - g
        for dy, dx in [(0, 0), (0, -1), (-1, 0)]: cols.append(ro[ys + dy, xs + dx])
    if with_thumb:
        cols.append(thumb(off)[ys, xs])
    X = np.stack(cols, 1).astype(np.float32) / 64.0
    y = off[ys, xs].astype(np.float32)
    # the hand-made predictor from img.rs: fit colour = a*green + b over W N NW NE WW NN
    nb = [(0, -1), (-1, 0), (-1, -1), (-1, 1), (0, -2), (-2, 0)]
    G = np.stack([g[ys + dy, xs + dx] for dy, dx in nb], 1).astype(np.int64)
    C = np.stack([c[ys + dy, xs + dx] for dy, dx in nb], 1).astype(np.int64)
    n = 6; sg, sr = G.sum(1), C.sum(1); sgg, sgr = (G * G).sum(1), (G * C).sum(1)
    den = n * sgg - sg * sg
    a = np.where(den > n * n * 16, np.clip((n * sgr - sg * sr) * 256 // np.maximum(den, 1), -512, 1024), 0)
    bb = (sr * 256 - a * sg) // n
    fit = np.where(den > n * n * 16, (a * gc + bb + 128) >> 8, gc + (sr - sg) // n)
    hand = (fit - gc).astype(np.float32)
    return X, y, hand

class Net(nn.Module):  # noqa
    def __init__(self, nin, hid=32):
        super().__init__()
        self.f = nn.Sequential(nn.Linear(nin, hid), nn.ReLU(), nn.Linear(hid, hid), nn.ReLU(), nn.Linear(hid, 2))
    def forward(self, x):
        o = self.f(x); return o[:, 0] * 64.0, o[:, 1]          # mean offset, log Laplace scale

def laplace_bits(y, mu, logb):
    """Bits per sample of integer y under a Laplace(mu, b) discretised to unit bins (CDF difference)."""
    b = torch.exp(logb).clamp(min=0.05)
    def cdf(t): return torch.where(t < mu, 0.5 * torch.exp((t - mu) / b), 1 - 0.5 * torch.exp(-(t - mu) / b))
    p = (cdf(y + 0.5) - cdf(y - 0.5)).clamp(min=1e-9)
    return -torch.log2(p)

def hand_bits(y, pred):
    """Same estimate for the hand-made predictor with one global Laplace scale. Pessimistic for it: the real
    coder adapts its scale per context. The decisive number is the coder's bpp, not this."""
    e = (y - pred); b = e.abs().mean().clamp(min=0.05)
    mu = pred
    return laplace_bits(y, mu, torch.log(b).expand_as(y)).mean().item()

coco = sorted(glob.glob(os.path.join(coco_dir, "*.jpg")))
kodak = sorted(glob.glob(os.path.join(kodak_dir, "kodak_*.ppm")))
print(f"train images {len(coco)}, test images {len(kodak)}, device {dev}", flush=True)
t0 = time.time()
train_imgs = [load_coco(p) for p in coco]
test_imgs = [load_ppm(p) for p in kodak]
print(f"loaded in {time.time() - t0:.0f}s", flush=True)

report = {}
for spec in SPECS:
    name, hid, steps = spec[0], int(spec[1]), int(spec[2])
    tag = f"{name}_h{hid}"
    if name == "green":
        fx = lambda im, frac=1.0: green_features(im, frac)
    else:
        ch = 0 if name.startswith("red") else 2; th_ = name.endswith("thumb")
        fx = lambda im, frac=1.0, ch=ch, th_=th_: features(im, ch, th_, frac)
    Xs, ys = [], []
    for im in train_imgs:
        X, y, _ = fx(im, 0.15); Xs.append(X); ys.append(y)
    X = torch.from_numpy(np.concatenate(Xs)).to(dev); y = torch.from_numpy(np.concatenate(ys)).to(dev)
    tX, ty, th = zip(*[fx(im) for im in test_imgs])
    tX = torch.from_numpy(np.concatenate(tX)).to(dev); ty = torch.from_numpy(np.concatenate(ty)).to(dev); th = torch.from_numpy(np.concatenate(th)).to(dev)
    net = Net(X.shape[1], hid).to(dev)
    opt = torch.optim.Adam(net.parameters(), lr=2e-3)
    bs = 8192
    sched = torch.optim.lr_scheduler.CosineAnnealingLR(opt, steps)
    t1 = time.time()
    for step in range(steps):
        i = torch.randint(0, len(y), (bs,), device=dev)
        mu, logb = net(X[i]); b = torch.exp(logb).clamp(min=0.05)
        loss = ((y[i] - mu).abs() / b + torch.log(b)).mean()
        opt.zero_grad(); loss.backward(); opt.step(); sched.step()
    with torch.no_grad():
        bits, maes = [], []
        for j in range(0, len(ty), 1 << 18):
            mu, logb = net(tX[j:j + (1 << 18)]); bits.append(laplace_bits(ty[j:j + (1 << 18)], mu, logb)); maes.append((mu - ty[j:j + (1 << 18)]).abs())
        bits = torch.cat(bits).mean().item(); mae = torch.cat(maes).mean().item()
        hb = hand_bits(ty, th); hmae = (th - ty).abs().mean().item()
    report[tag] = {"train_rows": len(y), "test_rows": len(ty), "learned_bits": bits, "learned_mae": mae, "hand_bits_global_scale": hb, "hand_mae": hmae, "secs": time.time() - t1}
    print(f"{tag:16s} learned {bits:.3f} bits/sample (MAE {mae:.2f})   hand-made {hb:.3f} bits/sample (MAE {hmae:.2f})   [{len(y)/1e6:.1f}M rows, {steps} steps, {time.time()-t1:.0f}s]", flush=True)
    with open(os.path.join(out_dir, f"net_{tag}.bin"), "wb") as f:
        for m in net.f:
            if isinstance(m, nn.Linear):
                f.write(np.array(m.weight.shape, np.uint32).tobytes())
                f.write(m.weight.detach().cpu().numpy().astype("<f4").tobytes()); f.write(m.bias.detach().cpu().numpy().astype("<f4").tobytes())
json.dump(report, open(os.path.join(out_dir, "fit_" + "_".join(t for t in report) + ".json"), "w"), indent=1)
