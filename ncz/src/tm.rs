//! The teacher's own entropy model, ported deterministically (tiny mode's latents at the teacher's own rate).
//!
//! cheng2020-anchor predicts each latent's mean and scale from a hyperprior (side information z, ~0.1 KB a photo) and
//! from the latents already decoded around it (a causal 5 x 5 context, all channels), position by position in raster
//! order. Its symbols are mean-centred: round(y - mean), rebuilt as y^ = symbol + mean. Through our coder (position-major
//! NCZY with its scales as context) that costs ~17% fewer bytes than our group predictor on round(y), at the same
//! picture quality (bench/results.md).
//!
//! The mean feeds the decoder and decides the encoder's symbols, so both sides must compute it bit for bit alike: plain
//! f32, every sum in a fixed order (8 running partial sums for the long dot products, combined in a fixed tree), no
//! fused multiply-add, no exp / log. The hyperprior's encoder half (h_a) runs with the picture encoder, outside this
//! crate: only its integer symbols arrive here.
//!
//! Weights: "NCZT" (bench/wonder/tm_export.py): u16 N | medians (N) | h_s: 3x3 N->N, N->4N + pixel shuffle, N->3N/2,
//! 3N/2->6N + shuffle, 3N/2->2N (leaky ReLU 0.01 between) | context: masked 5x5 N->2N, its 12 causal taps |
//! entropy parameters: 1x1 4N->10N/3->8N/3->2N (N scales, then N means). f32 LE, weight then bias per layer.
//! "NCZU": the same with every weight matrix 8-bit (per output row: f32 scale, then int8 levels; weight = level x
//! scale), medians and biases f32. 0.14% more bytes than f32 weights on Kodak, a quarter of the download.
//! Stream "NCZA": magic | version | z stream length u32 | z (NCZY v1, channel-major) | y symbols (NCZY v3, positions).
use crate::lat;
use crate::wonder::Conv;

pub const MAGIC: &[u8; 4] = b"NCZA";
pub const VERSION: u8 = 1;
const SCALE_MIN: f32 = 0.11;
const TAPS: [(isize, isize); 12] = [(-2, -2), (-2, -1), (-2, 0), (-2, 1), (-2, 2), (-1, -2), (-1, -1), (-1, 0), (-1, 1), (-1, 2), (0, -2), (0, -1)];

struct Dense { ci: usize, co: usize, w: Vec<f32>, b: Vec<f32> }
/// Sum of a[i] * b[i]: 8 running sums, combined in a fixed tree (vectorises; same result everywhere).
#[inline] fn dot(a: &[f32], b: &[f32]) -> f32 {
    let mut s = [0f32; 8]; let n = a.len() / 8 * 8;
    for (ca, cb) in a[..n].chunks_exact(8).zip(b[..n].chunks_exact(8)) { for l in 0..8 { s[l] += ca[l] * cb[l]; } }
    for i in n..a.len() { s[i - n] += a[i] * b[i]; }
    ((s[0] + s[1]) + (s[2] + s[3])) + ((s[4] + s[5]) + (s[6] + s[7]))
}
impl Dense { fn run(&self, x: &[f32], out: &mut [f32]) { for o in 0..self.co { out[o] = self.b[o] + dot(&self.w[o * self.ci..(o + 1) * self.ci], x); } } }
fn lrelu(v: &mut [f32]) { for x in v.iter_mut() { if *x < 0.0 { *x *= 0.01; } } }
/// (4c planes of h x w) -> (c planes of 2h x 2w), torch's PixelShuffle(2).
fn shuffle(x: &[f32], c: usize, h: usize, w: usize) -> Vec<f32> {
    let mut o = vec![0f32; x.len()]; let (h2, w2) = (2 * h, 2 * w);
    for ch in 0..c { for i in 0..2 { for j in 0..2 { let src = &x[((ch * 4 + i * 2 + j) * h) * w..((ch * 4 + i * 2 + j) * h + h) * w];
        for y in 0..h { for xx in 0..w { o[(ch * h2 + 2 * y + i) * w2 + 2 * xx + j] = src[y * w + xx]; } } } } }
    o
}

