#!/usr/bin/env python3
"""Score the teacher test: latents through our coder, SSIMULACRA2 vs JPEG XL at the same size.

    uv run --with numpy --with pillow --with imagecodecs --with ssimulacra2 python teacher_eval.py <tout_dir> <kodak_dir> [names...]

Bytes: the integer latents (channel-major, zigzag bytes) through ncz's byte coder, as a first, generic stand-in for a
latent context model; and the teacher's own rate (its learned entropy model), the bar for our coder. Quality:
SSIMULACRA2 of the teacher's reconstruction, and of JPEG XL (effort 7) at our byte count.
"""
import json, os, subprocess, sys, tempfile
import numpy as np, imagecodecs as ic, ssimulacra2
from PIL import Image

NCZ = os.path.join(os.path.dirname(os.path.abspath(__file__)), "../../target/release/nczcli")
TMP = tempfile.mkdtemp()
tdir, kdir = sys.argv[1], sys.argv[2]
rep = json.load(open(f"{tdir}/report.json"))
names = sys.argv[3:] or sorted(rep)

def ncz_bytes(raw):
    open(f"{TMP}/a", "wb").write(raw); subprocess.run([NCZ, "compress", f"{TMP}/a", f"{TMP}/b"], check=True); return os.path.getsize(f"{TMP}/b")
def s2(a, b):
    Image.fromarray(a).save(f"{TMP}/r.png"); Image.fromarray(b).save(f"{TMP}/d.png")
    return float(ssimulacra2.compute_ssimulacra2(f"{TMP}/r.png", f"{TMP}/d.png"))
def jxl_at(a8, nb):
    lo, hi = 0.05, 30.0
    for _ in range(14):
        d = (lo * hi) ** 0.5
        if len(ic.jpegxl_encode(a8, distance=d, effort=7)) <= nb: hi = d
        else: lo = d
    b = ic.jpegxl_encode(a8, distance=hi, effort=7); return len(b), ic.jpegxl_decode(b)

out = {}
for n in names:
    img = np.asarray(Image.open(f"{kdir}/{n}.ppm").convert("RGB")); px = img.shape[0] * img.shape[1]
    for q in sorted(rep[n], key=int):
        raw = open(f"{tdir}/{n}.q{q}.lat", "rb").read(); c, h, w = np.frombuffer(raw[:6], np.uint16)
        lat = np.frombuffer(raw[6:], np.int16).reshape(c, h, w).astype(np.int32)
        z = np.where(lat >= 0, 2 * lat, -2 * lat - 1); nb = ncz_bytes(z.astype(np.uint8).tobytes() if z.max() < 256 else z.astype(np.uint16).tobytes()) + 8
        rec = np.asarray(Image.open(f"{tdir}/{n}.q{q}.png").convert("RGB"))
        jb, jd = jxl_at(img, nb)
        own_b = rep[n][q]["own_bpp"] * px / 8; jb2, jd2 = jxl_at(img, own_b)
        r = dict(ours_bpp=round(8 * nb / px, 4), own_bpp=rep[n][q]["own_bpp"], s2=round(s2(img, rec), 2),
                 jxl_s2_at_ours=round(s2(img, jd), 2), jxl_s2_at_own=round(s2(img, jd2), 2))
        out.setdefault(n, {})[q] = r; print(n, q, json.dumps(r), flush=True)
json.dump(out, open(f"{tdir}/eval.json", "w"), indent=1)
