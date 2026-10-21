//! slm — a from-scratch, streaming forward pass for GPT-S3-3M (Axiomic Labs), the 3M-parameter
//! hybrid Gated-DeltaNet-2 / gated-GQA language model with Full Attention Residuals.
//! Token by token, float32, no dependencies. This is the reference the integer version
//! will be checked against; it itself is checked against PyTorch's logits.
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
const ROPE_THETA: f32 = 10000.0;
const ATTN_RES_EPS: f32 = 1e-6;
pub const fn is_attn(l: usize) -> bool { (l + 1) % 4 == 0 }

// ---------------------------------------------------------------------------
// weights
// ---------------------------------------------------------------------------
pub struct AttnRes { pub query: Vec<f32>, pub knw: Vec<f32> }
pub struct Gdn {
    pub q: Vec<f32>, pub k: Vec<f32>, pub v: Vec<f32>, pub f: Vec<f32>, pub b: Vec<f32>, pub w: Vec<f32>,
    pub g: Vec<f32>, pub g_bias: Vec<f32>, pub conv_q: Vec<f32>, pub conv_k: Vec<f32>, pub conv_v: Vec<f32>,
    pub a_log: Vec<f32>, pub dt_bias: Vec<f32>, pub o_norm: Vec<f32>, pub o: Vec<f32>,
}
pub struct Attn { pub q_gate: Vec<f32>, pub k: Vec<f32>, pub v: Vec<f32>, pub o: Vec<f32>, pub q_norm: Vec<f32>, pub k_norm: Vec<f32> }
pub enum Mixer { Gdn(Gdn), Attn(Attn) }
pub struct Layer { pub ln1: Vec<f32>, pub ln2: Vec<f32>, pub res1: AttnRes, pub res2: AttnRes, pub mixer: Mixer, pub w_gate: Vec<f32>, pub w_up: Vec<f32>, pub w_down: Vec<f32> }
pub struct Weights { pub wte: Vec<f32>, pub layers: Vec<Layer>, pub ln_f: Vec<f32>, pub res_f: AttnRes }

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
        let res = |p: &str| AttnRes { query: get(&format!("{p}.query")), knw: get(&format!("{p}.key_norm_weight")) };
        let mut layers = Vec::new();
        for l in 0..NL {
            let p = format!("transformer.h.{l}");
            let mixer = if is_attn(l) {
                Mixer::Attn(Attn { q_gate: get(&format!("{p}.attn.q_gate_proj.weight")), k: get(&format!("{p}.attn.k_proj.weight")), v: get(&format!("{p}.attn.v_proj.weight")),
                    o: get(&format!("{p}.attn.o_proj.weight")), q_norm: get(&format!("{p}.attn.q_norm.weight")), k_norm: get(&format!("{p}.attn.k_norm.weight")) })
            } else {
                let k = format!("{p}.attn.kernel");
                Mixer::Gdn(Gdn { q: get(&format!("{k}.q_proj.weight")), k: get(&format!("{k}.k_proj.weight")), v: get(&format!("{k}.v_proj.weight")),
                    f: get(&format!("{k}.f_proj.weight")), b: get(&format!("{k}.b_proj.weight")), w: get(&format!("{k}.w_proj.weight")),
                    g: get(&format!("{k}.g_proj.weight")), g_bias: get(&format!("{k}.g_proj.bias")),
                    conv_q: get(&format!("{k}.q_conv1d.weight")), conv_k: get(&format!("{k}.k_conv1d.weight")), conv_v: get(&format!("{k}.v_conv1d.weight")),
                    a_log: get(&format!("{k}.A_log")), dt_bias: get(&format!("{k}.dt_bias")), o_norm: get(&format!("{k}.o_norm.weight")), o: get(&format!("{k}.o_proj.weight")) })
            };
            layers.push(Layer { ln1: get(&format!("{p}.ln_1.weight")), ln2: get(&format!("{p}.ln_2.weight")), res1: res(&format!("{p}.attn_res_1")), res2: res(&format!("{p}.attn_res_2")), mixer,
                w_gate: get(&format!("{p}.mlp.w_gate.weight")), w_up: get(&format!("{p}.mlp.w_up.weight")), w_down: get(&format!("{p}.mlp.w_down.weight")) });
        }
        Weights { wte: get("transformer.wte.weight"), layers, ln_f: get("transformer.ln_f.weight"), res_f: res("transformer.attn_res_f") }
    }
}

