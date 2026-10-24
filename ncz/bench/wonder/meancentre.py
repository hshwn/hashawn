#!/usr/bin/env python3
"""Would mean-centred rounding pay? Kodak 8, our predictor, Gaussian bits per pixel (the coder lands near these).

    python meancentre.py <teacher.pth.tar> <pred.pt> <kodak_dir>      (CPU)

Now: symbols round(y), each group's mean / scale predicted from the earlier groups' symbols, bits of round(y) under
N(mu, s). Mean-centred: symbols round(y - mu), the decoder rebuilds y^ = symbols + mu (it has mu: the predictor is
deterministic), and the earlier groups' y^ are what the predictor sees, group by group as the decoder would. Also: how
far y^ is from y (the decoder network sees y^ instead of round(y)).
"""
import os, sys
rest = sys.argv[1:]; sys.argv = sys.argv[:1]
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pred
pred.dev = "cpu"
import numpy as np, torch
from PIL import Image
from pred import teacher, Predictor, nll_bits

wpath, ppath, kd = rest
T = teacher(wpath)
sd = torch.load(ppath, map_location="cpu"); C = sd["order"].numel(); P = Predictor(C); P.load_state_dict(sd, strict=False); P.eval()   # older checkpoints have no fscale (hyperprior input, unused)
tot = np.zeros(4); n_px = 0
with torch.no_grad():
    for name in ["01", "03", "05", "08", "13", "19", "21", "23"]:
        x = torch.from_numpy(np.asarray(Image.open(f"{kd}/kodak_kodim{name}.ppm").convert("RGB"), dtype=np.float32) / 255).permute(2, 0, 1)[None]
        y = T.g_a(x); yo = y[:, P.order]; hw = y.shape[-2:]; px = x.shape[-2] * x.shape[-1]
        # now: round(y), predictor on symbols
        _, mu0, ls0 = P(torch.round(y)); b0 = nll_bits(torch.round(yo), mu0, ls0).sum().item()
        # mean-centred, sequential
        yhat = torch.zeros_like(yo); bits = 0.0; st = 0
        for g, net in zip(P.groups, P.nets):
            o = net(yhat[:, :st] if st else yhat[:, :0], hw).expand(1, -1, -1, -1); mu, ls = o[:, :g], o[:, g:]
            s1 = torch.round(yo[:, st:st + g] - mu); bits += nll_bits(s1, torch.zeros_like(mu), ls).sum().item()
            yhat[:, st:st + g] = s1 + mu; st += g
        e0 = ((torch.round(yo) - yo) ** 2).mean().item(); e1 = ((yhat - yo) ** 2).mean().item()
        tot += [b0 / px, bits / px, e0, e1]; n_px += 1
        print(f"kodim{name}: now {b0 / px:.4f} bpp, mean-centred {bits / px:.4f} ({(bits / b0 - 1) * 100:+.1f}%); latent error now {e0:.4f}, centred {e1:.4f}", flush=True)
t = tot / n_px; print(f"mean: now {t[0]:.4f}, mean-centred {t[1]:.4f} bpp ({(t[1] / t[0] - 1) * 100:+.1f}%); squared latent error {t[2]:.4f} -> {t[3]:.4f}")
