//! Latent mode: the integer latents of a learned image codec (C channels x H x W, i16), coded by context mixing.
//!
//! The entropy decoding uses only already-decoded integers as context (no network output), in integer arithmetic,
//! so a file decodes to the same latents on every machine.
//! - Side info: one 5-bit size class per channel (log2 of its mean magnitude; 0 = all zero, not coded at all).
//!   Channels are coded strongest first (order follows from the classes: free).
//! - Each value: zero? / sign / magnitude (unary, then an Elias-gamma escape), every decision coded by a mix of
//!   adaptive tables. The main context is the energy at the same spot in the channels already coded (normalised by
//!   each channel's size): where the strong channels are busy, the weak ones are too. Plus same-channel neighbours,
//!   the channel and its class, and for signs the signs nearby.
use crate::img::{q16, sm_p, sm_update, Apm, BitIo, Dec, Enc, SM_INIT};
use crate::tables::{SQUASH, STRETCH};

pub const MAGIC: &[u8; 4] = b"NCZY";
pub const VERSION: u8 = 1;
const HEADER: usize = 4 + 1 + 6;

const NSETS: usize = 10;
const N_IN: usize = NSETS + 1;
const TBITS: u32 = 22;
const D_ZERO: usize = 0; const D_SIGN: usize = 1; const D_UN: usize = 2; const NUN: usize = 16; const D_ESC: usize = D_UN + NUN;
const ND: usize = D_ESC + 12;           // escape: 11 length decisions + 1 mantissa
const NCLS: usize = 32;

/// Size class of a channel: 0 = all zero, else 1.. from 2*log2(16 * mean |y|).
fn class_of(v: &[i16]) -> u8 {
    let s: u64 = v.iter().map(|&x| x.unsigned_abs() as u64).sum();
    if s == 0 { return 0; }
    // 2*log2(16*s/n) + 1, integer: compare (16 s)^2 against n^2 * 2^(k-1)
    let n = v.len() as u128; let a = (16 * s as u128) * (16 * s as u128);
    let mut k = 1u8; while (k as usize) < NCLS - 1 && a >= n * n * (1u128 << k) { k += 1; }
    k
}
/// Mean |y| of a class, x16 (at least 1).
fn s16_of(k: u8) -> i32 { if k == 0 { 1 } else { (((1u64 << (k as u32 - 1)) as f64).sqrt() as i32).max(1) } }   // 2^((k-1)/2); IEEE sqrt is exact-rounded everywhere

struct Model {
    c: usize, h: usize, w: usize,
    lat: Vec<i32>, cls: Vec<u8>, s16: Vec<i32>,
    esum: Vec<i32>, enext: Vec<i32>, ecount: i32, prev: Vec<usize>,
    sm: Vec<u32>, mix1: Vec<i32>, mix2: Vec<i32>, apm: Apm,
    ctx: [u32; NSETS], sctx: [u32; NSETS], st: [i32; N_IN], sidx: [usize; NSETS],
    ch: usize, qexp: usize, qloc: usize,
    pub bits: [f64; 4], chbits: Vec<f64>,
    /// A predictor's mean (x16) and scale buckets (two offsets) per latent, used as context (and the mean as prediction).
    aux: Option<(Vec<i16>, Vec<u8>, Vec<u8>)>, pred: i32,
    sel: Option<bool>,       // checkerboard pass being coded (None: plain raster)
    /// Position-major coding (VERSION_POS): channels coded so far at each position, the coding order, and the index in
    /// it of the channel being coded.
    pos: Option<(Vec<i32>, Vec<usize>, usize)>,
}

impl Model {
    fn new(c: usize, h: usize, w: usize) -> Model {
        let mut mix1 = vec![0i32; ND * NCLS * N_IN]; let mut mix2 = vec![0i32; ND * 17 * N_IN];
        for v in mix1.iter_mut().chain(mix2.iter_mut()) { *v = 65536 * 3 / 10; }
        Model { c, h, w, lat: vec![0; c * h * w], cls: vec![0; c], s16: vec![1; c], esum: vec![0; h * w], enext: vec![0; h * w], ecount: 0, prev: Vec::new(),
            sm: vec![SM_INIT; 1 << TBITS], mix1, mix2, apm: Apm::new(ND * 17),
            ctx: [0; NSETS], sctx: [0; NSETS], st: [0; N_IN], sidx: [0; NSETS], ch: 0, qexp: 0, qloc: 0, bits: [0.0; 4], chbits: vec![0.0; c], aux: None, pred: 0, sel: None, pos: None }
    }

