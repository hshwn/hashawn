//! Learned mode ("tiny"): a photo -> a learned encoder's integer latents (outside this crate) -> this file.
//!
//! The latents are coded by NCZY (lat.rs) channel group by channel group; each group's mean and scale come from a
//! small conv net over the groups already decoded (bench/wonder/pred.py trains it). That net feeds the entropy
//! decoder, so it has to give the same numbers everywhere: plain f32, a fixed order of every sum, no fused multiply-add
//! (Rust never fuses), no exp/log (scale buckets by comparison against constants). Pixels are decoded outside this
//! crate (a small learned decoder); they need not be bit-exact, the latents are.
//!
//! Container: "NCZW" | version | quality | picture width u16 | height u16 | NCZY v2 stream.
use crate::lat::{self, GroupModel};

pub const MAGIC: &[u8; 4] = b"NCZW";
pub const VERSION: u8 = 1;
const HEAD: usize = 4 + 1 + 1 + 4;

// scale buckets: log(scale) against log((((11 << b) >> 2) - 0.5) / 64); scale >= 0.11
const LN_T: [f32; 23] = [-3.753418, -2.654806, -1.807508, -1.09083, -0.3861221, 0.3127557, 1.008756, 1.703327, 2.397185, 3.090687, 3.784012, 4.477248,
    5.17044, 5.863609, 6.556767, 7.24992, 7.94307, 8.636219, 9.329366, 10.02251, 10.71566, 11.40881, 12.10196];
const LN_MIN: f32 = -2.2072749; const LN_1_5: f32 = 0.4054651;
fn bucket(ls: f32) -> u8 { let ls = if ls > LN_MIN { ls } else { LN_MIN }; let mut b = 0; while b < 23 && ls >= LN_T[b] { b += 1; } b as u8 }

struct Conv { ci: usize, co: usize, k: usize, w: Vec<f32>, b: Vec<f32> }
impl Conv {
    /// x: ci planes of h*w -> co planes, zero padding. Sum order per output: bias, then input channel, then kernel row,
    /// column (fixed: this feeds the entropy decoder). Computed one output row at a time for a block of output channels,
    /// so the accumulators stay in cache; the order of every sum is the same as a plain plane-by-plane loop.
    fn run(&self, x: &[f32], h: usize, w: usize) -> Vec<f32> {
        let (ci, co, k) = (self.ci, self.co, self.k); let hw = h * w; let p = (k / 2) as isize;
        const OB: usize = 16;
        let mut out = vec![0f32; co * hw];
        let mut acc = vec![0f32; OB * w];
        for y in 0..h {
            for o0 in (0..co).step_by(OB) {
                let ob = OB.min(co - o0);
                for j in 0..ob { for v in &mut acc[j * w..(j + 1) * w] { *v = self.b[o0 + j]; } }
                for i in 0..ci {
                    let xi = &x[i * hw..(i + 1) * hw];
                    for ky in 0..k {
                        let sy = y as isize + ky as isize - p; if sy < 0 || sy >= h as isize { continue; }
                        let row = &xi[sy as usize * w..(sy as usize + 1) * w];
                        for kx in 0..k {
                            let dx = kx as isize - p;
                            let (x0, x1) = ((-dx).max(0) as usize, (w as isize - dx).min(w as isize) as usize);
                            let src = &row[(x0 as isize + dx) as usize..(x1 as isize + dx) as usize];
                            for j in 0..ob {
                                let wt = self.w[(((o0 + j) * ci + i) * k + ky) * k + kx];
                                for (a, s) in acc[j * w + x0..j * w + x1].iter_mut().zip(src) { *a += wt * *s; }
                            }
                        }
                    }
                }
                for j in 0..ob { out[(o0 + j) * hw + y * w..(o0 + j) * hw + (y + 1) * w].copy_from_slice(&acc[j * w..(j + 1) * w]); }
            }
        }
        out
    }
}
fn lrelu(v: &mut [f32]) { for x in v.iter_mut() { if *x < 0.0 { *x *= 0.1; } } }

enum Net { Bias(Vec<f32>), Conv { red: Conv, a: Conv, b: Conv, out: Conv } }

