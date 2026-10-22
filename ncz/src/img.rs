//! Pixel mode: 8-bit RGB images, lossless or near-lossless (every sample within +-k of the original).
//!
//! Same recipe as the text side, predict then code the surprise, on pixels:
//! - Each sample is predicted by a blend of predictors (neighbours, gradients and, for red and blue,
//!   "colour from brightness": this pixel's green change carried over), weighted by how wrong each was
//!   on the neighbouring pixels (the self-correcting predictor idea from JPEG XL).
//! - A running bias per context corrects what the blend keeps getting wrong.
//! - The residual is binarised (zero? / size / mantissa / sign) and every decision is coded by a small
//!   context-mixing model: several adaptive tables (local activity, expected error, neighbour residuals,
//!   the other channels' residual at this pixel, residual signs) mixed by a learned logistic mixer, then
//!   refined by an APM, then a binary range coder.
//! - Near-lossless (k > 0) quantises the residual in steps of 2k+1 (as JPEG-LS does); every context uses
//!   reconstructed values, so the decoder stays in step.
//! All integer arithmetic: identical bytes on every machine.
use crate::tables::{SQUASH, STRETCH};

pub const MAGIC: &[u8; 4] = b"NCZI";
pub const VERSION: u8 = 4;
const HEADER: usize = 4 + 1 + 1 + 1 + 4 + 4;
/// Header flags: how the stream was coded (the decoder must do the same).
pub const F_PLANAR: u8 = 1; pub const F_LEARNED_COLOUR: u8 = 2; pub const F_LEARNED_GREEN: u8 = 4;
/// The pixels carry a fourth channel, alpha (RGBA): coded first, from its own neighbours only.
pub const F_ALPHA: u8 = 8;
/// Separate allowance for colour: red and blue within +-kc (one extra header byte), green (brightness) within +-k.
pub const F_SPLIT_K: u8 = 16;
const ALL_FLAGS: u8 = F_PLANAR | F_LEARNED_COLOUR | F_LEARNED_GREEN | F_ALPHA | F_SPLIT_K;
/// Red and blue lean on green; green and alpha only on their own neighbours.
#[inline(always)] fn colour(c: usize) -> bool { c == 0 || c == 2 }

const NP: usize = 15;        // predictors
const ND: usize = 74;        // binary decisions per sample: 0 zero?, 1..=7 size, 8..71 mantissa, 72 sign, 73 spare
const D_SIGN: usize = 72;
const NSETS: usize = 11;     // context sets feeding the mixer (six weak ones removed after ablation)
const TBITS: u32 = 22;       // one shared hashed table of adaptive probabilities, 2^22 cells
const N_IN: usize = NSETS + 1;
const LIMIT: u32 = 255;
const MIX_LR: i32 = 1;
const MIX_SHIFT: i32 = 11;

// ---------------------------------------------------------------------------
// range coder (same as ncz v3: 32-bit, 12-bit probabilities of a 1)
// ---------------------------------------------------------------------------
trait BitIo { fn bit(&mut self, p12: u32, want: u32) -> u32; }
struct Enc { x1: u32, x2: u32, out: Vec<u8> }
impl BitIo for Enc {
    #[inline(always)] fn bit(&mut self, p12: u32, y: u32) -> u32 {
        let p = p12.clamp(1, 4095);
        let xmid = self.x1 + ((self.x2 - self.x1) >> 12) * p;
        if y != 0 { self.x2 = xmid; } else { self.x1 = xmid + 1; }
        while (self.x1 ^ self.x2) & 0xFF00_0000 == 0 { self.out.push((self.x2 >> 24) as u8); self.x1 <<= 8; self.x2 = (self.x2 << 8) | 0xFF; }
        y
    }
}
struct Dec { x1: u32, x2: u32, x: u32, buf: Vec<u8>, pos: usize }
impl Dec { #[inline(always)] fn next_in(&mut self) -> u32 { let b = *self.buf.get(self.pos).unwrap_or(&0); self.pos += 1; b as u32 } }
impl BitIo for Dec {
    #[inline(always)] fn bit(&mut self, p12: u32, _want: u32) -> u32 {
        let p = p12.clamp(1, 4095);
        let xmid = self.x1 + ((self.x2 - self.x1) >> 12) * p;
        let y = if self.x <= xmid { self.x2 = xmid; 1 } else { self.x1 = xmid + 1; 0 };
        while (self.x1 ^ self.x2) & 0xFF00_0000 == 0 { self.x1 <<= 8; self.x2 = (self.x2 << 8) | 0xFF; self.x = (self.x << 8) | self.next_in(); }
        y
    }
}

// ---------------------------------------------------------------------------
// adaptive probability tables, mixer, APM
// ---------------------------------------------------------------------------
/// p in 16 bits and a hit count in the low bits: fast adaptation first, slower once the context is known.
#[inline(always)] fn sm_p(v: u32) -> i32 { (v >> 20) as i32 }                       // 12-bit probability
#[inline(always)] fn sm_update(v: u32, y: u32) -> u32 {
    let (mut p, mut n) = ((v >> 16) as i32, v & 0xFFFF);
    let target = if y != 0 { 65535 } else { 0 };
    p += (target - p) / (n as i32 + 2);
    if n < LIMIT { n += 1; }
    ((p.clamp(16, 65519) as u32) << 16) | n
}
const SM_INIT: u32 = 32768 << 16;

