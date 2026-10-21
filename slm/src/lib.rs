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
use simd::{F8, fexp, fln, dot, dots, axpy};

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
    pub const NAMES: [&str; 18] = ["attn_res", "rmsnorm", "gdn_proj", "gdn_conv", "gdn_state", "gdn_o", "attn_proj", "attn_scores", "attn_o", "mlp", "lm_head", "log_prob", "att_dots", "att_exp", "att_val", "gdn_coef", "gdn_erase", "gdn_upd"];
    #[cfg(feature = "prof")] pub static ACC: [std::sync::atomic::AtomicU64; 18] = [const { std::sync::atomic::AtomicU64::new(0) }; 18];
    #[cfg(feature = "prof")] pub static CNT: [std::sync::atomic::AtomicU64; 18] = [const { std::sync::atomic::AtomicU64::new(0) }; 18];
    #[inline(always)] pub fn t<R>(i: usize, f: impl FnOnce() -> R) -> R {
        #[cfg(feature = "prof")] { let t0 = std::time::Instant::now(); let r = f(); ACC[i].fetch_add(t0.elapsed().as_nanos() as u64, std::sync::atomic::Ordering::Relaxed); CNT[i].fetch_add(1, std::sync::atomic::Ordering::Relaxed); r }
        #[cfg(not(feature = "prof"))] { let _ = i; f() }
    }
    pub fn report() {
        #[cfg(feature = "prof")] {
            let tot: u64 = ACC[..12].iter().map(|a| a.load(std::sync::atomic::Ordering::Relaxed)).sum();
            for i in 0..NAMES.len() { let a = ACC[i].load(std::sync::atomic::Ordering::Relaxed); let c = CNT[i].load(std::sync::atomic::Ordering::Relaxed); if c > 0 { eprintln!("{:>12} {:6.1}%  {:8.0} ns/call  x{}", NAMES[i], 100.0 * a as f64 / tot as f64, a as f64 / c as f64, c); } }
        }
    }
}

