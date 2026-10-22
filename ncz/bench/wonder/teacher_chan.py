#!/usr/bin/env python3
"""Teacher's own bits per latent channel (y) and for its side info (z): teacher_chan.py <weights_dir> <kodak_dir> <out.json> <names...>"""
import json, os, sys
sys.argv, rest = sys.argv[:4] + ["x"] * 0, sys.argv[4:]
src = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "teacher.py")).read().split("wdir, kdir, out")[0]
exec(src)
wdir, kdir, outp = sys.argv[1], sys.argv[2], sys.argv[3]
res = {}
for f in sorted(x for x in os.listdir(wdir) if x.endswith(".pth.tar")):
    q = int(f.split("-")[2])
    net = Cheng2020Anchor.from_state_dict(load_state_dict(torch.load(os.path.join(wdir, f), map_location="cpu"))).cuda().eval(); net.update(force=True)
    for name in rest:
        x = torch.from_numpy(np.asarray(Image.open(f"{kdir}/{name}.ppm").convert("RGB"), dtype=np.float32) / 255).permute(2, 0, 1)[None].cuda()
        with torch.no_grad(): r = net(x)
        ly = (-torch.log2(r["likelihoods"]["y"]))[0].sum((1, 2)).cpu().numpy().tolist(); lz = float((-torch.log2(r["likelihoods"]["z"])).sum())
        res.setdefault(name, {})[q] = {"y": ly, "z": lz}
json.dump(res, open(outp, "w"))