struct Apm { t: Vec<u16>, idx: usize }
impl Apm {
    fn new(n: usize) -> Apm {
        let mut t = vec![0u16; n * 33];
        for i in 0..n { for j in 0..33 { let k = ((j as i32 - 16) * 128 + 2048).clamp(1, 4095) as usize; t[i * 33 + j] = (SQUASH[k] * 16) as u16; } }
        Apm { t, idx: 0 }
    }
    #[inline(always)] fn refine(&mut self, pr: i32, cxt: usize) -> i32 {
        let s = STRETCH[pr as usize] + 2048; let w = s & 127; let i = (s >> 7) as usize + cxt * 33;
        self.idx = i;
        (self.t[i] as i32 * (128 - w) + self.t[i + 1] as i32 * w) >> 11
    }
    #[inline(always)] fn update(&mut self, y: u32) {
        let g: i32 = if y != 0 { 65535 } else { 0 }; let i = self.idx;
        self.t[i] = (self.t[i] as i32 + ((g - self.t[i] as i32) >> 6)) as u16;
        self.t[i + 1] = (self.t[i + 1] as i32 + ((g - self.t[i + 1] as i32) >> 6)) as u16;
    }
}

/// Quantise a non-negative activity-like number into 0..16 buckets on a log-ish scale.
#[inline(always)] fn q16(v: i32) -> usize {
    const T: [i32; 15] = [1, 2, 3, 4, 6, 8, 11, 15, 20, 27, 36, 48, 64, 90, 128];
    let mut i = 0; while i < 15 && v >= T[i] { i += 1; } i
}

struct Model {
    w: usize, h: usize, k: i32, kc: i32, planar: bool,     // k: green and alpha; kc: red and blue
    rec: Vec<Vec<u8>>,                 // reconstructed samples per channel (R, G, B, alpha)
    res: Vec<Vec<i16>>,                // quantised residual per sample
    err: Vec<Vec<u16>>,                // per predictor abs error (x8), a two-row ring: [(pos % 2w) * NP + i]
    nch: usize, order: Vec<usize>,
    bias_sum: Vec<i32>, bias_n: Vec<i32>,
    sm: Vec<u32>,                      // hashed: (set, channel, context, decision, decoded-so-far) -> probability
    mix: Vec<i32>,                     // [ch][decision][activity 0..4][N_IN]
    mix2: Vec<i32>,                    // [ch][decision][expected error 0..16][N_IN]
    mix3: Vec<i32>,                    // [ch][decision][where W and N point vs the guess, 17x17][N_IN]
    offc: usize, pmix3: i32,
    fin: Vec<i32>, fin_st: [i32; 3], fin_base: usize,      // second layer: learned weights over the three mixers
    eqc: usize, mixbase2: usize, pmix2: i32,
    apm: Apm, apm2: Apm,
    // per-sample context state
    nets: Option<[Mlp; 2]>, gnet: Option<Mlp>,
    ch: usize, ctx: [u32; NSETS], act: usize, st: [i32; N_IN], sidx: [usize; NSETS], mixbase: usize, pmix: i32, apm_ctx: usize, apm2_ctx: usize,
}

// ---------------------------------------------------------------------------
// learned colour from brightness (bench/img/colorfit.py): a tiny MLP, f32 in a fixed order, no fused
// multiply-add (Rust never fuses), ReLU only, so every machine computes the same bits
// ---------------------------------------------------------------------------
struct Mlp { layers: Vec<(usize, usize, Vec<f32>, Vec<f32>)> }
impl Mlp {
    fn load(b: &[u8]) -> Mlp {
        let mut layers = Vec::new(); let mut i = 0;
        let rd = |b: &[u8], i: usize| u32::from_le_bytes(b[i..i + 4].try_into().unwrap());
        let f = |b: &[u8], i: usize, n: usize| -> Vec<f32> { b[i..i + 4 * n].chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect() };
        while i < b.len() {
            let (o, n) = (rd(b, i) as usize, rd(b, i + 4) as usize); i += 8;
            let w = f(b, i, o * n); i += 4 * o * n; let bias = f(b, i, o); i += 4 * o;
            layers.push((o, n, w, bias));
        }
        Mlp { layers }
    }
    fn run(&self, x: &[f32]) -> [f32; 2] {
        let mut cur: Vec<f32> = x.to_vec();
        for (li, (o, n, w, b)) in self.layers.iter().enumerate() {
            let mut nx = vec![0f32; *o];
            for r in 0..*o { let mut acc = b[r]; let row = &w[r * n..(r + 1) * n]; for j in 0..*n { acc += row[j] * cur[j]; } nx[r] = if li + 1 < self.layers.len() { acc.max(0.0) } else { acc }; }
            cur = nx;
        }
        [cur[0] * 64.0, cur[1]]
    }
}
static NET_RED: &[u8] = include_bytes!("../assets/color_red.bin");
static NET_BLUE: &[u8] = include_bytes!("../assets/color_blue.bin");
static NET_GREEN: &[u8] = include_bytes!("../assets/net_green_h64.bin");
/// Initial weights for mix, mix2 and fin, in that order, i16 LE at half scale (see bench/results.md).
static PRIOR: &[u8] = include_bytes!("../assets/img_mixer_prior.bin");
/// Causal neighbours the green network reads (dy, dx), relative to the anchor (W + N) / 2.
const GREEN_NB: [(isize, isize); 12] = [(0, -1), (0, -2), (-1, -2), (-1, -1), (-1, 0), (-1, 1), (-1, 2), (-2, -2), (-2, -1), (-2, 0), (-2, 1), (-2, 2)];

