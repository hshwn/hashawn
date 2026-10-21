### NCZ2 baseline (32df853)  (2026-10-21 12:23)

| file | sha | bytes | ncz | bpb | gzip -9 | xz -9 | enc MB/s | dec MB/s | ok |
|---|---|---:|---:|---:|---:|---:|---:|---:|:-:|
| binary-normal.webp | 720d88f2 | 301724 | 302096 | 8.010 | 301726 | 301796 | 0.47 | 1.15 | ✓ |
| binary-pfp.png | 33f12f13 | 52947 | 52854 | 7.986 | 52974 | 53008 | 1.31 | 1.01 | ✓ |
| code-js-py.txt | c563ff96 | 74227 | 22189 | 2.391 | 25600 | 23340 | 2.09 | 1.88 | ✓ |
| code-swift.txt | 4026dbe3 | 1150795 | 218335 | 1.518 | 308956 | 243248 | 3.00 | 2.63 | ✓ |
| text-nardo-docs.md | 052e85ec | 300964 | 80014 | 2.127 | 109543 | 93596 | 2.43 | 2.15 | ✓ |
| text-site-html.txt | c6e00939 | 15201 | 4536 | 2.387 | 5054 | 4916 | 1.14 | 1.10 | ✓ |
| **total** | | 1895858 | **680024** | **2.870** | 803853 | 719904 | | | |

ncz vs xz on the total: +5.5% smaller

### NCZ2 baseline + enwik8 (c7affa8)  (2026-10-21 12:26)

| file | sha | bytes | ncz | bpb | gzip -9 | xz -9 | enc MB/s | dec MB/s | ok |
|---|---|---:|---:|---:|---:|---:|---:|---:|:-:|
| binary-normal.webp | 720d88f2 | 301724 | 302096 | 8.010 | 301726 | 301796 | 0.58 | 1.35 | ✓ |
| binary-pfp.png | 33f12f13 | 52947 | 52854 | 7.986 | 52974 | 53008 | 1.45 | 1.17 | ✓ |
| code-js-py.txt | c563ff96 | 74227 | 22189 | 2.391 | 25600 | 23340 | 2.37 | 2.04 | ✓ |
| code-swift.txt | 4026dbe3 | 1150795 | 218335 | 1.518 | 308956 | 243248 | 3.50 | 3.14 | ✓ |
| text-enwik8-10mb.txt | 5985c81c | 10000000 | 2491447 | 1.993 | 3688458 | 2721280 | 2.95 | 2.45 | ✓ |
| text-nardo-docs.md | 052e85ec | 300964 | 80014 | 2.127 | 109543 | 93596 | 2.89 | 2.24 | ✓ |
| text-site-html.txt | c6e00939 | 15201 | 4536 | 2.387 | 5054 | 4916 | 0.98 | 0.95 | ✓ |
| **total** | | 11895858 | **3171471** | **2.133** | 4492311 | 3441184 | | | |

ncz vs xz on the total: +7.8% smaller

### v3 bit-history states + nibble buckets  (2026-10-21 12:31)

| file | sha | bytes | ncz | bpb | gzip -9 | xz -9 | enc MB/s | dec MB/s | ok |
|---|---|---:|---:|---:|---:|---:|---:|---:|:-:|
| binary-normal.webp | 720d88f2 | 301724 | 302413 | 8.018 | 301726 | 301796 | 0.54 | 0.98 | ✓ |
| binary-pfp.png | 33f12f13 | 52947 | 53010 | 8.010 | 52974 | 53008 | 0.98 | 0.83 | ✓ |
| code-js-py.txt | c563ff96 | 74227 | 20822 | 2.244 | 25600 | 23340 | 1.77 | 1.65 | ✓ |
| code-swift.txt | 4026dbe3 | 1150795 | 203800 | 1.417 | 308956 | 243248 | 2.53 | 2.27 | ✓ |
| text-enwik8-10mb.txt | 5985c81c | 10000000 | 2331844 | 1.865 | 3688458 | 2721280 | 2.23 | 2.02 | ✓ |
| text-nardo-docs.md | 052e85ec | 300964 | 77023 | 2.047 | 109543 | 93596 | 2.09 | 1.84 | ✓ |
| text-site-html.txt | c6e00939 | 15201 | 4447 | 2.340 | 5054 | 4916 | 0.69 | 0.78 | ✓ |
| **total** | | 11895858 | **2993359** | **2.013** | 4492311 | 3441184 | | | |

