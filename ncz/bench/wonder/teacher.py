#!/usr/bin/env python3
"""Teacher test: a pretrained learned codec (CompressAI cheng2020-anchor, qualities 1-6) as the base, run on Kodak.

    python teacher.py <weights_dir> <kodak_dir> <out_dir>      # on the rig (GPU)

For each picture and quality: y = encoder(x); integer latents = round(y) (plain rounding: our context-mixing coder
codes integers with only already-decoded integers as context, no network in the entropy decoding); reconstruction =
decoder(integers). Writes <name>.q<k>.png (reconstruction) and <name>.q<k>.lat (int16 latents, C x H x W, with a
small header) for coding on the Mac, and report.json with the teacher's OWN rate (its hyperprior + context model's
likelihoods, the bar our coder has to reach) for comparison.
"""
import importlib.abc, importlib.machinery, json, os, sys, types

class _Stub(importlib.abc.MetaPathFinder, importlib.abc.Loader):
    """compressai imports point-cloud deps (torch_geometric, ...) at package import; images do not need them."""
    NAMES = {"torch_geometric", "pointops", "torch_scatter", "torch_cluster"}
    def find_spec(self, name, path, target=None):
        if name.split(".")[0] in self.NAMES: return importlib.machinery.ModuleSpec(name, self, is_package=True)
    def create_module(self, spec):
        m = types.ModuleType(spec.name); m.__path__ = []
        def ga(attr):
            if attr.startswith("__"): raise AttributeError(attr)
            return type(attr, (), {"__init__": lambda s, *a, **k: None, "__call__": lambda s, *a, **k: None})
        m.__getattr__ = ga
        return m
    def exec_module(self, module): pass

_stub = _Stub(); sys.meta_path.insert(0, _stub)
for _ in range(20):                       # stub every optional dependency the point-cloud code wants, then import
    try:
        from compressai.models import Cheng2020Anchor
        from compressai.zoo.pretrained import load_pretrained as load_state_dict
        break
    except ModuleNotFoundError as e:
        _stub.NAMES.add(e.name.split(".")[0])
        for k in [k for k in sys.modules if k.startswith("compressai")]: del sys.modules[k]
import numpy as np, torch
from PIL import Image

wdir, kdir, out = sys.argv[1], sys.argv[2], sys.argv[3]; os.makedirs(out, exist_ok=True)
dev = "cuda"
files = sorted(f for f in os.listdir(wdir) if f.endswith(".pth.tar"))
report = {}
for f in files:
    q = int(f.split("-")[2])
    sd = load_state_dict(torch.load(os.path.join(wdir, f), map_location="cpu"))
    net = Cheng2020Anchor.from_state_dict(sd).to(dev).eval(); net.update(force=True)
    for img in sorted(os.listdir(kdir)):
        name = img.rsplit(".", 1)[0]
        x = torch.from_numpy(np.asarray(Image.open(os.path.join(kdir, img)).convert("RGB"), dtype=np.float32) / 255).permute(2, 0, 1)[None].to(dev)
        H, W = x.shape[-2:]; ph, pw = (-H) % 64, (-W) % 64
        xp = torch.nn.functional.pad(x, (0, pw, 0, ph), mode="replicate")
        with torch.no_grad():
            r = net(xp)                                           # the teacher's own likelihoods (its rate)
            own = sum((-torch.log2(l)).sum().item() for l in r["likelihoods"].values())
            y = net.g_a(xp); yi = torch.round(y)
            xh = net.g_s(yi).clamp(0, 1)[..., :H, :W]
        rec = (xh[0].permute(1, 2, 0).cpu().numpy() * 255 + 0.5).astype(np.uint8)
        Image.fromarray(rec).save(f"{out}/{name}.q{q}.png")
        lat = yi[0].to(torch.int16).cpu().numpy()
        with open(f"{out}/{name}.q{q}.lat", "wb") as fh:
            fh.write(np.array(lat.shape, dtype=np.uint16).tobytes()); fh.write(lat.tobytes())
        report.setdefault(name, {})[q] = dict(own_bpp=round(own / (H * W), 4), latent_range=[int(lat.min()), int(lat.max())], channels=int(lat.shape[0]),
                                            nonzero=round(float((lat != 0).mean()), 4))
        print(name, q, report[name][q], flush=True)
json.dump(report, open(f"{out}/report.json", "w"), indent=1)