thread_local! { pub static STATS: std::cell::Cell<[f64; 12]> = const { std::cell::Cell::new([0.0; 12]) }; }
static STATS_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Turn on the per-channel cost counter (diagnostic; off by default, it costs a log per decision).
pub fn enable_stats() { STATS_ON.store(true, std::sync::atomic::Ordering::Relaxed); }
/// Bits spent so far per channel (R, G, B) x decision kind (zero?, size, mantissa, sign).
pub fn take_stats() -> [f64; 12] { STATS.with(|s| s.replace([0.0; 12])) }

const CODE_ORDER: [usize; 3] = [1, 0, 2];   // green first, then red, then blue (each can lean on the ones before)

impl Model {
    fn new(w: usize, h: usize, k: i32, flags: u8) -> Model {
        let planar = flags & F_PLANAR != 0;
        let nch = if flags & F_ALPHA != 0 { 4 } else { 3 };
        let order: Vec<usize> = if nch == 4 { vec![3, 1, 0, 2] } else { CODE_ORDER.to_vec() };
        let n = w * h;
        let mut mix = vec![0i32; 4 * ND * 4 * N_IN];
        let mut mix3 = vec![0i32; 4 * ND * 289 * N_IN];
        for s in 0..4 * ND * 289 { for i in 0..NSETS { mix3[s * N_IN + i] = 65536 / 4; } }
        let mut mix2 = vec![0i32; 4 * ND * 16 * N_IN];
        let nets = if flags & F_LEARNED_COLOUR != 0 && planar { Some([Mlp::load(NET_RED), Mlp::load(NET_BLUE)]) } else { None };
        let gnet = if flags & F_LEARNED_GREEN != 0 { Some(Mlp::load(NET_GREEN)) } else { None };
        let mut fin = vec![0i32; 4 * ND * 16 * 3];
        // mix, mix2 and the final layer start from a prior (mix3 starts cold): the median of the trained
        // weights over 100 photos, i16 at half scale; same start for encoder and decoder
        assert_eq!(PRIOR.len(), 2 * (mix.len() + mix2.len() + fin.len()));
        let mut it = PRIOR.chunks_exact(2).map(|b| (i16::from_le_bytes([b[0], b[1]]) as i32) << 1);
        for v in mix.iter_mut().chain(mix2.iter_mut()).chain(fin.iter_mut()) { *v = it.next().unwrap(); }
        Model { kc: k, w, h, k, planar, mix2, mix3, offc: 0, pmix3: 2048, fin, fin_st: [0; 3], fin_base: 0, eqc: 0, mixbase2: 0, pmix2: 2048, nets, gnet,
            rec: vec![vec![0; n]; nch], res: vec![vec![0; n]; nch],
            err: vec![vec![0; 2 * w * NP]; nch], nch, order,
            bias_sum: vec![0; 4 * 16 * 27], bias_n: vec![0; 4 * 16 * 27],
            sm: vec![SM_INIT; 1 << TBITS], mix,
            apm: Apm::new(4 * ND * 16), apm2: Apm::new(4 * ND * 27),
            ch: 0, ctx: [0; NSETS], act: 0, st: [0; N_IN], sidx: [0; NSETS], mixbase: 0, pmix: 2048, apm_ctx: 0, apm2_ctx: 0 }
    }

