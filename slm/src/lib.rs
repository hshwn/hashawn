//! slm — a from-scratch, streaming forward pass for GPT-S3-3M (Axiomic Labs), the 3M-parameter
//! hybrid Gated-DeltaNet-2 / gated-GQA language model with Full Attention Residuals.
//!
//! Built for a compressor, so two things matter: speed, and the same bits on every machine.
//! - Every matrix is int8 per row; activations are quantised to int8 per vector; products are
//!   accumulated in i32, which is exact, so NEON `sdot`, WASM simd128 and the scalar loop all
//!   produce identical numbers. Measured cost on Nardo docs: 1.8688 -> 1.8708 bits per byte.
//! - No libm: exp and ln are our own polynomials; rope tables ship with the weights; sqrt and
//!   division are IEEE-exact. Rust never fuses multiply-adds, so f32 op order is the order written.
#![allow(clippy::needless_range_loop)]

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
    pub const NAMES: [&str; 12] = ["attn_res", "rmsnorm", "gdn_proj", "gdn_conv", "gdn_state", "gdn_o", "attn_proj", "attn_scores", "attn_o", "mlp", "lm_head", "log_prob"];
    #[cfg(feature = "prof")] pub static ACC: [std::sync::atomic::AtomicU64; 12] = [const { std::sync::atomic::AtomicU64::new(0) }; 12];
    #[cfg(feature = "prof")] pub static CNT: [std::sync::atomic::AtomicU64; 12] = [const { std::sync::atomic::AtomicU64::new(0) }; 12];
    #[inline(always)] pub fn t<R>(i: usize, f: impl FnOnce() -> R) -> R {
        #[cfg(feature = "prof")] { let t0 = std::time::Instant::now(); let r = f(); ACC[i].fetch_add(t0.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed); CNT[i].fetch_add(1, std::sync::atomic::Ordering::Relaxed); r }
        #[cfg(not(feature = "prof"))] { let _ = i; f() }
    }
    pub fn report() {
        #[cfg(feature = "prof")] {
            let tot: u64 = ACC.iter().map(|a| a.load(std::sync::atomic::Ordering::Relaxed)).sum();
            for i in 0..NAMES.len() { let a = ACC[i].load(std::sync::atomic::Ordering::Relaxed); let c = CNT[i].load(std::sync::atomic::Ordering::Relaxed); if c > 0 { eprintln!("{:>12} {:6.1}%  {:8.0} ns/call  x{}", NAMES[i], 100.0 * a as f64 / tot as f64, a as f64 / c as f64, c); } }
        }
    }
}

// ---------------------------------------------------------------------------
// deterministic maths (no libm)
// ---------------------------------------------------------------------------
const LOG2E: f32 = 1.442_695_f32;
const LN2_HI: f32 = 0.693_145_75_f32;      // ln2 split so k*LN2_HI is exact for |k| < 2^8
const LN2_LO: f32 = 1.428_606_8e-6_f32;
const LN2: f32 = 0.693_147_18_f32;

/// exp(x) to ~1 ulp, branch-free, vectorisable, identical on every target.
#[inline] pub fn fexp(x: f32) -> f32 {
    let x = x.clamp(-87.0, 88.0);
    let k = (x * LOG2E).round_ties_even();
    let r = (x - k * LN2_HI) - k * LN2_LO;           // |r| <= 0.347
    let p = 1.0 + r * (1.0 + r * (0.5 + r * (0.166_666_67 + r * (0.041_666_668 + r * (0.008_333_334 + r * 0.001_388_889)))));
    f32::from_bits(((k as i32 + 127) << 23) as u32) * p
}
/// ln(x) for x > 0, identical on every target. ~1e-7 relative.
#[inline] pub fn fln(x: f32) -> f32 {
    let bits = x.to_bits();
    let mut e = ((bits >> 23) & 0xff) as i32 - 127;
    let mut m = f32::from_bits((bits & 0x007f_ffff) | 0x3f80_0000);   // [1, 2)
    if m > 1.414_213_6 { m *= 0.5; e += 1; }                               // [0.707, 1.414)
    let t = (m - 1.0) / (m + 1.0);                                          // |t| <= 0.172
    let t2 = t * t;
    let s = t * (2.0 + t2 * (0.666_666_7 + t2 * (0.4 + t2 * (0.285_714_3 + t2 * (0.222_222_22 + t2 * 0.181_818_18)))));
    e as f32 * LN2 + s
}
#[inline] fn sigmoid(x: f32) -> f32 { 1.0 / (1.0 + fexp(-x)) }
#[inline] fn silu(x: f32) -> f32 { x * sigmoid(x) }
#[inline] fn softplus(x: f32) -> f32 { if x > 20.0 { x } else { fln(1.0 + fexp(x)) } }
#[inline] fn rms(x: &[f32], eps: f32) -> f32 { 1.0 / (sumsq(x) / x.len() as f32 + eps).sqrt() }
fn rmsnorm(x: &[f32], w: &[f32], out: &mut [f32]) { let r = rms(x, RMS_EPS); for i in 0..x.len() { out[i] = x[i] * r * w[i]; } }
fn offset_rmsnorm(x: &mut [f32], w: &[f32]) { let r = rms(x, RMS_EPS); for i in 0..x.len() { x[i] = x[i] * r * (1.0 + w[i]); } }
fn l2norm(x: &mut [f32]) { let r = 1.0 / (sumsq(x) + L2_EPS).sqrt(); for v in x.iter_mut() { *v *= r; } }

