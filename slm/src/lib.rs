//! slm — a from-scratch, streaming forward pass for GPT-S3-3M (Axiomic Labs), the 3M-parameter
//! hybrid Gated-DeltaNet-2 / gated-GQA language model with Full Attention Residuals.
//!
//! Built for a compressor, so two things matter: speed, and the same bits on every machine.
//! - Every matrix is int8 per row; activations are quantised to int8 per vector; products are
//!   accumulated in i32, which is exact, so NEON `sdot`, WASM simd128 and the scalar loop all
//!   produce identical numbers. Measured cost on Nardo docs: 1.8688 -> 1.8710 bits per byte.
//! - All f32 work goes through `simd::F8`: eight lanes, lane-wise IEEE ops, no fused multiply-add,
//!   fixed reduction order. No libm: exp and ln are our own; rope tables ship with the weights.
#![allow(clippy::needless_range_loop)]

pub mod simd;
pub mod pool;
pub mod bridge;
pub mod bpe;
pub mod codec;
use simd::{F8, fexp, fln, dot, dots, axpy};
use pool::{Pool, Shelf, split};

pub const V: usize = 4096;
pub const H: usize = 144;        // hidden
pub const NL: usize = 8;         // layers; layers 3 and 7 are attention, the rest GDN-2
pub const HD: usize = 48;        // head dim (both mixers)
pub const NH: usize = 3;         // heads
pub const FF: usize = 352;       // MLP width
pub const CONV: usize = 4;
pub const ROT: usize = 24;       // rotary dims (12 interleaved pairs)
pub const CTX: usize = 1024;
const RMS_EPS: f32 = 1e-6;
const GDN_EPS: f32 = 1e-5;
const L2_EPS: f32 = 1e-6;
const ATTN_RES_EPS: f32 = 1e-6;
pub const fn is_attn(l: usize) -> bool { (l + 1) % 4 == 0 }

/// Section timers (feature `prof`): zero cost when off.
pub mod prof {
    pub const NAMES: [&str; 8] = ["gdn_head", "attn_head", "o_proj", "serial", "mlp_gu", "mlp_down", "lm_head", "log_prob"];
    #[cfg(feature = "prof")] pub static ACC: [std::sync::atomic::AtomicU64; 8] = [const { std::sync::atomic::AtomicU64::new(0) }; 8];
    #[cfg(feature = "prof")] pub static CNT: [std::sync::atomic::AtomicU64; 8] = [const { std::sync::atomic::AtomicU64::new(0) }; 8];
    #[inline(always)] pub fn t<R>(i: usize, f: impl FnOnce() -> R) -> R {
        #[cfg(feature = "prof")] { let t0 = std::time::Instant::now(); let r = f(); ACC[i].fetch_add(t0.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed); CNT[i].fetch_add(1, std::sync::atomic::Ordering::Relaxed); r }
        #[cfg(not(feature = "prof"))] { let _ = i; f() }
    }
    pub fn report() {
        #[cfg(feature = "prof")] {
            let tot: u64 = ACC[..7].iter().map(|a| a.load(std::sync::atomic::Ordering::Relaxed)).sum();
            for i in 0..NAMES.len() { let a = ACC[i].load(std::sync::atomic::Ordering::Relaxed); let c = CNT[i].load(std::sync::atomic::Ordering::Relaxed); if c > 0 { eprintln!("{:>12} {:6.1}%  {:8.0} ns/call  x{}", NAMES[i], 100.0 * a as f64 / tot as f64, a as f64 / c as f64, c); } }
        }
    }
}

// ---------------------------------------------------------------------------
// small maths
// ---------------------------------------------------------------------------
#[inline] fn rms(x: &[f32], eps: f32) -> f32 { 1.0 / (dot(x, x) / x.len() as f32 + eps).sqrt() }
#[inline] fn rmsnorm(x: &[f32], w: &[f32], out: &mut [f32]) { simd::scale_mul(x, rms(x, RMS_EPS), w, out); }
#[inline] fn offset_rmsnorm(x: &mut [f32], w1: &[f32] /* 1 + w */) { let r = rms(x, RMS_EPS); let mut t = [0f32; HD]; simd::scale_mul(x, r, w1, &mut t); x.copy_from_slice(&t); }
#[inline] fn l2norm(x: &mut [f32]) { let r = 1.0 / (dot(x, x) + L2_EPS).sqrt(); let rv = F8::splat(r); let mut i = 0; while i < x.len() { F8::load(&x[i..]).mul(rv).store(&mut x[i..]); i += 8; } }

