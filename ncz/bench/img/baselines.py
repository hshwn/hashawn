#!/usr/bin/env python3
"""Image set + baselines for ncz's pixel mode.

    uv run --with imagecodecs --with numpy python baselines.py

Builds raw/<name>.ppm (8-bit RGB, what the Rust codec reads) for the fixed set: the 24 Kodak photos
(./fetch-kodak.sh) and the site's three posters (decoded from their lossy web files, so they stand in for
graphic content, not for camera originals). Then sizes every image with the standard lossless codecs and
JPEG-LS near-lossless (the classic bounded-error codec: every pixel within +-k), and writes baselines.json.
Sizes are in bits per pixel (bpp) over RGB, so raw = 24.
"""
import glob, json, os, sys, time
import numpy as np
import imagecodecs as ic

HERE = os.path.dirname(os.path.abspath(__file__))
SITE = os.path.abspath(os.path.join(HERE, "../../.."))
os.makedirs(os.path.join(HERE, "raw"), exist_ok=True)

def load(path):
    data = open(path, "rb").read()
    a = ic.imread(data)
    if a.ndim == 2: a = np.stack([a] * 3, -1)
    return np.ascontiguousarray(a[..., :3].astype(np.uint8))

items = [(f"kodak/{os.path.basename(p)[:-4]}", p, "photo") for p in sorted(glob.glob(os.path.join(HERE, "kodak/*.png")))]
for name in ["poupees.webp", "ram.webp", "harvest.jpg"]:
    items.append((f"poster/{name.split('.')[0]}", os.path.join(SITE, "assets", name), "poster"))
if len(items) < 27: sys.exit("missing images: run ./fetch-kodak.sh first")

def size_or_none(f):
    try: return len(f())
    except Exception as e: return None

rows = []
for name, path, group in items:
    a = load(path); h, w, _ = a.shape; n = h * w
    raw = os.path.join(HERE, "raw", name.replace("/", "_") + ".ppm")
    with open(raw, "wb") as f: f.write(f"P6\n{w} {h}\n255\n".encode() + a.tobytes())
    r = {"name": name, "group": group, "w": w, "h": h, "ppm": raw}
    t = time.time()
    r["png"] = size_or_none(lambda: ic.png_encode(a, level=9))
    r["webp_ll"] = size_or_none(lambda: ic.webp_encode(a, lossless=True, level=100, method=6))
    r["jxl_e7"] = size_or_none(lambda: ic.jpegxl_encode(a, lossless=True, effort=7))
    r["jxl_e9"] = size_or_none(lambda: ic.jpegxl_encode(a, lossless=True, effort=9))
    r["jpegls"] = size_or_none(lambda: ic.jpegls_encode(a, level=0))
    for k in (1, 2):
        r[f"jpegls_k{k}"] = size_or_none(lambda: ic.jpegls_encode(a, level=k))
    r["secs"] = round(time.time() - t, 1)
    rows.append(r)
    bpp = lambda b: f"{8 * b / n:6.3f}" if b else "   -  "
    print(f"{name:18s} png {bpp(r['png'])}  webp {bpp(r['webp_ll'])}  jxl7 {bpp(r['jxl_e7'])}  jxl9 {bpp(r['jxl_e9'])}  jls {bpp(r['jpegls'])}  jls±1 {bpp(r['jpegls_k1'])}  jls±2 {bpp(r['jpegls_k2'])}  ({r['secs']}s)", flush=True)

json.dump(rows, open(os.path.join(HERE, "baselines.json"), "w"), indent=1)
for group in ("photo", "poster"):
    g = [r for r in rows if r["group"] == group]; n = sum(r["w"] * r["h"] for r in g)
    tot = lambda k: f"{8 * sum(r[k] for r in g) / n:.3f}" if all(r[k] for r in g) else "-"
    print(f"== {group}: png {tot('png')}  webp {tot('webp_ll')}  jxl7 {tot('jxl_e7')}  jxl9 {tot('jxl_e9')}  jpegls {tot('jpegls')}  ±1 {tot('jpegls_k1')}  ±2 {tot('jpegls_k2')} bpp")
