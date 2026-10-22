#!/usr/bin/env python3
"""Lossy stack: learned models + split allowance (brightness tight, colour loose) + grain mode, vs JPEG XL.

    uv run --with numpy --with scipy --with pillow --with scikit-image --with imagecodecs python combo.py <out_dir> <a.ppm> ...

Configurations (pixel coder via `nczcli img-rt`, grain mode via `nczcli grain-rt`):
  u<k>        every sample within +-k, fast mode
  L<k>        same, learned models (flags 7)
  S<k>/<kc>   green within +-k, red and blue within +-kc, learned models
  G<k>/<kc>   grain mode (strip, regrow) with the clean picture coded as S<k>/<kc>
Each is matched by JPEG XL lossy (effort 7) at the same size. Quality: PSNR, CIEDE2000 mean and 99th percentile,
SSIMULACRA2 (models masking; the fair judge for errors hidden in texture), and "dE soft": CIEDE2000 after a light blur (sigma 1) of both pictures, closer to what is seen at normal viewing and
the only fair one for grain mode (its grain is new, so pixel-by-pixel scores punish it by design).
"""
import json, os, subprocess, sys, tempfile
import numpy as np, imagecodecs as ic
from PIL import Image
from scipy import ndimage as ndi
from skimage.color import rgb2lab, deltaE_ciede2000
import ssimulacra2

NCZ = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../target/release/nczcli")
out = sys.argv[1]; os.makedirs(out, exist_ok=True); tmp = tempfile.mkdtemp()
CFG = [("u1", "img", 1, 1, 0), ("u2", "img", 2, 2, 0), ("u3", "img", 3, 3, 0), ("u4", "img", 4, 4, 0), ("L2", "img", 2, 2, 7),
       ("S1/4", "img", 1, 4, 7), ("S1/6", "img", 1, 6, 7), ("S2/4", "img", 2, 4, 7), ("S2/6", "img", 2, 6, 7), ("S2/8", "img", 2, 8, 7),
       ("S3/8", "img", 3, 8, 7), ("G2/6", "grain", 2, 6, 7), ("G2/8", "grain", 2, 8, 7)]
# COMBO_CFG='[["A1/6t8","img",1,1,0,[6,6,8]], ...]': 6th field = adaptive allowance [kmax, kcmax, t] (A<k>...)
if os.environ.get("COMBO_CFG"): CFG = [tuple(c) for c in json.loads(os.environ["COMBO_CFG"])]
SAVE = set(os.environ.get("COMBO_SAVE", "u2,S2/6,G2/6").split(","))

def soft(a): return np.stack([ndi.gaussian_filter(a[..., c], 1.0) for c in range(3)], -1)
def ssim2(a, b):
    """SSIMULACRA2 (100 = identical, ~90 visually lossless, ~70 good): models masking, so it can judge where errors hide."""
    Image.fromarray(a.astype(np.uint8)).save(f"{tmp}/ref.png"); Image.fromarray(np.clip(b, 0, 255).astype(np.uint8)).save(f"{tmp}/dist.png")
    return round(float(ssimulacra2.compute_ssimulacra2(f"{tmp}/ref.png", f"{tmp}/dist.png")), 2)
def quality(a, b):
    b = np.clip(b, 0, 255); mse = ((a - b) ** 2).mean()
    la, lb = rgb2lab(a / 255), rgb2lab(b / 255); de = deltaE_ciede2000(la, lb)
    ds = deltaE_ciede2000(rgb2lab(soft(a) / 255), rgb2lab(soft(b) / 255))
    return dict(psnr=round(float(10 * np.log10(255 ** 2 / max(mse, 1e-9))), 2), de=round(float(de.mean()), 3),
                de99=round(float(np.quantile(de, 0.99)), 2), de_soft=round(float(ds.mean()), 3), ssim2=ssim2(a, b))

def run(kind, k, kc, flags, src, ad=None):
    o = f"{tmp}/o.ppm"
    cmd = [NCZ, "img-rt", str(k), src, o, str(kc), str(flags)] if kind == "img" else [NCZ, "grain-rt", str(k), str(kc), str(flags), src, o]
    if ad: cmd += [str(v) for v in ad]
    n = int(subprocess.run(cmd, capture_output=True, text=True, check=True).stdout)
    return n, np.asarray(Image.open(o), dtype=np.float64)

def jxl_at(a8, nbytes):
    lo, hi = 0.05, 20.0
    for _ in range(14):
        d = (lo * hi) ** 0.5
        if len(ic.jpegxl_encode(a8, distance=d, effort=7)) <= nbytes: hi = d
        else: lo = d
    b = ic.jpegxl_encode(a8, distance=hi, effort=7); return len(b), ic.jpegxl_decode(b).astype(np.float64)

report = {}
for p in sys.argv[2:]:
    name = os.path.basename(p).rsplit(".", 1)[0]; img = np.asarray(Image.open(p), dtype=np.float64); n = img.shape[0] * img.shape[1]
    r = {}
    for tag, kind, k, kc, fl, *ad in CFG:
        nb, dec = run(kind, k, kc, fl, p, ad[0] if ad else None); jb, jd = jxl_at(img.astype(np.uint8), nb)
        r[tag] = dict(bpp=round(8 * nb / n, 3), q=quality(img, dec), maxerr=int(np.abs(img - dec).max()), jxl_bpp=round(8 * jb / n, 3), jxl_q=quality(img, jd))
        if tag in SAVE:
            t = tag.replace('/', '_'); Image.fromarray(dec.astype(np.uint8)).save(f"{out}/{name}.{t}.png"); Image.fromarray(jd.astype(np.uint8)).save(f"{out}/{name}.jxl_{t}.png")
        print(name, tag, json.dumps(r[tag]), flush=True)
    report[name] = r
    json.dump(report, open(f"{out}/combo_report.json", "w"), indent=1)