ncz vs xz on the total: +13.0% smaller

### v3 + word model  (2026-10-21 12:31)

| file | sha | bytes | ncz | bpb | gzip -9 | xz -9 | enc MB/s | dec MB/s | ok |
|---|---|---:|---:|---:|---:|---:|---:|---:|:-:|
| binary-normal.webp | 720d88f2 | 301724 | 302416 | 8.018 | 301726 | 301796 | 0.42 | 0.74 | ✓ |
| binary-pfp.png | 33f12f13 | 52947 | 53025 | 8.012 | 52974 | 53008 | 0.77 | 0.66 | ✓ |
| code-js-py.txt | c563ff96 | 74227 | 20778 | 2.239 | 25600 | 23340 | 1.52 | 1.07 | ✓ |
| code-swift.txt | 4026dbe3 | 1150795 | 202488 | 1.408 | 308956 | 243248 | 2.02 | 1.81 | ✓ |
| text-enwik8-10mb.txt | 5985c81c | 10000000 | 2299190 | 1.839 | 3688458 | 2721280 | 1.76 | 1.63 | ✓ |
| text-nardo-docs.md | 052e85ec | 300964 | 76798 | 2.041 | 109543 | 93596 | 1.69 | 1.58 | ✓ |
| text-site-html.txt | c6e00939 | 15201 | 4457 | 2.346 | 5054 | 4916 | 0.82 | 0.80 | ✓ |
| **total** | | 11895858 | **2959152** | **1.990** | 4492311 | 3441184 | | | |

ncz vs xz on the total: +14.0% smaller

### v3 + word model, recip table  (2026-10-21 12:32)

| file | sha | bytes | ncz | bpb | gzip -9 | xz -9 | enc MB/s | dec MB/s | ok |
|---|---|---:|---:|---:|---:|---:|---:|---:|:-:|
| binary-normal.webp | 720d88f2 | 301724 | 302416 | 8.018 | 301726 | 301796 | 0.51 | 0.92 | ✓ |
| binary-pfp.png | 33f12f13 | 52947 | 53025 | 8.012 | 52974 | 53008 | 0.94 | 0.75 | ✓ |
| code-js-py.txt | c563ff96 | 74227 | 20778 | 2.239 | 25600 | 23340 | 1.62 | 1.53 | ✓ |
| code-swift.txt | 4026dbe3 | 1150795 | 202488 | 1.408 | 308956 | 243248 | 2.18 | 2.01 | ✓ |
| text-enwik8-10mb.txt | 5985c81c | 10000000 | 2299190 | 1.839 | 3688458 | 2721280 | 1.99 | 1.47 | ✓ |
| text-nardo-docs.md | 052e85ec | 300964 | 76798 | 2.041 | 109543 | 93596 | 1.89 | 1.75 | ✓ |
| text-site-html.txt | c6e00939 | 15201 | 4457 | 2.346 | 5054 | 4916 | 0.88 | 0.87 | ✓ |
| **total** | | 11895858 | **2959152** | **1.990** | 4492311 | 3441184 | | | |

ncz vs xz on the total: +14.0% smaller

### v3 final: hash fix, 6 orders, mix16, sm255, no word  (2026-10-21 12:56)

