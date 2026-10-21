# slm — GPT-S3-3M, streaming, integer, deterministic

A from-scratch Rust forward pass for [AxiomicLabs/GPT-S3-3M](https://huggingface.co/AxiomicLabs/GPT-S3-3M)
(2.97M parameters: 6 Gated-DeltaNet-2 layers, 2 gated-GQA attention layers, Full Attention Residuals,
4096-token vocab). It exists to be the neural input of ncz, so it is built for two things:

- **the same bits on every machine.** Weights are int8 per row, activations int8 per vector, products
  summed in i32 (exact). Every f32 operation goes through `simd::F8`: eight lanes, lane-wise IEEE ops,
  no fused multiply-add, fixed reduction order. exp and ln are our own polynomials; rope tables ship
  with the weights. `cargo build --features scalar` builds the plain-array backend; it hashes identical
  logits to the NEON build.
- **speed.** 15.6k tok/s on one M4 Pro core, 20k with three threads, in streaming mode ~50 KB/s of text.

## Build and run

```bash
uv run --with torch --with transformers --with safetensors --with numpy python export.py <hf model dir> <text file>
cargo build --release
./target/release/slmcli check                      # vs PyTorch logits on the first 1024 tokens
./target/release/slmcli bpb <ids.bin> <nbytes>     # bits per byte, streaming pass (SLM_WINDOWED=1 for 1024/512 windows)
./target/release/slmcli micro                      # hot-cache int8 matvec throughput
cargo build --release --features prof              # section timers, printed by bpb
```

`export.py` writes `model/weights.bin` (f32, quantised at load), `manifest.txt`, the rope tables, PyTorch
reference logits and the tokenizer's byte table. `model/weights.bin` is not committed (11.9 MB).

Knobs (environment): `SLM_THREADS` (default 3, one per head), `SLM_GDN_DECAY` (default 0.999, see below),
`SLM_ATT_WIN` (attention window, default 1024), `SLM_STRIDE` (windowed mode).

## Streaming and the decay

The model was trained on 1024-token windows. Run as a stream (no resets), its recurrent state drifts:
enwik8 1 MB goes from 1.534 bpb windowed to 1.645. Resetting attention is not the problem; a sliding window
of the last 1024 keys is fine. Resetting the GDN state every 1024 tokens loses context at each boundary
(1.897 on Nardo docs vs 1.871 windowed). A constant extra decay of 0.999 per token on every GDN state
keeps nearly everything: 1.8705 on Nardo docs (windowed 1.8711) and 1.5407 held out on enwik8 (windowed
1.5335), with every token computed once instead of 1.5 times. Full numbers in `../ncz/bench/results.md`.

## Byte bridge (`src/bridge.rs`, `slmcli side`)

A codec decodes bytes, the model predicts tokens. Letting the decoder guess token ends (greedy longest
match) costs 18% on the model's own tokens and ruins the end-of-token bytes (measured, see results.md).
Instead the encoder transmits the real BPE tokenisation: before every byte inside a token, one binary
flag "the token ends here" at the model's own probability; the byte is then predicted given "continues"
from subtree masses over a vocab trie. Byte bits plus flag bits equal the token cost exactly, so the
flags are free in total. `slmcli side <file> <nbytes> <ids.bin> <out.side>` writes ncz's side input
(stretch per bit) and reports the flag bits; `nczcli compress-side` then gives the hybrid size.

Fair hybrid: Nardo docs 100 KB 1.7827 bpb (ncz 2.3218, model 1.8701, xz 2.765); enwik8 1 MB 1.4159
(ncz 1.9071, model 1.5404, xz 2.326).

## The codec (`src/codec.rs`, `slmcli compress | decompress | roundtrip`)

NCZL = ncz with this model as a side input, one stream: `"NCZL" | version | mode | model id | length |
payload`. UTF-8 text takes the model path (token-end flags at the model's probability straight through
the coder, bytes through ncz's mixer with the bridge's predictions); anything else is ncz alone or
stored. `src/bpe.rs` is the model's tokenizer (identical to Hugging Face on 1.1 MB of text).
`slmcli pack model/model.slmq` writes the quantised weights + tokenizer for the browser; the model id in
every header is the hash of that pack, so native and wasm streams interoperate (verified by SHA-256).
`src/wasm.rs` is the C ABI the site's `tools/compress/worker.js` drives: `nczl_load`, streaming
`nczl_enc_*` / `nczl_dec_*`. Build: `RUSTFLAGS="-C target-feature=+simd128 -C strip=debuginfo" cargo build
--release --target wasm32-unknown-unknown`, copy `slm.wasm` to `tools/compress/nczl.wasm`.

Nardo docs 100 KB: 1.7706 bpb (ncz 2.3218, xz 2.765); enwik8 1 MB 1.4096 (ncz 1.9071). 44 KB/s native, 12 KB/s in the browser.
The token-end flag is predicted by `src/flags.rs`, a small online mixer over the model's estimate and byte-context counters.

## Layout

- `src/simd.rs` — `F8`, exp8/ln8/sigmoid8, dot/dots/axpy, quantize, int8 key scores (`sdot` via inline asm).
- `src/pool.rs` — fork-join pool for work inside one token; `split` for 8-aligned row ranges.
- `src/lib.rs` — weights (int8 `QMat`, head-major projection rows), `Model::step`, the per-head GDN and
  attention kernels, `log_prob`.
- `src/bridge.rs` — vocab trie in DFS order, prefix-sum masses, the transmitted-flag byte bridge.
- `src/bpe.rs` — the tokenizer. `src/codec.rs` — NCZL streams. `src/pack.rs` — model pack + id. `src/wasm.rs` — browser ABI.
- `src/main.rs` — `slmcli`.