// ---------------------------------------------------------------------------
// int8 matrices
// ---------------------------------------------------------------------------
/// [n_out][n_in] int8, one scale per row. n_in must be a multiple of 16.
pub struct QMat { pub n_in: usize, pub n_out: usize, pub w: Vec<i8>, pub scale: Vec<f32> }
impl QMat {
    pub fn from_f32(w: &[f32], n_in: usize) -> QMat {
        assert!(n_in % 16 == 0 && w.len() % n_in == 0);
        let n_out = w.len() / n_in;
        let mut q = vec![0i8; w.len()]; let mut scale = vec![0f32; n_out];
        for (r, row) in w.chunks_exact(n_in).enumerate() { scale[r] = simd::quantize(row, &mut q[r * n_in..(r + 1) * n_in]); }
        QMat { n_in, n_out, w: q, scale }
    }
    /// Stack several [*, n_in] matrices into one (one pass over the input, better streaming).
    pub fn concat(parts: &[&[f32]], n_in: usize) -> QMat {
        let mut all = Vec::new(); for p in parts { all.extend_from_slice(p); }
        QMat::from_f32(&all, n_in)
    }
    /// Row r dequantised (used for the tied embedding lookup).
    pub fn row_f32(&self, r: usize, out: &mut [f32]) { let s = self.scale[r]; for i in 0..self.n_in { out[i] = self.w[r * self.n_in + i] as f32 * s; } }
}

/// An activation vector quantised to int8 with one scale.
pub struct QVec { pub q: [i8; FF], pub n: usize, pub scale: f32 }
impl QVec {
    pub fn zero() -> QVec { QVec { q: [0; FF], n: 0, scale: 1.0 } }
    #[inline] pub fn set(&mut self, x: &[f32]) { self.n = x.len(); self.scale = simd::quantize(x, &mut self.q[..x.len()]); }
}

/// out[r] = (sum_i w[r][i] * x[i]) * (x.scale * w.scale[r]); the i32 sum is exact in every path.
#[inline] pub fn qmatvec(m: &QMat, x: &QVec, out: &mut [f32]) { qmatvec_rows(m, x, 0, m.n_out, out) }
/// Rows [r0, r1) of the product into out[0..r1-r0]. Each row's result is independent of who computes it.
#[inline] pub fn qmatvec_rows(m: &QMat, x: &QVec, r0: usize, r1: usize, out: &mut [f32]) {
    debug_assert_eq!(x.n, m.n_in); debug_assert!(r1 <= m.n_out && out.len() >= r1 - r0);
    #[cfg(all(target_arch = "aarch64", not(feature = "scalar")))]
    {
        if neon_dotprod() { unsafe { qmatvec_neon(m, x, r0, r1, out) }; return; }
    }
    qmatvec_scalar(m, x, r0, r1, out)
}
fn qmatvec_scalar(m: &QMat, x: &QVec, r0: usize, r1: usize, out: &mut [f32]) {
    let n = m.n_in;
    for r in r0..r1 {
        let row = &m.w[r * n..(r + 1) * n];
        let mut acc = [0i32; 4];
        for (c, xc) in row.chunks_exact(4).zip(x.q[..n].chunks_exact(4)) { for l in 0..4 { acc[l] += c[l] as i32 * xc[l] as i32; } }
        out[r - r0] = ((acc[0] + acc[1]) + (acc[2] + acc[3])) as f32 * (x.scale * m.scale[r]);
    }
}
#[cfg(all(target_arch = "aarch64", not(feature = "scalar")))]
fn neon_dotprod() -> bool {
    static F: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *F.get_or_init(|| std::arch::is_aarch64_feature_detected!("dotprod"))
}
/// `sdot`: 16 int8 products summed into 4 int32 lanes. The intrinsic is nightly-only, the instruction is not.
#[cfg(all(target_arch = "aarch64", not(feature = "scalar")))]
#[inline(always)]
pub(crate) unsafe fn sdot(acc: std::arch::aarch64::int32x4_t, a: std::arch::aarch64::int8x16_t, b: std::arch::aarch64::int8x16_t) -> std::arch::aarch64::int32x4_t {
    let mut r = acc;
    std::arch::asm!("sdot {r:v}.4s, {a:v}.16b, {b:v}.16b", r = inout(vreg) r, a = in(vreg) a, b = in(vreg) b, options(pure, nomem, nostack));
    r
}
#[cfg(all(target_arch = "aarch64", not(feature = "scalar")))]
#[target_feature(enable = "neon,dotprod")]
unsafe fn qmatvec_neon(m: &QMat, x: &QVec, r0: usize, r1: usize, out: &mut [f32]) {
    use std::arch::aarch64::*;
    let n = m.n_in; let xs = x.q.as_ptr(); let w = m.w.as_ptr(); let xsc = x.scale;
    let out = out.as_mut_ptr().sub(r0);   // so out[r] addresses row r
    let mut r = r0;
    // eight rows in flight: eight independent sdot chains hide the ~3-cycle sdot latency
    while r + 8 <= r1 {
        let base = w.add(r * n);
        let mut acc = [vdupq_n_s32(0); 8];
        let mut i = 0;
        while i < n {
            let xv = vld1q_s8(xs.add(i));
            for l in 0..8 { acc[l] = sdot(acc[l], vld1q_s8(base.add(l * n + i)), xv); }
            i += 16;
        }
        for l in 0..8 { *out.add(r + l) = vaddvq_s32(acc[l]) as f32 * (xsc * m.scale[r + l]); }
        r += 8;
    }
    while r < r1 {
        let w0 = w.add(r * n); let mut a0 = vdupq_n_s32(0); let mut i = 0;
        while i < n { a0 = sdot(a0, vld1q_s8(w0.add(i)), vld1q_s8(xs.add(i))); i += 16; }
        *out.add(r) = vaddvq_s32(a0) as f32 * (xsc * m.scale[r]);
        r += 1;
    }
}

