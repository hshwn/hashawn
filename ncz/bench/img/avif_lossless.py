"""AVIF lossless on the same set, verified exact by decoding. uv run --with imagecodecs --with numpy python avif_lossless.py"""
import glob, json, os, numpy as np, imagecodecs as ic
rows = json.load(open('baselines.json'))
def load_ppm(p):
    b = open(p, 'rb').read(); parts = b.split(maxsplit=4); w, h = int(parts[1]), int(parts[2])
    return np.frombuffer(parts[4][: w * h * 3], np.uint8).reshape(h, w, 3)
for r in rows:
    a = load_ppm(r['ppm']); best = None
    for kw in [dict(level=100, pixelformat='444'), dict(level=100), dict(level=-1)]:
        try:
            enc = ic.avif_encode(a, **kw); dec = ic.avif_decode(enc)
            if dec.shape == a.shape and np.array_equal(dec, a): best = (len(enc), str(kw)); break
        except Exception as e: pass
    r['avif_ll'] = best[0] if best else None; r['avif_mode'] = best[1] if best else 'not exact'
    print(f"{r['name']:18s} {8 * best[0] / (r['w'] * r['h']):6.3f} bpp  {best[1]}" if best else f"{r['name']:18s} no exact setting", flush=True)
json.dump(rows, open('baselines.json', 'w'), indent=1)