/// The group predictor. Weights file: "NCZP" | u16 C | u16 groups | u16 width | u16 group sizes | u16 channel order
/// (coding order -> channel) | per group f32 LE: bias (2g) or red(w x cin), red bias, a(w x w x 3 x 3), a bias, b, b bias,
/// out(2g x w x 3 x 3), out bias. Inputs: earlier groups in coding order, latents / 4.
pub struct Predictor { c: usize, groups: Vec<Vec<usize>>, order: Vec<usize>, nets: Vec<Net> }
impl Predictor {
    pub fn load(b: &[u8]) -> Option<Predictor> {
        if b.len() < 10 || &b[..4] != b"NCZP" { return None; }
        let mut i = 4;
        let mut u16_ = || -> Option<usize> { let v = u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?) as usize; i += 2; Some(v) };
        let (c, ng, wd) = (u16_()?, u16_()?, u16_()?);
        let sizes: Vec<usize> = (0..ng).map(|_| u16_()).collect::<Option<_>>()?;
        let order: Vec<usize> = (0..c).map(|_| u16_()).collect::<Option<_>>()?;
        if sizes.iter().sum::<usize>() != c || order.iter().any(|&o| o >= c) { return None; }
        let mut f = b.get(i..)?.chunks_exact(4).map(|x| f32::from_le_bytes([x[0], x[1], x[2], x[3]]));
        let mut take = |n: usize| -> Option<Vec<f32>> { let v: Vec<f32> = f.by_ref().take(n).collect(); if v.len() == n { Some(v) } else { None } };
        let mut nets = Vec::new(); let mut groups = Vec::new(); let mut st = 0;
        for &g in &sizes {
            groups.push(order[st..st + g].to_vec());
            if st == 0 { nets.push(Net::Bias(take(2 * g)?)); }
            else {
                let red = Conv { ci: st, co: wd, k: 1, w: take(wd * st)?, b: take(wd)? };
                let a = Conv { ci: wd, co: wd, k: 3, w: take(wd * wd * 9)?, b: take(wd)? };
                let bb = Conv { ci: wd, co: wd, k: 3, w: take(wd * wd * 9)?, b: take(wd)? };
                let out = Conv { ci: wd, co: 2 * g, k: 3, w: take(2 * g * wd * 9)?, b: take(2 * g)? };
                nets.push(Net::Conv { red, a, b: bb, out });
            }
            st += g;
        }
        Some(Predictor { c, groups, order, nets })
    }
    pub fn channels(&self) -> usize { self.c }
    /// Raw (mean, log-scale) for group g: 2 x g planes.
    fn raw(&self, lat: &[i32], h: usize, w: usize, g: usize) -> Vec<f32> {
        let hw = h * w; let st: usize = self.groups[..g].iter().map(|v| v.len()).sum(); let n = self.groups[g].len();
        match &self.nets[g] {
            Net::Bias(b) => { let mut o = vec![0f32; 2 * n * hw]; for k in 0..2 * n { for v in &mut o[k * hw..(k + 1) * hw] { *v = b[k]; } } o }
            Net::Conv { red, a, b, out } => {
                let mut x = vec![0f32; st * hw];
                for (j, &ch) in self.order[..st].iter().enumerate() { for p in 0..hw { x[j * hw + p] = lat[ch * hw + p] as f32 / 4.0; } }
                let mut t = red.run(&x, h, w); lrelu(&mut t);
                let mut u = a.run(&t, h, w); lrelu(&mut u); for (p, q) in t.iter_mut().zip(&u) { *p += *q; }
                let mut u = b.run(&t, h, w); lrelu(&mut u); for (p, q) in t.iter_mut().zip(&u) { *p += *q; }
                out.run(&t, h, w)
            }
        }
    }
}
impl GroupModel for Predictor {
    fn groups(&self) -> &[Vec<usize>] { &self.groups }
    fn predict(&self, lat: &[i32], _c: usize, h: usize, w: usize, g: usize) -> (Vec<i16>, Vec<u8>, Vec<u8>) {
        let hw = h * w; let n = self.groups[g].len(); let r = self.raw(lat, h, w, g);
        let mu = r[..n * hw].iter().map(|&m| (m * 16.0).round().clamp(-32767.0, 32767.0) as i16).collect();
        let qs = r[n * hw..].iter().map(|&l| bucket(l)).collect();
        let qs2 = r[n * hw..].iter().map(|&l| bucket(l + LN_1_5)).collect();
        (mu, qs, qs2)
    }
}

/// Compress latents (C x H x W, i16) of a `pw` x `ph` picture coded at quality `q` (the latents cover the picture
/// padded up to a multiple of 64 on each side).
pub fn compress(lat: &[i16], c: usize, h: usize, w: usize, pw: usize, ph: usize, q: u8, p: &Predictor) -> Option<Vec<u8>> {
    if c != p.channels() || pw > w * 16 || ph > h * 16 || pw + 64 <= w * 16 || ph + 64 <= h * 16 || pw > 65535 || ph > 65535 { return None; }
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC); out.push(VERSION); out.push(q);
    out.extend_from_slice(&(pw as u16).to_le_bytes()); out.extend_from_slice(&(ph as u16).to_le_bytes());
    out.extend_from_slice(&lat::compress_groups(lat, c, h, w, p)); Some(out)
}
/// (quality, picture width, height) of an NCZW stream, without decoding it.
pub fn info(b: &[u8]) -> Option<(u8, usize, usize)> {
    if b.len() < HEAD || &b[..4] != MAGIC || b[4] != VERSION { return None; }
    Some((b[5], u16::from_le_bytes([b[6], b[7]]) as usize, u16::from_le_bytes([b[8], b[9]]) as usize))
}
/// Returns (latents, c, h, w).
pub fn decompress(b: &[u8], p: &Predictor) -> Option<(Vec<i16>, usize, usize, usize)> { info(b)?; lat::decompress_groups(&b[HEAD..], p) }
