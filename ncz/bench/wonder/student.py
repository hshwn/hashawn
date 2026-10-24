#!/usr/bin/env python3
"""Student decoder: the teacher's integer latents -> picture, with a decoder 4-9x smaller than the teacher's.

    python student.py prof  <weights.pth.tar> <img_dir>
    python student.py train <weights.pth.tar> <img_dir> <steps> <out.pt>
    python student.py dump  <weights.pth.tar> <student.pt> <kodak_dir> <out_dir> <names...>

The teacher's encoder is fixed, so the latents (and the files) do not change: only the decoder is new. Every training
photo is encoded once (latents kept on the GPU next to the photo); crops are cut in step (latent L x L <-> pixels
16L x 16L). Loss: MSE against the original photo (the student learns its own best decoding, not the teacher's).
DISTIL=1 adds the teacher's decoder: MSE to its output (weight DW_OUT; computed once per photo, kept on the GPU as
uint8) and to its features at 1/8 and 1/4 resolution (its low-resolution layers run alongside, frozen, bf16) through
1x1 projections, each normalised by the teacher feature's RMS (weight DW_FEAT). The projections are training-only;
the student is unchanged.
"""
import os, sys, time
import torch, torch.nn as nn, torch.nn.functional as F
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pred
from pred import teacher, load_imgs, dev
import numpy as np
from PIL import Image

class QConv(nn.Conv2d):
    """Conv whose weights can be trained for low-bit storage (quantisation-aware: the forward pass sees the quantised
    weights, gradients pass straight through). Per output channel, symmetric. 4-bit: levels -8..7, scale
    min(max|w|, 3 rms) / 7. 8-bit: levels -128..127, scale max|w| / 127. QConv.bits picks (env QBITS)."""
    qat = False; bits = int(os.environ.get("QBITS", "4"))
    @staticmethod
    def scale(w):
        if QConv.bits == 8: return w.abs().amax((1, 2, 3), keepdim=True).clamp_min(1e-8) / 127
        return torch.minimum(w.abs().amax((1, 2, 3), keepdim=True), 3 * w.pow(2).mean((1, 2, 3), keepdim=True).sqrt()).clamp_min(1e-8) / 7
    @staticmethod
    def levels(): return (-128, 127) if QConv.bits == 8 else (-8, 7)
    def forward(s, x):
        if not QConv.qat: return super().forward(x)
        lo, hi = QConv.levels(); sc = QConv.scale(s.weight); wq = torch.clamp(torch.round(s.weight / sc), lo, hi) * sc
        return F.conv2d(x, s.weight + (wq - s.weight).detach(), s.bias, s.stride, s.padding)
class Res(nn.Module):
    def __init__(s, c): super().__init__(); s.a = QConv(c, c, 3, padding=1); s.b = QConv(c, c, 3, padding=1)
    def forward(s, x): return x + s.b(F.leaky_relu(s.a(x), 0.1))
class Loop(nn.Module):
    """The same block applied `r` times (weight-tied: more computation, no more weights), with a learned per-channel
    gain per pass (a few hundred numbers) so the passes can differ."""
    def __init__(s, block, c, r):
        super().__init__(); s.block, s.r = block, r; s.g = nn.Parameter(torch.ones(r, c))
    def forward(s, x):
        for i in range(s.r): x = x + (s.block(x) - x) * s.g[i].view(1, -1, 1, 1)
        return x
def res(c, r): return Res(c) if r == 1 else Loop(Res(c), c, r)
def upb(ci, co, r=1): return nn.Sequential(QConv(ci, co * 4, 3, padding=1), nn.PixelShuffle(2), nn.LeakyReLU(0.1), res(co, r))
# channel plans, coarse to fine (1/16, 1/8, 1/4, 1/2, full resolution)
ARCHS = {"v1": (96, 96, 64, 48, 32), "v2": (128, 128, 96, 64, 48), "v3": (128, 128, 128, 96, 64)}   # v3: wider at 1/4, 1/2 and full resolution
class Student(nn.Module):
    # channels shrink as resolution grows: the cost per decoded pixel stays browser-sized
    def __init__(s, M, N=96, r=1, chans=None):
        super().__init__(); s.register_buffer("inv", torch.ones(1, M, 1, 1))     # 1 / per-channel std of the latents (set from data)
        c = chans or (N, N, 64, 48, 32); s.chans = c
        s.n = nn.Sequential(QConv(M, c[0], 3, padding=1), nn.LeakyReLU(0.1), res(c[0], r), upb(c[0], c[1], r), upb(c[1], c[2], r), upb(c[2], c[3], r), upb(c[3], c[4], r), nn.Conv2d(c[4], 3, 3, padding=1))
    def forward(s, y): return s.n(y * s.inv) + 0.5