| file | sha | bytes | ncz | bpb | gzip -9 | xz -9 | enc MB/s | dec MB/s | ok |
|---|---|---:|---:|---:|---:|---:|---:|---:|:-:|
| binary-normal.webp | 720d88f2 | 301724 | 306405 | 8.124 | 301726 | 301796 | 0.70 | 1.39 | ✓ |
| binary-pfp.png | 33f12f13 | 52947 | 53075 | 8.019 | 52974 | 53008 | 1.52 | 1.14 | ✓ |
| code-js-py.txt | c563ff96 | 74227 | 20868 | 2.249 | 25600 | 23340 | 2.30 | 1.82 | ✓ |
| code-swift.txt | 4026dbe3 | 1150795 | 200790 | 1.396 | 308956 | 243248 | 3.34 | 2.54 | ✓ |
| text-enwik8-10mb.txt | 5985c81c | 10000000 | 2241529 | 1.793 | 3688458 | 2721280 | 2.90 | 2.23 | ✓ |
| text-nardo-docs.md | 052e85ec | 300964 | 77022 | 2.047 | 109543 | 93596 | 2.68 | 2.13 | ✓ |
| text-site-html.txt | c6e00939 | 15201 | 4439 | 2.336 | 5054 | 4916 | 1.13 | 1.04 | ✓ |
| **total** | | 11895858 | **2904128** | **1.953** | 4492311 | 3441184 | | | |

ncz vs xz on the total: +15.6% smaller

### v3 + second mixer by previous byte (averaged)  (2026-10-21 13:04)

| file | sha | bytes | ncz | bpb | gzip -9 | xz -9 | enc MB/s | dec MB/s | ok |
|---|---|---:|---:|---:|---:|---:|---:|---:|:-:|
| binary-normal.webp | 720d88f2 | 301724 | 302568 | 8.022 | 301726 | 301796 | 0.54 | 1.21 | ✓ |
| binary-pfp.png | 33f12f13 | 52947 | 52963 | 8.002 | 52974 | 53008 | 1.37 | 1.06 | ✓ |
| code-js-py.txt | c563ff96 | 74227 | 20683 | 2.229 | 25600 | 23340 | 2.07 | 1.73 | ✓ |
| code-swift.txt | 4026dbe3 | 1150795 | 198201 | 1.378 | 308956 | 243248 | 3.09 | 2.26 | ✓ |
| text-enwik8-10mb.txt | 5985c81c | 10000000 | 2222674 | 1.778 | 3688458 | 2721280 | 2.58 | 2.10 | ✓ |
| text-nardo-docs.md | 052e85ec | 300964 | 76225 | 2.026 | 109543 | 93596 | 2.40 | 1.95 | ✓ |
| text-site-html.txt | c6e00939 | 15201 | 4431 | 2.332 | 5054 | 4916 | 1.02 | 0.93 | ✓ |
| **total** | | 11895858 | **2877745** | **1.935** | 4492311 | 3441184 | | | |

ncz vs xz on the total: +16.4% smaller


## slm: GPT-S3-3M forward pass in Rust (2026-10-21)

Own streaming implementation of the 3M-parameter GDN-2/attention hybrid (`slm/`), built as the neural input
for ncz. Numbers are the model's bits per byte (ideal arithmetic coder), tokenizer included. Hardware: M4 Pro.

| build | docs100k bpb | tok/s (1 thread) | notes |
|---|---:|---:|---|
| f32 scalar, matches PyTorch to 1e-4 | 1.8688 | 1,294 | |
| f32, 8-lane dot | 1.8688 | 1,574 | |
| int8 weights + int8 activations, i32 exact, NEON `sdot` | 1.8710 | 9,659 | scalar path 5,266, identical logits hash |
| + eight-lane SIMD type for all f32 (NEON / wasm / arrays) | 1.8710 | 14,311 | array backend hashes identical |
| + int8 keys, vector softplus/ln | 1.8711 | 15,681 | |
| + fork-join threads inside the token | 1.8711 | 22,900 (3 thr) | 1.47x; sync-bound beyond 3 |

Quantisation cost measured before committing to it: int8 weights 1.8701, +int16 acts 1.8701, +int8 acts 1.8708.
4-bit weights: 2.115 per row, 2.035 per group of 32 — dead.

Hot-cache int8 matvec: ~100 GMAC/s = one 16-byte load per `sdot`, ~110 GB/s from L2. Weights are ~3 MB per
token, so ~27 µs is the single-core floor for the projections; that is why threads, not more kernel work.

### Windows vs streaming (same model, 3 threads)

The model was trained on 1024-token windows. Carrying the recurrent state forever drifts; a sliding attention
window is fine. A constant extra decay of 0.999 per token on every GDN state fixes the drift without resets.