    /// One binary decision: predict from the current contexts, code it, learn.
    #[inline(always)] fn decide<I: BitIo>(&mut self, io: &mut I, d: usize, prefix: u32, want: u32) -> u32 {
        let ch = self.ch;
        for s in 0..NSETS {
            let mut h = (s as u32 + 1).wrapping_mul(0x9E37_79B1) ^ (ch as u32).wrapping_mul(0x85EB_CA77) ^ self.ctx[s].wrapping_mul(0xC2B2_AE3D)
                ^ (d as u32).wrapping_mul(0x27D4_EB2F) ^ prefix.wrapping_mul(0x1656_67B1);
            h ^= h >> 15; h = h.wrapping_mul(0x2C1B_3C6D); h ^= h >> 13;
            let i = (h >> (32 - TBITS)) as usize;
            self.sidx[s] = i;
            self.st[s] = STRETCH[sm_p(self.sm[i]).clamp(1, 4095) as usize];
        }
        self.st[NSETS] = 256;
        let mb = ((ch * ND + d) * 4 + self.act) * N_IN; self.mixbase = mb;
        let mb2 = ((ch * ND + d) * 16 + self.eqc) * N_IN; self.mixbase2 = mb2;
        let mb3 = ((ch * ND + d) * 289 + self.offc) * N_IN;
        let (mut dot, mut dot2, mut dot3): (i64, i64, i64) = (0, 0, 0);
        for i in 0..N_IN { let x = self.st[i] as i64; dot += self.mix[mb + i] as i64 * x; dot2 += self.mix2[mb2 + i] as i64 * x; }
        for i in 0..N_IN { dot3 += self.mix3[mb3 + i] as i64 * self.st[i] as i64; }
        let dot = ((dot >> 16) as i32).clamp(-2047, 2047); let dot2 = ((dot2 >> 16) as i32).clamp(-2047, 2047); let dot3 = ((dot3 >> 16) as i32).clamp(-2047, 2047);
        self.pmix2 = SQUASH[(dot2 + 2048) as usize]; self.pmix3 = SQUASH[(dot3 + 2048) as usize];
        self.fin_st = [dot, dot2, dot3]; let fb = ((ch * ND + d) * 16 + self.act * 4 + (self.eqc >> 2)) * 3; self.fin_base = fb;
        let df = ((self.fin[fb] as i64 * dot as i64 + self.fin[fb + 1] as i64 * dot2 as i64 + self.fin[fb + 2] as i64 * dot3 as i64) >> 16) as i32;
        self.pmix = SQUASH[(df.clamp(-2047, 2047) + 2048) as usize];
        let p1 = SQUASH[(dot + 2048) as usize];
        let a1 = self.apm.refine(self.pmix, self.apm_ctx * ND + d);
        let a2 = self.apm2.refine(self.pmix, self.apm2_ctx * ND + d);
        let p = ((2 * self.pmix + a1 + a2) >> 2).clamp(1, 4095);
        let y = io.bit(p as u32, want);
        if STATS_ON.load(std::sync::atomic::Ordering::Relaxed) { STATS.with(|st| { let mut v = st.get(); let q = if y == 1 { p } else { 4096 - p } as f64 / 4096.0; let kind = if d == 0 { 0 } else if d < 8 { 1 } else if d < D_SIGN { 2 } else { 3 }; v[ch * 4 + kind] -= q.log2(); st.set(v); }); }
        // learn
        let e = (((y as i32) << 12) - p1) * MIX_LR;
        let e2 = (((y as i32) << 12) - self.pmix2) * MIX_LR;
        let e3 = (((y as i32) << 12) - self.pmix3) * MIX_LR;
        let ef = (((y as i32) << 12) - self.pmix) * MIX_LR; let fb = self.fin_base;
        for i in 0..3 { self.fin[fb + i] += (self.fin_st[i] * ef) >> (MIX_SHIFT + 1); }
        for i in 0..N_IN { self.mix3[mb3 + i] += (self.st[i] * e3) >> MIX_SHIFT; }
        for i in 0..N_IN { self.mix[mb + i] += (self.st[i] * e) >> MIX_SHIFT; self.mix2[mb2 + i] += (self.st[i] * e2) >> MIX_SHIFT; }
        for s in 0..NSETS { let i = self.sidx[s]; self.sm[i] = sm_update(self.sm[i], y); }
        self.apm.update(y); self.apm2.update(y);
        y
    }

    /// Code (or decode) one sample of channel `c` at pixel `pos`. Returns the reconstructed value.
    fn sample<I: BitIo>(&mut self, io: &mut I, c: usize, x: usize, y: usize, value: u8) -> u8 {
        let w = self.w; let pos = y * w + x;
        let r = &self.rec[c];
        // neighbours (edges fall back to what exists; the very first sample sees 128)
        let at = |xx: isize, yy: isize| -> Option<i32> { if xx < 0 || yy < 0 || xx >= w as isize { None } else { Some(r[yy as usize * w + xx as usize] as i32) } };
        let (xi, yi) = (x as isize, y as isize);
        let wv = at(xi - 1, yi); let nv = at(xi, yi - 1);
        let wv0 = wv.or(nv).unwrap_or(128); let nv0 = nv.unwrap_or(wv0);
        let nw = at(xi - 1, yi - 1).unwrap_or(if y > 0 { nv0 } else { wv0 });
        let ne = at(xi + 1, yi - 1).unwrap_or(nv0);
        let ww = at(xi - 2, yi).unwrap_or(wv0); let nn = at(xi, yi - 2).unwrap_or(nv0);
        let (wv, nv) = (wv0, nv0);

        // the green change at this pixel and its neighbours, for "colour from brightness"
        let g = &self.rec[1]; let gi = |p: usize| g[p] as i32;
        let (cross, cross_w, cross_n, cross_grad) = if colour(c) {
            let gw = if x > 0 { gi(pos - 1) } else if y > 0 { gi(pos - w) } else { 128 };
            let gn = if y > 0 { gi(pos - w) } else { gw };
            let gnw = if x > 0 && y > 0 { gi(pos - w - 1) } else { gn };
            let gc = gi(pos);
            (true, wv + gc - gw, nv + gc - gn, (wv + nv - nw) + (gc - (gw + gn - gnw)))
        } else { (false, 0, 0, 0) };
        // nearby (green, this channel) pairs: copy the colour offset of the neighbour closest in brightness,
        // and fit colour = a * green + b over them (chroma from luma)
        let (bilat, cfl) = if colour(c) {
            let gc = g[pos] as i32; let rc = &self.rec[c];
            let mut cand: [(i32, i32); 6] = [(0, 0); 6]; let mut n = 0usize;
            let mut add = |p: Option<usize>| if let Some(p) = p { cand[n] = (g[p] as i32, rc[p] as i32); n += 1; };
            add(if x > 0 { Some(pos - 1) } else { None }); add(if y > 0 { Some(pos - w) } else { None });
            add(if x > 0 && y > 0 { Some(pos - w - 1) } else { None }); add(if y > 0 && x + 1 < w { Some(pos - w + 1) } else { None });
            add(if x > 1 { Some(pos - 2) } else { None }); add(if y > 1 { Some(pos - 2 * w) } else { None });
            if n == 0 { (gc, gc) } else {
                let mut best = 0; for i in 1..n { if (cand[i].0 - gc).abs() < (cand[best].0 - gc).abs() { best = i; } }
                let bil = gc + cand[best].1 - cand[best].0;
                let (mut sg, mut sr, mut sgg, mut sgr) = (0i64, 0i64, 0i64, 0i64);
                for &(gg, rr) in &cand[..n] { sg += gg as i64; sr += rr as i64; sgg += (gg * gg) as i64; sgr += (gg * rr) as i64; }
                let nn = n as i64; let den = nn * sgg - sg * sg;
                let fit = if n >= 3 && den > nn * nn * 16 {
                    let a = ((nn * sgr - sg * sr) * 256 / den).clamp(-512, 1024);
                    let b = (sr * 256 - a * sg) / nn;
                    ((a * gc as i64 + b + 128) >> 8) as i32
                } else { gc + ((sr - sg) / nn) as i32 };
                (bil, fit)
            }
        } else { (wv, nv) };
        // learned: colour offset from the 5x5 brightness window (whole green plane known) + nearby offsets
        let (lpred, lsig) = match (&self.nets, c) {
            (Some(nets), 0) | (Some(nets), 2) if x >= 2 && y >= 2 && x + 2 < w && y + 2 < self.h => {
                let gc = g[pos] as i32; let rc = &self.rec[c];
                let mut f = [0f32; 33]; let mut n = 0;
                for dy in -2isize..=2 { for dx in -2isize..=2 { if dy == 0 && dx == 0 { continue; }
                    let p = (y as isize + dy) as usize * w + (x as isize + dx) as usize; f[n] = (g[p] as i32 - gc) as f32 / 64.0; n += 1; } }
                for (dy, dx) in [(0isize, -1isize), (-1, 0), (-1, -1), (-1, 1), (0, -2), (-2, 0)] {
                    let p = (y as isize + dy) as usize * w + (x as isize + dx) as usize; f[n] = (rc[p] as i32 - g[p] as i32) as f32 / 64.0; n += 1; }
                if c == 2 { let ro = &self.rec[0]; for p in [pos, pos - 1, pos - w] { f[n] = (ro[p] as i32 - g[p] as i32) as f32 / 64.0; n += 1; } }
                let o = nets[if c == 0 { 0 } else { 1 }].run(&f[..n]);
                (gc + o[0].round() as i32, (((o[1] + 1.0) * 4.0).floor() as i32).clamp(0, 15) as u32 + 1)
            }
            _ => match &self.gnet {
                Some(net) if c == 1 && x >= 2 && y >= 2 && x + 2 < w => {
                    // learned brightness: the 12 coded neighbours relative to the anchor (W + N) / 2
                    let anchor = (wv + nv) >> 1; let mut f = [0f32; 12];
                    for (i, (dy, dx)) in GREEN_NB.iter().enumerate() {
                        let p = (y as isize + dy) as usize * w + (x as isize + dx) as usize; f[i] = (r[p] as i32 - anchor) as f32 / 64.0; }
                    let o = net.run(&f);
                    (anchor + o[0].round() as i32, (((o[1] + 0.5) * 3.0).floor() as i32).clamp(0, 15) as u32 + 1)
                }
                _ => (cfl, 0),
            },
        };
        let med = { let (a, b, cc) = (wv, nv, nw); if cc >= a.max(b) { a.min(b) } else if cc <= a.min(b) { a.max(b) } else { a + b - cc } };
        let preds: [i32; NP] = [
            wv, nv, wv + nv - nw, med, (wv + ne + 1) >> 1, wv + ne - nv, (wv + nv + ne + nw + 2) >> 2, ne,
            if cross { cross_grad } else { 2 * wv - ww },
            if cross { cross_w } else { 2 * nv - nn },
            if cross { cross_n } else { (3 * wv + 3 * nv - 2 * nw + 2) >> 2 },
            if c == 2 { // blue may also lean on red's change
                let rr = &self.rec[0]; let rw = if x > 0 { rr[pos - 1] as i32 } else if y > 0 { rr[pos - w] as i32 } else { 128 };
                wv + rr[pos] as i32 - rw } else { nw + ((ne - nw) >> 1) },
            bilat, cfl, lpred,
        ];
        // weights from each predictor's recent error around this pixel
        let e = &self.err[c];
        let ring = 2 * w;
        let ebuf = |p: Option<usize>, i: usize| -> u32 { p.map_or(0, |p| e[(p % ring) * NP + i] as u32) };
        let pw = if x > 0 { Some(pos - 1) } else { None };
        let pn = if y > 0 { Some(pos - w) } else { None };
        let pnw = if x > 0 && y > 0 { Some(pos - w - 1) } else { None };
        let pne = if y > 0 && x + 1 < w { Some(pos - w + 1) } else { None };
        let mut num: i64 = 0; let mut den: i64 = 0; let mut emin = u32::MAX;
        for i in 0..NP {
            let s = ebuf(pw, i) + ebuf(pn, i) + ebuf(pnw, i) + ebuf(pne, i) + 1;
            emin = emin.min(s);
            let wt = (1i64 << 40) / (s as i64 * s as i64);
            num += wt * preds[i].clamp(-255, 510) as i64 * 8; den += wt;
        }
        let blend8 = (num / den) as i32;                       // prediction x8
        // contexts
        let act = (wv - nw).abs() + (nv - nw).abs() + (nv - ne).abs();
        let aq = q16(act);
        let eq = q16((emin as i32) >> 3);
        let sgn = |v: i32| -> usize { if v > 0 { 2 } else if v < 0 { 0 } else { 1 } };
        let bias_ctx = (c * 16 + eq) * 27 + sgn(nv - (blend8 >> 3)) * 9 + sgn(wv - (blend8 >> 3)) * 3 + sgn(ne - nv);
        let bias = if self.bias_n[bias_ctx] > 0 { (self.bias_sum[bias_ctx] * 8) / self.bias_n[bias_ctx] } else { 0 };
        let pred = ((blend8 + bias + 4) >> 3).clamp(0, 255);
        let resq = &self.res[c];
        let rw = if x > 0 { resq[pos - 1] as i32 } else { 0 }; let rn = if y > 0 { resq[pos - w] as i32 } else { 0 };
        let rne = if y > 0 && x + 1 < w { resq[pos - w + 1] as i32 } else { 0 };
        let (xres, xres2) = match c { 0 => (self.res[1][pos] as i32, 0), 2 => (self.res[1][pos] as i32, self.res[0][pos] as i32), _ => (0, 0) };
        let xmag = q16(xres.abs() + xres2.abs());
        self.ch = c; self.act = (aq >> 2).min(3); self.eqc = eq;
        { let qs2 = |v: i32| -> usize { let m = q16(v.abs()).min(8) as i32; (if v < 0 { 8 - m } else { 8 + m }) as usize };
          self.offc = qs2(wv - pred) * 17 + qs2(nv - pred); }
        // where each predictor points relative to the final guess, signed and log-quantised: 0..16
        let qs = |v: i32| -> u32 { let m = q16(v.abs()).min(8) as i32; (if v < 0 { 8 - m } else { 8 + m }) as u32 };
        let off = |i: usize| qs(preds[i] - pred);
        self.ctx = [
            (if !colour(c) { q16(rne.abs() + (rw.abs() >> 1)) } else { xmag }) as u32,
            (sgn(rw) * 9 + sgn(rn) * 3 + sgn(xres) + 27 * if !colour(c) { 0 } else { 1 + (xres.abs() > 2) as usize }) as u32,
            (eq * 16 + if !colour(c) { q16(rne.abs()) } else { xmag }) as u32,
            off(0) * 17 + off(1),
            off(2) * 17 + off(7),
            off(8) * 17 + off(9) + 289 * (eq as u32 >> 2),
            off(10) * 17 + off(11),
            off(12) * 17 + off(13) + 289 * if colour(c) && self.planar {
                let gc = g[pos] as i32; let ge = if x + 1 < w { g[pos + 1] as i32 } else { gc }; let gs = if y + 1 < self.h { g[pos + w] as i32 } else { gc };
                q16((ge - gc).abs() + (gs - gc).abs()) as u32 + 1 } else { 0 },
            off(14) * 17 + lsig * 289,
            // exact neighbour colours with the guess: on graphics the same local pattern repeats exactly
            (wv as u32 & 255) | ((nv as u32 & 255) << 8) | ((pred as u32) << 16),
            (nw as u32 & 255) | ((ne as u32 & 255) << 8) | ((pred as u32) << 16) | (1 << 24),
        ];
        self.apm_ctx = c * 16 + aq;
        self.apm2_ctx = c * 27 + sgn(rw) * 9 + sgn(rn) * 3 + sgn(xres);

        // residual, quantised for near-lossless
        let kk = if colour(c) { self.kc } else { self.k };
        let step = 2 * kk + 1;
        let er = value as i32 - pred;
        let eqv = if kk == 0 { er } else if er >= 0 { (er + kk) / step } else { -((-er + kk) / step) };
        let m = eqv.unsigned_abs();
        // binarise: zero? / size (unary over log2) / mantissa / sign
        let z = self.decide(io, 0, 0, (m == 0) as u32);
        let mut mag: u32 = 0;
        if z == 0 {
            let b = 31 - m.max(1).leading_zeros();            // floor(log2 m)
            let mut bb = 0u32;
            while bb < 7 { if self.decide(io, 1 + bb as usize, 0, (b > bb) as u32) == 1 { bb += 1; } else { break; } }
            mag = 1 << bb;
            for j in (0..bb).rev() {
                let d = 8 + bb as usize * 8 + j as usize;
                let bit = self.decide(io, d, mag >> (j + 1), (m >> j) & 1);
                mag |= bit << j;
            }
            let s = self.decide(io, D_SIGN, mag.min(63), (eqv < 0) as u32);
            if s == 1 { mag = mag.wrapping_neg(); }
        }
        let eqd = mag as i32;
        let recon = (pred + eqd * step).clamp(0, 255);
        // learn: bias, predictor errors, residual store
        if self.bias_n[bias_ctx] >= 64 { self.bias_sum[bias_ctx] /= 2; self.bias_n[bias_ctx] /= 2; }
        self.bias_sum[bias_ctx] += recon - pred; self.bias_n[bias_ctx] += 1;
        let slot = (pos % (2 * w)) * NP;
        for i in 0..NP { self.err[c][slot + i] = ((preds[i] - recon).unsigned_abs() * 8).min(65535) as u16; }
        self.res[c][pos] = eqd.clamp(-32768, 32767) as i16;
        self.rec[c][pos] = recon as u8;
        recon as u8
    }

    /// Number of coding units: rows of one channel (planar) or rows of all channels (interleaved).
    fn units(&self) -> usize { if self.planar { self.nch * self.h } else { self.h } }
    /// Code (or decode) unit `u`; `px` holds interleaved samples (nch per pixel), read when encoding, written when decoding.
    fn unit<I: BitIo>(&mut self, io: &mut I, u: usize, px: &mut [u8], encoding: bool) {
        let (w, nch) = (self.w, self.nch);
        if self.planar {
            let (c, y) = (self.order[u / self.h], u % self.h);
            for x in 0..w { let i = (y * w + x) * nch + c; let v = if encoding { px[i] } else { 0 }; px[i] = self.sample(io, c, x, y, v); }
        } else {
            let y = u;
            for x in 0..w { for oi in 0..nch { let c = self.order[oi]; let i = (y * w + x) * nch + c; let v = if encoding { px[i] } else { 0 }; px[i] = self.sample(io, c, x, y, v); } }
        }
    }
}

