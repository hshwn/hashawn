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

### Pixel mode v2: colour from brightness, hand-made and learned (2026-10-21, NCZI v2)

Header gains a flags byte: 1 = green plane first (red/blue then see all of brightness), 2 = learned colour
(needs 1), 4 = learned brightness. `IMG_FLAGS` selects in `nczcli img-bench`.

- Hand-made, always on: two more colour predictors per pixel. Copy the colour offset of the neighbour whose
  brightness is closest to this pixel's, and fit colour = a*brightness + b over six neighbours (a local version of
  AV1's chroma-from-luma). kodim01-04: 8.097 -> 7.904.
- Green plane first plus an "edge ahead in brightness" context: no gain on its own (7.907).
- Learned (`bench/img/colorfit.py`, trained on the rig's 3090 on 1,083 COCO photos downsampled 2x, ~10M pixels
  each; Kodak never seen): tiny MLPs, 2 hidden layers, predicting the value and its own uncertainty (Laplace NLL).
  Colour: 5x5 brightness window + nearby colour offsets, mean error on Kodak 1.21 vs 1.56 for the a*brightness+b
  fit. Brightness: 12 coded neighbours, mean error 5.05 vs 5.85 for the median edge predictor. In the coder each is
  one more predictor plus an uncertainty context. Inference is f32 in a fixed order (no fused multiply-add, ReLU
  only), so encoder and decoder agree everywhere. Bigger nets (64 hidden) no better for colour.
- Global hint (a 16x16-block colour thumbnail the encoder would send): estimated -0.4% for red, worse for blue,
  while sending it would cost ~0.03 bits per sample. Not worth it in this form; not built into the coder.

| set (bpp) | v1 | + hand-made colour | + learned (flags 7) | JPEG XL e9 |
|---|---:|---:|---:|---:|
| 4 photos tuned on | 8.097 | 7.906 | 7.842 | 8.148 |
| 20 photos held out | 8.560 | 8.402 | 8.332 | 8.551 |
| 3 posters | 3.823 | 3.768 | 3.730 | 4.116 |

Held out: hand-made -1.8%, learned another -0.8%; overall 2.6% under JPEG XL e9 on photos, 9.4% on posters.
Speed: ~2.7-3.2 MB/s without the nets, ~1 MB/s with them (each net runs for every sample).

### Does a pretrained vision encoder help? (2026-10-21, probe only)

`bench/img/kanana_probe.py`, rig 3090. Setup: coarse-first coding, the quarter image (every second pixel) known, the
other 3/4 of the samples predicted from it. A small head (64x64 MLP, Laplace mean + scale) gets the 4x4 quarter-image
neighbourhood; four arms differ only in one extra input, same seed/steps/size. Train COCO (downsampled 2x), test Kodak.

| extra input | Kodak bits per fill-in sample | vs local only |
|---|---:|---:|
| A: none (local neighbourhood) | 4.3795 | |
| B: Kanana-2 vision encoder tiny (98.7M, released 2026-10-21), features of the quarter image | 4.4133 | +0.77% |
| C: same architecture, random weights | 4.4115 | +0.73% |
| D: raw 4x4x3 thumbnail of the same block | 4.3781 | -0.03% |

Pretrained features are no better than random ones, and even the raw coarse block adds nothing: the immediate
neighbours already carry what a 64x64-pixel summary could. Same law as EG2's vision test: these encoders know
what is in the picture, not how the next pixel value behaves. Not pursued further. (LILIE, a 1.5M-32M invertible
language-informed encoder, was looked at but not run after this result.)

## PNG in the compress tool (2026-10-21, NCZP v1)

`ncz/src/pngc.rs`. A PNG's pixels go through the pixel model (fast mode, flags 0); every other chunk (IHDR, PLTE,
tRNS, iCCP, gAMA, text, EXIF, ...) is kept byte for byte in its original order as a skeleton. Unpacking re-encodes
only IDAT and splices it back, so pixels and metadata are exact; the PNG's bytes differ only inside IDAT (PNG's own
zlib stream can't be replayed byte for byte). Palette and 1-4 bit grey images are coded as their own sample bytes
through ncz's byte model (the pixel model expanded them to RGB and lost badly: up to 5.8x the PNG). 16-bit and
animated PNGs, and anything that would not get smaller, take the byte-exact generic path.

Pixel codec changes on the way: alpha as a fourth channel coded first, per-predictor error buffers cut from the whole
image (~1 GB at 12 MP) to a two-row ring, row-by-row encoder/decoder for progress. Existing RGB streams bit-identical.

| test PNG | PNG bytes | .ncz | | rebuilt PNG |
|---|---:|---:|---:|---:|
| Kodak photo (RGB) | 502,888 | 305,827 | 60.8% | 552,652 |
| poster (RGB, iCCP + zTXt) | 3,765,957 | 1,935,432 | 51.4% | 3,671,545 |
| RGBA | 607,749 | 306,088 | 50.4% | 612,093 |
| interlaced RGB | 604,696 | 305,916 | 50.6% | 552,835 |
| grey 8-bit | 196,002 | 155,790 | 79.5% | 207,620 |
| grey + alpha | 248,790 | 155,905 | 62.7% | 256,254 |
| palette 8-bit, 64 colours | 100,349 | 67,458 | 67.2% | 126,296 |
| palette 4-bit | 5,333 | 4,059 | 76.1% | 2,993 |
| grey 1-bit | 12,057 | 10,229 | 84.8% | 13,684 |
| palette 2-bit + tRNS (3 KB) | 3,235 | generic path | | |
| 16-bit RGB | 254,970 | generic path | | |

All pixel-exact (decoded pixels compared). Browser: compressed and rebuilt files hash identical to native for RGB,
RGBA, palette and 1-bit; kodim03 0.6 s each way. nczl.wasm grew 199 KB -> 539 KB (png crate + zlib).

### Pixel mode v4: exact-colour contexts, a third mixer, a mixer prior (2026-10-21, NCZI v4)

Path, fast mode (flags 0), dev = kodim01-04 mean / poster_ram (bpp), encode speed:

| step | photos 4 | ram poster | MB/s |
|---|---:|---:|---:|
| start (NCZI v2, committed) | 7.906 | 5.473 | 2.91 |
| + exact neighbour colours with the guess as contexts (W,N,guess and NW,NE,guess) | 7.877 | 5.294 | |
| + third mixer keyed by where W and N sit vs the guess (17x17) + learned final layer over the three | 7.847 | 5.254 | 1.94 |
| same without the third mixer (final layer over two) | 7.878 | 5.296 | |
| ablation: six context sets individually worthless, removed (15 + 2 new = 17 -> 11) | 7.839 | 5.259 | 2.59 |
| + mixer prior (below) | 7.833 | 5.258 | |

Mixer prior: start mix, mix2 and the final layer from the median of their trained weights over 100 COCO photos
(downsampled 2x); mix3 starts cold. Stored as i16 at half scale, 170 KB (`ncz/assets/img_mixer_prior.bin`), loaded
by encoder and decoder alike. The full prior including mix3 was 4.4 MB for almost the same gain:

| prior | photos 4 | ram | 8 thumbnails 96 px | 8 thumbnails 192 px |
|---|---:|---:|---:|---:|
| none | 7.839 | 5.259 | 11.448 | 10.257 |
| full (mix, mix2, mix3, fin; i32, 4.4 MB) | 7.831 | 5.258 | 11.276 | 10.182 |
| compact (mix, mix2, fin; i32) | 7.833 | 5.258 | 11.283 | 10.188 |
| compact, i16 (shipped, 170 KB) | 7.833 | 5.258 | 11.282 | 10.189 |

(Thumbnails: kodim05-12 downsampled.) Learned mode (flags 7) with the compact prior: photos 4 7.779 -> 7.773, 96 px
11.417 -> 11.274, 192 px 10.198 -> 10.139.

Held out (all OK, round trip verified):

| set (bpp) | fast v2 | fast v4 | learned v2 | learned v4 | JPEG XL e9 |
|---|---:|---:|---:|---:|---:|
| 20 photos held out | 8.402 | 8.343 | 8.332 | 8.280 | 8.551 |
| all 24 photos | | 8.258 | | 8.195 | 8.484 |
| 3 posters | 3.768 | 3.647 | 3.730 | 3.611 | 4.116 |
| 2 posters not tuned on (poupees, harvest) | | 2.503 | | 2.462 | 2.908 |

