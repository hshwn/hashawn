#!/usr/bin/env python3
"""Our mean/scale predictor for NCZY: decoded integer latents -> mean and scale of the next channel group.

    python pred.py prof  <weights.pth.tar> <img_dir> [batch] [crop]
    python pred.py train <weights.pth.tar> <img_dir> <steps> <out.pt>
    python pred.py dump  <weights.pth.tar> <pred.pt> <kodak_dir> <out_dir> <names...>     # .par files for nczcli LAT_AUX

Channels are coded in a fixed order (strongest first, measured on the training photos) and split into groups. Group g
gets a small conv net over ALL already-decoded groups, full picture (earlier channels are complete, so the net sees
what lies below and right too): 1x1 reduce -> 3x3 convs -> mean and log-scale for each of the group's channels at
every position. One pass per group, not per latent: parallel and cheap. Inside a group, the context mixer adds the
spatial and within-group contexts. Inputs are integers and the net is small, so the decoder can run it in
deterministic f32 (fixed order, no fused multiply-add); the log-scale goes into buckets by comparison (no exp).
Loss: discretised Gaussian NLL of round(y) (the symbols NCZY codes). Training data: the teacher's latents of each
whole photo, computed once and kept on the GPU (int16); training crops are cut in latent space.
"""
import math, os, sys, time, glob
import torch, torch.nn as nn, torch.nn.functional as F

exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "teacher.py")).read().split("import numpy as np, torch")[0])
import numpy as np
from PIL import Image
dev = "cuda"
GROUPS = {128: [4, 4, 8, 8, 16, 24, 32, 32], 192: [4, 4, 8, 8, 16, 24, 32, 48, 48], 320: [8, 8, 16, 16, 32, 48, 64, 64, 64]}
# TEACHER=hpcm: HPCM-Base (ICCV 2025, ~/wonder/hpcm) instead of cheng2020-anchor; its latents are 320 channels
TEACHER = os.environ.get("TEACHER", "cheng"); STRIDE = 16           # both downsample by 16
WIDTH = int(os.environ.get("WIDTH", "64"))

def teacher(path):
    if TEACHER == "hpcm":
        sys.path.insert(0, os.path.expanduser("~/wonder/hpcm")); from src.models.HPCM_Base import HPCM
        net = HPCM(); net.load_state_dict(torch.load(path, map_location="cpu"), strict=False)     # adaptive_params: never saved, never trained (1.0)
        net = net.to(dev).eval()
        for p in net.parameters(): p.requires_grad_(False)
        return net
    net = Cheng2020Anchor.from_state_dict(load_state_dict(torch.load(path, map_location="cpu"))).to(dev).eval()
    for p in net.parameters(): p.requires_grad_(False)
    return net

class GroupNet(nn.Module):
    def __init__(s, cin, cout, w=WIDTH):
        super().__init__(); s.cin = cin
        if cin: s.red = nn.Conv2d(cin, w, 1); s.a = nn.Conv2d(w, w, 3, padding=1); s.b = nn.Conv2d(w, w, 3, padding=1); s.out = nn.Conv2d(w, 2 * cout, 3, padding=1)
        else: s.bias = nn.Parameter(torch.zeros(1, 2 * cout, 1, 1))
    def forward(s, prev, hw):
        if not s.cin: return s.bias.expand(1, -1, *hw)
        x = F.leaky_relu(s.red(prev / 4.0), 0.1); x = x + F.leaky_relu(s.a(x), 0.1); x = x + F.leaky_relu(s.b(x), 0.1)
        return s.out(x)

