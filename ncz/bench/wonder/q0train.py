#!/usr/bin/env python3
"""A lower-rate teacher for tiny mode ("q0"): cheng2020-anchor quality 1 fine-tuned to a smaller lambda.

The smallest size used to be the teacher's lowest quality (lambda 0.0018). Now that swatches carry the fine texture,
the latents can afford to lose more of it: this retrains the q1 weights with CompressAI's own rate-distortion loss
(lambda * 255^2 * MSE + bits per pixel from its entropy model) at a smaller lambda. Rate is measured by the teacher's
own model; our coder tracks it (bench/results.md).

    python q0train.py prof <q1.pth.tar> <div2k_dir>
    python q0train.py train <q1.pth.tar> <div2k_dir> <steps> <out.pth.tar> <kodak_dir>
Env: LMBDA (default 0.0006), LR (5e-5, cosine to 1/10), BATCH (8), CROP (256).
"""
import math, os, sys, time
import numpy as np, torch, torch.nn.functional as F
from PIL import Image
cmd, rest = sys.argv[1], sys.argv[2:]; sys.argv = sys.argv[:1]
from pred import teacher, load_imgs, batch, dev

LMBDA = float(os.environ.get("LMBDA", "0.0006")); LR = float(os.environ.get("LR", "5e-5"))
B = int(os.environ.get("BATCH", "8")); S = int(os.environ.get("CROP", "256"))
KODAK = ["kodak_kodim" + t for t in ["01", "03", "05", "08", "13", "19", "21", "23"]]

def rd(net, x):
    out = net(x); n = x.shape[0] * x.shape[2] * x.shape[3]
    bpp = sum(-torch.log2(l.clamp_min(1e-9)).sum() for l in out["likelihoods"].values()) / n
    mse = F.mse_loss(out["x_hat"].clamp(0, 1), x)
    return LMBDA * 255 ** 2 * mse + bpp, bpp, mse

def evaluate(net, kd):
    """Kodak: PSNR of g_s(round(g_a(x))) (what tiny mode decodes) and the teacher's own bits for those latents."""
    net.eval(); ps, bp = [], []
    with torch.no_grad():
        for n in KODAK:
            x = torch.from_numpy(np.asarray(Image.open(f"{kd}/{n}.ppm").convert("RGB"), dtype=np.float32) / 255).permute(2, 0, 1)[None].to(dev)
            y = torch.round(net.g_a(x)); xh = net.g_s(y).clamp(0, 1)
            ps.append(10 * math.log10(1 / F.mse_loss(xh, x).item()))
            out = net(x); bp.append((sum(-torch.log2(l.clamp_min(1e-9)).sum() for l in out["likelihoods"].values()) / (x.shape[2] * x.shape[3])).item())
    net.train(); return float(np.mean(ps)), float(np.mean(bp))

if __name__ == "__main__":
    torch.backends.cudnn.benchmark = True; torch.backends.cuda.matmul.allow_tf32 = True; torch.backends.cudnn.allow_tf32 = True
    net = teacher(rest[0]); net.train()
    for p in net.parameters(): p.requires_grad_(True)
    ims = load_imgs(rest[1], S); print(f"{len(ims)} photos", flush=True)
    main = [p for n, p in net.named_parameters() if not n.endswith(".quantiles")]
    aux = [p for n, p in net.named_parameters() if n.endswith(".quantiles")]
    opt = torch.optim.Adam(main, LR); aopt = torch.optim.Adam(aux, 1e-3)
    def step(i, total):
        for g in opt.param_groups: g["lr"] = LR * (0.1 + 0.9 * 0.5 * (1 + math.cos(math.pi * i / total)))
        x = batch(ims, B, S)
        if torch.rand(1) < 0.5: x = x.flip(-1)
        loss, bpp, mse = rd(net, x); opt.zero_grad(set_to_none=True); loss.backward()
        torch.nn.utils.clip_grad_norm_(main, 1.0); opt.step()
        al = net.aux_loss(); aopt.zero_grad(set_to_none=True); al.backward(); aopt.step()
        return loss.item(), bpp.item(), mse.item()
    if cmd == "prof":
        for i in range(10): step(i, 100)
        torch.cuda.synchronize(); t = time.time()
        for i in range(40): step(i, 100)
        torch.cuda.synchronize(); print(f"batch {B} crops {S}px: {(time.time() - t) / 40 * 1000:.1f} ms/step, {torch.cuda.max_memory_allocated() / 1e9:.1f} GB", flush=True)
        sys.exit()
    steps, out, kd = int(rest[2]), rest[3], rest[4]
    p0, b0 = evaluate(net, kd); print(f"start (q1 weights): Kodak {p0:.2f} dB at {b0:.4f} bpp (teacher's own rate)", flush=True)
    t = time.time(); acc = np.zeros(3)
    for i in range(1, steps + 1):
        acc += step(i, steps)
        if i % 1000 == 0:
            a = acc / 1000; acc[:] = 0
            print(f"{i} loss {a[0]:.4f} bpp {a[1]:.4f} train {10 * math.log10(1 / a[2]):.2f} dB  {time.time() - t:.0f}s", flush=True)
        if i % 5000 == 0 or i == steps:
            p, b = evaluate(net, kd); print(f"  Kodak {p:.2f} dB at {b:.4f} bpp", flush=True)
            torch.save(net.state_dict(), out)
    print("finished", flush=True)