/// Eight fixed lanes: vectorisable, and the summation order is part of the format.
#[inline] pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len().min(b.len());
    let mut acc = [0f32; 8];
    let mut i = 0;
    while i + 8 <= n { for l in 0..8 { acc[l] += a[i + l] * b[i + l]; } i += 8; }
    let mut s = ((acc[0] + acc[4]) + (acc[1] + acc[5])) + ((acc[2] + acc[6]) + (acc[3] + acc[7]));
    while i < n { s += a[i] * b[i]; i += 1; }
    s
}
#[inline] fn sumsq(a: &[f32]) -> f32 { dot(a, a) }

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
        for (r, row) in w.chunks_exact(n_in).enumerate() {
            let amax = row.iter().fold(0f32, |m, &v| m.max(v.abs()));
            let sc = if amax == 0.0 { 1.0 } else { amax / 127.0 };
            let inv = 127.0 / amax.max(1e-30);
            scale[r] = sc;
            for (i, &v) in row.iter().enumerate() { q[r * n_in + i] = (v * inv).round_ties_even().clamp(-127.0, 127.0) as i8; }
        }
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
    #[inline] pub fn set(&mut self, x: &[f32]) {
        let n = x.len(); self.n = n;
        let amax = x.iter().fold(0f32, |m, &v| m.max(v.abs()));
        let inv = if amax == 0.0 { 0.0 } else { 127.0 / amax };
        self.scale = if amax == 0.0 { 0.0 } else { amax / 127.0 };
        for i in 0..n { self.q[i] = (x[i] * inv).round_ties_even() as i8; }
    }
}