| mode | docs100k bpb | enwik8-1MB bpb (held out) | KB/s |
|---|---:|---:|---:|
| windows 1024, stride 512 (recompute half) | 1.8711 | 1.5335 | 25.5 |
| windows 1024, stride 1024 | 1.9305 | | 39 |
| streaming, state carried forever | 1.9749 | 1.6449 | 45 |
| streaming, GDN reset every 1024 | 1.8970 | | |
| streaming, attention reset every 1024 (GDN carried) | 2.0325 | | |
| streaming, extra GDN decay 0.9993 | 1.8703 | 1.5414 | 46 |
| **streaming, extra GDN decay 0.999 (default)** | **1.8705** | **1.5407** | **46** |

Decay knob is flat between 0.999 and 0.9993; 0.998 costs 0.3%, 0.995 costs 1%. Hard resets lose context,
soft shrink (×0.7 every 512) 1.8727, continuous decay best. Held out: +0.5% bpb for 1.8× throughput.

### Byte bridge and the fair hybrid (2026-10-21)

A codec decodes bytes; the model predicts tokens. Two bridges measured on Nardo docs 100 KB:

| bridge | model on its tokens | LM-alone byte bpb | ncz + side |
|---|---:|---:|---:|
| greedy longest match decided by the decoder | 2.2052 (+18%: not BPE's cuts) | 4.4074 (end-of-token bytes cost 7.8 bits) | 2.1383 |
| **BPE tokenisation transmitted as per-byte end flags** | 1.8705 | 1.8701 (= byte 1.6432 + flags 0.2269; telescopes) | **1.7827** |

The transmitted-flag bridge (`slm/src/bridge.rs`, `slmcli side`): before every byte inside a token, one binary
flag "token ends here" at the model's own P(token = tail)/P(token starts with tail); the byte is then
predicted given "continues" from the trie masses. Flags plus bytes cost exactly the token cost, so the flags
are free in total and ncz only has to beat the model's byte bits (it does: 1.6432 -> 1.5558).

| | Nardo docs 100 KB | enwik8 1 MB |
|---|---:|---:|
| xz -9 | 2.765 | 2.326 |
| ncz alone | 2.3218 | 1.9071 |
| GPT-S3-3M alone (streaming, decay 0.999) | 1.8701 | 1.5404 |
| hybrid bytes (ncz + side input) | 1.5558 | 1.2367 |
| + flags coded by the model | 0.2269 | 0.1792 |
| **hybrid total** | **1.7827** | **1.4159** |

Decodable end to end: every quantity depends on past bytes and past flags. Measured at ~45 KB/s (3 threads).
Flags are coded by the model alone; a mixed flag predictor (model + byte contexts) is the obvious next cut.
The earlier "1.558" number was this hybrid's byte bits without the flags.

### NCZL: the real codec (2026-10-21)

`slmcli compress|decompress|roundtrip` (slm crate, ncz as a dependency). Stream: "NCZL" | version | mode
(stored / ncz / ncz+model) | model id | length | payload. Text (UTF-8) takes the model path: token-end
flags straight through the range coder at the model's probability, bytes through ncz's mixer with the
bridge's predictions. The tokenizer is ours (Digits + GPT-2 regex + byte-level BPE), 100% identical to
Hugging Face's ids on Nardo docs and enwik8. Binary falls back to ncz alone or stored (18-byte header).

| file | NCZL bytes | bpb | ncz alone | enc KB/s (M4 Pro, 3 threads) |
|---|---:|---:|---:|---:|
| Nardo docs 100 KB | 22292 | 1.7833 | 2.3218 | 46 |
| code-js-py 74 KB | 17456 | 1.8814 | 2.229 | 32 |
| site html 15 KB | 3128 | 1.6462 | 2.336 | 35 |
| pfp.png 53 KB | 52965 (ncz mode) | 8.0027 | 8.019 | 1400 |

Browser (tools/compress, nczl.wasm 199 KB + model pack 3.3 MB fetched once): byte-identical output to
the native build (same SHA-256 on a 30 KB slice: NEON/3 threads vs simd128/1 thread), 12.6 KB/s each way
with the simd128 int8 kernel (5.6 with the scalar one). Native single thread ~32 KB/s.

### Mixed flag predictor (2026-10-21)

The token-end flag was coded at the model's P(end) alone. `slm/src/flags.rs`: a six-input logistic mixer
(model stretch, adaptive counters keyed by tail bytes / tail + byte before the token / tail length + last
two bytes, an SSE cell by model bucket × length, bias), 32 weight sets by tail length × model bucket,
weights start at "model only". Learning rate flat between 0.005 and 0.015; 0.06 undoes the gain.

| file | model-only flags | mixed flags | change |
|---|---:|---:|---:|
| Nardo docs 100 KB | 22292 B, 1.7834 bpb | 22132 B, 1.7706 | -0.7% |
| enwik8 1 MB (held out) | 177013 B, 1.4161 bpb | 176199 B, 1.4096 | -0.5% |

About 5% of the flag cost. The rest is real uncertainty about the next character, which only the model
sees. Also in this change: the bridge's stretch now uses our own `fln` instead of the platform `ln`, so
native and wasm run identical code there too (browser output re-verified byte-identical).

### Byte-first coding with speculative steps (2026-10-21, NCZL v2)

Byte coded under (1-q)·P(b|continue) + q·P(b|new token), q = the flag model's P(end); the new-token
distribution comes from a speculative model step on the tail's token (snapshot/restore, kept if the token
ends). The flag is then coded given the byte, free when only one reading is possible. Below a threshold
on q the step is skipped; the fallback is an adaptive table of "first byte after a token ending in x"
(the current distribution, used before, is the wrong one: 2.33 bpb when never speculating).