// ---------------------------------------------------------------------------
// small maths
// ---------------------------------------------------------------------------
#[inline] fn softplus(x: f32) -> f32 { x.max(0.0) + fln(1.0 + fexp(-x.abs())) }
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
#[inline] pub fn qmatvec(m: &QMat, x: &QVec, out: &mut [f32]) {
    debug_assert_eq!(x.n, m.n_in);
    #[cfg(all(target_arch = "aarch64", not(feature = "scalar")))]
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
#[cfg(all(target_arch = "aarch64", not(feature = "scalar")))]
fn neon_dotprod() -> bool {
    static F: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *F.get_or_init(|| std::arch::is_aarch64_feature_detected!("dotprod"))
}
/// `sdot`: 16 int8 products summed into 4 int32 lanes. The intrinsic is nightly-only, the instruction is not.
#[cfg(all(target_arch = "aarch64", not(feature = "scalar")))]
#[inline(always)]
unsafe fn sdot(acc: std::arch::aarch64::int32x4_t, a: std::arch::aarch64::int8x16_t, b: std::arch::aarch64::int8x16_t) -> std::arch::aarch64::int32x4_t {
    let mut r = acc;
    std::arch::asm!("sdot {r:v}.4s, {a:v}.16b, {b:v}.16b", r = inout(vreg) r, a = in(vreg) a, b = in(vreg) b, options(pure, nomem, nostack));
    r
}
#[cfg(all(target_arch = "aarch64", not(feature = "scalar")))]
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
    pub g_bias: Vec<f32>, pub conv_q: Vec<f32>, pub conv_k: Vec<f32>, pub conv_v: Vec<f32>,   // conv: [CONV][H] (tap-major)
    pub a_log: Vec<f32>, pub dt_bias: Vec<f32>, pub o_norm: Vec<f32>, pub o: QMat,
}
pub struct Attn { pub proj: QMat /* rows: q_gate(2*NH*HD) k(HD) v(HD) */, pub o: QMat, pub q_norm1: Vec<f32>, pub k_norm1: Vec<f32> /* 1 + weight */ }
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
                Mixer::Gdn(Gdn {
                    proj: QMat::concat(&parts.iter().map(|v| v.as_slice()).collect::<Vec<_>>(), H),
                    g_bias: get(&format!("{k}.g_proj.bias")),
                    conv_q: conv(&format!("{k}.q_conv1d.weight")), conv_k: conv(&format!("{k}.k_conv1d.weight")), conv_v: conv(&format!("{k}.v_conv1d.weight")),
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
        let mut m = Model { w, st: Vec::new(), pos: 0, src: Vec::with_capacity(2 * NL + 1), src_r: Vec::with_capacity(2 * NL + 1), xq: QVec::zero(), sc: vec![0f32; CTX + 8], logits: vec![0f32; V] };
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
        let flat: &[f32] = unsafe { std::slice::from_raw_parts(self.src.as_ptr() as *const f32, n * H) };
        dots(&r.q, flat, 1.0, &mut lg[..n]);
        let mut mx = f32::NEG_INFINITY;
        for s in 0..n { lg[s] *= self.src_r[s]; if lg[s] > mx { mx = lg[s]; } }
        let mut sum = 0f32; for s in 0..n { lg[s] = fexp(lg[s] - mx); sum += lg[s]; }
        let inv = 1.0 / sum;
        *out = [0f32; H];
        for s in 0..n { axpy(out, lg[s] * inv, &self.src[s]); }
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
                simd::silu_inplace(g);
                let mut i = 0; while i < FF { F8::load(&g[i..]).mul(F8::load(&u[i..])).store(&mut g[i..]); i += 8; }
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

/// depthwise causal conv over [x_{t-3}, x_{t-2}, x_{t-1}, x_t] (taps cw[j*H + c]), then SiLU
fn short_conv(cw: &[f32], state: &mut [[f32; H]; CONV - 1], x: &[f32]) -> [f32; H] {
    let mut y = [0f32; H];
    let mut i = 0;
    while i < H {
        let mut s = F8::load(&cw[3 * H + i..]).mul(F8::load(&x[i..]));
        for j in 0..CONV - 1 { s = s.add(F8::load(&cw[j * H + i..]).mul(F8::load(&state[j][i..]))); }
        s.store(&mut y[i..]);
        i += 8;
    }
    simd::silu_inplace(&mut y);
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
    // gate coefficients for the whole layer at once
    let mut sb = [0f32; H]; let mut sw = [0f32; H]; let mut sg = [0f32; H];
    prof::t(15, || { simd::sigmoid_into(bb, &mut sb); simd::sigmoid_into(ww, &mut sw);
        let mut t = [0f32; H]; let mut i = 0; while i < H { F8::load(&gate[i..]).add(F8::load(&g.g_bias[i..])).store(&mut t[i..]); i += 8; } simd::sigmoid_into(&t, &mut sg); });
    prof::t(4, || for hh in 0..NH {
        let r = hh * HD;
        l2norm(&mut q[r..r + HD]); l2norm(&mut k[r..r + HD]);
        let decay = -fexp(g.a_log[hh]);
        let st = &mut s.s[hh * HD * HD..(hh + 1) * HD * HD];     // [k][v]
        // state *= exp(g) per k row; erase = (b*k)^T state; v_new = w*v - erase; state += k v_new^T; o = (q*scale)^T state
        let mut dk = [0f32; HD]; let mut bk = [0f32; HD]; let mut wv = [0f32; HD];
        for kk in 0..HD { dk[kk] = fexp(decay * softplus(gg[r + kk] + g.dt_bias[r + kk])); }
        for kk in 0..HD { bk[kk] = sb[r + kk] * k[r + kk]; }
        for vv in 0..HD { wv[vv] = sw[r + vv] * v[r + vv]; }
        let mut e = [F8::zero(); HD / 8];
        prof::t(16, || for kk in 0..HD {
            let (d, b) = (F8::splat(dk[kk]), F8::splat(bk[kk]));
            let row = &mut st[kk * HD..(kk + 1) * HD];
            for c in 0..HD / 8 { let rv = F8::load(&row[c * 8..]).mul(d); rv.store(&mut row[c * 8..]); e[c] = e[c].add(b.mul(rv)); }
        });
        let mut vn = [F8::zero(); HD / 8]; for c in 0..HD / 8 { vn[c] = F8::load(&wv[c * 8..]).sub(e[c]); }
        let mut oh = [F8::zero(); HD / 8];
        prof::t(17, || for kk in 0..HD {
            let (kv, qs) = (F8::splat(k[r + kk]), F8::splat(q[r + kk] * scale));
            let row = &mut st[kk * HD..(kk + 1) * HD];
            for c in 0..HD / 8 { let rv = F8::load(&row[c * 8..]).add(kv.mul(vn[c])); rv.store(&mut row[c * 8..]); oh[c] = oh[c].add(qs.mul(rv)); }
        });
        // gated RMSNorm per head
        let mut ohf = [0f32; HD]; for c in 0..HD / 8 { oh[c].store(&mut ohf[c * 8..]); }
        let rr = F8::splat(rms(&ohf, GDN_EPS));
        for c in 0..HD / 8 { oh[c].mul(rr).mul(F8::load(&g.o_norm[c * 8..])).mul(F8::load(&sg[r + c * 8..])).store(&mut o[r + c * 8..]); }
    });
    prof::t(5, || { let mut q = QVec::zero(); q.set(&o); let mut y = [0f32; H]; qmatvec(&g.o, &q, &mut y); y })
}

fn attn_step(a: &Attn, s: &mut AttnState, x: &QVec, pos: usize, rope_cos: &[f32], rope_sin: &[f32], sc: &mut [f32]) -> [f32; H] {
    let mut p = [0f32; 2 * NH * HD + 2 * HD];
    prof::t(6, || qmatvec(&a.proj, x, &mut p));
    let (qg, kv) = p.split_at_mut(2 * NH * HD); let (k, v) = kv.split_at_mut(HD);
    offset_rmsnorm(k, &a.k_norm1);
    let rope = |t: &mut [f32]| { for i in 0..ROT / 2 { let (c, sn) = (rope_cos[pos * ROT / 2 + i], rope_sin[pos * ROT / 2 + i]); let (e, o) = (t[2 * i], t[2 * i + 1]); t[2 * i] = e * c - o * sn; t[2 * i + 1] = e * sn + o * c; } };
    rope(k);
    s.kc.extend_from_slice(k); s.vc.extend_from_slice(v); s.n += 1;
    let scale = 1.0 / (HD as f32).sqrt();
    let mut y = [0f32; NH * HD];
    let n = s.n; let n8 = (n + 7) & !7;
    let sc = &mut sc[..n8];
    prof::t(7, || for hh in 0..NH {
        let (q, gate) = qg[hh * 2 * HD..(hh + 1) * 2 * HD].split_at_mut(HD);
        offset_rmsnorm(q, &a.q_norm1);
        rope(q);
        prof::t(12, || dots(q, &s.kc[..n * HD], scale, &mut sc[..n]));
        let inv = prof::t(13, || {
            for j in n..n8 { sc[j] = f32::NEG_INFINITY; }
            let mx = simd::max(sc);
            1.0 / simd::exp_sub_sum(sc, mx) });
        let mut acc = [0f32; HD];
        prof::t(14, || for (j, vj) in s.vc.chunks_exact(HD).enumerate() { axpy(&mut acc, sc[j] * inv, vj); });
        let mut gs = [0f32; HD]; simd::sigmoid_into(gate, &mut gs);
        for i in 0..HD { y[hh * HD + i] = acc[i] * gs[i]; }
    });
    prof::t(8, || { let mut q = QVec::zero(); q.set(&y); let mut out = [0f32; H]; qmatvec(&a.o, &q, &mut out); out })
}

/// log-softmax of the logits at index `t` (natural log). Leaves `logits` untouched.
pub fn log_prob(logits: &[f32], t: usize) -> f32 {
    let mx = simd::max(logits);
    let mv = F8::splat(mx); let mut s = F8::zero(); let mut i = 0;
    while i < logits.len() { s = s.add(simd::exp8(F8::load(&logits[i..]).sub(mv))); i += 8; }
    logits[t] - mx - fln(s.hsum())
}