Held out vs v2: fast -0.7% photos, -3.2% posters; learned -0.6% / -3.2%. Vs JPEG XL e9: fast -2.4% photos, -11.4%
posters; learned -3.2% / -12.3%. Speed: under the same (loaded) conditions the v4 fast coder ran 2.18 MB/s vs 1.96-2.07
for the committed v2; on a quiet machine the step table above applies (~2.6 MB/s). nczl.wasm 539 KB -> 725 KB (the prior).
Browser build: compressed and rebuilt files hash identical to native (kodim03, RGBA, palette).

## Grain lab: measure film grain, regenerate it better than AV1 (2026-10-21, research prototype)

`ncz/bench/grain/` (Python; `separate.py`, `grain_lab.py`). Goal: a "looks the same" mode that strips grain, codes
the clean picture, sends a grain fingerprint, and regenerates grain with the same character. Not lossless.

Separation: BM3D driven by the grain's own spectrum (estimated from the flattest 6% of patches, median), in an
opponent colour space. Non-local means leaves grain at edges; bilateral eats edges; a flattest-quarter spectrum
over-smoothed busy photos (kodim13's water texture counted as grain: 6.5 -> 3.6 levels after the fix).

Generators, scored against the real grain of kodim03/05/13/23 (averages):

| | AV1-style (AR lag 3, 64x64 template, 32x32 tiles) | ours (whole-image, 2D spectrum, colour coupling, 1025-quantile shape, strength per channel by brightness) | physical discs |
|---|---:|---:|---:|
| spectrum error (dB) | 0.82 | 0.29 | 2.7 |
| strength error | 1.2% | 1.0% | 1.2% |
| colour coupling error | 0.18 | 0.006 | 0.07 |
| shape error (W1) | 0.093 | 0.008 | 0.008 |
| clumping (real 0.72-1.27) | 0.34-0.47 | 0.48-0.76 | 0.49-0.94 |
| repetition (real 0.008-0.023) | 0.04-0.07 | 0.008-0.014 | 0.007-0.022 |

The physical disc model fits poorly here: Kodak is film scanned then downsized to 768x512, grain is sub-pixel
(the fit hit the smallest grain size tried). Clumping is the one gap left; needs higher-resolution grainy stills.

Size (bpp, pixel codec v4 fast mode; the fingerprint, ~0.02-0.05 bpp, not counted):
exact 8.55 -> +-1 5.65 (-34%) -> +-2 4.33 (-49%); grain removed + exact 7.26 (-15%), + +-1 4.60 (-46%), + +-2 3.62 (-58%).
Kodak's grain is weak (0.7-1.0 levels rms on three of four); stronger grain should save more.

### Grain lab on real 35mm grain scans (2026-10-22)

`ncz/bench/grain/plate_lab.py`. Three Wikimedia Commons CC0 grain scans by Wiley.andrew.r (Nikon Coolscan V ED, ~4000 dpi,
16-bit; film exposed to a white card, so pure grain at one density): Kodak Tri-X 400, T-Max 100, UltraMax 400 (colour).
Scans stay local (gitignored). 2048x2048 centre crops; grain = scan - heavy blur; dust masked (0.2-0.3%).
Clump size = connected blobs above +1 sigma.

| film | measure | real | AV1-style | ours | ours + envelope |
|---|---|---:|---:|---:|---:|
| Tri-X 400 | spectrum error (dB) | | 1.41 | 0.38 | 0.37 |
| | clumping | 0.555 | 0.410 | 0.461 | 0.535 |
| | mean / p90 clump (px) | 4.2 / 10 | 5.5 / 12 | 4.1 / 10 | 4.2 / 10 |
| | repetition | 0.004 | 0.092 | 0.004 | 0.005 |
| T-Max 100 | spectrum error (dB) | | 0.99 | 0.08 | 0.11 |
| | clumping | 0.410 | 0.345 | 0.366 | 0.412 |
| | mean / p90 clump (px) | 3.1 / 7 | 3.5 / 9 | 3.1 / 7 | 3.1 / 7 |
| UltraMax 400 | spectrum error (dB) | | 0.73 | 0.15 | 0.16 |
| | colour coupling error | | 0.468 | 0.003 | 0.003 |
| | clumping | 0.377 | 0.404 | 0.358 | 0.369 |
| | mean / p90 clump (px) | 3.8 / 9 | 3.9 / 8 | 3.7 / 8 | 3.7 / 9 |

"ours + envelope": the spectrum-matched grain modulated by a slow random envelope (grain locally denser in places),
its strength (0.05-0.15) fitted to the real, dust-masked clumping. AV1-style tiles show as visible seams and ~20x the
real repetition. A physical disc model (lognormal discs, 50% cover) overshoots clumping (0.61-0.73) and is dropped.

### Grain mode in the coder: NCZG, a LOOKS-THE-SAME mode (2026-10-22)

`ncz/src/grain.rs`, `nczcli grain <k> <ppm...>`. **Not exact.** The output is a different picture with the same look:
the encoder splits the photo into a clean picture and its grain, codes the clean picture with the pixel model (every
sample within +-2 of the clean picture), and stores the grain's fingerprint; the decoder grows new grain with that
fingerprint on top. Its own magic (`NCZG`) so it can never be mistaken for the exact (`NCZI`/`NCZP`) streams.

- Split (encoder only): white-noise pilot -> grain spectrum from (photo - pilot) in the flattest 6% of 32x32 patches
  (median) -> two-stage 16x16 block-DCT filter driven by that spectrum (hard threshold, then empirical Wiener), strength
  adapted per brightness band. BM3D's transform-domain core without block matching: more conservative in texture.
- Fingerprint (~1.3-1.5 KB, 0.016 bpp over the set), measured on FLAT pixels only: strength per channel in 16
  brightness bands, half of the 32x32 spectrum, channel coupling, 65 quantiles, envelope strength (clumping fit).
- Decoder (deterministic: integer RNG, own exp/cos/ln/inverse-normal, f64 in fixed order, no libm, no FFT): 17x17
  spectrum kernel over noise -> slow envelope -> coupling -> rank-map to the quantiles -> strength by the clean
  picture's brightness. Decoded twice per image: identical every time. Cross-platform (wasm) check not done yet; the
  browser tool does not have grain mode.
- No measurable grain (mean brightness-grain < 0.5 levels): nothing is split, the picture itself goes at +-2 and the
  decoder adds nothing (kodim20, kodim24, the two digital posters).

Size (bpp, fingerprint included; pixel codec v4 fast mode):

| | exact | +-2 | grain mode | vs exact | vs +-2 |
|---|---:|---:|---:|---:|---:|
| Kodak 24 (mean) | 8.258 | 4.145 | 3.696 | -55.2% | -10.8% |
| 3 posters (mean) | 3.686 | 1.878 | 1.734 | -53.0% | -7.7% |
| all 27 (pooled) | 6.325 | 3.214 | 2.878 | -54.5% | -10.4% |

Best: kodim09 2.30 vs 3.52 at +-2 (-35%), kodim10 -25%, kodim21 -21%, kodim23 -21%; scanned ram poster -14%.
Encode ~1.4 s, decode ~0.5 s per 768x512 photo (single thread).

Matched check on kodim03/05/13/23 vs the BM3D split from the lab: grain mode 3.94 bpp vs BM3D-clean +-2 3.62 (+ its
fingerprint). BM3D removes more, but on kodim13 part of what it removes is the water's sparkle (picture, not grain).

Grain fidelity (`bench/grain/mode_eval.py`; real = photo - clean, regenerated = output - decoded clean; flat pixels;
23 grainy pictures):

| | median | worst |
|---|---:|---:|
| strength error per brightness band | 4.2% | 14.7% (kodim06) |
| spectrum error (dB, radial) | 0.61 | 2.04 (kodim11, scanner line noise) |
| value shape (W1) | 0.023 | 0.155 (kodim06) |
| coupling error | 0.006 | 0.164 (ram poster: red pinned at 255) |
| clumping, regenerated / real | 0.89 | 0.75 |
| blob size, regenerated / real | 0.96 | 0.77-1.36 |

Known gaps: clumping a bit low on busy pictures; regenerated grain is laid evenly while the split leaves some real grain
in textured areas, so whole-picture grain is ~1.27x the removed grain there (texture hides it; flat areas match).

### Layers from a vision model: albedo x shading + residual (2026-10-22) — does not help exact coding

`ncz/bench/iid/`. Marigold-IID Lighting v1.1 (prs-eth, 2025; fp16 on the 3090, ~3 s per 768x512 photo, ensemble 5)
splits each photo into albedo (flat surface colour), diffuse shading and a residual. The model would run in the encoder
only. Visually the split is good (shadow-free caps, cast shadows in the shading layer). Kodak 03/05/13/23 + ram poster.

(c) Shading as side information: the coder (patch `shade_predictor.patch`, two extra predictors that take neighbours'
surface colour, sample / shading, and relight it with this pixel's shading) vs the same coder without it.
Same pictures, exact mode, total bpp incl. the shading's own cost:

| shading given to the coder | free (ceiling) | 1/8 res | 1/4 res | 1/2 res | full res |
|---|---:|---:|---:|---:|---:|
| Marigold | +0.09% | +1.53% | +5.30% | +22.1% | +113% |
| Retinex (blurred photo, sigma 8) | +0.09% | +1.40% | | | |

Even free, full-resolution model shading gains nothing. Sanity check, synthetic flat colours x shading with sharp
shadows: -3.45% without noise, -0.80% with 0.8-level noise, so the predictor works; the coder's gradient predictors
already follow smooth light, and shadow edges are too few pixels to matter.

(b) Layers coded separately (albedo exact + shading at 1/8 + residual): +123% to +154%. The albedo alone costs about
as much as the photo (wood grain, feathers and the model's own fine noise stay in it); the residual costs more again.

Verdict: for exact and +-k coding the photo is already the cheapest layer. The coder was left unchanged (streams
byte-identical); the experiment lives in the patch and scripts.

Follow-ups the same day (Hassan: "compress the flat colours, take advantage of the repeating colours"; then "a shading
layer and a colours layer"):

- Palette albedo (K repeated colours: label map + palette, 0.1-0.5 bpp) x shading at 1/8 as a guide, exact mode
  (`guide_predictor.patch`, `palette.py`): guide + separate residual +22% (was +135% with a full-colour albedo); the
  coder guided by it +3.9% (the guide is off by 18-32 levels rms, too rough to steer anything).
- Two layers, a looks-the-same mode (`two_layer.py`): colours = K region colours + label map; shading = one grey value
  per pixel, the encoder's least-squares fit given the region colour, coded at +-2; decode colour x shading / 255.
  kodim03 + kodim23, matched against the pixel coder at +-k and JPEG XL lossy at the same size (CIEDE2000 mean):

| regions | K | two-layer bpp | dE | pixel coder | JPEG XL, same size |
|---|---:|---:|---:|---:|---:|
| Marigold albedo | 16 (shading exact) | 3.42 / 3.64 | 3.2 / 5.5 | +-2: 2.86 / 3.16 bpp, dE 1.0 / 0.9 | dE 0.70 / 0.65 |
| Marigold albedo | 64 | 2.36 / 2.51 | 2.4 / 4.2 | +-3: 2.19 / 2.35 bpp, dE 1.3 / 1.2 | dE 0.82 / 0.78 |
| photo colour | 64 | 2.57 / 2.72 | 1.3 / 1.7 | +-3: 2.19 / 2.35 bpp, dE 1.3 / 1.2 | dE 0.79 / 0.75 |
| photo colour | 256 | 7.09 / 7.47 | 0.7 / 0.9 | +-1: 4.04 / 4.47 bpp, dE 0.6 / 0.6 | dE 0.43 / 0.40 |

The colours layer is either cheap and wrong or right and expensive (the label map grows fast); the grey shading alone
costs about as much as the +-k picture. Best two-layer (photo colours, 64) is ~15% bigger than +-3 at a similar colour
error. Side note: at matched size JPEG XL lossy has lower average colour error than the pixel coder's +-k (its
quantisation is perceptual, ours is a max-error bound), but it smooths the grain away.

### Everything stacked as a lossy mode: learned models + split allowance + grain mode, vs JPEG XL (2026-10-22)

`bench/img/combo.py`, 8 Kodak photos (01 03 05 08 13 19 21 23). New in the coder: `F_SPLIT_K` (flag 16, one header
byte): green (brightness) within +-k, red and blue within +-kc; old streams unchanged. Every configuration matched by
JPEG XL lossy (effort 7) at the same size. dE = CIEDE2000 mean; "soft" = after a sigma-1 blur of both (closer to
normal viewing; the only fair score for grain mode, whose grain is new by design).

| config | bpp | dE | soft | PSNR | JPEG XL same size: dE | soft | PSNR | we win (dE) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| +-1 | 5.84 | 0.69 | 0.20 | 49.96 | 0.83 | 0.25 | 45.52 | 7/8 |
| +-2 | 4.54 | 1.16 | 0.35 | 45.29 | 0.99 | 0.36 | 43.49 | 0/8 |
| +-2 learned | 4.48 | 1.16 | 0.35 | 45.30 | 1.00 | 0.37 | 43.39 | 0/8 |
| +-3 | 3.77 | 1.55 | 0.50 | 42.46 | 1.12 | 0.45 | 42.06 | 0/8 |
| +-4 | 3.24 | 1.91 | 0.66 | 40.43 | 1.24 | 0.58 | 41.03 | 0/8 |
| split 2/4 learned | 3.42 | 1.42 | 0.56 | 42.01 | 1.20 | 0.52 | 41.35 | 0/8 |
| split 2/6 learned | 2.94 | 1.70 | 0.89 | 39.99 | 1.34 | 0.67 | 40.40 | 0/8 |
| split 3/8 learned | 2.39 | 2.08 | 1.20 | 37.96 | 1.51 | 0.82 | 38.89 | 0/8 |
| grain + split 2/6 | 2.71 | 1.84 | 0.98 | 38.61 | 1.39 | 0.72 | 39.76 | 0/8 |

- The split allowance is a real gain over uniform +-k: split 2/6 is 9% smaller than +-4 with lower dE (1.70 vs 1.91).
  Its blurred error is higher though: loose red/blue steps show as slightly blotchy, oversaturated colour.
- Grain mode on top: 8% smaller again; pixel scores rise by design (new grain).
- Near-lossless (+-1) beats JPEG XL; from +-2 down, JPEG XL wins every photo at matched size. A max-error coder spends
  bits evenly; a transform codec puts its error where eyes do not look. By eye, at 2x on the crops, all five look alike.

### Context-adaptive allowance, judged by SSIMULACRA2 (2026-10-22)

`F_ADAPT_K` (flag 32, three header bytes kmax / kcmax / t): each sample's allowance is
min(kmax, k + activity / t), activity = five absolute differences among already-decoded neighbours, so the decoder
derives it and it costs no bits; errors stay bounded by kmax. Idea: eyes forgive errors in texture, not in flat areas.
CIEDE2000 cannot judge this (it counts every pixel's error the same), so quality is SSIMULACRA2 (100 identical, ~90
visually lossless; checked: JPEG XL distance 1 scores 87 on kodim03). Same 8 Kodak photos, JPEG XL matched by size:

| config | bpp | max err | SSIMULACRA2 | JPEG XL same size | we win |
|---|---:|---:|---:|---:|---:|
| +-1 | 5.84 | 1 | 95.46 | 95.81 | 3/8 |
| +-2 | 4.54 | 2 | 92.15 | 94.18 | 0/8 |
| +-3 | 3.77 | 3 | 88.30 | 92.65 | 0/8 |
| +-4 | 3.24 | 4 | 83.79 | 91.21 | 0/8 |
| split 2/6 learned | 2.94 | 6 | 84.83 | 90.17 | 0/8 |
| adaptive 1..4, t 8, learned | 3.89 | 4 | 89.68 | 92.87 | 0/8 |
| adaptive 1..6, t 16, learned | 4.21 | 6 | 90.02 | 93.53 | 0/8 |
| adaptive 2..8, t 8, learned | 2.95 | 8 | 82.87 | 90.09 | 0/8 |

Adaptive beats uniform +-k at matched size by ~1.4 points (3.89 bpp: 89.7 vs ~88.3 for uniform). Split brightness/colour
helps too. But to look as good as JPEG XL at ~3 bpp (SSIMULACRA2 ~90) this coder needs ~4.3 bpp, ~40% more. Errors
added sample by sample read as fine noise everywhere; a transform codec removes detail the eye does not track. The gap
is structural, not a tuning matter: this coder's ground is lossless and near-lossless (+-1), where it leads. By eye at
2x the crops (+-3, adaptive, JPEG XL) look alike.

### PNG modes in the browser tool: exact, +-1, looks the same (2026-10-22)

`tools/compress`: a picker under the drop zone (remembered per browser) for PNGs: exact (default), every sample
within +-1, or grain mode (looks the same). Inside the PNG container (NCZP): +-1 is coding 0 with k = 1 in the NCZI
header (old decoders already read it); grain mode is coding 2 carrying an NCZG stream. Palette / low-bit PNGs always
go exact; transparent PNGs or pictures outside 64x64 .. 12 MP fall back from grain mode to +-2 (grain mode needs ~170
bytes per pixel while splitting; it was ~200 before freeing buffers early, same output). The result card and the
unpacked card say which: "every pixel", "within +-1", or "looks the same: the grain is regrown, the pixels are not the
originals". Streaming grain encoder/decoder (split + fingerprint up front, then rows; grain regrown at the end).

Native vs wasm (`bench/wasm_png_check.mjs`): kodim03, kodim13, ram poster, harvest poster, a transparent picture and a
palette picture, every mode, both wasm builds (relaxed SIMD and plain SIMD): every .ncz stream and every rebuilt PNG
byte-identical to native (SHA-256). Same again through the page's own code in the browser. Text, code and binary files
compress to the same bytes with the old and new wasm. Grain mode in wasm runs at native speed (kodim03: 1.5 s encode,
0.7 s decode). kodim03: PNG 539 KB -> exact 299 KB, +-1 199 KB, looks the same 121 KB.

### Wonder format, prototype 0: structure kept, fine texture regrown from statistics (2026-10-22) — fails on real texture

`bench/wonder/proto.py`: YCoCg Laplacian pyramid coded closed-loop with real bytes (ncz byte coder); in the finest
band each 16x16 block is kept (quantised), dropped, or regrown (energy per block + the band's spectrum; band-limited
noise shaped and scaled on decode). kodim13 (river), SSIMULACRA2 at matched size:

| q | no regrowth: bpp / score | with regrowth: bpp / score | JPEG XL at the regrowth size |
|---|---|---|---|
| 6 | 4.67 / 83.4 | 1.84 / -19.6 | 71.0 |
| 16 | 2.61 / 57.0 | 0.92 / -26.4 | 46.6 |

Regrowth cuts the bytes by 2.5-3x but wrecks the picture: kodim13's finest band has rms 21 levels and carries
structure (water sparkle, leaf edges); phase-random noise with the right energy and spectrum reads as speckle, by eye
too. Statistical regrowth works for grain (low-amplitude noise, grain mode), not for real texture. The plain pyramid
coder is also far behind JPEG XL (57 vs 81 at 2.6 bpp): a base coder needs work too.

### Wonder format, prototype 1: fine detail generated from the coarse picture by small networks (2026-10-22)

`bench/wonder/train.py` (rig, 3090), `bench/wonder/proto1.py`. Two 66k-parameter conv nets (~16k MAC per pixel, half
resolution inside), trained on 1,071 COCO photos halved in size (kills JPEG blocks); Kodak held out. Input: the
coarse picture (everything but the finest band, quantisation simulated). P (L2) predicts the finest brightness band:
error 106 vs 289 for predicting zero (63% of the band's energy explained). T (L1 + local energy + patch critic, noise
and one stored energy byte per 16x16 block as inputs) generates it. Profiled before training: P 5 ms/step, T 9 ms/step
at batch 32 (batch 128 only +17% throughput); P 20k steps 1.7 min, T 45k steps 6.9 min.

Kodak 03 05 13 23, pyramid coder of proto.py, SSIMULACRA2 at matched size:

| q | plain bpp / score | + P predictor | + P + T regrowth | JPEG XL at the P size |
|---|---|---|---|---|
| 6 | 3.20 / 77.3 | 3.01 / 77.7 | 1.92 / -0.6 | 91.2 |
| 10 | 2.27 / 62.2 | 2.18 / 63.4 | 1.37 / -4.5 | 87.2 |
| 16 | 1.63 / 40.2 | 1.64 / 41.4 | 0.99 / -12.3 | 82.1 |

- P helps a little (3-6% fewer bytes at q 6-10, same score).
- T's texture has real shapes (parrot feathers look right; water reads as water, a little wormy), far better than
  prototype 0's speckle by eye, but SSIMULACRA2 punishes any regrown detail hard: 46 regrown blocks of 1,536 take
  kodim23 from 54 to 16. A full-reference metric cannot judge regrowth; the eye has to. Fixed on the way: picture
  borders (mirror padding; the nets only saw interior crops) and regrowth in smooth areas (strong detail only).
- The real gap is the base coder: before any texture, at the same bytes JPEG XL scores 87-91 and this pyramid coder
  54-78. Regrowth cannot close that; the base has to be competitive first.

### Wonder format: pretrained teacher as the base (cheng2020-anchor, CompressAI weights) (2026-10-22)

`bench/wonder/teacher.py` (rig) + `teacher_eval.py` (Mac). Kodak 01 03 05 08 13 19 21 23, SSIMULACRA2. "own" = the
teacher's own learned entropy model (hyperprior + context); "ours" = the same integer latents through ncz's generic
byte coder (no latent-aware model yet). JPEG XL effort 7 matched by size.

| quality | ours bpp | own bpp | teacher score | JPEG XL at ours | JPEG XL at own |
|---|---|---|---|---|---|
| 1 | 0.259 | 0.161 | 3.9 | 0.4 | -14.5 |
| 2 | 0.368 | 0.247 | 23.2 | 24.8 | -2.6 |
| 3 | 0.508 | 0.361 | 38.9 | 45.3 | 20.0 |
| 4 | 0.778 | 0.557 | 53.8 | 60.6 | 42.8 |
| 5 | 1.038 | 0.775 | 65.3 | 69.8 | 57.8 |
| 6 | 1.385 | 1.019 | 73.2 | 77.3 | 67.5 |

At the teacher's own rate it beats JPEG XL by 6-19 points at every quality (q1: a whole photo in ~2.8 KB that
JPEG XL cannot reach). Our generic coder spends 36-60% more bits on the same latents, which hands the win back to
JPEG XL: the latent coder is the gap. Next: a latent-aware context-mixing coder (contexts: spatial neighbours in the
same channel, co-located values in earlier channels, channel id), target <= the teacher's own rate.

### Latent coder (NCZY, `src/lat.rs`): context mixing on the teacher's integer latents (2026-10-22)

Integer-only contexts (entropy decoding never reads a network output): per-channel 5-bit size class (empty channels
free, strongest channels first, order implied by the classes), cross-channel energy at the same spot (normalised by
channel size), same-channel neighbours, signs nearby; two mixers + APM. Round trip exact on all 48 test files, 0.02 s
per picture. `bench/wonder/latcmp.sh`; 8 test photos, bits per image pixel:

| quality | teacher own | generic ncz | NCZY cold | NCZY primed (16 other Kodak) | NCZY + teacher's mean/scale as context (ceiling) |
|---|---|---|---|---|---|
| 1 | 0.161 | 0.259 | 0.223 | 0.220 | 0.177 |
| 2 | 0.248 | 0.368 | 0.321 | 0.317 | |
| 3 | 0.361 | 0.508 | 0.448 | 0.443 | |
| 4 | 0.557 | 0.778 | 0.685 | 0.679 | 0.574 |
| 5 | 0.775 | 1.038 | 0.924 | 0.917 | |
| 6 | 1.019 | 1.385 | 1.187 | 1.179 | 1.040 |

- NCZY cuts 12-17% off the generic coder but stays 16-38% above the teacher. Priming on other photos: only 1-2%
  (cold start is not the problem).
- Per channel (teacher_chan.py), we lose by a similar factor in every channel group, strongest first: the teacher's
  learned (nonlinear) mean/scale prediction is what we lack. Linear prediction from neighbours and earlier channels
  (ridge, fitted on 16, tested on 8): only ~10% off the order-0 entropy.
- Teacher's own Gaussian model on OUR symbols (plain round(y)): 0.182 / 0.567 / 1.027 at q1/4/6, i.e. its edge over
  us is partly mean-centred quantisation, round(y - mean), which needs the exact same mean on both sides.
- With the teacher's mean/scale as context, NCZY beats the teacher's Gaussian on the same symbols at q1 (0.177 vs
  0.182) and nearly matches at q4/q6. So the coder is fine; the missing piece is a deterministic learned predictor of
  mean and scale, which would also allow mean-centred quantisation.

### Wonder format: our predictor (pred.py) and a first student decoder (student.py) (2026-10-22)

Predictor (ours, 0.68M / 0.83M params, trained 15k steps on DIV2K latents, 3-4 min each): channels in a fixed
strongest-first order, 8-9 groups, each group's mean/scale from a small conv net over all earlier groups (full
picture). NCZY with it as context, 8 Kodak test photos: q1 0.2232 -> 0.1977 bpp, q4 0.6847 -> 0.6205 (teacher 0.1613 /
0.5568). Per group (Gaussian bits): from the 17th channel on we are within ~5% of the teacher (better on the weakest at
q1); the first group (4 strongest channels, no earlier groups to predict from) costs 2x the teacher's (q1 0.059 vs
0.030), most of the remaining gap. At these sizes the pictures beat JPEG XL on 5 of 6 (parrots, bikes, river x q1/q4).
Bug on the way: the first run trained 1 step in 50 (a conditional expression); caught by the impossible speed.