/// Header: "NCZI" | version | k | flags | w u32 | h u32 | [kc, with F_SPLIT_K] | payload.
fn header(k: u8, kc: u8, flags: u8, w: usize, h: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(HEADER + 1);
    v.extend_from_slice(MAGIC); v.push(VERSION); v.push(k); v.push(flags);
    v.extend_from_slice(&(w as u32).to_le_bytes()); v.extend_from_slice(&(h as u32).to_le_bytes());
    if flags & F_SPLIT_K != 0 { v.push(kc); }
    v
}

/// Streaming encoder over interleaved samples (3 per pixel, or 4 with F_ALPHA): `step(n)` codes n more units.
pub struct ImgEncoder { m: Model, e: Enc, px: Vec<u8>, u: usize, k: u8, kc: u8, flags: u8 }
impl ImgEncoder {
    pub fn new(px: Vec<u8>, w: usize, h: usize, k: u8, flags: u8) -> ImgEncoder { ImgEncoder::new_split(px, w, h, k, k, flags) }
    /// Red and blue within +-kc, green (and alpha) within +-k.
    pub fn new_split(px: Vec<u8>, w: usize, h: usize, k: u8, kc: u8, flags: u8) -> ImgEncoder {
        let flags = if flags & F_LEARNED_COLOUR != 0 { flags | F_PLANAR } else { flags };
        let flags = if kc != k { flags | F_SPLIT_K } else { flags & !F_SPLIT_K };
        let nch = if flags & F_ALPHA != 0 { 4 } else { 3 };
        assert_eq!(px.len(), w * h * nch);
        let mut m = Model::new(w, h, k as i32, flags); m.kc = kc as i32;
        ImgEncoder { m, e: Enc { x1: 0, x2: 0xFFFF_FFFF, out: Vec::new() }, px, u: 0, k, kc, flags }
    }
    pub fn units(&self) -> usize { self.m.units() }
    pub fn done(&self) -> usize { self.u }
    pub fn step(&mut self, n: usize) -> usize {
        let end = self.u.saturating_add(n).min(self.m.units()); let k = end - self.u;
        while self.u < end { let u = self.u; self.m.unit(&mut self.e, u, &mut self.px, true); self.u += 1; }
        k
    }
    pub fn finish(mut self) -> Vec<u8> {
        while self.step(usize::MAX) > 0 {}
        self.e.out.extend_from_slice(&self.e.x1.to_be_bytes());
        let mut v = header(self.k, self.kc, self.flags, self.m.w, self.m.h); v.extend_from_slice(&self.e.out); v
    }
}