/// out[r] = (sum_i w[r][i] * x[i]) * (x.scale * w.scale[r]); the i32 sum is exact in every path.
#[inline] pub fn qmatvec(m: &QMat, x: &QVec, out: &mut [f32]) {
    debug_assert_eq!(x.n, m.n_in);
    #[cfg(target_arch = "aarch64")]
    {
        if neon_dotprod() { unsafe { qmatvec_neon(m, x, out) }; return; }
    }
    qmatvec_scalar(m, x, out)
}
fn qmatvec_scalar(m: &QMat, x: &QVec, out: &mut [f32]) {
    let n = m.n_in;
    for (r, row) in m.w.chunks_exact(n).enumerate() {
        let mut acc = [0i32; 4];
        for (c, xc) in row.chunks_exact(4).zip(x.q[..n].chunks_exact(4)) { for l in 0..4 { acc[l] += c[l] as i32 * xc[l] as i32; } }
        out[r] = ((acc[0] + acc[1]) + (acc[2] + acc[3])) as f32 * (x.scale * m.scale[r]);
    }
}
#[cfg(target_arch = "aarch64")]
fn neon_dotprod() -> bool {
    static F: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *F.get_or_init(|| std::arch::is_aarch64_feature_detected!("dotprod") && std::env::var("SLM_SCALAR").is_err())
}
/// `sdot`: 16 int8 products summed into 4 int32 lanes. The intrinsic is nightly-only, the instruction is not.
#[cfg(target_arch = "aarch64")]
#[inline(always)]
unsafe fn sdot(acc: std::arch::aarch64::int32x4_t, a: std::arch::aarch64::int8x16_t, b: std::arch::aarch64::int8x16_t) -> std::arch::aarch64::int32x4_t {
    let mut r = acc;
    std::arch::asm!("sdot {r:v}.4s, {a:v}.16b, {b:v}.16b", r = inout(vreg) r, a = in(vreg) a, b = in(vreg) b, options(pure, nomem, nostack));
    r
}
#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon,dotprod")]
unsafe fn qmatvec_neon(m: &QMat, x: &QVec, out: &mut [f32]) {
    use std::arch::aarch64::*;
    let n = m.n_in; let xs = x.q.as_ptr(); let w = m.w.as_ptr();
    let mut r = 0;
    while r + 4 <= m.n_out {
        let (w0, w1, w2, w3) = (w.add(r * n), w.add((r + 1) * n), w.add((r + 2) * n), w.add((r + 3) * n));
        let (mut a0, mut a1, mut a2, mut a3) = (vdupq_n_s32(0), vdupq_n_s32(0), vdupq_n_s32(0), vdupq_n_s32(0));
        let mut i = 0;
        while i < n {
            let xv = vld1q_s8(xs.add(i));
            a0 = sdot(a0, vld1q_s8(w0.add(i)), xv);
            a1 = sdot(a1, vld1q_s8(w1.add(i)), xv);
            a2 = sdot(a2, vld1q_s8(w2.add(i)), xv);
            a3 = sdot(a3, vld1q_s8(w3.add(i)), xv);
            i += 16;
        }
        *out.get_unchecked_mut(r) = vaddvq_s32(a0) as f32 * (x.scale * m.scale[r]);
        *out.get_unchecked_mut(r + 1) = vaddvq_s32(a1) as f32 * (x.scale * m.scale[r + 1]);
        *out.get_unchecked_mut(r + 2) = vaddvq_s32(a2) as f32 * (x.scale * m.scale[r + 2]);
        *out.get_unchecked_mut(r + 3) = vaddvq_s32(a3) as f32 * (x.scale * m.scale[r + 3]);
        r += 4;
    }
    while r < m.n_out {
        let w0 = w.add(r * n); let mut a0 = vdupq_n_s32(0); let mut i = 0;
        while i < n { a0 = sdot(a0, vld1q_s8(w0.add(i)), vld1q_s8(xs.add(i))); i += 16; }
        *out.get_unchecked_mut(r) = vaddvq_s32(a0) as f32 * (x.scale * m.scale[r]);
        r += 1;
    }
}

// ---------------------------------------------------------------------------
// weights
// ---------------------------------------------------------------------------
pub struct AttnRes { pub q: Vec<f32> /* query * key_norm_weight */ }
pub struct Gdn {
    pub proj: QMat,   // rows: q(H) k(H) v(H) f(H) b(H) w(H) g(H)
    pub g_bias: Vec<f32>, pub conv_q: Vec<f32>, pub conv_k: Vec<f32>, pub conv_v: Vec<f32>,
    pub a_log: Vec<f32>, pub dt_bias: Vec<f32>, pub o_norm: Vec<f32>, pub o: QMat,
}
pub struct Attn { pub proj: QMat /* rows: q_gate(2*NH*HD) k(HD) v(HD) */, pub o: QMat, pub q_norm: Vec<f32>, pub k_norm: Vec<f32> }
pub enum Mixer { Gdn(Gdn), Attn(Attn) }
pub struct Layer { pub ln1: Vec<f32>, pub ln2: Vec<f32>, pub res1: AttnRes, pub res2: AttnRes, pub mixer: Mixer, pub gu: QMat /* gate(FF) up(FF) */, pub down: QMat }
pub struct Weights { pub wte: QMat, pub layers: Vec<Layer>, pub ln_f: Vec<f32>, pub res_f: AttnRes, pub rope_cos: Vec<f32>, pub rope_sin: Vec<f32> }

