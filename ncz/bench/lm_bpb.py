#!/usr/bin/env python3
"""Bits per byte of a causal LM on a text file, the number that matters for compression.

    uv run --with torch --with transformers --with safetensors --with sentencepiece \
        python lm_bpb.py AxiomicLabs/GPT-S3-3M corpus/text-enwik8-10mb.txt --bytes 1000000

Sums -log2 p(token) over the text under the model (teacher forcing, sliding windows the size
of the model's context), divides by the number of BYTES of input, so tokenizer efficiency is
included. This is exactly what an arithmetic coder driven by the model would produce, minus
a few bytes of coder overhead. Also prints tokens/s so you know the decode-side speed ceiling.

Loads the repo's own modelling code (trust_remote_code): read it first, it runs on your machine.
"""
import argparse, math, sys, time

import torch
from transformers import AutoModelForCausalLM, AutoTokenizer


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("model")
    ap.add_argument("file")
    ap.add_argument("--bytes", type=int, default=1_000_000, help="how much of the file to score")
    ap.add_argument("--ctx", type=int, default=0, help="window size (default: model's max)")
    ap.add_argument("--stride", type=int, default=0, help="window stride (default: ctx/2)")
    a = ap.parse_args()

    dev = "mps" if torch.backends.mps.is_available() else ("cuda" if torch.cuda.is_available() else "cpu")
    tok = AutoTokenizer.from_pretrained(a.model, trust_remote_code=True)
    model = AutoModelForCausalLM.from_pretrained(a.model, trust_remote_code=True, torch_dtype=torch.float32).to(dev).eval()

    raw = open(a.file, "rb").read()[: a.bytes]
    text = raw.decode("utf-8", errors="replace")
    ids = tok(text, return_tensors="pt", add_special_tokens=False).input_ids[0]
    n_tok = ids.numel()
    ctx = a.ctx or getattr(model.config, "max_position_embeddings", None) or getattr(model.config, "block_size", 1024)
    stride = a.stride or ctx // 2

    nll_bits = 0.0; scored = 0; t0 = time.time()
    with torch.no_grad():
        start = 0
        while start < n_tok:
            end = min(start + ctx, n_tok)
            window = ids[start:end].unsqueeze(0).to(dev)
            logits = model(window).logits[0]                       # [T, V]
            logp = torch.log_softmax(logits.float(), dim=-1)
            # score tokens 1..T-1 of the window given the ones before; on overlapping windows,
            # only count the part not scored by the previous window
            first = 1 if start == 0 else (ctx - stride)
            tgt = window[0, first:]
            lp = logp[first - 1:-1].gather(1, tgt.unsqueeze(1)).squeeze(1)
            nll_bits += float(-lp.sum() / math.log(2)); scored += int(tgt.numel())
            if end == n_tok: break
            start += stride
    dt = time.time() - t0
    # the very first token has no context: charge a flat 16 bits for it
    nll_bits += 16
    print(f"model: {a.model}  device: {dev}  ctx: {ctx}")
    print(f"bytes: {len(raw)}  tokens: {n_tok}  ({len(raw) / n_tok:.2f} bytes/token)")
    print(f"bits per byte: {nll_bits / len(raw):.4f}   (bits per token: {nll_bits / n_tok:.3f})")
    print(f"scored {scored} tokens in {dt:.1f}s = {scored / dt:.0f} tok/s teacher-forced ({scored / dt * len(raw) / n_tok / 1000:.1f} KB/s)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