Student decoder (1.35M params, 64k MAC per pixel; teacher q4 decoder 11.8M, 552k): teacher's encoder fixed, latents of
all 800 DIV2K photos cached on the GPU (81 s), MSE to the original, 30k steps (7.4 min, batch 16 of 256 px). First try
diverged (no input normalisation, no clipping, lr 1e-3: outputs ~1e4); fixed with per-channel input scaling, gradient
clip 0.5, lr 5e-4. Kodak 8, same latents (same file size):

| | SSIMULACRA2 | PSNR |
|---|---|---|
| teacher decoder | 53.8 | 32.8 dB |
| student, 30k steps | 34.6 | 29.0 dB |

Visibly softer (bushes, water, feathers, logo edges). Undertrained: train-crop PSNR still climbing at the end.

Distilled student (same 1.35M / 64k MAC): + MSE to the teacher's output (computed once per photo, uint8 on the GPU)
and to its features at 1/8 and 1/4 resolution (its low-resolution layers run per step; 1x1 projections, RMS-
normalised, weight 0.1). Profiled: full teacher per step 35.6 ms/step; cached output + low-res features 18.8 ms.
100k steps, 33 min. Kodak 8, same latents:

| | SSIMULACRA2 | PSNR |
|---|---|---|
| teacher decoder (11.8M) | 53.8 | 32.8 dB |
| student, plain, 30k steps | 34.6 | 29.0 dB |
| student, distilled, 100k steps | 40.9 | 30.0 dB |