    #[inline(always)] fn decide<I: BitIo>(&mut self, io: &mut I, d: usize, want: u32) -> u32 {
        let cx = if d == D_SIGN { self.sctx } else { self.ctx };
        for s in 0..NSETS {
            let mut h = (s as u32 + 1).wrapping_mul(0x9E37_79B1) ^ cx[s].wrapping_mul(0xC2B2_AE3D) ^ (d as u32).wrapping_mul(0x27D4_EB2F);
            h ^= h >> 15; h = h.wrapping_mul(0x2C1B_3C6D); h ^= h >> 13;
            let i = (h >> (32 - TBITS)) as usize; self.sidx[s] = i;
            self.st[s] = STRETCH[sm_p(self.sm[i]).clamp(1, 4095) as usize];
        }
        self.st[NSETS] = 256;
        let k = self.cls[self.ch] as usize;
        let b1 = (d * NCLS + k) * N_IN; let b2 = (d * 17 + self.qexp) * N_IN;
        let (mut d1, mut d2) = (0i64, 0i64);
        for i in 0..N_IN { let x = self.st[i] as i64; d1 += self.mix1[b1 + i] as i64 * x; d2 += self.mix2[b2 + i] as i64 * x; }
        let d1 = ((d1 >> 16) as i32).clamp(-2047, 2047); let d2 = ((d2 >> 16) as i32).clamp(-2047, 2047);
        let (p1, p2) = (SQUASH[(d1 + 2048) as usize], SQUASH[(d2 + 2048) as usize]);
        let pm = SQUASH[(((d1 + d2) >> 1) + 2048) as usize];
        let pa = self.apm.refine(pm, d * 17 + self.qloc);
        let p = ((pm * 3 + pa) >> 2).clamp(1, 4095);
        let y = io.bit(p as u32, want);
        let q = if y == 1 { p } else { 4096 - p } as f64 / 4096.0;
        self.chbits[self.ch] -= q.log2();
        self.bits[if d == D_ZERO { 0 } else if d == D_SIGN { 1 } else if d < D_ESC { 2 } else { 3 }] -= q.log2();
        let (e1, e2) = (((y as i32) << 12) - p1, ((y as i32) << 12) - p2);
        for i in 0..N_IN { self.mix1[b1 + i] += (self.st[i] * e1) >> 10; self.mix2[b2 + i] += (self.st[i] * e2) >> 10; }
        for s in 0..NSETS { let i = self.sidx[s]; self.sm[i] = sm_update(self.sm[i], y); }
        self.apm.update(y);
        y
    }

    /// Contexts for value (ch, x, y), from decoded values only.
    fn contexts(&mut self, ch: usize, x: usize, y: usize) {
        let (w, h) = (self.w, self.h); let base = ch * h * w; let pos = y * w + x;
        let at = |xx: isize, yy: isize| -> i32 { if xx < 0 || yy < 0 || xx >= w as isize || yy >= h as isize || (yy as usize * w + xx as usize) >= pos { 0 } else { self.lat[base + yy as usize * w + xx as usize] } };
        let (xi, yi) = (x as isize, y as isize);
        let (vw, vn, vnw, vne, vww, vnn) = (at(xi - 1, yi), at(xi, yi - 1), at(xi - 1, yi - 1), at(xi + 1, yi - 1), at(xi - 2, yi), at(xi, yi - 2));
        let k = self.cls[ch] as u32; let s16 = self.s16[ch];
        // cross-channel energy: mean normalised |y| here (x256) over the coded channels, and over the 3x3 around
        let (r, r3) = if let Some((cnt, _, _)) = &self.pos {
            // position-major: here, the channels already coded at this spot; around, the spots decoded so far
            let r = if cnt[pos] == 0 { 256 } else { self.esum[pos] / cnt[pos] };
            let (mut s3, mut n3) = (0, 0);
            for dy in -1i32..=1 { for dx in -1i32..=1 { let (xx, yy) = (x as i32 + dx, y as i32 + dy);
                if xx >= 0 && yy >= 0 && xx < w as i32 && yy < h as i32 { let q = yy as usize * w + xx as usize; if cnt[q] > 0 { s3 += self.esum[q] / cnt[q]; n3 += 1; } } } }
            (r, if n3 == 0 { 256 } else { s3 / n3 })
        } else {
            let ec = self.ecount.max(1);
            let r = if self.ecount == 0 { 256 } else { self.esum[pos] / ec };
            let mut r3 = 0; let mut n3 = 0;
            for dy in -1i32..=1 { for dx in -1i32..=1 { let (xx, yy) = (x as i32 + dx, y as i32 + dy); if xx >= 0 && yy >= 0 && xx < w as i32 && yy < h as i32 { r3 += self.esum[yy as usize * w + xx as usize]; n3 += 1; } } }
            (r, if self.ecount == 0 { 256 } else { r3 / (n3 * ec) })
        };
        let qexp = q16((r * s16) >> 10) as u32;              // expected |y| here, x4
        let qexp3 = q16((r3 * s16) >> 10) as u32;
        let qr = q16(r >> 5) as u32;                           // the ratio alone, x8
        // same-channel activity from the neighbours that are decoded: raster order uses W N NW NE; on a checkerboard
        // the anchors see their diagonals and two-away neighbours, the rest all four sides (anchors, done in pass 1)
        let get = |xx: isize, yy: isize| -> i32 { if xx < 0 || yy < 0 || xx >= w as isize || yy >= h as isize { 0 } else { self.lat[base + yy as usize * w + xx as usize] } };
        let loc = match self.sel {
            None => 2 * vw.abs() + 2 * vn.abs() + vnw.abs() + vne.abs(),
            Some(true) => 2 * vnw.abs() + 2 * vne.abs() + vww.abs() + vnn.abs(),
            Some(false) => vw.abs() + vn.abs() + get(xi + 1, yi).abs() + get(xi, yi + 1).abs() + vnw.abs() + vne.abs(),
        };
        let qloc = q16(loc) as u32;
        let qlocn = q16((loc * 16 * 4) / (6 * s16)) as u32;    // relative to the channel's size, x4
        let cb = (self.ecount.min(15) as u32 + 3) / 4;
        // co-located values in the last three coded channels
        let co = |i: usize| -> i32 { match &self.pos {
            Some((_, order, k)) => if *k > i { self.lat[order[k - 1 - i] * h * w + pos] } else { 0 },
            None => self.prev.len().checked_sub(i + 1).map_or(0, |j| self.lat[self.prev[j] * h * w + pos]) } };
        let (c1, c2, c3) = (co(0), co(1), co(2));
        let m3 = |v: i32| v.abs().min(3) as u32; let sg = |v: i32| (v.signum() + 1) as u32;
        let chn = ch as u32 + 1;
        self.qexp = qexp as usize; self.qloc = qloc as usize; self.ch = ch;
        self.pred = 0;
        if let Some((mu, qsv, qs2v)) = &self.aux {
            let i = base + pos; let m = mu[i] as i32; let pr = (m + 8).div_euclid(16); self.pred = pr;
            let fr = ((m - pr * 16 + 8) / 4).clamp(0, 3) as u32;          // where the mean sits inside the bin
            let (qs, qs2) = (qsv[i] as u32, qs2v[i] as u32);
            self.qexp = qs.min(16) as usize;
            self.ctx = [qs | fr << 5, k | qs << 5 | fr << 10, qs2 | fr << 5 | 1 << 20, k | qloc << 5 | qs << 10, qs | qlocn << 5 | 2 << 20, chn << 8 | qs, chn << 8 | qs | fr << 5 << 8,
                k | m3(vw) << 5 | m3(vn) << 7 | qs << 11, k | qexp << 5 | qs << 10 | 3 << 20, qs | qexp3 << 5 | fr << 10 | 4 << 20];
            self.sctx = [qs | fr << 5, chn << 8 | fr, k | qs << 5 | fr << 10, qs | fr << 5 | sg(vw) << 7 | sg(vn) << 9 | 1 << 20, chn << 8 | fr | sg(c1) << 2,
                fr | 2 << 20, k | fr << 5, chn << 8 | qs << 2 | fr, qs | fr << 5 | sg(c1) << 7 | 3 << 20, 0];
            return;
        }
        self.ctx = [k, k | qexp << 5, k | qexp3 << 5, k | qloc << 5, qexp | qlocn << 5 | 1 << 20, chn << 8, chn << 8 | qexp,
            k | m3(vw) << 5 | m3(vn) << 7 | m3(vnw).min(2) << 9 | qexp << 11, k | qexp << 5 | qloc << 10 | 2 << 20,
            qexp | qexp3 << 5 | cb << 10 | qr << 13 | 3 << 20 | (m3(c1) << 18)];
        self.sctx = [k, chn << 8, chn << 8 | sg(vw) | sg(vn) << 2, k | sg(vw) << 5 | sg(vn) << 7 | sg(vnw) << 9 | sg(vne) << 11,
            chn << 8 | sg(c1) | sg(c2) << 2 | sg(c3) << 4, k | sg(c1) << 5 | sg(c2) << 7 | m3(c1) << 9, sg(vw) | sg(vn) << 2 | sg(vww) << 4 | sg(vnn) << 6 | 1 << 20,
            chn << 8 | sg(c1) << 2 | m3(c1) << 4, k | qexp << 5 | sg(vw) << 10 | sg(c1) << 12, 0];
    }

    fn value<I: BitIo>(&mut self, io: &mut I, ch: usize, x: usize, y: usize, v: i32) -> i32 {
        self.contexts(ch, x, y);
        let v = v - self.pred; let pred = self.pred;
        let r = self.residual(io, v); r + pred
    }
    fn residual<I: BitIo>(&mut self, io: &mut I, v: i32) -> i32 {
        let a = v.abs();
        if self.decide(io, D_ZERO, (a == 0) as u32) == 1 { return 0; }
        let neg = self.decide(io, D_SIGN, (v < 0) as u32) == 1;
        let mut m = 0;
        while m < NUN as i32 { let stop = self.decide(io, D_UN + m as usize, (a - 1 == m) as u32); if stop == 1 { break; } m += 1; }
        let mag = if m < NUN as i32 { m + 1 } else {
            // Elias gamma of (a - 1 - NUN) + 1
            let e = (a - 1 - NUN as i32 + 1).max(1) as u32; let nb = 31 - e.leading_zeros();
            let mut n = 0u32;
            while self.decide(io, D_ESC + (n as usize).min(10), (n == nb) as u32) == 0 { n += 1; if n > 20 { break; } }
            let mut val = 1u32;
            for i in (0..n).rev() { val = (val << 1) | self.decide(io, D_ESC + 11, (e >> i) & 1); }
            val as i32 - 1 + NUN as i32 + 1
        };
        if neg { -mag } else { mag }
    }

    /// One channel, every position, then its energy joins the cross-channel map.
    fn channel<I: BitIo>(&mut self, io: &mut I, src: &[i16], ch: usize) { self.channel_part(io, src, ch, None, true) }
    /// The positions of one channel picked by `sel` (None: all; Some(true): the checkerboard's anchors, (x + y) even;
    /// Some(false): the rest). `done`: the channel is now complete, its energy joins the cross-channel map.
    fn channel_part<I: BitIo>(&mut self, io: &mut I, src: &[i16], ch: usize, sel: Option<bool>, done: bool) {
        let (h, w) = (self.h, self.w); self.sel = sel;
        for y in 0..h { for x in 0..w {
            if let Some(a) = sel { if ((x + y) % 2 == 0) != a { continue; } }
            let i = ch * h * w + y * w + x;
            let v = self.value(io, ch, x, y, src[i] as i32);
            self.lat[i] = v;
        } }
        if done {
            for p in 0..h * w { self.enext[p] = ((self.lat[ch * h * w + p].abs() * 16) << 8) / self.s16[ch]; }
            for p in 0..h * w { self.esum[p] += self.enext[p].min(256 * 64); }
            self.ecount += 1; self.prev.push(ch);
        }
    }

    /// Group by group: the model predicts each group from the groups already decoded, then the group is coded.
    fn run_groups<I: BitIo>(&mut self, io: &mut I, src: &[i16], gm: &dyn GroupModel) {
        let n = self.c * self.h * self.w; let hw = self.h * self.w;
        self.aux = Some((vec![0; n], vec![0; n], vec![0; n]));
        let (mut tp, mut tc) = (0.0f64, 0.0f64); let timing = cfg!(not(target_arch = "wasm32")) && std::env::var("LAT_TIMING").is_ok();
        let passes = gm.passes();
        for pass in 0..passes { for (g, chans) in gm.groups().iter().enumerate() {
            let t0 = if timing { Some(std::time::Instant::now()) } else { None };
            let (mu, qs, qs2) = gm.predict(&self.lat, self.c, self.h, self.w, g, pass);
            if let Some(t) = t0 { tp += t.elapsed().as_secs_f64(); }
            let t1 = if timing { Some(std::time::Instant::now()) } else { None };
            if let Some((am, aq, aq2)) = &mut self.aux {
                for (k, &ch) in chans.iter().enumerate() {
                    am[ch * hw..(ch + 1) * hw].copy_from_slice(&mu[k * hw..(k + 1) * hw]);
                    aq[ch * hw..(ch + 1) * hw].copy_from_slice(&qs[k * hw..(k + 1) * hw]);
                    aq2[ch * hw..(ch + 1) * hw].copy_from_slice(&qs2[k * hw..(k + 1) * hw]);
                }
            }
            let sel = if passes == 1 { None } else { Some(pass == 0) };
            for &ch in chans { self.channel_part(io, src, ch, sel, pass + 1 == passes); }
            if let Some(t) = t1 { tc += t.elapsed().as_secs_f64(); }
        } }
        if timing { eprintln!("predictor {tp:.2}s  coding {tc:.2}s"); }
    }

    /// Code (or decode) all latents. `src`: the values (encoder) or zeros (decoder). Classes must be set.
    fn run<I: BitIo>(&mut self, io: &mut I, src: &[i16]) {
        let (h, w) = (self.h, self.w);
        let mut order: Vec<usize> = (0..self.c).filter(|&c| self.cls[c] > 0).collect();
        order.sort_by_key(|&c| std::cmp::Reverse(self.cls[c]));              // stable: ties keep channel order
        let _ = (h, w);
        for &ch in &order { self.channel(io, src, ch); }
    }
}