// ---------------------------------------------------------------------------
// weights
// ---------------------------------------------------------------------------
pub struct AttnRes { pub q: Vec<f32> /* query * key_norm_weight */ }
pub struct Gdn {
    pub proj: QMat,   // head-major rows: for each head [q k v f b w g] x HD  (7*HD = 336 rows per head)
    pub g_bias: Vec<f32>, pub conv_q: Vec<f32>, pub conv_k: Vec<f32>, pub conv_v: Vec<f32>,   // conv: [CONV][H] (tap-major)
    pub a_log: Vec<f32>, pub dt_bias: Vec<f32>, pub o_norm: Vec<f32>, pub o: QMat,
}
pub struct Attn { pub proj: QMat /* rows: q_gate(2*NH*HD) k(HD) v(HD) */, pub o: QMat, pub q_norm1: Vec<f32>, pub k_norm1: Vec<f32> /* 1 + weight */ }
pub enum Mixer { Gdn(Gdn), Attn(Attn) }
pub struct Layer { pub ln1: Vec<f32>, pub ln2: Vec<f32>, pub res1: AttnRes, pub res2: AttnRes, pub mixer: Mixer, pub gu: QMat /* gate(FF) up(FF) */, pub down: QMat }
pub struct Weights { pub wte: QMat, pub layers: Vec<Layer>, pub ln_f: Vec<f32>, pub res_f: AttnRes, pub rope_cos: Vec<f32>, pub rope_sin: Vec<f32>, pub rope_ccos: Vec<f32>, pub rope_csin: Vec<f32> }