// ---------------------------------------------------------------------------
// maths
// ---------------------------------------------------------------------------
#[inline] fn matvec(w: &[f32], x: &[f32], out: &mut [f32]) {            // w is [out][in], row-major (PyTorch Linear)
    let n = x.len();
    for (o, row) in out.iter_mut().zip(w.chunks_exact(n)) { let mut s = 0f32; for i in 0..n { s += row[i] * x[i]; } *o = s; }
}
#[inline] fn sigmoid(x: f32) -> f32 { 1.0 / (1.0 + (-x).exp()) }
#[inline] fn silu(x: f32) -> f32 { x * sigmoid(x) }
#[inline] fn softplus(x: f32) -> f32 { if x > 20.0 { x } else { (1.0 + x.exp()).ln() } }
#[inline] fn rms(x: &[f32], eps: f32) -> f32 { let mut s = 0f32; for &v in x { s += v * v; } 1.0 / (s / x.len() as f32 + eps).sqrt() }
fn rmsnorm(x: &[f32], w: &[f32], out: &mut [f32]) { let r = rms(x, RMS_EPS); for i in 0..x.len() { out[i] = x[i] * r * w[i]; } }
fn offset_rmsnorm(x: &mut [f32], w: &[f32]) { let r = rms(x, RMS_EPS); for i in 0..x.len() { x[i] = x[i] * r * (1.0 + w[i]); } }
fn l2norm(x: &mut [f32]) { let mut s = 0f32; for &v in x.iter() { s += v * v; } let r = 1.0 / (s + L2_EPS).sqrt(); for v in x.iter_mut() { *v *= r; } }

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
    rope_cos: Vec<f32>, rope_sin: Vec<f32>,          // [CTX][ROT/2]
    // scratch
    src: Vec<[f32; H]>, src_r: Vec<f32>,
    pub logits: Vec<f32>,
}