/// Learns without coding (priming: encoder and decoder both run it over the same example latents first).
struct Null;
impl BitIo for Null { #[inline(always)] fn bit(&mut self, _p: u32, want: u32) -> u32 { want } }

/// A model already trained on example latents (same teacher, same quality). Encoder and decoder must use the same.
pub struct Prior { m: Model }
impl Prior {
    pub fn new(c: usize, h: usize, w: usize) -> Prior { Prior { m: Model::new(c, h, w) } }
    /// Learn from one example (its own shape: the tables and mixers carry over, the per-image state does not).
    pub fn learn(&mut self, lat: &[i16], c: usize, h: usize, w: usize) {
        let mut m = Model::new(c, h, w);
        std::mem::swap(&mut m.sm, &mut self.m.sm); std::mem::swap(&mut m.mix1, &mut self.m.mix1); std::mem::swap(&mut m.mix2, &mut self.m.mix2); std::mem::swap(&mut m.apm, &mut self.m.apm);
        for ch in 0..c { m.cls[ch] = class_of(&lat[ch * h * w..(ch + 1) * h * w]); m.s16[ch] = s16_of(m.cls[ch]); }
        m.run(&mut Null, lat);
        std::mem::swap(&mut m.sm, &mut self.m.sm); std::mem::swap(&mut m.mix1, &mut self.m.mix1); std::mem::swap(&mut m.mix2, &mut self.m.mix2); std::mem::swap(&mut m.apm, &mut self.m.apm);
    }
    fn model(&self, c: usize, h: usize, w: usize) -> Model {
        let mut m = Model::new(c, h, w);
        m.sm = self.m.sm.clone(); m.mix1 = self.m.mix1.clone(); m.mix2 = self.m.mix2.clone(); m.apm = self.m.apm.clone(); m
    }
}

/// Classes as 5 bits each, through the coder with the previous class as context.
fn code_classes<I: BitIo>(io: &mut I, cls: &mut [u8], sm: &mut [u32]) {
    let mut prev = 0usize;
    for k in cls.iter_mut() {
        let mut v = 0u8;
        for b in (0..5).rev() {
            let node = prev * 32 + ((1usize << (4 - b)) | v as usize);          // binary tree node under the previous class
            let bit = io.bit(sm_p(sm[node]).clamp(1, 4095) as u32, ((*k >> b) & 1) as u32);
            sm[node] = sm_update(sm[node], bit); v = (v << 1) | bit as u8;
        }
        *k = v; prev = v as usize;
    }
}

/// Compress latents (channel-major, i16). Also returns the bits spent on [zero?, sign, magnitude, escape].
pub fn compress_stats(lat: &[i16], c: usize, h: usize, w: usize, prior: Option<&Prior>) -> (Vec<u8>, [f64; 4]) {
    assert_eq!(lat.len(), c * h * w);
    let mut m = prior.map_or_else(|| Model::new(c, h, w), |p| p.model(c, h, w));
    let mut cls: Vec<u8> = (0..c).map(|ch| class_of(&lat[ch * h * w..(ch + 1) * h * w])).collect();
    let mut e = Enc { x1: 0, x2: 0xFFFF_FFFF, out: Vec::new() };
    let mut csm = vec![SM_INIT; NCLS * 32]; code_classes(&mut e, &mut cls, &mut csm);
    for ch in 0..c { m.cls[ch] = cls[ch]; m.s16[ch] = s16_of(cls[ch]); }
    m.run(&mut e, lat);
    e.out.extend_from_slice(&e.x1.to_be_bytes());
    let mut out = Vec::with_capacity(HEADER + e.out.len());
    out.extend_from_slice(MAGIC); out.push(VERSION);
    for v in [c, h, w] { out.extend_from_slice(&(v as u16).to_le_bytes()); }
    out.extend_from_slice(&e.out);
    (out, m.bits)
}
/// Bits spent per channel (diagnostics).
pub fn channel_bits(lat: &[i16], c: usize, h: usize, w: usize) -> Vec<f64> {
    let mut m = Model::new(c, h, w);
    for ch in 0..c { m.cls[ch] = class_of(&lat[ch * h * w..(ch + 1) * h * w]); m.s16[ch] = s16_of(m.cls[ch]); }
    m.run(&mut Null, lat); m.chbits
}
/// Scale buckets from a scale x64 (diagnostic input path).
fn qs_of(v: i32) -> u8 { let mut b = 0; while b < 23 && v >= (11 << b) >> 2 { b += 1; } b as u8 }
/// Diagnostic: bits with an outside model's mean/scale as context (round trip not provided).
pub fn compress_aux(lat: &[i16], c: usize, h: usize, w: usize, mu16: Vec<i16>, sc64: Vec<u16>) -> (usize, [f64; 4]) {
    let qs = sc64.iter().map(|&v| qs_of(v as i32)).collect(); let qs2 = sc64.iter().map(|&v| qs_of(v as i32 * 3 / 2)).collect();
    let mut m = Model::new(c, h, w); m.aux = Some((mu16, qs, qs2));
    let mut cls: Vec<u8> = (0..c).map(|ch| class_of(&lat[ch * h * w..(ch + 1) * h * w])).collect();
    let mut e = Enc { x1: 0, x2: 0xFFFF_FFFF, out: Vec::new() };
    let mut csm = vec![SM_INIT; NCLS * 32]; code_classes(&mut e, &mut cls, &mut csm);
    for ch in 0..c { m.cls[ch] = cls[ch].max(1); m.s16[ch] = s16_of(cls[ch]); }      // with a model, empty channels are coded too (cheap)
    m.run(&mut e, lat); (e.out.len() + 4 + HEADER, m.bits)
}
pub fn compress(lat: &[i16], c: usize, h: usize, w: usize, prior: Option<&Prior>) -> Vec<u8> { compress_stats(lat, c, h, w, prior).0 }

/// Returns (latents, c, h, w).
pub fn decompress(blob: &[u8], prior: Option<&Prior>) -> Option<(Vec<i16>, usize, usize, usize)> {
    if blob.len() < HEADER || &blob[..4] != MAGIC || blob[4] != VERSION { return None; }
    let g = |i: usize| u16::from_le_bytes([blob[i], blob[i + 1]]) as usize;
    let (c, h, w) = (g(5), g(7), g(9));
    if c == 0 || h == 0 || w == 0 || c * h * w > 1 << 26 { return None; }
    let mut d = Dec { x1: 0, x2: 0xFFFF_FFFF, x: 0, buf: blob[HEADER..].to_vec(), pos: 0 };
    for _ in 0..4 { d.x = (d.x << 8) | d.next_in(); }
    let mut cls = vec![0u8; c]; let mut csm = vec![SM_INIT; NCLS * 32]; code_classes(&mut d, &mut cls, &mut csm);
    let mut m = prior.map_or_else(|| Model::new(c, h, w), |p| p.model(c, h, w));
    for ch in 0..c { m.cls[ch] = cls[ch]; m.s16[ch] = s16_of(cls[ch]); }
    let zeros = vec![0i16; c * h * w];
    m.run(&mut d, &zeros);
    Some((m.lat.iter().map(|&v| v as i16).collect(), c, h, w))
}

pub const VERSION_POS: u8 = 3;
impl Model {
    /// One position, every channel in coding order (position-major). The outside scales for this position are in aux.
    fn position<I: BitIo>(&mut self, io: &mut I, src: &[i16], x: usize, y: usize) {
        let (h, w) = (self.h, self.w); let p = y * w + x; let n = self.pos.as_ref().map_or(0, |v| v.1.len());
        for k in 0..n {
            let ch = { let ps = self.pos.as_mut().unwrap(); ps.2 = k; ps.1[k] };
            let i = ch * h * w + p;
            let v = self.value(io, ch, x, y, src[i] as i32); self.lat[i] = v;
            self.esum[p] += (((v.abs() * 16) << 8) / self.s16[ch]).min(256 * 64);
            self.pos.as_mut().unwrap().0[p] += 1;
        }
    }
    fn set_scales(&mut self, x: usize, y: usize, sc64: &[u16]) {
        let (h, w) = (self.h, self.w); let p = y * w + x;
        if let Some((mu, qs, qs2)) = &mut self.aux {
            for (ch, &v) in sc64.iter().enumerate() { let i = ch * h * w + p; mu[i] = 0; qs[i] = qs_of(v as i32); qs2[i] = qs_of(v as i32 * 3 / 2); }
        }
    }
    fn start_positions(&mut self) {
        let n = self.c * self.h * self.w; self.aux = Some((vec![0; n], vec![0; n], vec![0; n]));
        let mut order: Vec<usize> = (0..self.c).filter(|&c| self.cls[c] > 0).collect();
        order.sort_by_key(|&c| std::cmp::Reverse(self.cls[c]));
        self.pos = Some((vec![0; self.h * self.w], order, 0));
    }
}
/// Position-major NCZY (version 3), for an outside autoregressive model (tm.rs): positions in raster order, at each one
/// every channel strongest class first, each value with the model's scale at that spot as context (symbols already
/// mean-centred: mean 0). `sc64`: scale x64, channel-major, as the decoder will compute them.
pub fn compress_positions(sym: &[i16], c: usize, h: usize, w: usize, sc64: &[u16]) -> Vec<u8> {
    assert_eq!(sym.len(), c * h * w); assert_eq!(sc64.len(), c * h * w);
    let mut m = Model::new(c, h, w);
    let mut cls: Vec<u8> = (0..c).map(|ch| class_of(&sym[ch * h * w..(ch + 1) * h * w])).collect();
    let mut e = Enc { x1: 0, x2: 0xFFFF_FFFF, out: Vec::new() };
    let mut csm = vec![SM_INIT; NCLS * 32]; code_classes(&mut e, &mut cls, &mut csm);
    for ch in 0..c { m.cls[ch] = cls[ch]; m.s16[ch] = s16_of(cls[ch]); }
    m.start_positions(); let hw = h * w; let mut sc = vec![0u16; c];
    for y in 0..h { for x in 0..w {
        for ch in 0..c { sc[ch] = sc64[ch * hw + y * w + x]; }
        m.set_scales(x, y, &sc); m.position(&mut e, sym, x, y);
    } }
    e.out.extend_from_slice(&e.x1.to_be_bytes());
    let mut out = Vec::with_capacity(HEADER + e.out.len());
    out.extend_from_slice(MAGIC); out.push(VERSION_POS);
    for v in [c, h, w] { out.extend_from_slice(&(v as u16).to_le_bytes()); }
    out.extend_from_slice(&e.out); out
}
/// Decoding side of compress_positions: the caller computes each position's scales from what is decoded so far.
pub struct PosDecoder { m: Model, d: Dec, zeros: Vec<i16>, pub c: usize, pub h: usize, pub w: usize }
impl PosDecoder {
    pub fn new(blob: &[u8]) -> Option<PosDecoder> {
        if blob.len() < HEADER || &blob[..4] != MAGIC || blob[4] != VERSION_POS { return None; }
        let g = |i: usize| u16::from_le_bytes([blob[i], blob[i + 1]]) as usize;
        let (c, h, w) = (g(5), g(7), g(9));
        if c == 0 || h == 0 || w == 0 || c * h * w > 1 << 26 { return None; }
        let mut d = Dec { x1: 0, x2: 0xFFFF_FFFF, x: 0, buf: blob[HEADER..].to_vec(), pos: 0 };
        for _ in 0..4 { d.x = (d.x << 8) | d.next_in(); }
        let mut cls = vec![0u8; c]; let mut csm = vec![SM_INIT; NCLS * 32]; code_classes(&mut d, &mut cls, &mut csm);
        let mut m = Model::new(c, h, w);
        for ch in 0..c { m.cls[ch] = cls[ch]; m.s16[ch] = s16_of(cls[ch]); }
        m.start_positions();
        Some(PosDecoder { m, d, zeros: vec![0; c * h * w], c, h, w })
    }
    /// The symbols at (x, y), every channel, given that position's scales x64 (per channel).
    pub fn at(&mut self, x: usize, y: usize, sc64: &[u16]) -> Vec<i32> {
        self.m.set_scales(x, y, sc64); self.m.position(&mut self.d, &self.zeros, x, y);
        let hw = self.h * self.w; (0..self.c).map(|ch| self.m.lat[ch * hw + y * self.w + x]).collect()
    }
}

/// A model that predicts each channel group's mean and scale from the groups before it (see wonder::Predictor).
/// Must give identical output for identical input on every machine: it feeds the entropy decoder.
pub trait GroupModel {
    /// Channel groups in coding order.
    fn groups(&self) -> &[Vec<usize>];
    /// 1: each group whole; 2: checkerboard (every group's anchors first, then every group's rest).
    fn passes(&self) -> usize { 1 }
    /// For group `g`, from the latents decoded so far (earlier groups filled, the rest zero): per channel of the group
    /// (in group order) x position: mean x16, scale bucket, scale bucket at 1.5x.
    fn predict(&self, lat: &[i32], c: usize, h: usize, w: usize, g: usize, pass: usize) -> (Vec<i16>, Vec<u8>, Vec<u8>);
}

pub const VERSION_GROUPS: u8 = 2;
/// Compress latents with a group model (NCZY version 2: channels in the model's group order).
pub fn compress_groups(lat: &[i16], c: usize, h: usize, w: usize, gm: &dyn GroupModel) -> Vec<u8> {
    assert_eq!(lat.len(), c * h * w);
    let mut m = Model::new(c, h, w);
    let mut cls: Vec<u8> = (0..c).map(|ch| class_of(&lat[ch * h * w..(ch + 1) * h * w])).collect();
    let mut e = Enc { x1: 0, x2: 0xFFFF_FFFF, out: Vec::new() };
    let mut csm = vec![SM_INIT; NCLS * 32]; code_classes(&mut e, &mut cls, &mut csm);
    for ch in 0..c { m.cls[ch] = cls[ch].max(1); m.s16[ch] = s16_of(cls[ch]); }
    m.run_groups(&mut e, lat, gm);
    e.out.extend_from_slice(&e.x1.to_be_bytes());
    let mut out = Vec::with_capacity(HEADER + e.out.len());
    out.extend_from_slice(MAGIC); out.push(VERSION_GROUPS);
    for v in [c, h, w] { out.extend_from_slice(&(v as u16).to_le_bytes()); }
    out.extend_from_slice(&e.out); out
}
/// Returns (latents, c, h, w).
pub fn decompress_groups(blob: &[u8], gm: &dyn GroupModel) -> Option<(Vec<i16>, usize, usize, usize)> {
    if blob.len() < HEADER || &blob[..4] != MAGIC || blob[4] != VERSION_GROUPS { return None; }
    let g = |i: usize| u16::from_le_bytes([blob[i], blob[i + 1]]) as usize;
    let (c, h, w) = (g(5), g(7), g(9));
    if c == 0 || h == 0 || w == 0 || c * h * w > 1 << 26 || gm.groups().iter().map(|v| v.len()).sum::<usize>() != c { return None; }
    let mut d = Dec { x1: 0, x2: 0xFFFF_FFFF, x: 0, buf: blob[HEADER..].to_vec(), pos: 0 };
    for _ in 0..4 { d.x = (d.x << 8) | d.next_in(); }
    let mut cls = vec![0u8; c]; let mut csm = vec![SM_INIT; NCLS * 32]; code_classes(&mut d, &mut cls, &mut csm);
    let mut m = Model::new(c, h, w);
    for ch in 0..c { m.cls[ch] = cls[ch].max(1); m.s16[ch] = s16_of(cls[ch]); }
    let zeros = vec![0i16; c * h * w];
    m.run_groups(&mut d, &zeros, gm);
    Some((m.lat.iter().map(|&v| v as i16).collect(), c, h, w))
}