HOST = bool(os.environ.get("HOST"))
def setup(wpath, idir, L, keep_teacher=False):
    # HOST: photos, latents and teacher outputs live in CPU memory, the teacher runs one photo at a time and each batch
    # goes to the GPU: the GPU holds one photo's activations at most (shares the GPU with other jobs)
    T = teacher(wpath); lats = []
    if HOST:
        import pred as _p; _d = _p.dev; _p.dev = "cpu"; ims = load_imgs(idir, 16 * L); _p.dev = _d
    else: ims = load_imgs(idir, 16 * L)
    t = time.time()
    gs = T.g_s if keep_teacher else None; recs = []
    with torch.no_grad():
        for i, im in enumerate(ims):
            H, W = im.shape[-2:]; H, W = H // 64 * 64, W // 64 * 64; ims[i] = im[:, :H, :W].contiguous()
            y = torch.round(T.g_a(ims[i][None].to(dev).float() / 255))[0].to(torch.int16)
            if keep_teacher:
                with torch.autocast("cuda", torch.bfloat16): r = (gs(y[None].float()).float().clamp(0, 1)[0] * 255 + 0.5).to(torch.uint8)
                recs.append(r.cpu() if HOST else r)
            lats.append(y.cpu() if HOST else y)
    del T; torch.cuda.empty_cache(); torch.cuda.synchronize()
    print(f"latents of {len(ims)} photos in {time.time() - t:.0f}s", flush=True)
    torch.backends.cudnn.benchmark = True; torch.backends.cuda.matmul.allow_tf32 = True; torch.backends.cudnn.allow_tf32 = True
    return ims, lats, gs, recs

T_TAPS, S_TAPS = (2, 4), (3, 4)             # teacher g_s / student layers at 1/8 and 1/4 resolution
def run_taps(seq, x, taps):
    feats = []
    for i, m in enumerate(seq):
        x = m(x)
        if i in taps: feats.append(x)
    return x, feats

def pair_batch(ims, lats, B, L, recs=None):
    ys, xs, ts = [], [], []
    for _ in range(B):
        k = torch.randint(len(ims), (1,)).item(); y = lats[k]; H, W = y.shape[-2:]
        a, b = torch.randint(H - L + 1, (1,)).item(), torch.randint(W - L + 1, (1,)).item()
        ys.append(y[:, a:a + L, b:b + L]); xs.append(ims[k][:, 16 * a:16 * (a + L), 16 * b:16 * (b + L)])
        if recs: ts.append(recs[k][:, 16 * a:16 * (a + L), 16 * b:16 * (b + L)])
    g = lambda v: torch.stack(v).to(dev, non_blocking=True).float()
    return g(ys), g(xs) / 255, (g(ts) / 255 if recs else None)

def macs_per_px(net, M):
    tot = 0
    def hook(m, i, o):
        nonlocal tot
        if isinstance(m, nn.Conv2d): tot += o.shape[2] * o.shape[3] * m.in_channels * m.out_channels * m.kernel_size[0] * m.kernel_size[1]
    hs = [m.register_forward_hook(hook) for m in net.modules()]
    with torch.no_grad(): net(torch.zeros(1, M, 8, 8, device=dev))
    for h in hs: h.remove()
    return tot / (128 * 128)