Nardo docs 100 KB, M4 Pro 3 threads (v1 = flag-first with the mixed flag model: 22132 B, 1.7706, 44 KB/s):

| threshold | no fallback table | with table | enc KB/s |
|---|---:|---:|---:|
| 0 (always) | 21958 B, 1.7566 | | 19 |
| 0.005 | 21974, 1.7579 | 21964, 1.7571 | 23 |
| **0.02 (default)** | 22023, 1.7618 | **21984, 1.7587** | 27 |
| 0.05 | 22182, 1.7746 | 22040, 1.7632 | 29 |
| 0.1 | 22475, 1.7980 | 22145, 1.7716 | 32 |
| never | 29181, 2.3345 | 26012, 2.0810 | 44 |

enwik8 1 MB held out: v1 176199 B (1.4096) -> v2 at 0.02 with table 174984 B (1.3999), -0.7%.
Gains flatten below 0.02 while speed keeps falling, so 0.02 is the default. Cost vs v1: ~40% slower
(27 vs 44 KB/s native, 8 vs 12 KB/s in the browser). Browser output byte-identical to native (SHA-256).

### Speed pass on the codec (2026-10-21, NCZL v5)

| change | Nardo docs bytes | native 3 thr KB/s | browser KB/s |
|---|---:|---:|---:|
| v2 byte-first, threshold 0.02 | 21984 | 27 | 8 |
| + trie-ordered vocab (SIMD softmax), double-buffered state | 21984 | 31.5 | |
| + flag first when P(end) < 0.2 (1.53 -> 1.16 model steps per token) | 22012 | 40 | |
| + 7-bit activations (+0.1% bytes), relaxed-SIMD wasm (V8 lowers it to `sdot`) | 22033 | 40 | 22.8 |
| + 144x144 output projection on one thread (sync cost more than the work) | 22033 | 42 | 22.8 |

Plain-SIMD fallback (`nczl-base.wasm`, browsers without relaxed SIMD): 11.6 -> 14.8 KB/s by summing four
exact int16 products before widening. All builds produce identical bytes (SHA-256 on a 30 KB slice).
enwik8 1 MB held out: 1.4032 bpb. Native single thread 28 KB/s, wasm single thread 22.8: wasm is now within
20% of native. Tried and dropped: fusing the GDN state passes with a pending rank-one update (no gain,
the state lives in L1); interleaving rows in the old wasm kernel (no gain, it was instruction-bound).

Where a single thread stands: the int8 matvecs read ~3 MB of weights per model step, ~30 µs at L2
bandwidth, which is about what they take. More single-thread speed needs fewer weights read per byte.

### Segments in parallel (2026-10-21, NCZL v5 + MODE_SEG)