+6.3 SSIMULACRA2 / +1.0 dB; parrot feathers mostly back, foliage and water still soft. The remaining 2.8 dB looks
like capacity (train-crop PSNR flattening at 30.3 near the end of the schedule).

### Tiny mode in the browser tool (learned codec, q4) (2026-10-22)

`tools/compress` gets a fourth picture mode, "tiny", for photos (PNG, JPEG, WebP). Encode: the teacher's encoder
(ONNX, fp16-stored weights, 8.4 MB) in ONNX Runtime Web (WebGPU, else WebAssembly) -> rounded latents -> NCZW
(`src/wonder.rs`: "NCZW" | version | quality | picture size | NCZY v2). NCZY v2 codes the channels in the predictor's
group order; before each group, our predictor (`wonder::Predictor`, 3.3 MB, plain f32 with a fixed order of every sum,
no exp/log: scale buckets by comparison) predicts it from the groups already decoded. Decode: NCZW -> latents (wasm)
-> our student decoder (ONNX, 2.7 MB) -> PNG. Encoder weights: CompressAI cheng2020-anchor q4, BSD 3-Clause Clear,
notice on the page.

- Native round trip exact on the 8 Kodak test photos, 0.6201 bpp, 0.2 s each way for the latents. Native and both
  wasm builds (relaxed and plain SIMD) write byte-identical NCZW streams and decode each other's exactly
  (`bench/wasm_wonder_check.mjs`).