if __name__ == "__main__":
    cmd = sys.argv[1]
    if cmd in ("prof", "train"):
        B = int(os.environ.get("BATCH", "16")); L = int(os.environ.get("LCROP", "16"))
        DISTIL = bool(os.environ.get("DISTIL")); DW_OUT = float(os.environ.get("DW_OUT", "1")); DW_FEAT = float(os.environ.get("DW_FEAT", "0.1"))
        ims, lats, gs, recs = setup(sys.argv[2], sys.argv[3], L, DISTIL); M = lats[0].shape[0]
        LOOPS = int(os.environ.get("LOOPS", "1")); ARCH = os.environ.get("ARCH", "v1"); QAT_FROM = float(os.environ.get("QAT_FROM", "2"))
        S = Student(M, r=LOOPS, chans=ARCHS[ARCH]).to(dev, memory_format=torch.channels_last)
        sd_ = torch.stack([l.float().pow(2).mean((1, 2)) for l in lats]).mean(0).sqrt().clamp_min(0.05)
        S.inv.copy_((1 / sd_).view(1, M, 1, 1))
        print(f"student: {sum(p.numel() for p in S.parameters()) / 1e6:.2f}M params, {macs_per_px(S, M) / 1e3:.0f}k MAC per pixel", flush=True)
        LR = float(os.environ.get("LR", "5e-4"))
        proj = nn.ModuleList([nn.Conv2d(c, 192 if M == 192 else 128, 1) for c in S.chans[1:3]]).to(dev, memory_format=torch.channels_last)
        if DISTIL: gs = gs[:T_TAPS[-1] + 1].to(memory_format=torch.channels_last)     # only the low-resolution part runs per step
        params = list(S.parameters()) + (list(proj.parameters()) if DISTIL else [])
        opt = torch.optim.AdamW(params, LR, weight_decay=1e-5)
        def step():
            y, x, xt = pair_batch(ims, lats, B, L, recs); yc = y.contiguous(memory_format=torch.channels_last)
            with torch.autocast("cuda", torch.bfloat16):
                if DISTIL:
                    with torch.no_grad(): _, tf = run_taps(gs, yc, T_TAPS)
                    xh, sf = run_taps(S.n, yc * S.inv, S_TAPS); xh = xh + 0.5
                else: xh = S(yc)
            loss_px = F.mse_loss(xh.float(), x); loss = loss_px
            if DISTIL:
                loss = loss + DW_OUT * F.mse_loss(xh.float(), xt)
                for pj, a, b in zip(proj, sf, tf):
                    b = b.float(); loss = loss + DW_FEAT * F.mse_loss(pj(a.float()), b) / b.pow(2).mean().clamp_min(1e-6)
            opt.zero_grad(set_to_none=True); loss.backward()
            torch.nn.utils.clip_grad_norm_(params, 0.5); opt.step(); return loss_px.detach()
        if os.environ.get("COMPILE"): step = torch.compile(step)
        if cmd == "prof":
            for _ in range(15): step()
            torch.cuda.synchronize(); t = time.time()
            for _ in range(40): step()
            torch.cuda.synchronize(); dt = (time.time() - t) / 40
            print(f"batch {B}, crops {16 * L}px: {dt * 1000:.1f} ms/step, {B * (16 * L) ** 2 / dt / 1e6:.0f} Mpx/s, {torch.cuda.max_memory_allocated() / 1e9:.1f} GB", flush=True)
            sys.exit()
        steps, out = int(sys.argv[4]), sys.argv[5]
        sched = torch.optim.lr_scheduler.OneCycleLR(opt, LR, total_steps=steps, pct_start=0.03)
        t0 = time.time(); acc = 0.0
        for i in range(1, steps + 1):
            if not QConv.qat and i > QAT_FROM * steps:
                torch.save(S.state_dict(), out.replace(".pt", ".float.pt"))      # the float model, before quantising
                QConv.qat = True; print(f"{i}: {QConv.bits}-bit weights from here", flush=True)
            l = step(); sched.step()
            if i % 50 == 0: acc += float(l)
            every = int(os.environ.get("EVERY", "2000"))
            if i % every == 0 or i == steps:
                mse = acc / (every // 50) * 255 ** 2; print(f"{i} psnr(train crops) {10 * np.log10(255 ** 2 / max(mse, 1e-9)):.2f}  {time.time() - t0:.0f}s", flush=True); acc = 0.0
                torch.save(S.state_dict(), out)
    elif cmd == "dump":
        T = teacher(sys.argv[2]); sd = torch.load(sys.argv[3]); M = sd["n.0.weight"].shape[1]
        r = sd["n.2.g"].shape[0] if "n.2.g" in sd else 1
        chans = (sd["n.0.weight"].shape[0],) + tuple(sd[f"n.{i}.0.weight"].shape[0] // 4 for i in (3, 4, 5, 6))
        S = Student(M, r=r, chans=chans).to(dev); QConv.qat = bool(os.environ.get("QAT_DUMP")); S.load_state_dict(sd); S.eval()
        kd, od = sys.argv[4], sys.argv[5]; os.makedirs(od, exist_ok=True); q = int(os.path.basename(sys.argv[2]).split("-")[2])
        for name in sys.argv[6:]:
            x = torch.from_numpy(np.asarray(Image.open(f"{kd}/{name}.ppm").convert("RGB"), dtype=np.float32) / 255).permute(2, 0, 1)[None].to(dev)
            with torch.no_grad(): xh = S(torch.round(T.g_a(x))).clamp(0, 1)
            Image.fromarray((xh[0].permute(1, 2, 0).cpu().numpy() * 255 + 0.5).astype(np.uint8)).save(f"{od}/{name}.q{q}.png")
        print("ok")
