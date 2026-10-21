#!/usr/bin/env python3
"""Turn a causal LM's token predictions into a per-BIT prediction for every bit of a file,
for feeding into ncz's mixer as a side input.

    uv run --with torch --with transformers --with safetensors --with numpy \
        python lm_side.py <model> <file> --bytes 100000 --out file.side

For each token boundary the model gives a distribution over the next token. Inside a token,
the probability of the next byte given the bytes of the token so far is the mass of tokens
that extend the prefix, divided by the mass that match the prefix (exact marginalisation over
the vocabulary). Each byte's 8 binary decisions follow the same way. This is exact under the
chain rule, so summing -log2 over the actual bits reproduces the model's bits per byte.

Caveat it reports: this assumes the decoder knows where tokens end. A real codec must spend
bits on that; the "boundary cost" line is -log2 P(token ends here) summed over tokens.

Side file: little-endian i16 per bit, stretch(P(bit=1)) = round(256 * ln(p/(1-p))), clamped
to +-2047, in bit order (MSB first), 8 per byte.
"""
import argparse, math, struct, sys, time

import numpy as np
import torch
from transformers import AutoModelForCausalLM, AutoTokenizer


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("model"); ap.add_argument("file")
    ap.add_argument("--bytes", type=int, default=100_000)
    ap.add_argument("--out", required=True)
    a = ap.parse_args()
    dev = "mps" if torch.backends.mps.is_available() else "cpu"
    tok = AutoTokenizer.from_pretrained(a.model, trust_remote_code=True)
    model = AutoModelForCausalLM.from_pretrained(a.model, trust_remote_code=True, torch_dtype=torch.float32).to(dev).eval()

    raw = open(a.file, "rb").read()[: a.bytes]
    text = raw.decode("utf-8", errors="strict")
    enc = tok(text, return_tensors="pt", add_special_tokens=False)
    ids = enc.input_ids[0]
    # bytes of every vocab entry, so we can marginalise over prefixes
    V = len(tok)
    # byte-level BPE (GPT-2 style): every byte is a stand-in unicode char in the token string.
    # Invert that table so each token maps to its exact bytes, multi-byte UTF-8 included.
    bs = list(range(ord("!"), ord("~") + 1)) + list(range(ord("¡"), ord("¬") + 1)) + list(range(ord("®"), ord("ÿ") + 1))
    cs = bs[:]; n = 0
    for b in range(256):
        if b not in bs: bs.append(b); cs.append(256 + n); n += 1
    uni2byte = {chr(c): b for b, c in zip(bs, cs)}
    def tok2bytes(tstr): return bytes(uni2byte[ch] for ch in tstr) if all(ch in uni2byte for ch in tstr) else b""
    vocab_bytes = [tok2bytes(tok.convert_ids_to_tokens(i)) for i in range(V)]
    tok_bytes = [vocab_bytes[int(i)] for i in ids]
    assert b"".join(tok_bytes) == raw, "tokenisation is not byte-reversible on this input"
    maxlen = max(len(b) for b in vocab_bytes)
    # table[k][v] = k-th byte of token v, or -1
    byte_at = np.full((maxlen, V), -1, dtype=np.int16)
    for v, b in enumerate(vocab_bytes):
        for k, c in enumerate(b): byte_at[k, v] = c
    length = np.array([len(b) for b in vocab_bytes], dtype=np.int32)

    ctx = getattr(model.config, "max_position_embeddings", None) or getattr(model.config, "block_size", 1024)
    stride = ctx // 2
    n_tok = ids.numel()
    # next-token distributions at every boundary: probs[t] = P(token t | tokens < t)
    probs = np.zeros((n_tok, V), dtype=np.float32)
    t0 = time.time()
    with torch.no_grad():
        start = 0
        while start < n_tok:
            end = min(start + ctx, n_tok)
            window = ids[start:end].unsqueeze(0).to(dev)
            lp = torch.softmax(model(window).logits[0].float(), dim=-1).cpu().numpy()   # [T, V]
            first = 1 if start == 0 else (ctx - stride)
            # lp[i] predicts token i+1 of the window
            probs[start + first: end] = lp[first - 1: end - start - 1]
            if end == n_tok: break
            start += stride
    probs[0] = 1.0 / V                       # first token: uniform
    print(f"model pass: {n_tok} tokens in {time.time() - t0:.1f}s", file=sys.stderr)

    side = np.zeros(len(raw) * 8, dtype=np.int16)
    lm_bits = 0.0; boundary_bits = 0.0
    pos = 0
    t1 = time.time()
    for t in range(n_tok):
        D = probs[t]
        tb = tok_bytes[t]
        match = np.ones(V, dtype=bool)          # tokens matching the prefix of this token so far
        for k, c in enumerate(tb):
            match &= length > k
            kb = byte_at[k]
            mass_prefix = float(D[match].sum())
            # 8 binary decisions for byte c, MSB first
            m = match.copy()
            for j in range(8):
                shift = 7 - j
                m1 = m & (((kb >> shift) & 1) == 1)
                p1 = float(D[m1].sum()); p0 = float(D[m & ~(((kb >> shift) & 1) == 1)].sum())
                tot = p1 + p0
                p = (p1 + 1e-9) / (tot + 2e-9) if tot > 0 else 0.5
                bit = (c >> shift) & 1
                lm_bits += -math.log2(p if bit else 1 - p)
                st = int(round(256 * math.log(p / (1 - p)))); st = max(-2047, min(2047, st))
                side[(pos + k) * 8 + j] = st
                m = m1 if bit else (m & ~m1)
            match = m
            # cost of telling the decoder whether the token ends after this byte
            if k == len(tb) - 1:
                exact = float(D[match & (length == len(tb))].sum())
                boundary_bits += -math.log2(max(1e-12, exact / max(1e-12, float(D[match].sum()))))
        pos += len(tb)
    n = len(raw)
    print(f"per-bit pass: {time.time() - t1:.1f}s", file=sys.stderr)
    print(f"bytes {n}  tokens {n_tok}")
    print(f"LM bits per byte (exact, chain rule): {lm_bits / n:.4f}")
    print(f"token-boundary cost a real codec pays: {boundary_bits / n:.4f} bpb  -> LM codec total {(lm_bits + boundary_bits) / n:.4f}")
    with open(a.out, "wb") as f: f.write(side.astype("<i2").tobytes())
    print(f"wrote {a.out}: {len(side)} bit predictions")


if __name__ == "__main__":
    sys.exit(main())