impl Weights {
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
        let mut layers = Vec::new();
        for l in 0..NL {
            let p = format!("transformer.h.{l}");
            let mixer = if is_attn(l) {
                Mixer::Attn(Attn {
                    proj: QMat::concat(&[&get(&format!("{p}.attn.q_gate_proj.weight")), &get(&format!("{p}.attn.k_proj.weight")), &get(&format!("{p}.attn.v_proj.weight"))], H),
                    o: QMat::from_f32(&get(&format!("{p}.attn.o_proj.weight")), NH * HD),
                    q_norm: get(&format!("{p}.attn.q_norm.weight")), k_norm: get(&format!("{p}.attn.k_norm.weight")) })
            } else {
                let k = format!("{p}.attn.kernel");
                let parts = ["q", "k", "v", "f", "b", "w", "g"].map(|n| get(&format!("{k}.{n}_proj.weight")));
                Mixer::Gdn(Gdn {
                    proj: QMat::concat(&parts.iter().map(|v| v.as_slice()).collect::<Vec<_>>(), H),
                    g_bias: get(&format!("{k}.g_proj.bias")),
                    conv_q: get(&format!("{k}.q_conv1d.weight")), conv_k: get(&format!("{k}.k_conv1d.weight")), conv_v: get(&format!("{k}.v_conv1d.weight")),
                    a_log: get(&format!("{k}.A_log")), dt_bias: get(&format!("{k}.dt_bias")), o_norm: get(&format!("{k}.o_norm.weight")),
                    o: QMat::from_f32(&get(&format!("{k}.o_proj.weight")), NH * HD) })
            };
            layers.push(Layer { ln1: get(&format!("{p}.ln_1.weight")), ln2: get(&format!("{p}.ln_2.weight")), res1: res(&format!("{p}.attn_res_1")), res2: res(&format!("{p}.attn_res_2")), mixer,
                gu: QMat::concat(&[&get(&format!("{p}.mlp.w_gate.weight")), &get(&format!("{p}.mlp.w_up.weight"))], H), down: QMat::from_f32(&get(&format!("{p}.mlp.w_down.weight")), FF) });
        }
        Weights { wte: QMat::from_f32(&get("transformer.wte.weight"), H), layers, ln_f: get("transformer.ln_f.weight"), res_f: res("transformer.attn_res_f"), rope_cos: get("rope.cos"), rope_sin: get("rope.sin") }
    }
}

// ---------------------------------------------------------------------------
// streaming state
// ---------------------------------------------------------------------------
struct GdnState { conv_q: [[f32; H]; CONV - 1], conv_k: [[f32; H]; CONV - 1], conv_v: [[f32; H]; CONV - 1], s: Vec<f32> /* NH*HD*HD */ }
struct AttnState { kc: Vec<f32>, vc: Vec<f32>, n: usize }   // caches [n][HD]
enum MixState { Gdn(GdnState), Attn(AttnState) }

pub struct Model<'a> {
    w: &'a Weights,
    st: Vec<MixState>,
    pos: usize,
    src: Vec<[f32; H]>, src_r: Vec<f32>,
    xq: QVec, sc: Vec<f32>,
    pub logits: Vec<f32>,
}

