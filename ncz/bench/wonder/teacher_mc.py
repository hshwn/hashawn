#!/usr/bin/env python3
"""What would the teacher's own entropy model be worth through our coder? Kodak 8, CPU.

    python teacher_mc.py <teacher.pth.tar> <kodak_dir> <out_dir>

Runs cheng2020-anchor's hyperprior and its autoregressive context model exactly as its decoder does (raster order over
latent positions, each position's mean / scale from h_s(z^) and the 5x5 causal neighbourhood of the already rebuilt
y^), and writes for each photo:
  <name>.mc.lat   mean-centred symbols round(y - mean) (what a port of that model would code)
  <name>.mc.par   their context for our coder's LAT_AUX path: mean x16 = 0 (centred), scale x64
  <name>.z.lat    the hyperprior's symbols round(z - medians) (side information, coded too)
  <name>.mc.png   g_s(y^): the teacher's reconstruction from mean-centred latents
and prints the teacher's own bits for both parts. Then: nczcli lat with LAT_AUX=<out_dir> on the .mc.lat files.
"""
import os, sys
rest = sys.argv[1:]; sys.argv = sys.argv[:1]
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pred
pred.dev = "cpu"
import numpy as np, torch, torch.nn.functional as F
from PIL import Image
from pred import teacher

wpath, kd, od = rest; os.makedirs(od, exist_ok=True)
T = teacher(wpath); T.update(force=True)
def save_lat(path, a):
    a = a.astype(np.int16); open(path, "wb").write(np.array(a.shape, dtype=np.uint16).tobytes() + a.tobytes())
own_y = own_z = 0.0; n_px = 0
with torch.no_grad():
    for name in ["01", "03", "05", "08", "13", "19", "21", "23"]:
        x = torch.from_numpy(np.asarray(Image.open(f"{kd}/kodak_kodim{name}.ppm").convert("RGB"), dtype=np.float32) / 255).permute(2, 0, 1)[None]
        y = T.g_a(x); z = T.h_a(y)
        med = T.entropy_bottleneck._get_medians().reshape(1, -1, 1, 1); zq = torch.round(z - med); z_hat = zq + med
        params = T.h_s(z_hat)
        B, C, H, W = y.shape; k = 5; pad = k // 2
        w = T.context_prediction.weight * T.context_prediction.mask; b = T.context_prediction.bias
        y_hat = F.pad(torch.zeros_like(y), (pad, pad, pad, pad)); mu = torch.zeros_like(y); sc = torch.zeros_like(y)
        for h in range(H):
            for ww in range(W):
                ctx = F.conv2d(y_hat[:, :, h:h + k, ww:ww + k], w, b)
                gp = T.entropy_parameters(torch.cat([params[:, :, h:h + 1, ww:ww + 1], ctx], 1))
                s_, m_ = gp.chunk(2, 1)
                q = torch.round(y[:, :, h:h + 1, ww:ww + 1] - m_)
                y_hat[:, :, h + pad:h + pad + 1, ww + pad:ww + pad + 1] = q + m_
                mu[:, :, h:h + 1, ww:ww + 1] = m_; sc[:, :, h:h + 1, ww:ww + 1] = s_
        yh = y_hat[:, :, pad:-pad, pad:-pad]; sym = torch.round(y - mu)
        scl = T.gaussian_conditional.lower_bound_scale(sc)
        ny = T.gaussian_conditional._likelihood(yh, scl, means=mu); ny = T.gaussian_conditional.likelihood_lower_bound(ny)
        _, zlik = T.entropy_bottleneck(z, training=False)     # eval mode: z rounded around its medians, as coded
        px = x.shape[-2] * x.shape[-1]; by = -torch.log2(ny).sum().item() / px; bz = -torch.log2(zlik).sum().item() / px
        own_y += by; own_z += bz; n_px += 1
        save_lat(f"{od}/kodak_kodim{name}.mc.lat", sym[0].numpy())
        mu16 = np.zeros(sym[0].shape, np.int16); sc64 = np.clip(np.round(scl[0].numpy() * 64), 0, 65535).astype(np.uint16)
        open(f"{od}/kodak_kodim{name}.mc.par", "wb").write(np.array(sym[0].shape, dtype=np.uint16).tobytes() + mu16.tobytes() + sc64.tobytes())
        save_lat(f"{od}/kodak_kodim{name}.z.lat", zq[0].numpy())
        rec = (T.g_s(yh).clamp(0, 1)[0].permute(1, 2, 0).numpy() * 255 + 0.5).astype(np.uint8); Image.fromarray(rec).save(f"{od}/kodak_kodim{name}.mc.png")
        print(f"kodim{name}: teacher's own bits y {by:.4f} + z {bz:.4f} bpp", flush=True)
print(f"mean: y {own_y / n_px:.4f} + z {own_z / n_px:.4f} = {(own_y + own_z) / n_px:.4f} bpp")
