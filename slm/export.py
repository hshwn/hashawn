#!/usr/bin/env python3
"""Export GPT-S3-3M to a flat little-endian f32 blob + JSON manifest, and dump reference
logits for the first N tokens of a text so the Rust port can be checked against PyTorch.
    uv run --with torch --with transformers --with safetensors --with numpy python export.py <model_dir> <text_file> [N]
"""
import json, struct, sys
import numpy as np, torch
from safetensors.torch import load_file
from transformers import AutoTokenizer, AutoModelForCausalLM

mdir, tfile = sys.argv[1], sys.argv[2]; N = int(sys.argv[3]) if len(sys.argv) > 3 else 64
sd = load_file(f"{mdir}/model.safetensors")
manifest, blob, off = [], bytearray(), 0
for name in sorted(sd):
    t = sd[name].float().contiguous().numpy().astype("<f4")
    manifest.append({"name": name, "shape": list(t.shape), "offset": off, "count": int(t.size)})
    blob += t.tobytes(); off += t.size * 4
# rotary tables, so the Rust side needs no libm: fine table for pos & 1023, coarse table for (pos >> 10) * 1024;
# angle = coarse + fine, combined with the angle-sum identities (positions up to 2^20)
import math
ROT, TH = 24, 10000.0
def table(name, angles):
    global off
    cos = [math.cos(a) for a in angles]; sin = [math.sin(a) for a in angles]
    for nm, v in ((f"{name}cos", cos), (f"{name}sin", sin)):
        t = np.array(v, dtype="<f4"); manifest.append({"name": "rope." + nm, "shape": [len(angles) // (ROT // 2), ROT // 2], "offset": off, "count": int(t.size)})
        blob.extend(t.tobytes()); off += t.size * 4
table("", [p * (1.0 / TH ** ((2 * i) / ROT)) for p in range(1024) for i in range(ROT // 2)])
table("c", [(k * 1024) * (1.0 / TH ** ((2 * i) / ROT)) for k in range(1024) for i in range(ROT // 2)])
open("model/weights.bin", "wb").write(blob)
json.dump({"tensors": manifest, "config": json.load(open(f"{mdir}/config.json"))}, open("model/manifest.json", "w"), indent=1)
with open("model/manifest.txt", "w") as f:
    for m in manifest: f.write(f"{m['name']} {m['offset']} {m['count']} {'x'.join(str(d) for d in m['shape'])}\n")
print(f"weights: {len(manifest)} tensors, {off/1e6:.2f} MB f32")
for m in manifest: print(f"  {m['name']:48s} {m['shape']}")

tok = AutoTokenizer.from_pretrained(mdir, trust_remote_code=True)
model = AutoModelForCausalLM.from_pretrained(mdir, trust_remote_code=True, torch_dtype=torch.float32).eval()
text = open(tfile, "rb").read(4000).decode("utf-8", errors="strict")
ids = tok(text, add_special_tokens=False).input_ids[:N]
with torch.no_grad():
    logits = model(torch.tensor([ids])).logits[0].numpy().astype("<f4")   # [N, V], chunked path
    # also the token-by-token path with cache, which is what the streaming port mirrors
    from transformers.cache_utils import DynamicCache
    cache = DynamicCache(); rec = []
    for i, t in enumerate(ids):
        out = model(torch.tensor([[t]]), past_key_values=cache, use_cache=True); cache = out.past_key_values
        rec.append(out.logits[0, 0].numpy())
    rec = np.stack(rec).astype("<f4")
print("chunked vs recurrent max |diff| in PyTorch itself:", float(np.abs(logits - rec).max()))
open("model/ref_ids.bin", "wb").write(struct.pack(f"<{len(ids)}I", *ids))
open("model/ref_logits.bin", "wb").write(rec.tobytes())
# the tokenizer's byte table, so the Rust side can map ids to bytes
bs = list(range(ord("!"), ord("~") + 1)) + list(range(ord("¡"), ord("¬") + 1)) + list(range(ord("®"), ord("ÿ") + 1))
cs = bs[:]; n = 0
for b in range(256):
    if b not in bs: bs.append(b); cs.append(256 + n); n += 1
u2b = {chr(c): b for b, c in zip(bs, cs)}
vocab = []
for i in range(len(tok)):
    s = tok.convert_ids_to_tokens(i); vocab.append(bytes(u2b[ch] for ch in s) if all(ch in u2b for ch in s) else b"")
with open("model/vocab.bin", "wb") as f:
    for b in vocab: f.write(struct.pack("<H", len(b)) + b)
print(f"ref: {len(ids)} tokens; vocab {len(vocab)} entries written")
# BPE merges as ids: (left, right, merged) u16 LE per rank, for the Rust tokenizer
tj = json.load(open(f"{mdir}/tokenizer.json")); tv = tj["model"]["vocab"]
with open("model/merges.bin", "wb") as f:
    for m in tj["model"]["merges"]:
        a, b = (m if isinstance(m, list) else m.split(" ", 1))
        f.write(struct.pack("<HHH", tv[a], tv[b], tv[a + b]))
print(f"merges: {len(tj['model']['merges'])}")