impl<'a> Model<'a> {
    pub fn new(w: &'a Weights) -> Model<'a> {
        let mut m = Model { w, st: Vec::new(), pos: 0, src: Vec::with_capacity(2 * NL + 1), src_r: Vec::with_capacity(2 * NL + 1), xq: QVec::zero(), sc: vec![0f32; CTX], logits: vec![0f32; V] };
        m.reset();
        m
    }
    /// Forget everything (start of a new context window).
    pub fn reset(&mut self) {
        self.st.clear(); self.pos = 0;
        for l in 0..NL {
            self.st.push(if is_attn(l) { MixState::Attn(AttnState { kc: Vec::with_capacity(CTX * HD), vc: Vec::with_capacity(CTX * HD), n: 0 }) }
                else { MixState::Gdn(GdnState { conv_q: [[0.0; H]; CONV - 1], conv_k: [[0.0; H]; CONV - 1], conv_v: [[0.0; H]; CONV - 1], s: vec![0f32; NH * HD * HD] }) });
        }
    }
    pub fn pos(&self) -> usize { self.pos }

    fn attn_res(&self, r: &AttnRes, out: &mut [f32; H]) {
        let n = self.src.len();
        let mut lg = [0f32; 2 * NL + 1];
        let mut mx = f32::NEG_INFINITY;
        for s in 0..n { lg[s] = dot(&self.src[s], &r.q) * self.src_r[s]; if lg[s] > mx { mx = lg[s]; } }
        let mut sum = 0f32; for s in 0..n { lg[s] = fexp(lg[s] - mx); sum += lg[s]; }
        let inv = 1.0 / sum;
        *out = [0f32; H];
        for s in 0..n { let wgt = lg[s] * inv; let src = &self.src[s]; for i in 0..H { out[i] += wgt * src[i]; } }
    }
    fn push_src(&mut self, v: [f32; H]) { self.src_r.push(rms(&v, ATTN_RES_EPS)); self.src.push(v); }

    /// Feed one token; `self.logits` then holds the next-token logits. Panics past CTX tokens: reset first.
    pub fn step(&mut self, tok: u32) {
        assert!(self.pos < CTX, "context full: call reset()");
        let w = self.w;
        self.src.clear(); self.src_r.clear();
        let mut x = [0f32; H]; w.wte.row_f32(tok as usize, &mut x);
        self.push_src(x);
        let mut h = [0f32; H]; let mut hn = [0f32; H];
        for l in 0..NL {
            let lw = &w.layers[l];
            prof::t(0, || self.attn_res(&lw.res1, &mut h));
            prof::t(1, || rmsnorm(&h, &lw.ln1, &mut hn));
            self.xq.set(&hn);
            let out = match (&lw.mixer, &mut self.st[l]) {
                (Mixer::Gdn(g), MixState::Gdn(s)) => gdn_step(g, s, &self.xq),
                (Mixer::Attn(a), MixState::Attn(s)) => attn_step(a, s, &self.xq, self.pos, &w.rope_cos, &w.rope_sin, &mut self.sc),
                _ => unreachable!(),
            };
            self.push_src(out);
            prof::t(0, || self.attn_res(&lw.res2, &mut h));
            prof::t(1, || rmsnorm(&h, &lw.ln2, &mut hn));
            self.xq.set(&hn);
            let y = prof::t(9, || {
                let mut gu = [0f32; 2 * FF]; qmatvec(&lw.gu, &self.xq, &mut gu);
                let (g, u) = gu.split_at_mut(FF);
                for i in 0..FF { g[i] = silu(g[i]) * u[i]; }
                let mut q = QVec::zero(); q.set(g);
                let mut y = [0f32; H]; qmatvec(&lw.down, &q, &mut y); y
            });
            self.push_src(y);
        }
        prof::t(0, || self.attn_res(&w.res_f, &mut h));
        prof::t(1, || rmsnorm(&h, &w.ln_f, &mut hn));
        self.xq.set(&hn);
        prof::t(10, || qmatvec(&w.wte, &self.xq, &mut self.logits));
        self.pos += 1;
    }
}

fn short_conv(cw: &[f32], state: &mut [[f32; H]; CONV - 1], x: &[f32]) -> [f32; H] {
    // depthwise causal conv over [x_{t-3}, x_{t-2}, x_{t-1}, x_t] with weights cw[c*4 + j], then SiLU
    let mut y = [0f32; H];
    for c in 0..H {
        let mut s = cw[c * CONV + CONV - 1] * x[c];
        for j in 0..CONV - 1 { s += cw[c * CONV + j] * state[j][c]; }
        y[c] = silu(s);
    }
    state[0] = state[1]; state[1] = state[2]; state[2].copy_from_slice(x);
    y
}

fn gdn_step(g: &Gdn, s: &mut GdnState, x: &QVec) -> [f32; H] {
    let mut p = [0f32; 7 * H];
    prof::t(2, || qmatvec(&g.proj, x, &mut p));
    let (qi, rest) = p.split_at(H); let (ki, rest) = rest.split_at(H); let (vi, rest) = rest.split_at(H);
    let (gg, rest) = rest.split_at(H); let (bb, rest) = rest.split_at(H); let (ww, gate) = rest.split_at(H);
    let (mut q, mut k, v) = prof::t(3, || (short_conv(&g.conv_q, &mut s.conv_q, qi), short_conv(&g.conv_k, &mut s.conv_k, ki), short_conv(&g.conv_v, &mut s.conv_v, vi)));
    let scale = 1.0 / (HD as f32).sqrt();
    let mut o = [0f32; H];
    prof::t(4, || for hh in 0..NH {
        let r = hh * HD;
        l2norm(&mut q[r..r + HD]); l2norm(&mut k[r..r + HD]);
        let decay = -fexp(g.a_log[hh]);
        let st = &mut s.s[hh * HD * HD..(hh + 1) * HD * HD];     // [k][v]
        // state *= exp(g) per k row; erase = (b*k)^T state; v_new = w*v - erase; state += k v_new^T; o = (q*scale)^T state
        let mut erase = [0f32; HD];
        for kk in 0..HD {
            let d = fexp(decay * softplus(gg[r + kk] + g.dt_bias[r + kk]));
            let bk = sigmoid(bb[r + kk]) * k[r + kk];
            let row = &mut st[kk * HD..(kk + 1) * HD];
            for vv in 0..HD { row[vv] *= d; erase[vv] += bk * row[vv]; }
        }
        let mut vn = [0f32; HD]; for vv in 0..HD { vn[vv] = sigmoid(ww[r + vv]) * v[r + vv] - erase[vv]; }
        let mut oh = [0f32; HD];
        for kk in 0..HD {
            let kv = k[r + kk]; let qs = q[r + kk] * scale;
            let row = &mut st[kk * HD..(kk + 1) * HD];
            for vv in 0..HD { row[vv] += kv * vn[vv]; oh[vv] += qs * row[vv]; }
        }
        // gated RMSNorm per head
        let rr = rms(&oh, GDN_EPS);
        for vv in 0..HD { o[r + vv] = oh[vv] * rr * g.o_norm[vv] * sigmoid(gate[r + vv] + g.g_bias[r + vv]); }
    });
    prof::t(5, || { let mut q = QVec::zero(); q.set(&o); let mut y = [0f32; H]; qmatvec(&g.o, &q, &mut y); y })
}

fn attn_step(a: &Attn, s: &mut AttnState, x: &QVec, pos: usize, rope_cos: &[f32], rope_sin: &[f32], sc: &mut [f32]) -> [f32; H] {
    let mut p = [0f32; 2 * NH * HD + 2 * HD];
    prof::t(6, || qmatvec(&a.proj, x, &mut p));
    let (qg, kv) = p.split_at_mut(2 * NH * HD); let (k, v) = kv.split_at_mut(HD);
    offset_rmsnorm(k, &a.k_norm);
    let rope = |t: &mut [f32]| { for i in 0..ROT / 2 { let (c, sn) = (rope_cos[pos * ROT / 2 + i], rope_sin[pos * ROT / 2 + i]); let (e, o) = (t[2 * i], t[2 * i + 1]); t[2 * i] = e * c - o * sn; t[2 * i + 1] = e * sn + o * c; } };
    rope(k);
    s.kc.extend_from_slice(k); s.vc.extend_from_slice(v); s.n += 1;
    let scale = 1.0 / (HD as f32).sqrt();
    let mut y = [0f32; NH * HD];
    let n = s.n; let sc = &mut sc[..n];
    prof::t(7, || for hh in 0..NH {
        let (q, gate) = qg[hh * 2 * HD..(hh + 1) * 2 * HD].split_at_mut(HD);
        offset_rmsnorm(q, &a.q_norm);
        rope(q);
        let mut mx = f32::NEG_INFINITY;
        for (j, kj) in s.kc.chunks_exact(HD).enumerate() { sc[j] = dot(q, kj) * scale; if sc[j] > mx { mx = sc[j]; } }
        let mut sum = 0f32; for j in 0..n { sc[j] = fexp(sc[j] - mx); sum += sc[j]; }
        let inv = 1.0 / sum;
        let mut acc = [0f32; HD];
        for (j, vj) in s.vc.chunks_exact(HD).enumerate() { let p = sc[j] * inv; for i in 0..HD { acc[i] += p * vj[i]; } }
        for i in 0..HD { y[hh * HD + i] = acc[i] * sigmoid(gate[i]); }
    });
    prof::t(8, || { let mut q = QVec::zero(); q.set(&y); let mut out = [0f32; H]; qmatvec(&a.o, &q, &mut out); out })
}

/// log-softmax of the logits at index `t` (natural log).
pub fn log_prob(logits: &[f32], t: usize) -> f32 {
    let mx = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut acc = [0f32; 8];
    for c in logits.chunks_exact(8) { for l in 0..8 { acc[l] += fexp(c[l] - mx); } }
    let sum = ((acc[0] + acc[4]) + (acc[1] + acc[5])) + ((acc[2] + acc[6]) + (acc[3] + acc[7]));
    logits[t] - mx - fln(sum)
}