/// Streaming decoder; `finish` returns the interleaved samples.
pub struct ImgDecoder { m: Model, d: Dec, px: Vec<u8>, u: usize, pub w: usize, pub h: usize, pub k: u8, pub flags: u8 }
impl ImgDecoder {
    pub fn new(blob: &[u8]) -> Option<ImgDecoder> {
        if blob.len() < HEADER || &blob[..4] != MAGIC || blob[4] != VERSION { return None; }
        let (k, flags) = (blob[5], blob[6]);
        if flags & !ALL_FLAGS != 0 { return None; }
        let w = u32::from_le_bytes(blob[7..11].try_into().ok()?) as usize; let h = u32::from_le_bytes(blob[11..15].try_into().ok()?) as usize;
        if w == 0 || h == 0 || w.checked_mul(h)? > 1 << 28 { return None; }
        let nch = if flags & F_ALPHA != 0 { 4 } else { 3 };
        let (kc, body) = if flags & F_SPLIT_K != 0 { (*blob.get(HEADER)?, HEADER + 1) } else { (k, HEADER) };
        let mut d = Dec { x1: 0, x2: 0xFFFF_FFFF, x: 0, buf: blob[body..].to_vec(), pos: 0 };
        for _ in 0..4 { d.x = (d.x << 8) | d.next_in(); }
        let mut m = Model::new(w, h, k as i32, flags); m.kc = kc as i32;
        Some(ImgDecoder { m, d, px: vec![0; w * h * nch], u: 0, w, h, k, flags })
    }
    pub fn units(&self) -> usize { self.m.units() }
    pub fn done(&self) -> usize { self.u }
    pub fn step(&mut self, n: usize) -> usize {
        let end = self.u.saturating_add(n).min(self.m.units()); let k = end - self.u;
        while self.u < end { let u = self.u; self.m.unit(&mut self.d, u, &mut self.px, false); self.u += 1; }
        k
    }
    pub fn finish(mut self) -> Vec<u8> { while self.step(usize::MAX) > 0 {} self.px }
}