class Predictor(nn.Module):
    def __init__(s, C):
        super().__init__(); s.C = C; s.groups = GROUPS[C]
        s.register_buffer("order", torch.arange(C))
        starts = np.cumsum([0] + s.groups[:-1]).tolist()
        s.nets = nn.ModuleList([GroupNet(st, g) for st, g in zip(starts, s.groups)])
    def forward(s, y):                      # y: B,C,H,W integers (as float), in natural channel order
        yo = y[:, s.order]; outs = []; st = 0
        for g, net in zip(s.groups, s.nets):
            o = net(yo[:, :st], y.shape[-2:]); outs.append(o.expand(y.shape[0], -1, -1, -1)); st += g
        mu = torch.cat([o[:, : o.shape[1] // 2] for o in outs], 1); ls = torch.cat([o[:, o.shape[1] // 2:] for o in outs], 1)
        return yo, mu, ls                    # all in coding order

def nll_bits(y, mu, ls):
    s = torch.exp(ls).clamp_min(0.11); n = torch.distributions.Normal(mu, s)
    p = (n.cdf(y + 0.5) - n.cdf(y - 0.5)).clamp_min(1e-9)
    return -torch.log2(p)

def load_imgs(d, minside):
    ims = []
    for p in sorted(glob.glob(f"{d}/*.jpg") + glob.glob(f"{d}/*.png") + glob.glob(f"{d}/**/*.png", recursive=True)):
        im = np.asarray(Image.open(p).convert("RGB"))
        if min(im.shape[:2]) >= minside: ims.append(torch.from_numpy(im.copy()).permute(2, 0, 1).to(dev))
    return ims

def batch(ims, B, S):
    out = torch.empty(B, 3, S, S, device=dev)
    for i in range(B):
        im = ims[torch.randint(len(ims), (1,)).item()]; H, W = im.shape[-2:]
        y0, x0 = torch.randint(H - S + 1, (1,)).item(), torch.randint(W - S + 1, (1,)).item()
        out[i] = im[:, y0:y0 + S, x0:x0 + S].float() / 255
    return out

def latents(T, ims):
    """Teacher latents of every whole photo (fp32), int16 on the GPU; the photos themselves are then dropped."""
    lats = []
    with torch.no_grad():
        for im in ims:
            H, W = im.shape[-2:]; H, W = H // 64 * 64, W // 64 * 64
            lats.append(torch.round(T.g_a(im[None, :, :H, :W].float() / 255))[0].to(torch.int16))
    return lats

def lat_batch(lats, B, L):
    out = []
    for _ in range(B):
        y = lats[torch.randint(len(lats), (1,)).item()]; H, W = y.shape[-2:]
        y0, x0 = torch.randint(H - L + 1, (1,)).item(), torch.randint(W - L + 1, (1,)).item()
        out.append(y[:, y0:y0 + L, x0:x0 + L])
    return torch.stack(out).float()

def setup(wpath, idir, L):
    T = teacher(wpath); ims = load_imgs(idir, STRIDE * L)
    t = time.time(); lats = latents(T, ims); del ims; torch.cuda.synchronize()
    print(f"latents of {len(lats)} photos in {time.time() - t:.0f}s, {sum(l.numel() for l in lats) * 2 / 1e9:.2f} GB", flush=True)
    torch.backends.cudnn.benchmark = True; torch.backends.cuda.matmul.allow_tf32 = True; torch.backends.cudnn.allow_tf32 = True
    C = lats[0].shape[0]                     # fixed channel order: strongest first, over the training photos
    order = torch.argsort(torch.stack([l.abs().float().mean((1, 2)) for l in lats]).mean(0), descending=True)
    P = Predictor(C).to(dev); P.order.copy_(order)
    return T, lats, P

def step_fn(T, lats, P, B, L):
    opt = torch.optim.AdamW(P.parameters(), 2e-3, weight_decay=1e-5)
    def step():
        y = lat_batch(lats, B, L)
        if torch.rand(1) < 0.5: y = y.flip(-1)
        with torch.autocast("cuda", torch.bfloat16): yo, mu, ls = P(y)
        bits = nll_bits(yo, mu.float(), ls.float())
        loss = bits.mean(); opt.zero_grad(set_to_none=True); loss.backward(); opt.step()
        return bits.detach().sum() / (B * L * L * STRIDE * STRIDE)      # bits per image pixel
    return opt, step

if __name__ == "__main__":
    cmd = sys.argv[1]
    if cmd in ("prof", "train"):
        B = int(os.environ.get("BATCH", "16")); S = int(os.environ.get("LCROP", "24"))      # crop in latents (x16 pixels)
        T, ims, P = setup(sys.argv[2], sys.argv[3], S)
        print(f"C {P.C}, groups {P.groups}, {sum(p.numel() for p in P.parameters()) / 1e6:.2f}M params", flush=True)
        opt, step = step_fn(T, ims, P, B, S)
        if cmd == "prof":
            for _ in range(10): step()
            torch.cuda.synchronize(); t = time.time()
            for _ in range(30): step()
            torch.cuda.synchronize(); dt = (time.time() - t) / 30
            print(f"batch {B} latent crop {S}: {dt * 1000:.1f} ms/step, {B * S * S / dt / 1e6:.2f}M latent positions/s, {torch.cuda.max_memory_allocated() / 1e9:.1f} GB", flush=True)
            sys.exit()
        steps, out = int(sys.argv[4]), sys.argv[5]
        sched = torch.optim.lr_scheduler.OneCycleLR(opt, 2e-3, total_steps=steps, pct_start=0.05)
        t0 = time.time(); acc = 0.0
        for i in range(1, steps + 1):
            b = step(); sched.step()
            if i % 50 == 0: acc += float(b)
            if i % 1000 == 0 or i == steps:
                print(f"{i} bpp(train crops) {acc / 20:.4f}  {time.time() - t0:.0f}s", flush=True); acc = 0.0
                torch.save(P.state_dict(), out)
    elif cmd == "dump":
        T = teacher(sys.argv[2]); sd = torch.load(sys.argv[3]); C = sd["order"].numel()
        P = Predictor(C).to(dev); P.load_state_dict(sd); P.eval()
        kd, od = sys.argv[4], sys.argv[5]; os.makedirs(od, exist_ok=True)
        q = int(os.environ.get("Q", "0")) or int(os.path.basename(sys.argv[2]).split("-")[2])
        inv = torch.argsort(P.order)
        for name in sys.argv[6:]:
            x = torch.from_numpy(np.asarray(Image.open(f"{kd}/{name}.ppm").convert("RGB"), dtype=np.float32) / 255).permute(2, 0, 1)[None].to(dev)
            with torch.no_grad():
                y = torch.round(T.g_a(x)); _, mu, ls = P(y)
                mu, sc = mu[:, inv], torch.exp(ls[:, inv]).clamp_min(0.11)       # back to natural channel order
            m = torch.clamp(torch.round(mu * 16), -32767, 32767)[0].to(torch.int16).cpu().numpy()
            s = torch.clamp(torch.round(sc * 64), 0, 65535)[0].cpu().numpy().astype(np.uint16)
            with open(f"{od}/{name}.q{q}.par", "wb") as fh: fh.write(np.array(m.shape, dtype=np.uint16).tobytes()); fh.write(m.tobytes()); fh.write(s.tobytes())
        print("ok")