impl Weights {
    /// cos/sin of pos*theta_i for the 12 rotary pairs, any position < 2^20: angle = coarse(pos>>10) + fine(pos&1023).
    /// For pos < 1024 the coarse part is (1, 0) and the result equals the fine table bit for bit.
    pub fn rope(&self, pos: usize, cos: &mut [f32; ROT / 2], sin: &mut [f32; ROT / 2]) {
        let (c, f) = ((pos >> 10) * ROT / 2, (pos & 1023) * ROT / 2);
        for i in 0..ROT / 2 {
            let (ca, sa, cb, sb) = (self.rope_ccos[c + i], self.rope_csin[c + i], self.rope_cos[f + i], self.rope_sin[f + i]);
            cos[i] = ca * cb - sa * sb; sin[i] = sa * cb + ca * sb;
        }
    }
    /// `manifest`: lines "name offset count shape"; `blob`: little-endian f32.
    pub fn load(manifest: &str, blob: &[u8]) -> Weights {
        let mut map = std::collections::HashMap::new();
        for line in manifest.lines() {
            let p: Vec<&str> = line.split_whitespace().collect();
            if p.len() >= 3 { map.insert(p[0].to_string(), (p[1].parse::<usize>().unwrap(), p[2].parse::<usize>().unwrap())); }
        }
        let get = |name: &str| -> Vec<f32> {
            let (off, n) = *map.get(name).unwrap_or_else(|| panic!("missing tensor {name}"));
            blob[off..off + n * 4].chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect()
        };
        let res = |p: &str| { let q = get(&format!("{p}.query")); let k = get(&format!("{p}.key_norm_weight")); AttnRes { q: q.iter().zip(&k).map(|(a, b)| a * b).collect() } };
        let plus1 = |v: Vec<f32>| -> Vec<f32> { v.iter().map(|x| 1.0 + x).collect() };
        // PyTorch conv weight is [H][1][CONV] (channel-major); store tap-major [CONV][H] so each tap is a vector op
        let conv = |name: &str| -> Vec<f32> { let w = get(name); let mut t = vec![0f32; CONV * H]; for c in 0..H { for j in 0..CONV { t[j * H + c] = w[c * CONV + j]; } } t };
        let mut layers = Vec::new();
        for l in 0..NL {
            let p = format!("transformer.h.{l}");
            let mixer = if is_attn(l) {
                Mixer::Attn(Attn {
                    proj: QMat::concat(&[&get(&format!("{p}.attn.q_gate_proj.weight")), &get(&format!("{p}.attn.k_proj.weight")), &get(&format!("{p}.attn.v_proj.weight"))], H),
                    o: QMat::from_f32(&get(&format!("{p}.attn.o_proj.weight")), NH * HD),
                    q_norm1: plus1(get(&format!("{p}.attn.q_norm.weight"))), k_norm1: plus1(get(&format!("{p}.attn.k_norm.weight"))) })
            } else {
                let k = format!("{p}.attn.kernel");
                let parts = ["q", "k", "v", "f", "b", "w", "g"].map(|n| get(&format!("{k}.{n}_proj.weight")));
                let mut hm = Vec::with_capacity(7 * H * H);
                for hh in 0..NH { for p in &parts { hm.extend_from_slice(&p[hh * HD * H..(hh + 1) * HD * H]); } }
                Mixer::Gdn(Gdn {
                    proj: QMat::from_f32(&hm, H),
                    g_bias: get(&format!("{k}.g_proj.bias")),
                    conv_q: conv(&format!("{k}.q_conv1d.weight")), conv_k: conv(&format!("{k}.k_conv1d.weight")), conv_v: conv(&format!("{k}.v_conv1d.weight")),
                    a_log: get(&format!("{k}.A_log")), dt_bias: get(&format!("{k}.dt_bias")), o_norm: get(&format!("{k}.o_norm.weight")),
                    o: QMat::from_f32(&get(&format!("{k}.o_proj.weight")), NH * HD) })
            };
            layers.push(Layer { ln1: get(&format!("{p}.ln_1.weight")), ln2: get(&format!("{p}.ln_2.weight")), res1: res(&format!("{p}.attn_res_1")), res2: res(&format!("{p}.attn_res_2")), mixer,
                gu: QMat::concat(&[&get(&format!("{p}.mlp.w_gate.weight")), &get(&format!("{p}.mlp.w_up.weight"))], H), down: QMat::from_f32(&get(&format!("{p}.mlp.w_down.weight")), FF) });
        }
        Weights { wte: QMat::from_f32(&get("transformer.wte.weight"), H), layers, ln_f: get("transformer.ln_f.weight"), res_f: res("transformer.attn_res_f"), rope_cos: get("rope.cos"), rope_sin: get("rope.sin"), rope_ccos: get("rope.ccos"), rope_csin: get("rope.csin") }
    }
}

// ---------------------------------------------------------------------------
// streaming state
// ---------------------------------------------------------------------------
pub const GP: usize = 7 * HD;   // GDN projection rows per head
/// Streaming forgetting: the model was trained on 1024-token windows and its recurrent state drifts
/// when carried for longer (enwik8 1MB: 1.645 bpb carried forever vs 1.534 windowed). A constant extra
/// decay per token on every GDN state fixes most of that (1.541) without resets or recompute.
/// Tuned on Nardo docs, checked held-out on enwik8; flat between 0.999 and 0.9993. SLM_GDN_DECAY overrides.
pub const EXTRA_DECAY: f32 = 0.999;
fn extra_decay() -> f32 { static D: std::sync::OnceLock<f32> = std::sync::OnceLock::new(); *D.get_or_init(|| std::env::var("SLM_GDN_DECAY").ok().and_then(|v| v.parse().ok()).unwrap_or(EXTRA_DECAY)) }
struct GdnState { conv: Vec<f32> /* [3 inputs][CONV-1 taps][H] */, s: Vec<f32> /* [NH][HD][HD] */ }
/// One head's own copy of the attention cache (every thread appends the same k, v: no sharing needed).
struct AttnHead { kq: Vec<i8>, ks: Vec<f32>, vc: Vec<f32>, total: usize }   // CTX slots; slot = total % CTX; live = min(total, CTX)
enum MixState { Gdn(GdnState), Attn(Vec<AttnHead>) }
/// Per-head scratch owned by whichever thread runs that head.
struct HeadScratch { p: Vec<f32>, sc: Vec<f32> }