pub struct Tm { pub n: usize, pub medians: Vec<f32>, hs: Vec<Conv>, ctx: Dense, ep: [Dense; 3], pub id: u32 }
impl Tm {
    pub fn load(b: &[u8]) -> Option<Tm> {
        if b.len() < 6 || !(&b[..4] == b"NCZT" || &b[..4] == b"NCZU") { return None; }
        let n = u16::from_le_bytes([b[4], b[5]]) as usize;
        let mut r = Reader { b, at: 6, q8: &b[..4] == b"NCZU" };
        let medians = r.f32s(n)?;
        let mut hs = Vec::new();
        for (ci, co) in [(n, n), (n, 4 * n), (n, 3 * n / 2), (3 * n / 2, 6 * n), (3 * n / 2, 2 * n)] { let w = r.mat(co, ci * 9)?; let bb = r.f32s(co)?; hs.push(Conv { ci, co, k: 3, w, b: bb }); }
        let ctx = Dense { ci: n * 12, co: 2 * n, w: r.mat(2 * n, n * 12)?, b: r.f32s(2 * n)? };
        let (a1, a2) = (n * 10 / 3, n * 8 / 3);
        let mut d = |ci: usize, co: usize| -> Option<Dense> { Some(Dense { ci, co, w: r.mat(co, ci)?, b: r.f32s(co)? }) };
        let e1 = d(4 * n, a1)?; let e2 = d(a1, a2)?; let e3 = d(a2, 2 * n)?;
        if r.at != b.len() { return None; }
        // id: FNV-1a of the weights file (ties a stream to the exact model that made it, as wonder::Predictor's id)
        let mut id = 0x811c_9dc5u32; for &x in b { id = (id ^ x as u32).wrapping_mul(0x0100_0193); }
        Some(Tm { n, medians, hs, ctx, ep: [e1, e2, e3], id })
    }
    /// h_s of the hyperprior symbols (N planes of hz x wz): 2N planes at (4 hz) x (4 wz).
    fn params(&self, zq: &[i16], hz: usize, wz: usize) -> Vec<f32> {
        let n = self.n; let mut x: Vec<f32> = (0..n * hz * wz).map(|i| zq[i] as f32 + self.medians[i / (hz * wz)]).collect();
        let (mut h, mut w) = (hz, wz);
        for (l, c) in self.hs.iter().enumerate() {
            x = c.run(&x, h, w);
            if l == 1 || l == 3 { x = shuffle(&x, c.co / 4, h, w); h *= 2; w *= 2; }
            if l < 4 { lrelu(&mut x); }
        }
        x
    }
    /// Mean and scale of every channel at (px, py), from the hyperprior params there and the causal neighbourhood of
    /// y^ (N planes of h x w, decoded so far; the rest is never read).
    fn at(&self, params: &[f32], yh: &[f32], h: usize, w: usize, px: usize, py: usize, buf: &mut Buf) -> () {
        let n = self.n; let hw = h * w;
        for (t, &(dy, dx)) in TAPS.iter().enumerate() {
            let (yy, xx) = (py as isize + dy, px as isize + dx);
            let ok = yy >= 0 && xx >= 0 && xx < w as isize;
            for i in 0..n { buf.nb[i * 12 + t] = if ok { yh[i * hw + yy as usize * w + xx as usize] } else { 0.0 }; }
        }
        for c in 0..2 * n { buf.e0[c] = params[c * hw + py * w + px]; }
        self.ctx.run(&buf.nb, &mut buf.e0[2 * n..]);
        self.ep[0].run(&buf.e0, &mut buf.e1); lrelu(&mut buf.e1);
        self.ep[1].run(&buf.e1, &mut buf.e2); lrelu(&mut buf.e2);
        self.ep[2].run(&buf.e2, &mut buf.out);
    }
}
struct Reader<'a> { b: &'a [u8], at: usize, q8: bool }
impl Reader<'_> {
    fn f32s(&mut self, k: usize) -> Option<Vec<f32>> { let v = self.b.get(self.at..self.at + 4 * k)?.chunks_exact(4).map(|x| f32::from_le_bytes([x[0], x[1], x[2], x[3]])).collect(); self.at += 4 * k; Some(v) }
    fn mat(&mut self, rows: usize, cols: usize) -> Option<Vec<f32>> {
        if !self.q8 { return self.f32s(rows * cols); }
        let sc = self.f32s(rows)?; let lv = self.b.get(self.at..self.at + rows * cols)?; self.at += rows * cols;
        Some((0..rows * cols).map(|i| (lv[i] as i8) as f32 * sc[i / cols]).collect())
    }
}
struct Buf { nb: Vec<f32>, e0: Vec<f32>, e1: Vec<f32>, e2: Vec<f32>, out: Vec<f32> }
impl Buf { fn new(tm: &Tm) -> Buf { let n = tm.n; Buf { nb: vec![0.0; 12 * n], e0: vec![0.0; 4 * n], e1: vec![0.0; tm.ep[0].co], e2: vec![0.0; tm.ep[1].co], out: vec![0.0; 2 * n] } } }
fn sc64(s: f32) -> u16 { (s.max(SCALE_MIN) * 64.0 + 0.5).min(65535.0) as u16 }

