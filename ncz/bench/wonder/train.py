#!/usr/bin/env python3
"""Wonder format, prototype 1: learn the finest detail band from the coarse picture.

    python train.py prep  <img_dir> <data.pt>                      # photos -> halved (kills JPEG blocks), uint8, one file
    python train.py prof  <data.pt> <P|T>                           # time steps, no training
    python train.py train <data.pt> <P|T> <steps> <out.pt> [P.pt]   # T starts from P's weights

Split (same as proto.py): Y of YCoCg, 5-tap Laplacian; fine = g0 - up(g1); the decoder has coarse = up(g1), quantised
in the real codec, so g1 is perturbed by uniform noise of a random step during training.
P (predictor): coarse YCoCg -> fine Y, L2. Used to predict every block (stored residual shrinks; dropped blocks get P).
T (texture): coarse YCoCg + noise + per-16x16-block log energy of the true fine band (stored, 1 byte a block) ->
fine Y; loss L1 + local energy match + a small patch critic (hinge GAN), so texture has real shapes, not a blur.
Both: pixel-unshuffle 2x2 -> C-channel residual blocks at half resolution -> pixel-shuffle; ~15k MAC per pixel at C=24.
Everything lives on the GPU (the dataset too); bf16 autocast, channels_last.
"""
import math, os, sys, time, glob
import torch, torch.nn as nn, torch.nn.functional as F

dev = "cuda"
BATCH = int(os.environ.get("BATCH", "32"))
K5 = torch.tensor([1, 4, 6, 4, 1], dtype=torch.float32) / 16
def blur(x):
    k = K5.to(x.device, x.dtype); c = x.shape[1]
    x = F.conv2d(F.pad(x, (2, 2, 0, 0), mode="reflect"), k.view(1, 1, 1, 5).repeat(c, 1, 1, 1), groups=c)
    return F.conv2d(F.pad(x, (0, 0, 2, 2), mode="reflect"), k.view(1, 1, 5, 1).repeat(c, 1, 1, 1), groups=c)