/// Compress 8-bit RGB pixels (row-major, 3 bytes per pixel). `k` = 0 lossless, else max abs error per sample.
/// `flags`: F_PLANAR (green plane first), F_LEARNED_COLOUR (needs planar), F_LEARNED_GREEN; 0 = fastest.
pub fn compress_rgb(rgb: &[u8], w: usize, h: usize, k: u8) -> Vec<u8> { compress_rgb_flags(rgb, w, h, k, 0) }
pub fn compress_rgb_flags(rgb: &[u8], w: usize, h: usize, k: u8, flags: u8) -> Vec<u8> { ImgEncoder::new(rgb.to_vec(), w, h, k, flags & !F_ALPHA).finish() }
/// Green within +-k, red and blue within +-kc (eyes see brightness detail, barely colour detail).
pub fn compress_rgb_split(rgb: &[u8], w: usize, h: usize, k: u8, kc: u8, flags: u8) -> Vec<u8> { ImgEncoder::new_split(rgb.to_vec(), w, h, k, kc, flags & !F_ALPHA).finish() }
/// RGBA (4 bytes per pixel).
pub fn compress_rgba_flags(rgba: &[u8], w: usize, h: usize, k: u8, flags: u8) -> Vec<u8> { ImgEncoder::new(rgba.to_vec(), w, h, k, flags | F_ALPHA).finish() }