/// The hyperprior's symbols from its encoder's float output z (N x hz x wz): round(z - median).
pub fn z_symbols(z: &[f32], hz: usize, wz: usize, tm: &Tm) -> Vec<i16> {
    let k = hz * wz; (0..z.len()).map(|i| (z[i] - tm.medians[i / k]).round().clamp(-32767.0, 32767.0) as i16).collect()
}
/// Code latents y (N x h x w, f32 from the encoder network) and the hyperprior's symbols (N x h/4 x w/4). Returns the
/// stream and y^ (what the decoder will rebuild: the picture decoder's input).
pub fn compress(y: &[f32], h: usize, w: usize, zq: &[i16], tm: &Tm) -> Option<(Vec<u8>, Vec<f32>)> {
    let n = tm.n; let hw = h * w;
    if y.len() != n * hw || h % 4 != 0 || w % 4 != 0 || zq.len() != n * hw / 16 { return None; }
    let params = tm.params(zq, h / 4, w / 4); let mut buf = Buf::new(tm);
    let mut yh = vec![0f32; n * hw]; let mut sym = vec![0i16; n * hw]; let mut sc = vec![0u16; n * hw];
    for py in 0..h { for px in 0..w {
        tm.at(&params, &yh, h, w, px, py, &mut buf); let p = py * w + px;
        for c in 0..n {
            let (s, m) = (buf.out[c], buf.out[n + c]); let i = c * hw + p;
            let q = (y[i] - m).round().clamp(-32767.0, 32767.0);
            sym[i] = q as i16; yh[i] = q + m; sc[i] = sc64(s);
        }
    } }
    let zs = lat::compress(zq, n, h / 4, w / 4, None); let ys = lat::compress_positions(&sym, n, h, w, &sc);
    let mut out = Vec::with_capacity(9 + zs.len() + ys.len());
    out.extend_from_slice(MAGIC); out.push(VERSION); out.extend_from_slice(&(zs.len() as u32).to_le_bytes());
    out.extend_from_slice(&zs); out.extend_from_slice(&ys);
    Some((out, yh))
}
/// y^ (N x h x w) and (N, h, w) from a stream.
pub fn decompress(b: &[u8], tm: &Tm) -> Option<(Vec<f32>, usize, usize, usize)> {
    if b.len() < 9 || &b[..4] != MAGIC || b[4] != VERSION { return None; }
    let zn = u32::from_le_bytes(b[5..9].try_into().ok()?) as usize;
    let (zq, zc, hz, wz) = lat::decompress(b.get(9..9 + zn)?, None)?;
    let mut d = lat::PosDecoder::new(b.get(9 + zn..)?)?;
    let (n, h, w) = (tm.n, d.h, d.w);
    if zc != n || d.c != n || h != 4 * hz || w != 4 * wz { return None; }
    let params = tm.params(&zq, hz, wz); let mut buf = Buf::new(tm); let hw = h * w;
    let mut yh = vec![0f32; n * hw]; let mut sc = vec![0u16; n];
    for py in 0..h { for px in 0..w {
        tm.at(&params, &yh, h, w, px, py, &mut buf);
        for c in 0..n { sc[c] = sc64(buf.out[c]); }
        let s = d.at(px, py, &sc); let p = py * w + px;
        for c in 0..n { yh[c * hw + p] = s[c] as f32 + buf.out[n + c]; }
    } }
    Some((yh, n, h, w))
}