def down(x): return blur(x)[..., ::2, ::2]
def up(x, hw):
    u = torch.zeros(x.shape[0], x.shape[1], hw[0], hw[1], device=x.device, dtype=x.dtype); u[..., ::2, ::2] = x[..., : (hw[0] + 1) // 2, : (hw[1] + 1) // 2]
    return blur(u) * 4
def to_ycc(rgb):   # rgb: B,3,H,W in 0..255
    r, g, b = rgb[:, 0:1], rgb[:, 1:2], rgb[:, 2:3]; co = r - b; t = b + co / 2; cg = g - t; y = t + cg / 2
    return torch.cat([y, co, cg], 1)

def split(rgb, qmax=12.0):
    """-> (coarse YCoCg / 64, fine Y, block log-energy map). Coarse level perturbed like a quantised one."""
    ycc = to_ycc(rgb); g1 = down(ycc)
    q = torch.rand(rgb.shape[0], 1, 1, 1, device=rgb.device) * qmax
    g1q = g1 + (torch.rand_like(g1) - 0.5) * q
    coarse = up(g1q, ycc.shape[-2:]); fine = ycc[:, :1] - up(g1[:, :1], ycc.shape[-2:])
    e = F.avg_pool2d(fine ** 2, 16).clamp_min(1e-4).sqrt()
    eq = torch.round(torch.log2(e) * 8) / 8                          # what the file stores: 1/8-octave steps
    return coarse / 64.0, fine, eq

class Res(nn.Module):
    def __init__(s, c): super().__init__(); s.a = nn.Conv2d(c, c, 3, padding=1); s.b = nn.Conv2d(c, c, 3, padding=1)
    def forward(s, x): return x + s.b(F.leaky_relu(s.a(x), 0.1))
class Net(nn.Module):
    """in: B,cin,H,W (H,W even) -> B,1,H,W. Half resolution inside."""
    def __init__(s, cin, c=24, nres=6):
        super().__init__(); s.inp = nn.Conv2d(cin * 4, c, 3, padding=1); s.body = nn.Sequential(*[Res(c) for _ in range(nres)]); s.out = nn.Conv2d(c, 4, 3, padding=1)
    def forward(s, x): return F.pixel_shuffle(s.out(F.leaky_relu(s.body(F.leaky_relu(s.inp(F.pixel_unshuffle(x, 2)), 0.1)), 0.1)), 2) * 16.0
class Critic(nn.Module):
    def __init__(s, c=48):
        super().__init__()
        s.n = nn.Sequential(nn.Conv2d(2, c, 4, 2, 1), nn.LeakyReLU(0.2), nn.Conv2d(c, 2 * c, 4, 2, 1), nn.LeakyReLU(0.2),
                            nn.Conv2d(2 * c, 2 * c, 4, 2, 1), nn.LeakyReLU(0.2), nn.Conv2d(2 * c, 1, 3, 1, 1))
    def forward(s, fine, coarse_y): return s.n(torch.cat([fine / 16.0, coarse_y], 1))

def t_input(coarse, eq, noise): return torch.cat([coarse, noise, F.interpolate(eq, scale_factor=16, mode="nearest") / 4.0], 1)

def batch(data, B=None, S=128):
    B = B or BATCH
    """Random crops (+ flips / 90-degree turns) from the GPU-resident photos."""
    out = torch.empty(B, 3, S, S, device=dev)
    for i in range(B):
        im = data[torch.randint(len(data), (1,)).item()]; H, W = im.shape[-2:]
        y, x = torch.randint(H - S + 1, (1,)).item(), torch.randint(W - S + 1, (1,)).item()
        c = im[:, y:y + S, x:x + S].float()
        if torch.rand(1) < 0.5: c = c.flip(-1)
        out[i] = torch.rot90(c, int(torch.randint(4, (1,))), (-2, -1))
    return out

def prep(src, dst):
    from PIL import Image
    ims = []
    for p in sorted(glob.glob(f"{src}/*.jpg")):
        im = Image.open(p).convert("RGB"); w, h = im.size
        im = im.resize((w // 2, h // 2), Image.BOX)
        if min(im.size) >= 128: ims.append(torch.from_numpy(__import__("numpy").asarray(im)).permute(2, 0, 1).contiguous())
    torch.save(ims, dst); print(len(ims), "photos,", sum(i.shape[1] * i.shape[2] for i in ims) / 1e6, "Mpx")

def step_fn(kind, P=None):
    torch.backends.cudnn.benchmark = True; torch.backends.cuda.matmul.allow_tf32 = True; torch.backends.cudnn.allow_tf32 = True
    if kind == "P":
        net = Net(3).to(dev, memory_format=torch.channels_last); opt = torch.optim.AdamW(net.parameters(), 2e-3, weight_decay=1e-4)
        def step(rgb):
            coarse, fine, _ = split(rgb)
            with torch.autocast("cuda", torch.bfloat16): pred = net(coarse.contiguous(memory_format=torch.channels_last))
            loss = F.mse_loss(pred.float(), fine); opt.zero_grad(set_to_none=True); loss.backward(); opt.step(); return {"mse": loss.detach()}
        return net, opt, step
    net = Net(5).to(dev, memory_format=torch.channels_last)
    if P is not None:   # start T from P (first layer extended with zeroed noise/energy inputs)
        sd = torch.load(P); w = sd["inp.weight"]; nw = torch.zeros(w.shape[0], 20, 3, 3); nw[:, :12] = w; sd["inp.weight"] = nw
        # pixel_unshuffle orders channels as (cin, 2, 2): the 3 coarse channels are the first 12
        net.load_state_dict(sd)
    D = Critic().to(dev, memory_format=torch.channels_last)
    opt = torch.optim.AdamW(net.parameters(), 1e-3, betas=(0.5, 0.99)); optd = torch.optim.AdamW(D.parameters(), 1e-3, betas=(0.5, 0.99))
    def step(rgb):
        coarse, fine, eq = split(rgb); noise = torch.randn_like(fine)
        with torch.autocast("cuda", torch.bfloat16):
            fake = net(t_input(coarse, eq, noise).contiguous(memory_format=torch.channels_last)).float()
            dr, df = D(fine, coarse[:, :1]), D(fake.detach(), coarse[:, :1])
        ld = F.relu(1 - dr.float()).mean() + F.relu(1 + df.float()).mean(); optd.zero_grad(set_to_none=True); ld.backward(); optd.step()
        with torch.autocast("cuda", torch.bfloat16): g = D(fake, coarse[:, :1]).float()
        ef = F.avg_pool2d(fake ** 2, 8).clamp_min(1e-4).sqrt(); er = F.avg_pool2d(fine ** 2, 8).clamp_min(1e-4).sqrt()
        l1 = (fake - fine).abs().mean(); le = (torch.log(ef) - torch.log(er)).abs().mean(); lg = -g.mean()
        loss = l1 + 4.0 * le + 0.5 * lg; opt.zero_grad(set_to_none=True); loss.backward(); opt.step()
        return {"l1": l1.detach(), "energy": le.detach(), "gan": lg.detach(), "d": ld.detach()}
    return net, opt, step

def macs(net, cin):
    tot = 0
    for m in net.modules():
        if isinstance(m, nn.Conv2d): tot += m.in_channels * m.out_channels * 9 // 4      # convs run at half resolution
    return tot

if __name__ == "__main__":
    cmd = sys.argv[1]
    if cmd == "prep": prep(sys.argv[2], sys.argv[3]); sys.exit()
    data = [d.to(dev) for d in torch.load(sys.argv[2])]
    kind = sys.argv[3]; P = sys.argv[6] if len(sys.argv) > 6 else None
    net, opt, step = step_fn(kind, P)
    print(f"{kind}: {sum(p.numel() for p in net.parameters())} params, ~{macs(net, 3)} MAC per pixel", flush=True)
    if cmd == "prof":
        for i in range(30): step(batch(data))
        torch.cuda.synchronize(); t = time.time()
        for i in range(100): step(batch(data))
        torch.cuda.synchronize(); dt = (time.time() - t) / 100
        t = time.time()
        for i in range(100): batch(data)
        torch.cuda.synchronize(); db = (time.time() - t) / 100
        print(f"{kind} batch {BATCH}: {dt * 1000:.1f} ms/step = {BATCH / dt:.0f} crops/s (batching {db * 1000:.1f} ms), {torch.cuda.max_memory_allocated() / 1e9:.2f} GB", flush=True)
        sys.exit()
    steps, out = int(sys.argv[4]), sys.argv[5]
    sched = torch.optim.lr_scheduler.OneCycleLR(opt, opt.param_groups[0]["lr"], total_steps=steps, pct_start=0.05)
    t0 = time.time(); acc = {}
    for i in range(1, steps + 1):
        r = step(batch(data)); sched.step()
        for k, v in r.items(): acc[k] = acc.get(k, 0) + v
        if i % 500 == 0 or i == steps:
            print(f"{i} " + " ".join(f"{k} {float(v) / 500:.4f}" for k, v in acc.items()) + f"  {time.time() - t0:.0f}s", flush=True); acc = {}
            torch.save(net.state_dict(), out)
