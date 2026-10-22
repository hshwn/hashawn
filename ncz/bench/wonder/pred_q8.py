#!/usr/bin/env python3
"""NCZP predictor (f32) -> NCZQ (8-bit weight matrices: per-row f32 scale + int8 levels; biases f32).
    python pred_q8.py <in.pred> <out.pred> [<dequantised-f32 check out>]"""
import struct, sys, numpy as np
b = open(sys.argv[1], 'rb').read(); assert b[:4] == b'NCZP'
C, ng, wd = struct.unpack('<HHH', b[4:10]); sizes = struct.unpack(f'<{ng}H', b[10:10 + 2 * ng]); off = 10 + 2 * ng + 2 * C
f = np.frombuffer(b[off:], '<f4'); i = 0; q = bytearray(b'NCZQ' + b[4:off]); deq = bytearray(b[:off])
for gi, g in enumerate(sizes):
    st = sum(sizes[:gi])
    for sh in ([(2 * g,)] if gi == 0 else [(wd, st), (wd,), (wd, wd * 9), (wd,), (wd, wd * 9), (wd,), (2 * g, wd * 9), (2 * g,)]):
        n = int(np.prod(sh)); a = f[i:i + n].reshape(sh); i += n
        if len(sh) == 2:
            sc = (np.abs(a).max(1) / 127).astype(np.float32); sc[sc == 0] = 1
            k = np.clip(np.round(a / sc[:, None]), -128, 127).astype(np.int8)
            q += sc.astype('<f4').tobytes() + k.tobytes(); deq += (k.astype(np.float32) * sc[:, None]).astype('<f4').tobytes()
        else: q += a.astype('<f4').tobytes(); deq += a.astype('<f4').tobytes()
assert i == len(f)
open(sys.argv[2], 'wb').write(q)
if len(sys.argv) > 3: open(sys.argv[3], 'wb').write(deq)
print(len(b), '->', len(q), 'bytes')