- Browser (Chrome, WebGPU): kodim03 PNG 551,636 B -> 12,514 B, compress 2.4 s with the network fetch, unpack 0.5 s,
  33.15 dB (the same decoder in PyTorch: 33.21). kodim05 as a q92 JPEG 174 KB -> 37.9 KB. A 2048x1024 WebP 301 KB ->
  6.7 KB, back at full size. CPU fallback (forced): same 12,514 B file, but 29 s / 6 s on one thread (threads need a
  cross-origin isolated page, not set up).

### No more size caps: grain mode v2 (tiled) and tiny mode in tiles (2026-10-22)

Grain mode (NCZG v2; v1 files still decode): the fingerprint is measured on a centre crop of at most 2048x2048, the
clean picture is made in 1024 px tiles with 32 px margins on the denoiser's 4 px block grid (test: bit-identical to
the whole-picture pass, `tiles_join_exactly`), and regrowth is local: noise is a hash of the pixel position, fields are
normalised by their theoretical spread, the quantile map is per pixel (test: a tile's field equals the same window of
a bigger one). Cap 12 MP -> 268 MP. Pictures up to 2048 px a side: same clean picture and same size as v1 (kodim03,
kodim13, ram poster: identical bpp); the regrown grain is new (envelope fit 0.05 -> 0.10 on kodim03, 0.20 -> 0.15 on
kodim13). 48 MP (8000x6000) native: 160 s for compress + decompress, 1.05 GB peak (v1 would need ~8 GB); no seam at
tile corners by eye or in column statistics. Browser: a 24 MP PNG 42.5 MB -> 7.48 MB in grain mode, 60 s / 40 s.

Tiny mode: encoder and decoder networks run in tiles (1024 px cores, 128 px margins, on the 64 px grid; pictures up to
1024 px a side are one tile, same bytes as before). The predictor's convolution now runs a row at a time over 16
output channels (accumulators in cache, same order of every sum: streams bit-identical): 48 MP-sized latents 14.6 ->
8.8 s native, 14.3 s wasm. Browser, a 48 MP JPEG (10.4 MB) -> 669 KB, 8000x6000 back; compress 146 -> 46 s, unpack
115 -> 24.5 s. Cap 16 MP -> 65535 px a side.

Weight-tied student (each residual block run twice, a learned per-channel gain per pass; same 1.35M weights, 100k vs
64k MAC per pixel, CPU decode 147 vs 85 ms for 768x512), same distillation and 100k steps (52 min). Kodak 8:

| | SSIMULACRA2 | PSNR |
|---|---|---|
| teacher | 53.8 | 32.8 dB |
| 1 pass | 40.9 | 30.0 dB |
| 2 passes | 41.2 | 30.1 dB |

+0.3 / +0.1 dB for 1.7x the decode time; indistinguishable by eye. Kept the 1-pass student in tiny mode. Reusing the
blocks does not add capacity here; the gap to the teacher needs real capacity (wider/deeper with its own weights) or a
different loss.

Student v2 (channels 128/128/96/64/48, capacity moved toward full resolution; 2.46M weights, 129k MAC per pixel), same
distillation, 100k steps, 4-bit weights trained in from step 60k (QConv: per-channel scale min(max, 3 rms)/7, levels
-8..7, straight-through):
- float, before 4-bit: train-crop PSNR 30.69 at 60k vs 29.84 for v1 at the same step (+0.85 dB): the capacity helps.
- 4-bit from 60k: dropped to 27.3, recovered only to 28.2. Kodak: 29.38 dB / SSIMULACRA2 18.8 vs v1 8-bit 29.99 /
  40.9 (softer, blotchy smooth areas). The scheme was too crude (fixed scales, sudden switch, schedule already decaying).
- Post-training quantisation of v1 (Kodak 5): float 30.17, 8-bit 29.95, 6-bit 28.53, 4-bit 20.37 dB.
- 4-bit ONNX path works: int8 levels + DequantizeLinear, file packed with ncz to 2.9 bits a weight (v1: 1.35M weights
  -> 489 KB, fp16 was 2.7 MB); browser output equals ONNX Runtime in Python.
- Downloads: the decoder is 2.7 of tiny mode's 14.4 MB; the encoder (8.4 MB fp16) is the big one.

Tiny mode downloads 14.4 -> 6.4 MB:
- Encoder: conv weights 8-bit per output channel (DequantizeLinear; GDN parameters fp16), packed with ncz: 8.36 MB ->
  3.08 MB. Kodak 8: 29.76 -> 29.90 KB per photo (+0.5%), 30.00 -> 29.98 dB, 1.07% of latents differ (`quant_onnx.py`).
- Predictor: NCZQ format, weight matrices as per-row f32 scale + int8 levels (weight = level * scale, one f32
  multiply: deterministic), biases f32: 3.34 MB -> 847 KB, packed 637 KB. 0.6201 -> 0.6205 bpp; streams identical to
  the f32 file with the dequantised values; native and both wasm builds identical (`pred_q8.py`).
