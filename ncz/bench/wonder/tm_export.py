#!/usr/bin/env python3
"""The teacher's entropy model (hyperprior decoder + autoregressive context model) -> NCZT weights for ncz::tm, plus
Kodak inputs for `nczcli tm`.

    python tm_export.py <teacher.pth.tar> <out.tm> [<kodak_dir> <out_dir>]

NCZT: "NCZT" | u16 N | medians (N f32) | h_s: 5 convs 3x3 (N->N, N->4N [shuffle], N->3N/2, 3N/2->6N [shuffle],
3N/2->2N), each weight (co, ci, 3, 3) then bias | context: masked 5x5 N->2N, only the 12 causal taps (rows 0-1 all
five, row 2 the two left), weight (2N, N, 12) then bias | entropy parameters: 1x1 4N->10N/3, 10N/3->8N/3, 8N/3->2N,
weight (co, ci) then bias. All f32 LE. Output channels of the last layer: N scales, then N means.
Per photo (with kodak_dir): <name>.y.f32 = u16 C, H, W + f32 latents (channel-major), <name>.z.lat = u16 C, H, W +
i16 round(z - medians).
"""
import os, struct, sys
rest = sys.argv[1:]; sys.argv = sys.argv[:1]
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import pred
pred.dev = "cpu"
import numpy as np, torch
from PIL import Image
from pred import teacher

T = teacher(rest[0]); T.update(force=True)
N = T.h_a[0].in_channels
out = bytearray(b"NCZT"); out += struct.pack("<H", N)
f = lambda t: t.detach().float().contiguous().numpy().astype("<f4").tobytes()
out += f(T.entropy_bottleneck._get_medians().reshape(-1))
for m in (T.h_s[0], T.h_s[2][0], T.h_s[4], T.h_s[6][0], T.h_s[8]): out += f(m.weight) + f(m.bias)
taps = [(r, c) for r in range(5) for c in range(5) if T.context_prediction.mask[0, 0, r, c] > 0]
assert taps == [(0, 0), (0, 1), (0, 2), (0, 3), (0, 4), (1, 0), (1, 1), (1, 2), (1, 3), (1, 4), (2, 0), (2, 1)], taps
w = T.context_prediction.weight
out += f(torch.stack([w[:, :, r, c] for r, c in taps], -1)) + f(T.context_prediction.bias)
for m in (T.entropy_parameters[0], T.entropy_parameters[2], T.entropy_parameters[4]): out += f(m.weight.reshape(m.out_channels, -1)) + f(m.bias)
open(rest[1], "wb").write(bytes(out)); print(f"{rest[1]}: {len(out)} bytes, N={N}")
# NCZU: the 8-bit form (per output row: f32 scale, int8 levels), next to the f32 file
def q8form(buf):
    n = struct.unpack("<H", buf[4:6])[0]; at = 6; o = bytearray(b"NCZU") + buf[4:6]
    def take(k):
        nonlocal at; v = np.frombuffer(buf, "<f4", k, at); at += 4 * k; return v
    o += take(n).tobytes()
    mats = [(n, n * 9), (4 * n, n * 9), (3 * n // 2, n * 9), (6 * n, 3 * n // 2 * 9), (2 * n, 3 * n // 2 * 9), (2 * n, 12 * n),
            (n * 10 // 3, 4 * n), (n * 8 // 3, n * 10 // 3), (2 * n, n * 8 // 3)]
    for rows, cols in mats:
        w = take(rows * cols).reshape(rows, cols); sc = (np.abs(w).max(1) / 127).astype("<f4"); sc[sc == 0] = 1e-12
        o += sc.tobytes() + np.clip(np.round(w / sc[:, None]), -127, 127).astype(np.int8).tobytes(); o += take(rows).tobytes()
    assert at == len(buf); return bytes(o)
q = q8form(bytes(out)); open(rest[1].replace(".tm", ".q8.tm"), "wb").write(q); print(f"8-bit: {len(q)} bytes")
if len(rest) > 2:
    kd, od = rest[2], rest[3]; os.makedirs(od, exist_ok=True)
    with torch.no_grad():
        for name in ["01", "03", "05", "08", "13", "19", "21", "23"]:
            x = torch.from_numpy(np.asarray(Image.open(f"{kd}/kodak_kodim{name}.ppm").convert("RGB"), dtype=np.float32) / 255).permute(2, 0, 1)[None]
            y = T.g_a(x)[0]; z = T.h_a(y[None])[0]
            zq = torch.round(z - T.entropy_bottleneck._get_medians().reshape(-1, 1, 1)).to(torch.int16).numpy()
            open(f"{od}/kodak_kodim{name}.y.f32", "wb").write(np.array(y.shape, np.uint16).tobytes() + y.numpy().astype("<f4").tobytes())
            open(f"{od}/kodak_kodim{name}.z.lat", "wb").write(np.array(zq.shape, np.uint16).tobytes() + zq.tobytes())
    print("kodak inputs ->", od)
