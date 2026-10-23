#!/usr/bin/env python3
"""Texture fine-tune v2 (texture2.py): like texture.py, plus a detail map and a flat-area penalty.

The encoder measures the original's fine-detail energy per 32 x 32 block (high-pass of brightness, rms, log2 in half
steps: what the file would carry, ~144 B a photo) and the decoder gets it as an extra input (upsampled to latent
resolution), the critic too. W_FLAT penalises drawing more fine detail than the original has, measured on 8 x 8
blocks during training (the decoder must learn from the latents and the map where the photo is flat).

Texture fine-tune of a student decoder: same latents (same files), the decoder learns to draw texture instead of
the blur that pure pixel error rewards.

    python texture.py prof  <weights.pth.tar> <student.pt> <img_dir>
    python texture.py train <weights.pth.tar> <student.pt> <img_dir> <steps> <out.pt> <kodak_dir> <dump_dir>

Loss of the decoder: W_PX * MSE (normalised to ~1 at the student's level) + W_GAN * adversarial + W_FM * feature
matching. The critic is a patch discriminator that sees the picture AND its latents (1x1-reduced, upsampled), so it
judges "plausible texture for these latents": the decoder cannot drift away from what the file says. Every DUMP_EVERY
steps the 8 Kodak test photos are decoded into <dump_dir>/step<k>/ to look at.
"""
import os, sys, time
import numpy as np, torch, torch.nn as nn, torch.nn.functional as F
from torch.nn.utils.parametrizations import spectral_norm
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
rest = sys.argv[1:]; sys.argv = sys.argv[:1]
import student as stu
from pred import teacher, dev
from PIL import Image

W_PX = float(os.environ.get("W_PX", "1")); W_GAN = float(os.environ.get("W_GAN", "0.05")); W_FM = float(os.environ.get("W_FM", "1"))
# W_CON: consistency, the decoded picture must re-encode (teacher encoder, frozen) to the file's latents; in units of
# rounding noise (1/12). With it, the pixel term can be a small anchor: the file pins down what it pins down, the critic
# draws the rest.
W_CON = float(os.environ.get("W_CON", "0")); W_FLAT = float(os.environ.get("W_FLAT", "1"))
# ADV_MAP=1: the decoder feels the critic only where the original had detail (map weight 0 below ~1.4 levels of
# high-pass rms, 1 above ~4), so nothing pulls flat, glossy areas toward texture
ADV_MAP = bool(os.environ.get("ADV_MAP"))
_g = torch.exp(-torch.arange(-4, 5, dtype=torch.float32) ** 2 / (2 * 1.5 ** 2)); _g = (_g / _g.sum()).to(dev)
def highpass(x):
    """Brightness minus its Gaussian blur (sigma 1.5), in 8-bit units."""
    l = (x[:, 0:1] * 0.299 + x[:, 1:2] * 0.587 + x[:, 2:3] * 0.114) * 255
    b = F.conv2d(F.pad(l, (4, 4, 0, 0), mode="reflect"), _g.view(1, 1, 1, 9)); b = F.conv2d(F.pad(b, (0, 0, 4, 4), mode="reflect"), _g.view(1, 1, 9, 1))
    return l - b
def block_rms(h, k): return F.avg_pool2d(h * h, k).clamp_min(1e-6).sqrt()
def detail_map(x):
    """The map the file carries: per 32 x 32 block, log2 of the high-pass rms in half steps (-4..15)."""
    return torch.clamp(torch.round(2 * torch.log2(block_rms(highpass(x), 32).clamp_min(0.25))), -4, 15)
B = int(os.environ.get("BATCH", "12")); L = int(os.environ.get("LCROP", "16")); DUMP_EVERY = int(os.environ.get("DUMP_EVERY", "5000"))
TEST = ["kodak_kodim" + t for t in ["01", "03", "05", "08", "13", "19", "21", "23"]]

class Critic(nn.Module):
    def __init__(s, M, c=64):
        super().__init__(); s.lat = nn.Conv2d(M, 16, 1)
        sn = lambda *a, **k: spectral_norm(nn.Conv2d(*a, **k))
        s.layers = nn.ModuleList([sn(3 + 16 + 1, c, 4, 2, 1), sn(c, 2 * c, 4, 2, 1), sn(2 * c, 4 * c, 4, 2, 1), sn(4 * c, 4 * c, 3, 1, 1)])
        s.out = sn(4 * c, 1, 3, 1, 1)
    def forward(s, x, y, m):
        f = F.interpolate(s.lat(y), scale_factor=16, mode="nearest"); mm = F.interpolate(m / 8, scale_factor=32, mode="nearest")
        h = torch.cat([x * 2 - 1, f, mm], 1); feats = []
        for l in s.layers: h = F.leaky_relu(l(h), 0.2); feats.append(h)
        return s.out(h), feats

class MapStudent(nn.Module):
    """A student whose first conv also takes the detail map (one more input channel, started at zero)."""
    def __init__(s, S, M):
        super().__init__(); s.S = S; c0 = S.n[0].out_channels
        conv = stu.QConv(M + 1, c0, 3, padding=1)
        with torch.no_grad(): conv.weight.zero_(); conv.weight[:, :M] = S.n[0].weight; conv.bias.copy_(S.n[0].bias)
        S.n[0] = conv
    def forward(s, y, m):
        mm = F.interpolate(m / 8, scale_factor=2, mode="nearest")[..., :y.shape[-2], :y.shape[-1]]
        return s.S.n(torch.cat([y * s.S.inv, mm], 1)) + 0.5
