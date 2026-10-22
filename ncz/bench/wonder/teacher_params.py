#!/usr/bin/env python3
"""Dump the teacher's predicted mean and scale per latent (diagnostic ceiling, not a deterministic decoder):
teacher_params.py <weights_dir> <kodak_dir> <out_dir> <names...> -> <name>.q<k>.par: u16 C,H,W | i16 mean*16 | u16 scale*64"""
import os, sys
rest = sys.argv[4:]; sys.argv = sys.argv[:4]
exec(open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "teacher.py")).read().split("wdir, kdir, out")[0])
wdir, kdir, out = sys.argv[1], sys.argv[2], sys.argv[3]; os.makedirs(out, exist_ok=True)
for f in sorted(x for x in os.listdir(wdir) if x.endswith(".pth.tar")):
    q = int(f.split("-")[2])
    net = Cheng2020Anchor.from_state_dict(load_state_dict(torch.load(os.path.join(wdir, f), map_location="cpu"))).cuda().eval(); net.update(force=True)
    for name in rest:
        x = torch.from_numpy(np.asarray(Image.open(f"{kdir}/{name}.ppm").convert("RGB"), dtype=np.float32) / 255).permute(2, 0, 1)[None].cuda()
        with torch.no_grad():
            y = net.g_a(x); z_hat, _ = net.entropy_bottleneck(net.h_a(y)); params = net.h_s(z_hat)
            y_hat = torch.round(y); ctx = net.context_prediction(y_hat)
            scales, means = net.entropy_parameters(torch.cat((params, ctx), 1)).chunk(2, 1)
        mu = torch.clamp(torch.round(means * 16), -32767, 32767)[0].to(torch.int16).cpu().numpy()
        sc = torch.clamp(torch.round(scales.abs() * 64), 0, 65535)[0].cpu().numpy().astype(np.uint16)
        with open(f"{out}/{name}.q{q}.par", "wb") as fh: fh.write(np.array(mu.shape, dtype=np.uint16).tobytes()); fh.write(mu.tobytes()); fh.write(sc.tobytes())
print("ok")
