#!/usr/bin/env python3
"""Wonder format, learned base: picture -> integer latents (coded by our context-mixing coder) -> structure -> texture.

    python base.py prof [batch] [crop]          # time a training step on synthetic data (no training)

Encoder: YCoCg/64 -> 4 stride-2 stages -> M-channel latent at 1/16 per side, scaled by a per-quality gain vector
(one model, every quality: the gain picks the step size per channel), rounded to integers.
Rate (training only): a causal context model over the integer latents (5x5 masked conv + channel context) gives a
Gaussian per latent; bits = -log2 P(bin). At coding time the integers go to the context-mixing coder instead, which
uses only already-decoded integers as context, so entropy decoding never depends on a network's output and no
cross-device float mismatch can break a file. (The proxy only has to teach the encoder what is cheap.)
Decoder: inverse gain -> 4 upsampling stages (pixel shuffle) -> structure; texture head (as T in train.py: structure +
noise -> fine detail) on top. Stage 1 trains rate + distortion; stage 2 adds the texture head with a patch critic.
"""
import sys, time
import torch, torch.nn as nn, torch.nn.functional as F

dev = "cuda"
N, M, NQ = 96, 160, 8          # trunk channels, latent channels, quality levels (gain vectors)

class Res(nn.Module):
    def __init__(s, c): super().__init__(); s.a = nn.Conv2d(c, c, 3, padding=1); s.b = nn.Conv2d(c, c, 3, padding=1)
    def forward(s, x): return x + s.b(F.leaky_relu(s.a(x), 0.1))
def down(ci, co): return nn.Sequential(nn.Conv2d(ci, co, 5, 2, 2), nn.LeakyReLU(0.1), Res(co))
def upb(ci, co): return nn.Sequential(nn.Conv2d(ci, co * 4, 3, padding=1), nn.PixelShuffle(2), nn.LeakyReLU(0.1), Res(co))

class Enc(nn.Module):
    def __init__(s): super().__init__(); s.n = nn.Sequential(down(3, 48), down(48, 64), down(64, N), nn.Conv2d(N, M, 5, 2, 2))
    def forward(s, x): return s.n(x)
class Dec(nn.Module):
    # channels shrink as resolution grows: the cost per decoded pixel stays browser-sized
    def __init__(s): super().__init__(); s.n = nn.Sequential(upb(M, N), upb(N, 64), upb(64, 48), upb(48, 32), nn.Conv2d(32, 3, 3, padding=1))
    def forward(s, y): return s.n(y)

class MaskedConv(nn.Conv2d):
    def __init__(s, ci, co, k):
        super().__init__(ci, co, k, padding=k // 2); m = torch.ones_like(s.weight); m[:, :, k // 2, k // 2:] = 0; m[:, :, k // 2 + 1:] = 0
        s.register_buffer("mask", m)
    def forward(s, x): return F.conv2d(x, s.weight * s.mask, s.bias, padding=s.padding)
class Rate(nn.Module):
    """Training-time bit estimate for integer latents (causal spatial context)."""
    def __init__(s):
        super().__init__(); s.ctx = MaskedConv(M, 2 * M, 5)
        s.par = nn.Sequential(nn.Conv2d(2 * M, 2 * M, 1), nn.LeakyReLU(0.1), nn.Conv2d(2 * M, 2 * M, 1))
    def forward(s, yq):
        mu, ls = s.par(s.ctx(yq)).chunk(2, 1); sig = F.softplus(ls) + 0.11
        d = torch.distributions.Normal(mu, sig)
        p = (d.cdf(yq + 0.5) - d.cdf(yq - 0.5)).clamp_min(1e-9)
        return -torch.log2(p).sum((1, 2, 3))

class Codec(nn.Module):
    def __init__(s):
        super().__init__(); s.enc, s.dec, s.rate = Enc(), Dec(), Rate()
        s.gain = nn.Parameter(torch.ones(NQ, M)); s.igain = nn.Parameter(torch.ones(NQ, M))
    def forward(s, x, qi, train=True):
        y = s.enc(x) * s.gain[qi][..., None, None]
        if train: yn = y + torch.rand_like(y) - 0.5; yq = (torch.round(y) - y).detach() + y      # noise for rate, straight-through for the decoder
        else: yn = yq = torch.round(y)
        bits = s.rate(yn)
        xh = s.dec(yq * s.igain[qi][..., None, None])
        return xh, bits

LAMBDAS = torch.tensor([0.0016, 0.003, 0.0055, 0.01, 0.018, 0.032, 0.06, 0.11])    # quality levels (MSE in 8-bit units^2)

def step_fn():
    torch.backends.cudnn.benchmark = True; torch.backends.cuda.matmul.allow_tf32 = True; torch.backends.cudnn.allow_tf32 = True
    net = Codec().to(dev, memory_format=torch.channels_last)
    opt = torch.optim.AdamW(net.parameters(), 1e-4)
    lam = LAMBDAS.to(dev)
    def step(x):                                   # x: B,3,S,S YCoCg/64
        qi = torch.randint(NQ, (x.shape[0],), device=dev)
        with torch.autocast("cuda", torch.bfloat16): xh, bits = net(x.contiguous(memory_format=torch.channels_last), qi)
        npx = x.shape[-1] * x.shape[-2]
        mse = ((xh.float() - x) * 64).pow(2).mean((1, 2, 3))
        loss = (bits.float() / npx + lam[qi] * mse).mean()
        opt.zero_grad(set_to_none=True); loss.backward(); torch.nn.utils.clip_grad_norm_(net.parameters(), 1.0); opt.step()
        return loss.detach()
    return net, step

def macs_per_px(net, S=256):
    tot = 0
    def hook(m, i, o):
        nonlocal tot
        if isinstance(m, nn.Conv2d): tot += o.shape[2] * o.shape[3] * m.in_channels * m.out_channels * m.kernel_size[0] * m.kernel_size[1] // m.groups
    hs = [m.register_forward_hook(hook) for m in net.dec.modules()]
    with torch.no_grad(): net.dec(torch.zeros(1, M, S // 16, S // 16, device=dev))
    for h in hs: h.remove()
    return tot / (S * S)

if __name__ == "__main__":
    B = int(sys.argv[2]) if len(sys.argv) > 2 else 16; S = int(sys.argv[3]) if len(sys.argv) > 3 else 256
    net, step = step_fn()
    import os
    if os.environ.get("COMPILE"): step = torch.compile(step)
    print(f"params {sum(p.numel() for p in net.parameters()) / 1e6:.2f}M (decoder {sum(p.numel() for p in net.dec.parameters()) / 1e6:.2f}M), decoder {macs_per_px(net) / 1e3:.0f}k MAC per pixel", flush=True)
    x = torch.randn(B, 3, S, S, device=dev) * 0.5
    for _ in range(10): step(x)
    torch.cuda.synchronize(); t = time.time()
    for _ in range(40): step(x)
    torch.cuda.synchronize(); dt = (time.time() - t) / 40
    print(f"batch {B} crop {S}: {dt * 1000:.1f} ms/step, {B / dt:.0f} crops/s, {torch.cuda.max_memory_allocated() / 1e9:.1f} GB", flush=True)