impl<'a> Model<'a> {
    pub fn new(w: &'a Weights) -> Model<'a> {
        let mut rope_cos = vec![0f32; CTX * ROT / 2]; let mut rope_sin = vec![0f32; CTX * ROT / 2];
        for p in 0..CTX { for i in 0..ROT / 2 {
            let inv = 1.0 / ROPE_THETA.powf((2 * i) as f32 / ROT as f32);
            let a = p as f32 * inv; rope_cos[p * ROT / 2 + i] = a.cos(); rope_sin[p * ROT / 2 + i] = a.sin();
        } }
        let mut m = Model { w, st: Vec::new(), pos: 0, rope_cos, rope_sin, src: Vec::with_capacity(2 * NL + 1), src_r: Vec::with_capacity(2 * NL + 1), logits: vec![0f32; V] };
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
        let mut q = [0f32; H]; for i in 0..H { q[i] = r.query[i] * r.knw[i]; }
        let mut lg = [0f32; 2 * NL + 1];
        let mut mx = f32::NEG_INFINITY;
        for s in 0..n { let mut d = 0f32; for i in 0..H { d += self.src[s][i] * q[i]; } lg[s] = d * self.src_r[s]; if lg[s] > mx { mx = lg[s]; } }
        let mut sum = 0f32; for s in 0..n { lg[s] = (lg[s] - mx).exp(); sum += lg[s]; }
        *out = [0f32; H];
        for s in 0..n { let wgt = lg[s] / sum; for i in 0..H { out[i] += wgt * self.src[s][i]; } }
    }
    fn push_src(&mut self, v: [f32; H]) { self.src_r.push(rms(&v, ATTN_RES_EPS)); self.src.push(v); }

    /// Feed one token; `self.logits` then holds the next-token logits. Panics past CTX tokens: reset first.
    pub fn step(&mut self, tok: u32) {
        assert!(self.pos < CTX, "context full: call reset()");
        let w = self.w;
        self.src.clear(); self.src_r.clear();
        let mut x = [0f32; H]; x.copy_from_slice(&w.wte[tok as usize * H..(tok as usize + 1) * H]);
        self.push_src(x);
        let mut h = [0f32; H]; let mut hn = [0f32; H];
        for l in 0..NL {
            let lw = &w.layers[l];
            self.attn_res(&lw.res1, &mut h);
            rmsnorm(&h, &lw.ln1, &mut hn);
            let out = match (&lw.mixer, &mut self.st[l]) {
                (Mixer::Gdn(g), MixState::Gdn(s)) => gdn_step(g, s, &hn),
                (Mixer::Attn(a), MixState::Attn(s)) => attn_step(a, s, &hn, self.pos, &self.rope_cos, &self.rope_sin),
                _ => unreachable!(),
            };
            self.push_src(out);
            self.attn_res(&lw.res2, &mut h);
            rmsnorm(&h, &lw.ln2, &mut hn);
            let mut g = [0f32; FF]; let mut u = [0f32; FF]; matvec(&lw.w_gate, &hn, &mut g); matvec(&lw.w_up, &hn, &mut u);
            for i in 0..FF { g[i] = silu(g[i]) * u[i]; }
            let mut y = [0f32; H]; matvec(&lw.w_down, &g, &mut y);
            self.push_src(y);
        }
        self.attn_res(&w.res_f, &mut h);
        rmsnorm(&h, &w.ln_f, &mut hn);
        matvec(&w.wte, &hn, &mut self.logits);
        self.pos += 1;
    }
}

fn short_conv(cw: &[f32], state: &mut [[f32; H]; CONV - 1], x: &[f32; H]) -> [f32; H] {
    // depthwise causal conv over [x_{t-3}, x_{t-2}, x_{t-1}, x_t] with weights cw[c*4 + j], then SiLU
    let mut y = [0f32; H];
    for c in 0..H {
        let mut s = cw[c * CONV + CONV - 1] * x[c];
        for j in 0..CONV - 1 { s += cw[c * CONV + j] * state[j][c]; }
        y[c] = silu(s);
    }
    state[0] = state[1]; state[1] = state[2]; state[2] = *x;
    y
}

fn gdn_step(g: &Gdn, s: &mut GdnState, x: &[f32; H]) -> [f32; H] {
    let mut qi = [0f32; H]; let mut ki = [0f32; H]; let mut vi = [0f32; H];
    let mut bb = [0f32; H]; let mut ww = [0f32; H]; let mut gg = [0f32; H]; let mut gate = [0f32; H];
    matvec(&g.q, x, &mut qi); matvec(&g.k, x, &mut ki); matvec(&g.v, x, &mut vi);
    matvec(&g.b, x, &mut bb); matvec(&g.w, x, &mut ww); matvec(&g.f, x, &mut gg); matvec(&g.g, x, &mut gate);
    for i in 0..H { bb[i] = sigmoid(bb[i]); ww[i] = sigmoid(ww[i]); gg[i] = softplus(gg[i] + g.dt_bias[i]); gate[i] += g.g_bias[i]; }
    let mut q = short_conv(&g.conv_q, &mut s.conv_q, &qi);
    let mut k = short_conv(&g.conv_k, &mut s.conv_k, &ki);
    let v = short_conv(&g.conv_v, &mut s.conv_v, &vi);
    let scale = 1.0 / (HD as f32).sqrt();
    let mut o = [0f32; H];
    for hh in 0..NH {
        let r = hh * HD;
        l2norm(&mut q[r..r + HD]); l2norm(&mut k[r..r + HD]);
        let decay = -g.a_log[hh].exp();
        let st = &mut s.s[hh * HD * HD..(hh + 1) * HD * HD];     // [k][v]
        // state *= exp(g) per k row; erase = (b*k)^T state; v_new = w*v - erase; state += k v_new^T; o = (q*scale)^T state
        let mut erase = [0f32; HD];
        for kk in 0..HD {
            let d = (decay * gg[r + kk]).exp();
            let bk = bb[r + kk] * k[r + kk];
            let row = &mut st[kk * HD..(kk + 1) * HD];
            for vv in 0..HD { row[vv] *= d; erase[vv] += bk * row[vv]; }
        }
        let mut vn = [0f32; HD]; for vv in 0..HD { vn[vv] = ww[r + vv] * v[r + vv] - erase[vv]; }
        let mut oh = [0f32; HD];
        for kk in 0..HD {
            let kv = k[r + kk]; let qs = q[r + kk] * scale;
            let row = &mut st[kk * HD..(kk + 1) * HD];
            for vv in 0..HD { row[vv] += kv * vn[vv]; oh[vv] += qs * row[vv]; }
        }
        // gated RMSNorm per head
        let rr = rms(&oh, GDN_EPS);
        for vv in 0..HD { o[r + vv] = oh[vv] * rr * g.o_norm[vv] * sigmoid(gate[r + vv]); }
    }
    let mut y = [0f32; H]; matvec(&g.o, &o, &mut y); y
}

fn attn_step(a: &Attn, s: &mut AttnState, x: &[f32; H], pos: usize, rope_cos: &[f32], rope_sin: &[f32]) -> [f32; H] {
    let mut qg = [0f32; 2 * NH * HD]; matvec(&a.q_gate, x, &mut qg);
    let mut k = [0f32; HD]; let mut v = [0f32; HD]; matvec(&a.k, x, &mut k); matvec(&a.v, x, &mut v);
    offset_rmsnorm(&mut k, &a.k_norm);
    let rope = |t: &mut [f32]| { for i in 0..ROT / 2 { let (c, sn) = (rope_cos[pos * ROT / 2 + i], rope_sin[pos * ROT / 2 + i]); let (e, o) = (t[2 * i], t[2 * i + 1]); t[2 * i] = e * c - o * sn; t[2 * i + 1] = e * sn + o * c; } };
    rope(&mut k);
    s.kc.extend_from_slice(&k); s.vc.extend_from_slice(&v); s.n += 1;
    let scale = 1.0 / (HD as f32).sqrt();
    let mut y = [0f32; NH * HD];
    let mut sc = vec![0f32; s.n];
    for hh in 0..NH {
        let (q, gate) = qg[hh * 2 * HD..(hh + 1) * 2 * HD].split_at_mut(HD);
        offset_rmsnorm(q, &a.q_norm);
        rope(q);
        let mut mx = f32::NEG_INFINITY;
        for j in 0..s.n { let kj = &s.kc[j * HD..(j + 1) * HD]; let mut d = 0f32; for i in 0..HD { d += q[i] * kj[i]; } sc[j] = d * scale; if sc[j] > mx { mx = sc[j]; } }
        let mut sum = 0f32; for j in 0..s.n { sc[j] = (sc[j] - mx).exp(); sum += sc[j]; }
        let mut acc = [0f32; HD];
        for j in 0..s.n { let p = sc[j] / sum; let vj = &s.vc[j * HD..(j + 1) * HD]; for i in 0..HD { acc[i] += p * vj[i]; } }
        for i in 0..HD { y[hh * HD + i] = acc[i] * sigmoid(gate[i]); }
    }
    let mut out = [0f32; H]; matvec(&a.o, &y, &mut out); out
}

/// log-softmax of the logits at index `t` (natural log).
pub fn log_prob(logits: &[f32], t: usize) -> f32 {
    let mx = logits.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mut sum = 0f32; for &l in logits { sum += (l - mx).exp(); }
    logits[t] - mx - sum.ln()
}