/// Returns (samples, w, h, k) — 3 per pixel, or 4 if the stream carries alpha — or None if not an NCZI stream.
pub fn decompress_rgb(blob: &[u8]) -> Option<(Vec<u8>, usize, usize, u8)> {
    let d = ImgDecoder::new(blob)?; let (w, h, k) = (d.w, d.h, d.k);
    Some((d.finish(), w, h, k))
}

/// Read a binary PPM (P6, maxval 255).
pub fn read_ppm(b: &[u8]) -> Option<(Vec<u8>, usize, usize)> {
    let mut fields = Vec::new(); let mut i = 0;
    while fields.len() < 4 && i < b.len() {
        while i < b.len() && (b[i] as char).is_ascii_whitespace() { i += 1; }
        if i < b.len() && b[i] == b'#' { while i < b.len() && b[i] != b'\n' { i += 1; } continue; }
        let s = i; while i < b.len() && !(b[i] as char).is_ascii_whitespace() { i += 1; }
        fields.push(std::str::from_utf8(&b[s..i]).ok()?.to_string());
    }
    if fields.len() < 4 || fields[0] != "P6" || fields[3] != "255" { return None; }
    let (w, h): (usize, usize) = (fields[1].parse().ok()?, fields[2].parse().ok()?);
    let data = &b[i + 1..]; if data.len() < w * h * 3 { return None; }
    Some((data[..w * h * 3].to_vec(), w, h))
}