- NCZW v2 header carries the predictor id (FNV-1a of its file); a file from another predictor is refused ("made with a
  different version of tiny mode"), never decoded into garbage. Browser: kodim03 12,606 B, 33.15 dB.

Student v2 shipped (8-bit, first layer fp16). Same run with 8-bit QAT from 90k (scale max/127): train-crop PSNR
31.33 -> 30.68 at the switch; Kodak 30.59 dB / SSIMULACRA2 38.4. The float checkpoint from 90k: 30.91 / 44.7 (v1
shipping: 29.99 / 40.9). Post-training 8-bit of the float model: -0.32 dB / -5.7 (max or MSE-picked scales alike);
one layer at a time: the first conv (192 latents -> 128, weights max/rms 15) alone costs -0.34 dB, every other layer
under -0.01. So: first conv fp16, the other 14 at 8 bits (no QAT): Kodak 8 30.88 dB / 44.4, decoder 2.71 MB ONNX,
2.20 MB packed with ncz (v1 fp16 was 2.72 MB). Browser: kodim03 33.15 -> 33.61 dB, same 12,606 B file, unpack 0.24 s
after the networks load. Tiny mode download 5.9 MB (encoder 3.08, decoder 2.20, predictor 0.64).

Tiny mode + regrown grain (NCZW v3: grain fingerprint between the header and the latents; v2 still decodes). The
encoder measures the original's grain fingerprint (grain::measure_on, centre crop of at most 1024 x 1024 in the
browser), the decoder regrows it (regrain_v2) on the decoded picture. Kodak 8 with the v2 decoder: every photo has
measurable grain, fingerprints 1309-1549 B (+5-11% on these 12-55 KB files); 30.88 -> 30.76 dB, SSIMULACRA2 44.4 ->
43.9 (both metrics count new grain as error); by eye the smooth areas read as film instead of plastic. Browser:
kodim23 12,584 B with grain, 34.58 dB (native prototype 34.59); native and both wasm builds identical on every
NCZW and PNG-mode check.

### Better teacher? HPCM-Base (ICCV 2025, MIT) vs cheng2020-anchor, through our pipeline (2026-10-22)

HPCM-Base lambda 0.013 (`~/wonder/hpcm`; its rANS coder rebuilt for Python 3.14; the checkpoint predates the
adaptive_params fix, they were never saved or trained, loaded at 1.0). Its latents: 320 channels at 1/16 (cheng q4:
192). Our predictor trained on them the same way (15k steps on DIV2K, groups 8/8/16/16/32/48/64/64/64, 1.0M params).
Kodak 8:

| | bpp | PSNR |
|---|---|---|
| HPCM own codec (real bitstream) | 0.554 | 33.69 |
| HPCM latents through our pipeline (NCZY + our predictor, its decoder on round(y)) | 0.724 | 33.45 |
| cheng2020 own (likelihoods) q4 | 0.557 | ~32.8 |
| cheng2020 through our pipeline q4 / q5 | 0.620 / 0.848 | 32.84 / 34.64 |

At equal quality (cheng interpolated in log bpp): at 33.45 dB cheng through our pipeline needs 0.689 bpp, HPCM 0.724
(+5%); at HPCM's 0.724 bpp cheng gives 33.73 dB (+0.28). HPCM's ~15-18% edge is real for its own codec, but it lives in
its entropy model: our predictor is 31% above HPCM's own rate on its latents (11% on cheng's). Not switching teachers
until our predictor closes that gap (side information for the first groups, more capacity for 320 channels).

### Teacher's hyperprior as predictor input: no gain (2026-10-22)

`pred.py HYPER=1`: z = round(h_a(y) - medians) (the side information, 192 ch at 1/64), its h_s features (384 ch at
latent resolution, fp32 without TF32, cached fp16) as extra input to every group's net, the first group included.
Two NaN blow-ups on the way: exp(log-scale) overflowing to inf (inf * 0 = NaN gradients; photo 561 has features up to
114, median 7); fixed by clamping the log-scale at 10 before exp and the scaled features to +-16.
q4, 15k steps: train crops 0.512 -> 0.480 bpp, but Kodak 8: Gaussian 0.645 -> 0.639 (strongest 8 channels 0.126 ->
0.122; teacher 0.074), NCZY 0.6205 -> 0.6271, plus z 186 B a photo (0.0038 bpp). Not worth porting. The teacher's edge
on the strong channels comes from its autoregressive context (causal neighbours across all channels), not its
hyperprior. The HPCM rerun is moot until that changes.

### Two-pass checkerboard predictor (2026-10-22)

`pred.py CB=1`: pass 1 codes the anchor half ((x + y) even) of every channel group by group from the anchors decoded so
far; pass 2 the rest, each group from ALL anchors (every channel, four sides) plus the complete earlier groups. 1.91M
params, 29 ms/step, 15k steps. Rust: NCZC/NCZD predictor formats, `GroupModel::passes`, NCZY codes each pass's positions
(one-pass streams unchanged, checked byte for byte), exact round trip on Kodak 8.

| q4, Kodak 8 | Gaussian bpp (strongest 8 ch) | NCZY bpp |
|---|---|---|
| one-pass predictor | 0.645 (0.126) | 0.6201 |
| checkerboard | 0.610 (0.099) | 0.6105; with checkerboard-aware neighbour contexts 0.6102 |
| teacher's own model | 0.567 (0.074) | — |

The raw model gains 5.6%, the coder only 1.6%: the context mixer's own neighbour contexts were already finding most of
that spatial information (one-pass: coder 4% under its Gaussian; checkerboard: equal). Costs 2x predictor weights
(~1.4 MB packed vs 0.64) and 2x predictor time. Not shipped; the Rust path stays.

### Tiny mode: three sizes (q1, q2, q4) (2026-10-22)

q1/q2 (128-channel latents): predictors (q1 from before, q2 15k steps; NCZQ: q1 0.1977 -> 0.1978 bpp, q2 0.2884, ~0.54 MB
packed each), v2 students (2.39M weights, distilled, 80k steps, 25 ms/step, ~35 min each; float, then 8-bit with the
first layer fp16; 2.0 MB packed each), encoders 8-bit (1.45 MB packed each). Kodak 8, with regrown grain, JPEG XL at the
same size:

| size | KB | tiny PSNR / SSIMULACRA2 | teacher decoder | JPEG XL |
|---|---|---|---|---|
| smallest (q1) | 10.9 | 27.25 / 0.8 | 27.72 / 3.9 | 25.02 / -5.4 |
| small (q2) | 15.3 | 28.50 / 19.0 | 29.12 / 23.2 | 25.81 / 11.4 |

The students sit 0.5-0.6 dB under the teacher here (q4: 2 dB). Page: a size row under tiny (smallest / small / sharp),
remembered per browser; each size fetches its own networks once (4.0 / 4.0 / 5.9 MB); unpacking reads the size from
the file. Browser (WebGPU), bikes: q1 15.2 KB 25.67 dB, q2 21.1 KB 27.15 dB, q4 40.2 KB 29.81 dB.

### Text model check: SupraLabs/Supra2.5-Pico vs our GPT-S3-3M (2026-10-23)

Same size class (2.98M vs 2.97M params, Apache-2.0, 4104-token BPE, 10 transformer layers, trained on 12B tokens).
`bench/lm_bpb.py`, first 300 KB, bits per byte (its custom modelling code read first: torch/transformers only):

| text | GPT-S3-3M | Supra2.5-Pico |
|---|---|---|
| Nardo docs | 1.857 | 1.938 (+4%) |
| enwik8 | 1.510 | 1.687 (+12%) |
| JS/Python code | 2.090 | 2.370 (+13%) |

Worse on all three; the card's "bits-per-byte gap of 0.09 vs GPT-S3-3M" is its deficit. Keeping GPT-S3-3M.

### Texture decoder for the smallest size (q1) (2026-10-23)

`bench/wonder/texture.py` / `texture2.py`: fine-tune of the q1 v2 student (same files) with a patch critic that also
sees the latents. Runs, 15k steps each (58-82 ms/step), Kodak 4 (bikes, river, parrots, houses), PSNR vs smooth
25.73 / 23.82 / 31.53 / 25.38:
- critic weight 0.05: barely any texture (-0.1 dB).
- critic 0.3 + feature matching 5: real texture (leaves, tiles, treads, feathers), -1.1..-1.4 dB, but fuzz on
  smooth plastic (red tank, fenders).
