//! The token-end flag, predicted by more than the model.
//!
//! Before every byte inside a token the codec codes "does the token end here?". The model's own
//! P(token = tail) / P(token starts with tail) is a good input, but tokenisation is also a very regular
//! function of the bytes: " the" ends here far more often than " th". So: a tiny logistic mixer over the
//! model's estimate plus adaptive counters keyed by the tail bytes, the tail and the byte before the token,
//! and the tail length with its last two bytes. Weights start at "the model only" and learn online.
//! All arithmetic is f32 in fixed order through `simd::{fexp, fln}`: identical on every machine.
use crate::simd::{fexp, fln};

const N_IN: usize = 6;
const TBITS: usize = 16;
const LIMIT: u8 = 255;
pub const LR: f32 = 0.005;

#[derive(Clone, Copy)] struct Cell { p: f32, n: u8 }
struct Table { c: Vec<Cell> }
impl Table {
    fn new() -> Table { Table { c: vec![Cell { p: 0.5, n: 0 }; 1 << TBITS] } }
    #[inline] fn update(&mut self, i: usize, y: f32) { let c = &mut self.c[i]; c.p += (y - c.p) / (c.n as f32 + 1.5); if c.n < LIMIT { c.n += 1; } }
}

pub struct FlagModel { t: [Table; 3], sse: Vec<Cell>, w: Vec<f32>, lr: f32, x: [f32; N_IN], idx: [usize; 3], sse_i: usize, set: usize, p: f32 }

#[inline] fn stretch(p: f32) -> f32 { let p = p.clamp(1e-5, 1.0 - 1e-5); fln(p / (1.0 - p)) }
#[inline] fn squash(x: f32) -> f32 { 1.0 / (1.0 + fexp(-x)) }
#[inline] fn hash(seed: u32, bytes: &[u8]) -> usize { let mut h = 0x811c9dc5u32 ^ seed.wrapping_mul(0x9e3779b1); for &b in bytes { h = (h ^ b as u32).wrapping_mul(0x01000193); } h ^= h >> 15; h = h.wrapping_mul(0x2c1b3c6d); h ^= h >> 12; (h as usize) & ((1 << TBITS) - 1) }

impl Default for FlagModel { fn default() -> Self { FlagModel::new() } }
impl FlagModel {
    pub fn new() -> FlagModel { FlagModel::with_lr(LR) }
    pub fn with_lr(lr: f32) -> FlagModel {
        let sets = 8 * 4;
        let mut w = vec![0f32; sets * N_IN];
        for s in 0..sets { w[s * N_IN] = 1.0; }   // trust the model until taught otherwise
        FlagModel { t: [Table::new(), Table::new(), Table::new()], sse: vec![Cell { p: 0.5, n: 0 }; 33 * 8], w, lr, x: [0.0; N_IN], idx: [0; 3], sse_i: 0, set: 0, p: 0.5 }
    }
    /// P(the token ends after `tail`), given the model's estimate and the byte before the token. Remembers its inputs for `update`.
    pub fn p(&mut self, p_model: f64, tail: &[u8], prev: u8) -> f64 {
        let len = tail.len().clamp(1, 8);
        let sm = stretch(p_model as f32);
        self.idx[0] = hash(1, tail);
        self.idx[1] = hash(2 + prev as u32 * 4, tail);
        let last2 = &tail[tail.len().saturating_sub(2)..];
        self.idx[2] = hash(3 + len as u32 * 8, last2);
        let bucket = ((sm + 8.0) * 2.0).clamp(0.0, 32.0) as usize;     // 33 buckets over [-8, 8]
        self.sse_i = bucket * 8 + (len - 1).min(7);
        self.x = [sm, stretch(self.t[0].c[self.idx[0]].p), stretch(self.t[1].c[self.idx[1]].p), stretch(self.t[2].c[self.idx[2]].p), stretch(self.sse[self.sse_i].p), 0.3];
        self.set = (len - 1).min(7) * 4 + if sm < -2.0 { 0 } else if sm < 0.0 { 1 } else if sm < 2.0 { 2 } else { 3 };
        let w = &self.w[self.set * N_IN..(self.set + 1) * N_IN];
        let mut dot = 0f32; for i in 0..N_IN { dot += w[i] * self.x[i]; }
        self.p = squash(dot).clamp(1.0 / 4096.0, 1.0 - 1.0 / 4096.0);
        self.p as f64
    }
    pub fn update(&mut self, ended: bool) {
        let y = if ended { 1.0 } else { 0.0 };
        let err = (y - self.p) * self.lr;
        let w = &mut self.w[self.set * N_IN..(self.set + 1) * N_IN];
        for i in 0..N_IN { w[i] += err * self.x[i]; }
        for k in 0..3 { self.t[k].update(self.idx[k], y); }
        let c = &mut self.sse[self.sse_i]; c.p += (y - c.p) / (c.n as f32 + 1.5); if c.n < LIMIT { c.n += 1; }
    }
}