def load_student(path, M):
    sd = torch.load(path, map_location="cpu")
    if "S.n.0.weight" in sd:   # already a map student
        chans = (sd["S.n.0.weight"].shape[0],) + tuple(sd[f"S.n.{i}.0.weight"].shape[0] // 4 for i in (3, 4, 5, 6))
        S = MapStudent(stu.Student(M, chans=chans), M); S.load_state_dict(sd); return S.to(dev)
    chans = (sd["n.0.weight"].shape[0],) + tuple(sd[f"n.{i}.0.weight"].shape[0] // 4 for i in (3, 4, 5, 6))
    S = stu.Student(M, chans=chans); S.load_state_dict(sd); return MapStudent(S, M).to(dev)

def dump(S, T, kd, od):
    os.makedirs(od, exist_ok=True); S.eval(); agree = []
    for n in TEST:
        x = torch.from_numpy(np.asarray(Image.open(f"{kd}/{n}.ppm").convert("RGB"), dtype=np.float32) / 255).permute(2, 0, 1)[None].to(dev)
        with torch.no_grad():
            y = torch.round(T.g_a(x)); xh = S(y, detail_map(x)).clamp(0, 1)
            xq = torch.round(xh * 255) / 255; agree.append((torch.round(T.g_a(xq)) == y).float().mean().item())   # re-encode the saved picture
        Image.fromarray((xh[0].permute(1, 2, 0).cpu().numpy() * 255 + 0.5).astype(np.uint8)).save(f"{od}/{n}.png")
    print(f"  {od}: re-encodes to the same latents: {100 * np.mean(agree):.2f}%", flush=True)
    S.train()

if __name__ == "__main__":
    cmd = rest[0]; wpath, spath = rest[1], rest[2]
    ims, lats, _, _ = stu.setup(wpath, rest[3], L, False); M = lats[0].shape[0]
    S = load_student(spath, M); D = Critic(M).to(dev)
    TE = teacher(wpath) if W_CON else None
    print(f"student {sum(p.numel() for p in S.parameters()) / 1e6:.2f}M, critic {sum(p.numel() for p in D.parameters()) / 1e6:.2f}M", flush=True)
    with torch.no_grad():   # the student's own pixel error, to normalise
        ref = np.mean([F.mse_loss(S(y, detail_map(x)).clamp(0, 1), x).item() for y, x, _ in (stu.pair_batch(ims, lats, B, L) for _ in range(8))])
    print(f"student MSE {ref:.5f} ({10 * np.log10(1 / ref):.2f} dB on train crops)", flush=True)
    optg = torch.optim.Adam(S.parameters(), float(os.environ.get("LR_G", "5e-5")), betas=(0.5, 0.9))
    optd = torch.optim.Adam(D.parameters(), float(os.environ.get("LR_D", "2e-4")), betas=(0.5, 0.9))
    def step():
        y, x, _ = stu.pair_batch(ims, lats, B, L); m = detail_map(x)
        with torch.autocast("cuda", torch.bfloat16): xh = S(y, m).float()
        dr, _ = D(x, y, m); df, _ = D(xh.detach().clamp(0, 1), y, m)
        ld = F.relu(1 - dr).mean() + F.relu(1 + df).mean(); optd.zero_grad(set_to_none=True); ld.backward(); optd.step()
        g, ff = D(xh, y, m)
        with torch.no_grad(): _, fr = D(x, y, m)
        fm = sum(F.l1_loss(a, b) for a, b in zip(ff, fr)) / len(ff)
        if ADV_MAP:
            wgt = F.interpolate(torch.clamp((m - 1) / 4, 0, 1), size=g.shape[-2:], mode="nearest")
            adv = -(g * wgt).sum() / wgt.sum().clamp_min(1.0)
        else: adv = -g.mean()
        px = F.mse_loss(xh, x) / ref
        loss = W_PX * px + W_GAN * adv + W_FM * fm
        # flat areas: no more fine detail than the original has (8 x 8 blocks, 8-bit units, 25% + 0.5 slack)
        flat = F.relu(block_rms(highpass(xh), 8) - 1.25 * block_rms(highpass(x), 8) - 0.5).pow(2).mean()
        loss = loss + W_FLAT * flat; ld_flat = flat.detach()
        if W_CON:
            with torch.autocast("cuda", torch.bfloat16): ye = TE.g_a(xh.clamp(0, 1)).float()
            con = F.mse_loss(ye, y) * 12; loss = loss + W_CON * con; px = con.detach()      # report consistency in the px slot
        optg.zero_grad(set_to_none=True); loss.backward(); torch.nn.utils.clip_grad_norm_(S.parameters(), 1.0); optg.step()
        return px.detach(), adv.detach(), ld_flat, ld.detach()
    if cmd == "prof":
        for _ in range(10): step()
        torch.cuda.synchronize(); t = time.time()
        for _ in range(30): step()
        torch.cuda.synchronize(); dt = (time.time() - t) / 30
        print(f"batch {B} crops {16 * L}px: {dt * 1000:.1f} ms/step, {torch.cuda.max_memory_allocated() / 1e9:.1f} GB", flush=True); sys.exit()
    steps, out, kd, od = int(rest[4]), rest[5], rest[6], rest[7]
    T = teacher(wpath); dump(S, T, kd, f"{od}/step0")
    t0 = time.time(); acc = np.zeros(4)
    for i in range(1, steps + 1):
        r = step()
        if i % 50 == 0: acc += np.array([float(v) for v in r])
        if i % 1000 == 0:
            a = acc / 20; print(f"{i} px/con {a[0]:.3f}  adv {a[1]:.3f}  flat {a[2]:.3f}  critic {a[3]:.3f}  {time.time() - t0:.0f}s", flush=True); acc[:] = 0
        if i % DUMP_EVERY == 0 or i == steps:
            torch.save(S.state_dict(), out); dump(S, T, kd, f"{od}/step{i}")
    print("finished", flush=True)
