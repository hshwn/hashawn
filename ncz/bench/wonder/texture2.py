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
# W_CHROMA: no more fine colour detail than the original has (8 x 8 blocks, 0.25 slack); the flat penalty and the map
# see brightness only, and v5 drew colour mottle (fine B - Y 1.6-2.8x the original's) on smooth red paint
W_CHROMA = float(os.environ.get("W_CHROMA", "0"))
# W_PHASE: no repeating pattern on the 16 px latent grid. A grid of period 16, 8, 4 or 2 px is exactly the part of the
# fine detail that is the same at every pixel with the same (x mod 16, y mod 16): its energy (phase-class means, crops
# are latent-aligned) may not exceed the original's. v5/v6 drew a 4 px one (the 1/4-resolution stage's pixel shuffle)
W_PHASE = float(os.environ.get("W_PHASE", "0"))
def phase_energy(h):
    B, _, H, W = h.shape; m = h.reshape(B, 1, H // 16, 16, W // 16, 16).mean((2, 4))
    return (m - m.mean((-2, -1), keepdim=True)).pow(2).mean((1, 2, 3))
# ADV_MAP=1: the decoder feels the critic only where the original had detail (map weight 0 below ~1.4 levels of
# high-pass rms, 1 above ~4), so nothing pulls flat, glossy areas toward texture
ADV_MAP = bool(os.environ.get("ADV_MAP"))
# NOISE=1: a full-resolution noise image as one more input; after every upsampling stage it is pooled to that stage's
# resolution (rescaled to unit spread) and added with a learned per-channel gain (started at zero), so neighbouring
# cells that look alike still draw different texture (no 16 px tiling). In the browser the noise is a hash of the
# pixel position, so a file still unpacks the same every time.
NOISE = bool(os.environ.get("NOISE"))
# NOISE=2: in addition, 8 noise channels concatenated into the inputs of the last two stages' residual blocks (weights
# started non-zero), so drawing from noise is the easy path; MSCRIT=1: the critic also judges the picture at 1/2 and
# 1/4 size, so a pattern repeating every 16 px is visible to it
NOISE2 = os.environ.get("NOISE") == "2"; MSCRIT = bool(os.environ.get("MSCRIT"))
# W_SPEC: the drawn fine detail's power spectrum (batch mean, log) must match the original's: no grid spikes
W_SPEC = float(os.environ.get("W_SPEC", "0"))
def log_spectrum(h):
    f = torch.fft.rfft2(h - h.mean((-2, -1), keepdim=True)); return torch.log(f.abs().pow(2).mean(0) + 1e-3)
_g = torch.exp(-torch.arange(-4, 5, dtype=torch.float32) ** 2 / (2 * 1.5 ** 2)); _g = (_g / _g.sum()).to(dev)
def highpass(x):
    """Brightness minus its Gaussian blur (sigma 1.5), in 8-bit units."""
    l = (x[:, 0:1] * 0.299 + x[:, 1:2] * 0.587 + x[:, 2:3] * 0.114) * 255
    b = F.conv2d(F.pad(l, (4, 4, 0, 0), mode="reflect"), _g.view(1, 1, 1, 9)); b = F.conv2d(F.pad(b, (0, 0, 4, 4), mode="reflect"), _g.view(1, 1, 9, 1))
    return l - b
def chroma_highpass(x):
    """Colour (B - Y, R - Y) minus its Gaussian blur (sigma 1.5), in 8-bit units: 2 channels."""
    l = x[:, 0:1] * 0.299 + x[:, 1:2] * 0.587 + x[:, 2:3] * 0.114; c = torch.cat([x[:, 2:3] - l, x[:, 0:1] - l], 1) * 255
    b = F.conv2d(F.pad(c, (4, 4, 0, 0), mode="reflect"), _g.view(1, 1, 1, 9).expand(2, 1, 1, 9), groups=2)
    b = F.conv2d(F.pad(b, (0, 0, 4, 4), mode="reflect"), _g.view(1, 1, 9, 1).expand(2, 1, 9, 1), groups=2)
    return c - b
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
    """A student whose first conv also takes the detail map (one more input channel, started at zero); NOISE: plus
    per-stage noise with learned gains."""
    def __init__(s, S, M, noise=False):
        super().__init__(); s.S = S; c0 = S.n[0].out_channels; s.noise = noise
        conv = stu.QConv(M + 1, c0, 3, padding=1)
        with torch.no_grad(): conv.weight.zero_(); conv.weight[:, :M] = S.n[0].weight; conv.bias.copy_(S.n[0].bias)
        S.n[0] = conv
        if noise: s.ng = nn.ParameterList([nn.Parameter(torch.zeros(1, S.n[i][0].out_channels // 4, 1, 1)) for i in (3, 4, 5, 6)])
        if NOISE2:   # stages 5 and 6 (1/2 and full resolution): a 1x1 mix of 8 noise channels into their features
            s.nmix = nn.ModuleList([nn.Conv2d(8, S.n[i][0].out_channels // 4, 1) for i in (5, 6)])
            for c in s.nmix: nn.init.normal_(c.weight, std=0.1); nn.init.zeros_(c.bias)
    def forward(s, y, m, nz=None):
        mm = F.interpolate(m / 8, scale_factor=2, mode="nearest")[..., :y.shape[-2], :y.shape[-1]]
        h = torch.cat([y * s.S.inv, mm], 1)
        if not s.noise: return s.S.n(h) + 0.5
        if nz is None: nz = torch.randn(y.shape[0], 9 if NOISE2 else 1, y.shape[-2] * 16, y.shape[-1] * 16, device=y.device, dtype=h.dtype)
        for i, layer in enumerate(s.S.n):
            if NOISE2 and i in (5, 6):  # the stage's upsampling conv, then noise mixed in before its residual block
                h = layer[2](layer[1](layer[0](h)))
                k = 2 ** (6 - i); n8 = nz[:, 1:9]; n8 = F.avg_pool2d(n8, k) * k if k > 1 else n8
                h = layer[3](h + s.nmix[i - 5](n8))
            else: h = layer(h)
            if 3 <= i <= 6:            # after upsampling stage i-3: resolution 1/8, 1/4, 1/2, 1
                k = 2 ** (6 - i); n = F.avg_pool2d(nz[:, :1], k) * k if k > 1 else nz[:, :1]
                h = h + s.ng[i - 3] * n
        return h + 0.5
def load_student(path, M):
    sd = torch.load(path, map_location="cpu")
    if "S.n.0.weight" in sd:   # already a map student
        chans = (sd["S.n.0.weight"].shape[0],) + tuple(sd[f"S.n.{i}.0.weight"].shape[0] // 4 for i in (3, 4, 5, 6))
        S = MapStudent(stu.Student(M, chans=chans), M, noise="ng.0" in sd); S.load_state_dict(sd); return S.to(dev)
    chans = (sd["n.0.weight"].shape[0],) + tuple(sd[f"n.{i}.0.weight"].shape[0] // 4 for i in (3, 4, 5, 6))
    S = stu.Student(M, chans=chans); S.load_state_dict(sd); return MapStudent(S, M, NOISE).to(dev)

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
    D2 = Critic(M).to(dev) if MSCRIT else None; D4 = Critic(M).to(dev) if MSCRIT else None
    def crit(xi, y, m):
        """(output, features) of the full-size critic, plus the 1/2 and 1/4 critics' outputs when MSCRIT."""
        o, f = D(xi, y, m)
        if not MSCRIT: return [o], f
        x2, x4 = F.avg_pool2d(xi, 2), F.avg_pool2d(xi, 4)
        y2, y4 = F.avg_pool2d(y, 2), F.avg_pool2d(y, 4); m2, m4 = F.avg_pool2d(m, 2), F.avg_pool2d(m, 4)
        return [o, D2(x2, y2, m2)[0], D4(x4, y4, m4)[0]], f
    TE = teacher(wpath) if W_CON else None
    print(f"student {sum(p.numel() for p in S.parameters()) / 1e6:.2f}M, critic {sum(p.numel() for p in D.parameters()) / 1e6:.2f}M", flush=True)
    with torch.no_grad():   # the student's own pixel error, to normalise
        ref = np.mean([F.mse_loss(S(y, detail_map(x)).clamp(0, 1), x).item() for y, x, _ in (stu.pair_batch(ims, lats, B, L) for _ in range(8))])
    print(f"student MSE {ref:.5f} ({10 * np.log10(1 / ref):.2f} dB on train crops)", flush=True)
    optg = torch.optim.Adam(S.parameters(), float(os.environ.get("LR_G", "5e-5")), betas=(0.5, 0.9))
    dparams = list(D.parameters()) + (list(D2.parameters()) + list(D4.parameters()) if MSCRIT else [])
    optd = torch.optim.Adam(dparams, float(os.environ.get("LR_D", "2e-4")), betas=(0.5, 0.9))
    def step():
        y, x, _ = stu.pair_batch(ims, lats, B, L); m = detail_map(x)
        with torch.autocast("cuda", torch.bfloat16): xh = S(y, m).float()
        drs, _ = crit(x, y, m); dfs, _ = crit(xh.detach().clamp(0, 1), y, m)
        ld = sum(F.relu(1 - dr).mean() + F.relu(1 + df).mean() for dr, df in zip(drs, dfs)); optd.zero_grad(set_to_none=True); ld.backward(); optd.step()
        gs, ff = crit(xh, y, m); g = gs[0]
        with torch.no_grad(): _, fr = crit(x, y, m)
        fm = sum(F.l1_loss(a, b) for a, b in zip(ff, fr)) / len(ff)
        if ADV_MAP:
            wgt = F.interpolate(torch.clamp((m - 1) / 4, 0, 1), size=g.shape[-2:], mode="nearest")
            adv = -(g * wgt).sum() / wgt.sum().clamp_min(1.0)
        else: adv = -g.mean()
        if MSCRIT: adv = adv - sum(gg.mean() for gg in gs[1:])
        px = F.mse_loss(xh, x) / ref
        loss = W_PX * px + W_GAN * adv + W_FM * fm
        # flat areas: no more fine detail than the original has (8 x 8 blocks, 8-bit units, 25% + 0.5 slack)
        hh, hx = highpass(xh), highpass(x)
        flat = F.relu(block_rms(hh, 8) - 1.25 * block_rms(hx, 8) - 0.5).pow(2).mean()
        loss = loss + W_FLAT * flat; ld_flat = flat.detach()
        if W_CHROMA:
            chroma = F.relu(block_rms(chroma_highpass(xh), 8) - block_rms(chroma_highpass(x), 8) - 0.25).pow(2).mean()
            loss = loss + W_CHROMA * chroma; ld_flat = ld_flat + chroma.detach()
        if W_PHASE:
            phase = F.relu(phase_energy(hh.float()) - phase_energy(hx.float())).mean()
            loss = loss + W_PHASE * phase; ld_flat = ld_flat + phase.detach()
        if W_SPEC: loss = loss + W_SPEC * (log_spectrum(hh.float()) - log_spectrum(hx.float())).abs().mean()
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