pub struct Model<'a> {
    w: &'a Weights,
    st: Vec<MixState>,
    pos: usize,
    pool: Pool,
    src: Vec<[f32; H]>, src_r: Vec<f32>,
    xq: QVec,
    heads: Vec<HeadScratch>,
    mix: Vec<f32>, y: Vec<f32>, gu: Vec<f32>,
    attn_win: usize,
    pub logits: Vec<f32>,
}

impl<'a> Model<'a> {
    pub fn new(w: &'a Weights) -> Model<'a> { Model::with_threads(w, std::env::var("SLM_THREADS").ok().and_then(|v| v.parse().ok()).unwrap_or(NH)) }
    pub fn with_threads(w: &'a Weights, threads: usize) -> Model<'a> {
        let mut m = Model { w, st: Vec::new(), pos: 0, pool: Pool::new(threads), src: Vec::with_capacity(2 * NL + 1), src_r: Vec::with_capacity(2 * NL + 1), xq: QVec::zero(),
            heads: (0..NH).map(|_| HeadScratch { p: vec![0f32; GP], sc: vec![0f32; CTX + 8] }).collect(),
            mix: vec![0f32; H], y: vec![0f32; H], gu: vec![0f32; 2 * FF], attn_win: CTX, logits: vec![0f32; V] };
        m.reset();
        m
    }
    pub fn threads(&self) -> usize { self.pool.threads() }
    /// Forget everything (start of a new context window).
    pub fn reset(&mut self) {
        self.st.clear(); self.pos = 0;
        for l in 0..NL {
            self.st.push(if is_attn(l) { MixState::Attn((0..NH).map(|_| AttnHead { kq: vec![0; CTX * HD], ks: vec![0f32; CTX], vc: vec![0f32; CTX * HD], total: 0 }).collect()) }
                else { MixState::Gdn(GdnState { conv: vec![0f32; 3 * (CONV - 1) * H], s: vec![0f32; NH * HD * HD] }) });
        }
    }
    pub fn pos(&self) -> usize { self.pos }
    /// Experiment knobs: forget only the recurrent (GDN) state, or only the attention cache.
    /// Forget the recurrent state of one GDN layer only.
    pub fn reset_gdn_layer(&mut self, l: usize) { if let MixState::Gdn(g) = &mut self.st[l] { g.conv.iter_mut().for_each(|v| *v = 0.0); g.s.iter_mut().for_each(|v| *v = 0.0); } }
    /// Experiment: shrink every GDN state by `f` instead of zeroing it.
    pub fn soften_gdn(&mut self, f: f32) { for st in self.st.iter_mut() { if let MixState::Gdn(g) = st { g.s.iter_mut().for_each(|v| *v *= f); } } }
    pub fn reset_gdn(&mut self) { for st in self.st.iter_mut() { if let MixState::Gdn(g) = st { g.conv.iter_mut().for_each(|v| *v = 0.0); g.s.iter_mut().for_each(|v| *v = 0.0); } } }
    pub fn reset_attn(&mut self) { for st in self.st.iter_mut() { if let MixState::Attn(hs) = st { for h in hs { h.total = 0; } } } }
    pub fn set_attn_window(&mut self, w: usize) { self.attn_win = w.clamp(1, CTX); }

    fn attn_res(&self, r: &AttnRes, out: &mut [f32; H]) {
        let n = self.src.len();
        let mut lg = [0f32; 2 * NL + 1];
        let flat: &[f32] = unsafe { std::slice::from_raw_parts(self.src.as_ptr() as *const f32, n * H) };
        dots(&r.q, flat, 1.0, &mut lg[..n]);
        let mut mx = f32::NEG_INFINITY;
        for s in 0..n { lg[s] *= self.src_r[s]; if lg[s] > mx { mx = lg[s]; } }
        let mut sum = 0f32; for s in 0..n { lg[s] = fexp(lg[s] - mx); sum += lg[s]; }
        let inv = 1.0 / sum;
        *out = [0f32; H];
        for s in 0..n { axpy(out, lg[s] * inv, &self.src[s]); }
    }
    fn push_src(&mut self, v: &[f32]) { let mut a = [0f32; H]; a.copy_from_slice(v); self.src_r.push(rms(&a, ATTN_RES_EPS)); self.src.push(a); }

    /// Feed one token; `self.logits` then holds the next-token logits. Panics past CTX tokens: reset first.
    pub fn step(&mut self, tok: u32) {
        let w = self.w; let t = self.pool.threads(); let pos = self.pos;
        self.src.clear(); self.src_r.clear();
        let mut x = [0f32; H]; w.wte.row_f32(tok as usize, &mut x);
        self.push_src(&x);
        let mut h = [0f32; H]; let mut hn = [0f32; H];
        let (mix, y, gu, logits) = (Shelf::new(&mut self.mix), Shelf::new(&mut self.y), Shelf::new(&mut self.gu), Shelf::new(&mut self.logits));
        let heads = Shelf::new(&mut self.heads);
        for l in 0..NL {
            let lw = &w.layers[l];
            prof::t(3, || { self.attn_res(&lw.res1, &mut h); rmsnorm(&h, &lw.ln1, &mut hn); self.xq.set(&hn); });
            let xq = &self.xq;
            // phase A: one head per thread (threads beyond NH idle here)
            match (&lw.mixer, &mut self.st[l]) {
                (Mixer::Gdn(g), MixState::Gdn(s)) => {
                    let (conv, st) = (Shelf::new(&mut s.conv), Shelf::new(&mut s.s));
                    prof::t(0, || self.pool.run(&|tid| { let mut hh = tid; while hh < NH { unsafe { gdn_head(g, hh, xq, conv, st, &mut heads.range(hh, hh + 1)[0], mix.range(hh * HD, (hh + 1) * HD)) }; hh += t; } }));
                }
                (Mixer::Attn(a), MixState::Attn(hs)) => {
                    let hs = Shelf::new(hs);
                    let (mut rc, mut rs) = ([0f32; ROT / 2], [0f32; ROT / 2]); w.rope(pos, &mut rc, &mut rs);
                    let win = self.attn_win;
                    prof::t(1, || self.pool.run(&|tid| { let mut hh = tid; while hh < NH { unsafe { attn_head(a, hh, xq, &rc, &rs, win, &mut hs.range(hh, hh + 1)[0], &mut heads.range(hh, hh + 1)[0], mix.range(hh * HD, (hh + 1) * HD)) }; hh += t; } }));
                }
                _ => unreachable!(),
            }
            // phase B: output projection, rows split
            let o = match &lw.mixer { Mixer::Gdn(g) => &g.o, Mixer::Attn(a) => &a.o };
            prof::t(2, || self.pool.run(&|tid| { let mut q = QVec::zero(); q.set(mix.all()); let (a, b) = split(H, t, tid, 8); if a < b { qmatvec_rows(o, &q, a, b, unsafe { y.range(a, b) }); } }));
            prof::t(3, || { let yv: [f32; H] = y.all().try_into().unwrap(); self.push_src(&yv); self.attn_res(&lw.res2, &mut h); rmsnorm(&h, &lw.ln2, &mut hn); self.xq.set(&hn); });
            let xq = &self.xq;
            // phase C: gate/up rows split over the FF axis, silu(g)*u for the same range
            prof::t(4, || self.pool.run(&|tid| {
                let (a, b) = split(FF, t, tid, 8); if a >= b { return; }
                let g = unsafe { gu.range(a, b) }; qmatvec_rows(&lw.gu, xq, a, b, g);
                let u = unsafe { gu.range(FF + a, FF + b) }; qmatvec_rows(&lw.gu, xq, FF + a, FF + b, u);
                simd::silu_inplace(g);
                let mut i = 0; while i < b - a { F8::load(&g[i..]).mul(F8::load(&u[i..])).store(&mut g[i..]); i += 8; }
            }));
            // phase D: down projection, rows split
            prof::t(5, || self.pool.run(&|tid| { let mut q = QVec::zero(); q.set(&gu.all()[..FF]); let (a, b) = split(H, t, tid, 8); if a < b { qmatvec_rows(&lw.down, &q, a, b, unsafe { y.range(a, b) }); } }));
            let yv: [f32; H] = y.all().try_into().unwrap(); self.push_src(&yv);
        }
        prof::t(3, || { self.attn_res(&w.res_f, &mut h); rmsnorm(&h, &w.ln_f, &mut hn); self.xq.set(&hn); });
        let xq = &self.xq;
        prof::t(6, || self.pool.run(&|tid| { let (a, b) = split(V, t, tid, 8); if a < b { qmatvec_rows(&w.wte, xq, a, b, unsafe { logits.range(a, b) }); } }));
        self.pos += 1;
    }
}

/// depthwise causal conv over [x_{t-3}, x_{t-2}, x_{t-1}, x_t] for channels [r, r+HD), taps cw[j*H + c], then SiLU.
/// `state` is this input's [CONV-1][H] history (only the head's channels are touched).
#[inline] fn short_conv(cw: &[f32], state: &mut [f32], r: usize, x: &[f32], y: &mut [f32]) {
    let mut c = 0;
    while c < HD {
        let mut s = F8::load(&cw[3 * H + r + c..]).mul(F8::load(&x[c..]));
        for j in 0..CONV - 1 { s = s.add(F8::load(&cw[j * H + r + c..]).mul(F8::load(&state[j * H + r + c..]))); }
        s.store(&mut y[c..]);
        c += 8;
    }
    simd::silu_inplace(y);
    for j in 0..CONV - 2 { let (lo, hi) = state.split_at_mut((j + 1) * H); lo[j * H + r..j * H + r + HD].copy_from_slice(&hi[r..r + HD]); }
    state[(CONV - 2) * H + r..(CONV - 2) * H + r + HD].copy_from_slice(x);
}

/// One GDN head for one token: projections for this head, conv, gates, the delta-rule state update, gated norm -> out[0..HD].
unsafe fn gdn_head(g: &Gdn, hh: usize, x: &QVec, conv: Shelf<f32>, st: Shelf<f32>, hs: &mut HeadScratch, out: &mut [f32]) {
    let r = hh * HD;
    let p = &mut hs.p[..];
    qmatvec_rows(&g.proj, x, hh * GP, (hh + 1) * GP, p);
    let (qi, rest) = p.split_at_mut(HD); let (ki, rest) = rest.split_at_mut(HD); let (vi, rest) = rest.split_at_mut(HD);
    let (gg, rest) = rest.split_at_mut(HD); let (bb, rest) = rest.split_at_mut(HD); let (ww, gate) = rest.split_at_mut(HD);
    let (mut q, mut k, mut v) = ([0f32; HD], [0f32; HD], [0f32; HD]);
    let cs = conv.range(0, 3 * (CONV - 1) * H);
    let (cq, rest) = cs.split_at_mut((CONV - 1) * H); let (ck, cv) = rest.split_at_mut((CONV - 1) * H);
    short_conv(&g.conv_q, cq, r, qi, &mut q); short_conv(&g.conv_k, ck, r, ki, &mut k); short_conv(&g.conv_v, cv, r, vi, &mut v);
    l2norm(&mut q); l2norm(&mut k);
    let (mut sb, mut sw, mut sg) = ([0f32; HD], [0f32; HD], [0f32; HD]);
    simd::sigmoid_into(bb, &mut sb); simd::sigmoid_into(ww, &mut sw);
    { let mut i = 0; while i < HD { F8::load(&gate[i..]).add(F8::load(&g.g_bias[r + i..])).store(&mut gate[i..]); i += 8; } }
    simd::sigmoid_into(gate, &mut sg);
    let decay = F8::splat(-fexp(g.a_log[hh]));
    let extra = F8::splat(extra_decay());
    let scale = 1.0 / (HD as f32).sqrt();
    let s = st.range(hh * HD * HD, (hh + 1) * HD * HD);     // [k][v]
    // state *= exp(g) per k row; erase = (b*k)^T state; v_new = w*v - erase; state += k v_new^T; o = (q*scale)^T state
    let (mut dk, mut bk, mut wv) = ([0f32; HD], [0f32; HD], [0f32; HD]);
    for c in 0..HD / 8 {
        let i = c * 8;
        simd::exp8(decay.mul(simd::softplus8(F8::load(&gg[i..]).add(F8::load(&g.dt_bias[r + i..]))))).mul(extra).store(&mut dk[i..]);
        F8::load(&sb[i..]).mul(F8::load(&k[i..])).store(&mut bk[i..]);
        F8::load(&sw[i..]).mul(F8::load(&v[i..])).store(&mut wv[i..]);
    }
    let mut e = [F8::zero(); HD / 8];
    for kk in 0..HD {
        let (d, b) = (F8::splat(dk[kk]), F8::splat(bk[kk]));
        let row = &mut s[kk * HD..(kk + 1) * HD];
        for c in 0..HD / 8 { let rv = F8::load(&row[c * 8..]).mul(d); rv.store(&mut row[c * 8..]); e[c] = e[c].add(b.mul(rv)); }
    }
    let mut vn = [F8::zero(); HD / 8]; for c in 0..HD / 8 { vn[c] = F8::load(&wv[c * 8..]).sub(e[c]); }
    let mut oh = [F8::zero(); HD / 8];
    for kk in 0..HD {
        let (kv, qs) = (F8::splat(k[kk]), F8::splat(q[kk] * scale));
        let row = &mut s[kk * HD..(kk + 1) * HD];
        for c in 0..HD / 8 { let rv = F8::load(&row[c * 8..]).add(kv.mul(vn[c])); rv.store(&mut row[c * 8..]); oh[c] = oh[c].add(qs.mul(rv)); }
    }
    // gated RMSNorm per head
    let mut ohf = [0f32; HD]; for c in 0..HD / 8 { oh[c].store(&mut ohf[c * 8..]); }
    let rr = F8::splat(rms(&ohf, GDN_EPS));
    for c in 0..HD / 8 { oh[c].mul(rr).mul(F8::load(&g.o_norm[c * 8..])).mul(F8::load(&sg[c * 8..])).store(&mut out[c * 8..]); }
}

/// One attention head for one token. Every head recomputes k and v (96 rows, identical bits) and keeps its own cache copy.
unsafe fn attn_head(a: &Attn, hh: usize, x: &QVec, rc: &[f32; ROT / 2], rs: &[f32; ROT / 2], win: usize, st: &mut AttnHead, hs: &mut HeadScratch, out: &mut [f32]) {
    let p = &mut hs.p[..];
    let (qg, rest) = p.split_at_mut(2 * HD); let (k, rest) = rest.split_at_mut(HD); let v = &mut rest[..HD];
    qmatvec_rows(&a.proj, x, hh * 2 * HD, (hh + 1) * 2 * HD, qg);
    qmatvec_rows(&a.proj, x, 2 * NH * HD, 2 * NH * HD + HD, k);
    qmatvec_rows(&a.proj, x, 2 * NH * HD + HD, 2 * NH * HD + 2 * HD, v);
    let (q, gate) = qg.split_at_mut(HD);
    let rope = |t: &mut [f32]| { for i in 0..ROT / 2 { let (c, sn) = (rc[i], rs[i]); let (e, o) = (t[2 * i], t[2 * i + 1]); t[2 * i] = e * c - o * sn; t[2 * i + 1] = e * sn + o * c; } };
    offset_rmsnorm(k, &a.k_norm1); rope(k);
    let slot = st.total % win;
    st.ks[slot] = simd::quantize(k, &mut st.kq[slot * HD..(slot + 1) * HD]);
    st.vc[slot * HD..(slot + 1) * HD].copy_from_slice(v); st.total += 1;
    let n = st.total.min(win); let n8 = (n + 7) & !7;
    let sc = &mut hs.sc[..n8];
    offset_rmsnorm(q, &a.q_norm1); rope(q);
    let mut qq = [0i8; HD]; let sq = simd::quantize(q, &mut qq);
    let scale = 1.0 / (HD as f32).sqrt();
    simd::qdots(&qq, sq, &st.kq[..n * HD], &st.ks[..n], scale, &mut sc[..n]);
    for j in n..n8 { sc[j] = f32::NEG_INFINITY; }
    let mx = simd::max(sc);
    let inv = 1.0 / simd::exp_sub_sum(sc, mx);
    let mut acc = [0f32; HD];
    let mut j = 0;
    while j + 2 <= n { simd::axpy2(&mut acc, sc[j] * inv, &st.vc[j * HD..], sc[j + 1] * inv, &st.vc[(j + 1) * HD..]); j += 2; }
    if j < n { axpy(&mut acc, sc[j] * inv, &st.vc[j * HD..(j + 1) * HD]); }
    let mut gs = [0f32; HD]; simd::sigmoid_into(gate, &mut gs);
    for i in 0..HD { out[i] = acc[i] * gs[i]; }
}

/// log-softmax of the logits at index `t` (natural log). Leaves `logits` untouched.
pub fn log_prob(logits: &[f32], t: usize) -> f32 {
    let mx = simd::max(logits);
    let mv = F8::splat(mx); let mut s = F8::zero(); let mut i = 0;
    while i < logits.len() { s = s.add(simd::exp8(F8::load(&logits[i..]).sub(mv))); i += 8; }
    logits[t] - mx - fln(s.hsum())
}