- + consistency (the decoded picture must re-encode to the file's latents, frozen teacher encoder; pixel term 0.1):
  -1.6..-1.9 dB; re-encodes 99.1% to the same latents (the smooth decoder: 97.4%). Fuzz unchanged.
- blending smooth/textured per 32 px block to the original's measured detail (~144 B map): +0.2-0.4 dB, fuzz
  unchanged (fender blocks also hold edges and lettering).
- v2: detail map as decoder + critic input, penalty on more 8 px detail than the original (0.3): yellow/green
  fenders clean, red tank still blotchy; 99.24% re-encode.
- v3: penalty 1.5 + critic pressure only where the map says the original had detail: same picture as v2, a bit
  calmer; 99.26% re-encode; -0.05..-0.25 dB vs v2. The red tank's blotches are most likely the original's film grain
  imitated as texture (the map sees grain as detail). Next: grain-free training targets.

Shipped v3 as an unpack choice: NCZW v4 carries the detail map (~150 B, packed with ncz; v2/v3 files still decode),
the page computes it when packing the smallest size, and "unpack: smooth / textured" picks the decoder (2.05 MB
packed, 8-bit, first layer fp16; ONNX within 46-50 dB of PyTorch). Browser, bikes q1: 15,375 B; smooth 25.67 dB,
textured 23.83 dB (PyTorch 23.84).

### Texture decoder v5: the grid fixed (2026-10-23)

v4 (noise added after each upsampling stage, gains started at zero, spectrum loss 0.2) left the grid: the learned noise
gains stayed ~0.002, the decoder copied texture per 16 px latent cell. v5 (texture2.py NOISE=2 MSCRIT=1 W_SPEC=2): 8 more
noise channels mixed (1x1 conv, weights started at sd 0.1) into the last two stages before their residual blocks, the
critic also judges the picture at 1/2 and 1/4 size, spectrum loss x10. 15k steps, 51 min (GPU shared). Grid peaks in
(textured - smooth), 16 / 8 / 4 px period, 1 = no grid:

| | kodim13 | PSNR | kodim08 | PSNR |
|---|---|---|---|---|
| v3 (was shipping) | 10.4 / 6.6 / 9.9 | 21.90 | 2.4 / 2.4 / 2.4 | 23.53 |
| v4 | 9.9 / 8.2 / 13.1 | 22.21 | 2.2 / 2.4 / 3.5 | 23.75 |
| v5 | 2.9 / 4.3 / 5.3 | 22.13 | 2.2 / 2.8 / 2.6 | 23.66 |

Re-encodes to the same latents: 99.09%. Browser: the decoder takes the noise as input z (1 x 9 x H x W); the page makes
it from a hash of the pixel position (sum of 4 hash bytes, sd 1), so tiles agree and a file always unpacks the same.
Hash noise vs randn through the 8-bit ONNX (texparity.py): same PSNR (22.14) and grid peaks; ONNX vs PyTorch with the
same noise, max 0.037 (8-bit weights), PSNR 22.14 vs 22.13. In the browser: kodim13 q1 17.2 KB, textured 22.11 dB,
2.8 s to unpack. Download 2.05 MB (same as v3). The red tank still has some fuzz.

### Texture decoder v6 / v7: colour mottle and the 4 px grid (2026-10-23)

The red-tank fuzz was colour, not texture: v5 drew fine B - Y detail 2.8x the original's on the tank (5.4 vs 1.9; whole
photos 1.6x), which the brightness-only flat penalty and map never see. v6 = v5 + W_CHROMA 1.5 (per 8 x 8 block, fine
B - Y / R - Y rms may not exceed the original's + 0.25): tank 5.4 -> 2.5, whole photos at the original's level. It
brought back a 4 px grid on foliage (kodim13 4 px peak 5.3 -> 8.1). That grid is in the decoder itself, not the noise
(zero noise: 10.5; noise lowers it), from the 1/4-resolution stage's pixel shuffle. Measure: the part of the fine detail
that is the same at every pixel with the same (x mod 16, y mod 16) over its rms ("grid share"; originals 0.074-0.105,
v3 0.197 on kodim13). v7 = v6 + W_PHASE 0.2 (that energy may not exceed the original's; crops are latent-aligned):

| all 8 Kodak | PSNR | grid share | 4 px peak |
|---|---|---|---|
| v6 | 25.46 | 0.121 | 3.6 |
| v7 | 25.59 | 0.080 | 1.2 |

Shipped v7 (ONNX vs PyTorch max 0.045, browser kodim13 22.05 dB, repeatable). By eye the texture still reads as
pixel-level speckle ("bit-crushed"): matching energy, spectrum and grid statistics does not make it look like leaves.

### Swatches: real texture instead of invented texture (2026-10-23, CPU test, bench/wonder/swatch_ceiling.py)

The file carries a few small grey samples ("swatches", 48 px) of the photo's own textures, cut from areas where every
16 px block is texture the smooth picture lost (no edges inside: they get pasted onto bare walls); per block a swatch
index (or none: only blocks where the smooth picture kept < 60% of the original's fine detail); per 32 px block the
detail level. The decoder lays the swatch's fine detail (sigma 2 band) on top of the smooth q1 picture, patch by patch
(cost: coarse-shape match + 4 px overlap match, mirror allowed), scaled to the detail the smooth picture lacks.

- Perfect swatches: foliage reads as real leaves, no speckle or grid; mean PSNR 26.4 dB (v7: 25.6).
- Coded pixel-wise (ncz near +-K): +-8 costs 2.1-6.4 KB (too much); +-16 fits but its quantisation noise is as strong
  as the texture: dots on walls, sparkle on paint (bit-crushed again).
- Transform-coded (8 x 8 DCT, dead zone, step 10; bytes estimated from adaptive level costs): swatches 1.1-3.0 KB +
  labels/levels 0.35-0.45 KB. q1 + swatches 107.2 KB vs q2 125.1 KB over the 8 photos (each photo at or below q2's
  size, within 0.4% on kodim03/19/23). Mean PSNR 26.44 vs q2 28.50.
- By eye vs q2: foliage clearly better (texture vs blur); walls get 8 x 8 DCT basis stripes; on the bikes, swatches of
  tread land on red paint and springs (wrong kind of texture for the block).

Swatches v2 (bench/wonder/swatch2.py): the encoder tries, per block, every swatch, grain, and nothing, and keeps the
one whose result's texture statistics (energy in two bands, energy by direction, peakiness) are closest to the
original's there; the decoder repeats the walk from the labels (asserted identical). Grain: position-hashed noise,
softness matched to the photo (1 byte), scaled to the missing level. Unused swatches are dropped. Blocks: none 55-79%,
grain 4-18%, swatch 13-27%. q1 + swatches + labels 107.0 KB vs q2 125.1 KB (-14.5%; kodim03/23 +0.6% / +1.3%).
By eye vs q2: plaster walls get stucco-like grain close to the original's (q2: flat), bushes crisper, water similar;
still wrong: the corrugated fork boots on kodim05 get random red speckle (regular structured texture the smooth
picture lost, no swatch can rebuild it, and the statistics still prefer a swatch to nothing). Bug on the way: the
swatch gain cap (x2) also capped grain at rms 2.

Regular-texture guard: blocks whose original fine detail repeats (normalised autocorrelation peak at 3-10 px, 32 px
window: fork boots / springs 0.4-0.8, foliage 0.12-0.24, plaster 0.24-0.41, tread 0.26-0.30) get nothing, as the
smooth picture: own score >= 0.5, or >= 0.35 next to one that is. Encoder only, no bytes. Fork boots and springs now
clean (as q2). Cost: some plaster next to windows is caught too and goes flat (kodim08 yellow wall); kodim03 / 23 flag
about half their texture blocks. Total q1 + swatches 106.6 KB vs q2 125.1 KB (-14.7%). Still seen: a few bush swatch
patches land in the kodim13 river.

### Swatches in the real format (2026-10-23)

ncz/src/swatch.rs (port of bench/wonder/swatch2.py; f32, deterministic k-means start instead of random): encoder
(cut swatches, regularity guard, per-block choice, drop unused), DCT atlas coded by the NCZY latent coder as 64
frequency channels, labels + levels by ncz v3. NCZW v5 adds the section after the detail map (v2-v4 still decode).
`nczcli swatch` runs it natively: Kodak 8, 0.9-2.0 KB per photo, encode 0.4-1.6 s, lay 0.1-0.2 s, mean PSNR 26.66
(Python reference 26.66). Browser (tools/compress): q1 files carry swatches (no detail map any more); the encoder
decodes its own latents and regrows grain to see what unpack will show, then builds the section; "textured" lays
it (files from before keep the texture decoder). End to end in the browser, Kodak 8:

| | bytes (8 photos) | mean PSNR |
|---|---|---|
| q1 + swatches | 101,622 | 26.60 |
| q1 smooth (same files) | | 27.24 |
| q2 | 125,264 | 28.48 |

Every photo smaller than its q2 (-4% to -27%, -18.9% total). kodim13: encode 4.1 s (WebGPU), unpack textured 0.69 s
vs smooth 0.44 s. By eye vs q2: foliage textured where q2 is blur, plaster grain, bikes clean; a few bush patches still
land in the kodim13 river.

### Swatches on big photos (2026-10-23)

First version (whole-picture planes): 12 MP native encode 71 s, lay 9.4 s, 809 MB peak; 48 MP 825 s, 2.6 GB (would not
fit a browser). Now: stripes of 16 block rows (256 px) with 32 px filter margins (patches cross-fade across stripe
borders as anywhere); grain field normalised from a fixed 256 x 256 field instead of the whole picture; dot products in 8
running sums (vectorise); above 4 MP, swatch spots on a 2 px grid and the encoder tries only the 3 swatches whose kind is
nearest the block's, and swatches dropped for too few blocks leave those blocks bare instead of a second pass
(section v2 records the stride; v1 decodes). Kodak unchanged (stride 1: same choices, 2.5x faster).

| | native encode / lay | wasm (node) encode / lay | wasm memory | section |
|---|---|---|---|---|
| 12 MP | 5.1 / 0.9 s | 6.7 / 1.2 s | 196 MB | 11.2 KB |
| 48 MP | 39 / 1.3 s | 48 / 2.3 s | 402 MB (288 MB of it the two pictures) | 20.7 KB |

(48 MP: native peak RSS 677 MB including the three 144 MB pictures the CLI holds.) Labels cost ~1.3 bits a block, the
same share of the file as on Kodak; coding them as a 2-D plane with the latent coder was no smaller than ncz v3.
Browser end to end (WebGPU; the pane was hidden, so the page ran as a background page and every time below is
inflated, e.g. 48 MP smooth unpack 196 s vs 24.5 s measured visible on 10-22): 12 MP JPEG -> 101.7 KB, textured unpack
works (31.33 dB vs smooth 32.13; fur and stone texture back); 48 MP -> 301 KB, textured unpack works, wasm memory 518 MB.
Bug on the way: std::time::Instant::now() panics on wasm32-unknown-unknown ("unreachable"); the timing probes are gone.

### Swatches: browser timings, wrong-surface patches, all three sizes, faster grain (2026-10-24)

Browser, pane visible (WebGPU): 12 MP JPEG compress 29.5 s, unpack smooth 13.4 s / textured 14.4 s; 48 MP compress
112 s, unpack 56.5 / 58.2 s, wasm 518 MB. Laying swatches adds 1-2 s. The 48 MP unpack is slower than 10-22's 24.5 s
because grain regrowth landed after that measurement (~20 s native at 48 MP), not because of swatches.

Bush patches in the kodim13 river: not a wrong kind (a penalty on the distance between a block's descriptor and the
swatch's kind removed forest texture before river patches) and not skew (foam vs foliage skew as a statistic: no
change). The cause: grain overshot there (finest detail ~1.9x the original's: the 32 px level is inflated by foam
edges), so the bush swatch won by default. Section v3: grain at 1, 0.6 or 0.35 of the missing level, chosen per block.
River patches nearly gone, forest kept, Kodak mean PSNR 26.66 -> 26.73.

Swatches now at every size (browser, Kodak 8, swatch share of the files and mean PSNR smooth / textured):

| | total | swatches | PSNR smooth | textured |
|---|---|---|---|---|
| q1 | 101.8 KB | 12.4 KB (12.2%) | 27.24 | 26.66 |
| q2 | 132.6 KB | 7.4 KB (5.6%) | 28.48 | 28.09 |
| q4 | 259.2 KB | 2.3 KB (0.9%) | 30.75 | 30.69 |

By eye q2 + swatches: forest, plaster and lighthouse stucco textured where q2 smooth is flat; q4: small gain on
plaster. Grain regrowth: the noise filters loop over pixels innermost (each output adds the same terms in the same
order: identical bytes, wasm PNG modes still byte-identical to native), 12 MP regrowth ~6.3 -> ~2.2 s native.

Swatch DCT step (Kodak 8, swatch bytes): 10: 11.8 KB, 13: 10.0, 16: 8.3, 20: 7.5. 16 looks the same as 10 by eye
(foliage, plaster, stucco, stone); 20 starts to flatten. Step 16 now (the step is in the section header).

### Against AVIF at the same size, and a compact grain fingerprint (2026-10-24, bench/wonder/scoreboard.py)

Kodak 8, browser-made tiny files; every other codec at its largest quality whose file fits the tiny file's size
(imagecodecs: libjxl effort 7, libaom AVIF speed 4, libwebp method 6). SSIMULACRA2 / PSNR, mean:

| | q1 size (12.7 KB) | q2 size (16.6 KB) |
|---|---|---|
| tiny textured | -5.3 / 26.66 | 13.5 / 28.09 |
| tiny smooth | 0.7 / 27.24 | 18.8 / 28.48 |
| JPEG XL | 0.8 / 25.29 | 15.5 / 26.10 |
| AVIF | 25.8 / 28.02 (12.1 KB) | 36.7 / 29.09 (16.0 KB) |
| WebP | cannot get that small | cannot get that small |

AVIF wins clearly on both metrics; by eye its edges and structure are crisper (bikes, window frames), tiny wins only on
foliage texture. Earlier comparisons were against JPEG XL only, which is weak at these rates. Part of tiny's budget
was not picture: the grain fingerprint was 1.3-1.5 KB a photo (~11% of a q1 file) and swatches ~1 KB.

Compact grain fingerprint: the 32 x 32 grain spectrum per channel stored as 8 x 8 cell means of its log (rebuilt by
wrap-around bilinear interpolation, kept point-symmetric), 17 quantiles instead of 65, strengths as u8 on a square-root
scale: 1.38-1.55 KB -> 0.31-0.32 KB packed. Regrown grain: same rms per channel to 0.01 level, amplified grain fields
look the same. Old fingerprints (fixed 2139 bytes raw) still decode: a grain-mode file made by the old build unpacks
to the identical PNG. Grain-mode PNGs shrink by the same ~1.2 KB.

After the compact fingerprint (browser files again, Kodak 8): q1 12.73 -> 11.19 KB, q2 16.58 -> 15.23 KB. At those sizes,
SSIMULACRA2 / PSNR: q1 tiny smooth 0.7 / 27.24, textured -5.4 / 26.64, AVIF 19.9 / 27.52, JPEG XL -5.4 / 21.45;
q2 tiny smooth 18.8 / 28.48, textured 13.4 / 28.09, AVIF 31.9 / 28.61. The PSNR gap to AVIF is now 0.28 / 0.13 dB.
Where it is: at equal latent bytes the teacher's own decoder beats AVIF by 0.8 dB PSNR (q1 27.72 vs 26.94, q2 29.12
vs 28.28) and loses on SSIMULACRA2 (3.9 vs 11.7, 23.2 vs 28.4); our student is ~0.5 dB below the teacher. Queued on
the rig (behind the qcompress jobs): q0 (teacher fine-tuned to lambda 0.0006 + predictor + student) and a wider q1
student (v3: 128/128/128/96/64).
Decode-side sharpening (unsharp mask, sigma 0.8 / 1.5, amount 0.3-1.0) on tiny q1: SSIMULACRA2 0.7 -> 0.8 at best
(sigma 0.8, 0.3), worse above, PSNR always down. AVIF's lead on that metric is not edge softness; rejected.

### q0: the teacher fine-tuned to a lower rate (2026-10-24, bench/wonder/q0train.py)

cheng2020-anchor q1 fine-tuned 20k steps (batch 16, 256 px DIV2K crops, lambda 0.0006 vs q1's 0.0018, its own RD loss;
43 min): Kodak 8 27.72 dB at 0.161 bpp (own rate) -> 25.17 dB at 0.068 bpp (stable from 5k steps). Our predictor
(15k steps) codes those latents in 2.4-8.0 KB (4.76 KB mean): 44% above the teacher's own rate (q1: 22%). Teacher
decode at ~5.7 KB total (latents + compact grain + ~0.6 KB swatches): SSIMULACRA2 -32.5 / 25.17 dB; AVIF at the same
bytes -14.3 / 25.26. By eye q0 is a soft painting (forest a blur), AVIF blocky but more detail; swatches put texture
back on q0's foliage and tyres but cost 1.4-2.2 KB at this size (up to 27% of the file). Not shipped. The q0 student
run died of a CUDA OOM next to a second qcompress queue worker (capped at 60% of the GPU); re-queued at batch 8.
Note: the rig's CompressAI saves EntropyBottleneck keys as matrices.N / biases.N / factors.N; the Mac's expects
_matrixN / _biasN / _factorN (renamed for the CPU run).
