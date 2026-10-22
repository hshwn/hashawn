#!/usr/bin/env python3
"""Two layers, a LOOKS-THE-SAME mode: colours (K flat region colours + label map) x shading (one grey value per pixel).

    python two_layer.py <regions: marigold:<npz_dir> | photo> <K> <smooth> <k> <out_dir> <a.ppm> ...

colours:  regions from k-means (Marigold albedo, or the photo's own chromaticity), mode-filtered (smooth x smooth);
          one colour per region, scaled so its brightest channel is 255; stored as a label map + palette.
shading:  per pixel, the grey value that best explains the photo given the region colour (least squares), 8 bit,
          coded with the pixel coder at +-k. All brightness detail (texture, edges, grain) lives here.
decode:   colour[label] * shading / 255.  Lost: colour variation inside a region.
Costs: shading + labels (both coded with nczcli img-rt; grey pictures as R = G = B) + palette bytes.
Compared at matched size against the pixel coder at +-k and JPEG XL lossy (imagecodecs), by PSNR and CIEDE2000.
"""
import json, os, subprocess, sys, tempfile
import numpy as np, imagecodecs as ic
from PIL import Image
from scipy import ndimage as ndi
from scipy.cluster.vq import kmeans2
from skimage.color import rgb2lab, deltaE_ciede2000

NCZ = os.path.join(os.path.dirname(__file__), "../../target/release/nczcli")
src, K, sm, k, out = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), int(sys.argv[4]), sys.argv[5]; os.makedirs(out, exist_ok=True)
tmp = tempfile.mkdtemp()

def save(a, p): Image.fromarray(np.clip(a, 0, 255).astype(np.uint8)).save(p)
def rt(a, kk):
    """Pixel coder round trip: (bytes, decoded)."""
    pi, po = f"{tmp}/i.ppm", f"{tmp}/o.ppm"; save(a, pi)
    n = int(subprocess.run([NCZ, "img-rt", str(kk), pi, po], capture_output=True, text=True, check=True).stdout)
    return n, np.asarray(Image.open(po), dtype=np.float64)
def grey3(g): return np.repeat(g[..., None], 3, -1)
def quality(a, b):
    mse = ((a - b) ** 2).mean(); psnr = 10 * np.log10(255 ** 2 / max(mse, 1e-9))
    de = deltaE_ciede2000(rgb2lab(a / 255), rgb2lab(np.clip(b, 0, 255) / 255))
    return round(float(psnr), 2), round(float(de.mean()), 3), round(float(np.quantile(de, 0.99)), 2)
def mode_filter(lab, s):
    if s <= 1: return lab
    votes = np.stack([ndi.uniform_filter((lab == j).astype(np.float32), s) for j in range(K)], 0)
    return votes.argmax(0).astype(np.uint8)

report = {}
for p in sys.argv[6:]:
    name = os.path.basename(p).rsplit(".", 1)[0]; img = np.asarray(Image.open(p), dtype=np.float64); H, W, _ = img.shape; n = H * W
    if src.startswith("marigold:"):
        z = np.load(f"{src.split(':', 1)[1]}/{name}.npz"); feat = np.clip(z["albedo"].astype(np.float64), 0, 1) ** (1 / 2.2)
    else:
        feat = img / np.maximum(img.max(-1, keepdims=True), 1)          # chromaticity: brightness divided out
    rng = np.random.default_rng(0); smp = feat.reshape(-1, 3)[rng.choice(n, 20000, replace=False)]
    cent, _ = kmeans2(smp, K, minit="++", seed=1, iter=20)
    lab = ((feat.reshape(-1, 1, 3) - cent[None]) ** 2).sum(-1).argmin(1).reshape(H, W).astype(np.uint8)
    lab = mode_filter(lab, sm)
    # alternate: region colour <-> per-pixel shading (least squares both ways)
    col = np.array([img[lab == j].mean(0) if (lab == j).any() else np.full(3, 128.0) for j in range(K)])
    for _ in range(4):
        col = col / np.maximum(col.max(-1, keepdims=True), 1e-6) * 255
        c = col[lab]; Y = np.clip((img * c).sum(-1) / np.maximum((c * c).sum(-1), 1e-6) * 255, 0, 255)
        for j in range(K):
            m = lab == j
            if m.any(): col[j] = (img[m] * Y[m, None]).sum(0) / max((Y[m] ** 2).sum(), 1e-6) * 255
    col8 = np.clip(np.round(col / np.maximum(col.max(-1, keepdims=True), 1e-6) * 255), 0, 255)
    c = col8[lab]; Y = np.clip(np.round((img * c).sum(-1) / np.maximum((c * c).sum(-1), 1e-6) * 255), 0, 255)
    nb_s, Yd = rt(grey3(Y), k); Yd = Yd[..., 0]
    nb_l, _ = rt(grey3(lab.astype(np.float64)), 0)
    total = nb_s + nb_l + K * 3
    rec = np.clip(np.round(col8[lab] * Yd[..., None] / 255), 0, 255)
    save(rec, f"{out}/{name}.two.ppm")
    r = {"bpp": round(8 * total / n, 3), "shading_bpp": round(8 * nb_s / n, 3), "colours_bpp": round(8 * (nb_l + 3 * K) / n, 3), "two_layer_q": quality(img, rec)}
    # baselines: pixel coder exact and +-k, JPEG XL lossy at the same size
    for kk in (0, 1, 2, 3, 4, 6, 8):
        nb, dec = rt(img, kk); r[f"ncz_pm{kk}"] = [round(8 * nb / n, 3), quality(img, dec)]
        if kk == 0: continue
        if nb <= total and "ncz_matched" not in r: r["ncz_matched"] = [kk, round(8 * nb / n, 3), quality(img, dec)]; save(dec, f"{out}/{name}.ncz.ppm")
    a8 = img.astype(np.uint8); lo, hi = 0.05, 15.0
    for _ in range(18):
        d = (lo * hi) ** 0.5; b = ic.jpegxl_encode(a8, distance=d, effort=7)
        lo, hi = (lo, d) if len(b) <= total else (d, hi)
    b = ic.jpegxl_encode(a8, distance=hi, effort=7); dj = ic.jpegxl_decode(b).astype(np.float64)
    r["jxl_matched"] = [round(hi, 3), round(8 * len(b) / n, 3), quality(img, dj)]; save(dj, f"{out}/{name}.jxl.ppm")
    report[name] = r; print(name, json.dumps(r), flush=True)
json.dump(report, open(f"{out}/report.json", "w"), indent=1)