Big inputs are split into independent NCZL streams inside one container ("NCZL" | version | 3 | 0 | total |
n | lengths | streams), compressed and decompressed on separate cores/workers. The split depends only on the
input: at least 1 MiB per segment, at most 8, cut after a newline near each boundary. Same bytes on every
machine and any worker count (native threads, Node worker_threads and browser workers all hash identical).

Cost of starting cold, enwik8 1 MB / Swift 1.15 MB, 8 workers (one stream: 1.4032 / 1.2442 bpb):

| segment | enwik8 | Swift | enwik8 KB/s |
|---|---:|---:|---:|
| 1 MiB | | +1.8% | |
| 512 KiB | +1.4% | +5.2% | 48 |
| 256 KiB | +3.0% | +8.7% | 95 |
| 128 KiB | +5.0% | +12.5% | 157 |
| 64 KiB | +7.6% | +17.6% | 150 |

enwik8 10 MB under the policy (8 x 1.25 MB): one stream 1.4347 bpb at 31.7 KB/s; segmented 1.4852 (+3.5%)
at 156 KB/s compress / 146 decompress (4.9x). Workers: 4 -> 92 KB/s, 8 -> 150, 12 -> 147 (8 segments).
The cost grows with file size: one stream keeps learning across the whole file, segments restart.
wasm (Node, 2 workers, 3 MB = 2 segments): 43.8 KB/s. Also: rope tables gained a third level (positions
up to 2^30; the 10 MB one-stream run crashed at 2^20), pack v2, model id 1d4f6388.

## Pixel mode (2026-10-21, NCZI v1)

`ncz/src/img.rs`: 8-bit RGB, lossless or near-lossless (every sample within +-k). Green, then red, then blue.
Each sample: a blend of 12 predictors (neighbours, gradients, MED, and for red/blue "colour from
brightness": this pixel's green change carried over) weighted by each predictor's recent error around the
pixel, plus a running bias per context. The residual is binarised (zero? / size / mantissa / sign) and each
decision is coded by context mixing: 13 context sets in one hashed table (activity, expected error,
neighbour residuals, the other channels' residual here, and where each predictor points relative to the
guess), two mixers (by activity and by expected error) averaged, two APMs, range coder. Integer only.

Set: the 24 Kodak photos (`bench/img/fetch-kodak.sh`) + the site's 3 posters (decoded from their web files).
Baselines from `bench/img/baselines.py` (imagecodecs: libpng 9, libwebp lossless method 6, libjxl lossless).
Bits per pixel over RGB (raw = 24). Tuned on kodim01-04 only; the other 20 are held out.

| set | ncz pixel | JPEG XL e9 | JPEG XL e7 | WebP lossless | PNG |
|---|---:|---:|---:|---:|---:|
| 4 photos tuned on | 8.097 | 8.148 | 8.234 | 9.096 | 12.916 |
| 20 photos held out | 8.560 | 8.551 | 8.660 | 9.578 | 13.274 |
| all 24 photos | 8.483 | 8.484 | 8.589 | 9.498 | 13.214 |
| 3 posters | 3.823 | 4.116 | 4.193 | 4.812 | 6.807 |

Near-lossless, every sample within +-k (verified): photos 5.523 bpp at +-1, 4.277 at +-2; posters 2.815 / 2.249.
JPEG-LS (CharLS via imagecodecs) gave 8.684 / 6.903 on photos, but its lossless mode came out worse than PNG
(13.07 bpp), so that encoder setup is probably wrong; treat those JPEG-LS numbers as unreliable.
Speed ~2.6 MB/s of raw RGB, encode and decode alike (JPEG XL decodes far faster; its e9 encode took 5-12 s per
photo here).

Path, measured on kodim01-04 (bpp): v1 single blend + 7 small tables 8.929 -> hashed table with "where each
predictor points" contexts and decoded-prefix keys 8.486 -> remaining predictor offsets 8.227 -> second mixer by
expected error 8.145 -> mixer step /2 8.101 -> table limit 255 8.097. No gain: predictor-disagreement and
blend-position contexts, far neighbours (NN, WW, NNE), scale-aware offset pairs. Bits split (kodim01): green
5.1 bpp, red 2.5, blue 2.4.
