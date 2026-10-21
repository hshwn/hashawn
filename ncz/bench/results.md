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
