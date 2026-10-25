//! Learned mode ("tiny"): a photo -> a learned encoder's integer latents (outside this crate) -> this file.
//!
//! The latents are coded by NCZY (lat.rs) channel group by channel group; each group's mean and scale come from a
//! small conv net over the groups already decoded (bench/wonder/pred.py trains it). That net feeds the entropy
//! decoder, so it has to give the same numbers everywhere: plain f32, a fixed order of every sum, no fused multiply-add
//! (Rust never fuses), no exp/log (scale buckets by comparison against constants). Pixels are decoded outside this
//! crate (a small learned decoder); they need not be bit-exact, the latents are.
//!
//! Container: "NCZW" | version | quality | picture width u16 | height u16 | predictor id u32 | grain length u32 |
//! grain fingerprint (grain::measure; 0 bytes = none) | NCZY v2 stream. The decoder regrows the photo's grain from the
//! fingerprint on top of the decoded picture (the learned decoder smooths it away). Version 2 (no grain) still decodes.
//! Version 4 adds a detail map (length u32 + bytes) after the grain, version 5 a swatch section (swatch.rs; length u32 +
//! bytes) after that. Version 6 ends with a CRC-32 (LE) of every byte before it: a damaged or cut-short file is
//! refused instead of decoding to garbage.
//! The predictor id (FNV-1a of its file) ties a file to the exact predictor that made it: another one would decode
//! garbage, so the decoder refuses.
use crate::lat::{self, GroupModel};

pub const MAGIC: &[u8; 4] = b"NCZW";
pub const VERSION: u8 = 7;      // 7: latents may be an NCZA stream (tm.rs); 6: + CRC-32; 5: + swatch section; 4: + detail map; 3: + grain; 2: none (all still decode)
const HEAD2: usize = 4 + 1 + 1 + 4 + 4;

// scale buckets: log(scale) against log((((11 << b) >> 2) - 0.5) / 64); scale >= 0.11
const LN_T: [f32; 23] = [-3.753418, -2.654806, -1.807508, -1.09083, -0.3861221, 0.3127557, 1.008756, 1.703327, 2.397185, 3.090687, 3.784012, 4.477248,
    5.17044, 5.863609, 6.556767, 7.24992, 7.94307, 8.636219, 9.329366, 10.02251, 10.71566, 11.40881, 12.10196];
const LN_MIN: f32 = -2.2072749; const LN_1_5: f32 = 0.4054651;
fn bucket(ls: f32) -> u8 { let ls = if ls > LN_MIN { ls } else { LN_MIN }; let mut b = 0; while b < 23 && ls >= LN_T[b] { b += 1; } b as u8 }

pub(crate) struct Conv { pub(crate) ci: usize, pub(crate) co: usize, pub(crate) k: usize, pub(crate) w: Vec<f32>, pub(crate) b: Vec<f32> }
impl Conv {
    /// x: ci planes of h*w -> co planes, zero padding. Sum order per output: bias, then input channel, then kernel row,
    /// column (fixed: this feeds the entropy decoder). Computed one output row at a time for a block of output channels,
    /// so the accumulators stay in cache; the order of every sum is the same as a plain plane-by-plane loop.
    pub(crate) fn run(&self, x: &[f32], h: usize, w: usize) -> Vec<f32> {
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

struct Reader<'a> { b: &'a [u8], at: usize }
impl Reader<'_> {
    fn u16(&mut self) -> Option<usize> { let v = u16::from_le_bytes(self.b.get(self.at..self.at + 2)?.try_into().ok()?) as usize; self.at += 2; Some(v) }
    fn f32s(&mut self, n: usize) -> Option<Vec<f32>> {
        let v = self.b.get(self.at..self.at + 4 * n)?.chunks_exact(4).map(|x| f32::from_le_bytes([x[0], x[1], x[2], x[3]])).collect(); self.at += 4 * n; Some(v)
    }
    /// A rows x cols weight matrix: f32, or (q8) per-row scales then int8 levels, weight = level * scale.
    fn mat(&mut self, rows: usize, cols: usize, q8: bool) -> Option<Vec<f32>> {
        if !q8 { return self.f32s(rows * cols); }
        let sc = self.f32s(rows)?; let lv = self.b.get(self.at..self.at + rows * cols)?; self.at += rows * cols;
        Some((0..rows * cols).map(|i| (lv[i] as i8) as f32 * sc[i / cols]).collect())
    }
}

/// The group predictor. Weights file: "NCZP" | u16 C | u16 groups | u16 width | u16 group sizes | u16 channel order
/// (coding order -> channel) | per group f32 LE: bias (2g) or red(w x cin), red bias, a(w x w x 3 x 3), a bias, b, b bias,
/// out(2g x w x 3 x 3), out bias. Inputs: earlier groups in coding order, latents / 4.
/// "NCZC" / "NCZD" (f32 / 8-bit): two-pass checkerboard. Pass 1 codes the anchor half ((x + y) even) of every channel,
/// group by group; pass 2 the rest. After the header: pass-1 nets for every group (input: anchors of the earlier groups,
/// then the anchor mask; red is w x (st + 1)), then pass-2 nets (input: earlier groups complete, the group's and later
/// groups' anchors, then the mask; red is w x (C + 1)). Inputs / 4, the mask too (as trained).
/// "NCZQ": the same with every weight matrix stored as 8 bits: per row an f32 scale, then the rows' int8 levels; a
/// weight is level * scale (one f32 multiply, the same everywhere). Biases stay f32. A quarter of the size.
pub struct Predictor { c: usize, groups: Vec<Vec<usize>>, order: Vec<usize>, nets: Vec<Net>, nets2: Vec<Net>, cb: bool, pub id: u32 }
impl Predictor {
    pub fn load(b: &[u8]) -> Option<Predictor> {
        let magic = b.get(..4)?;
        if b.len() < 10 || !matches!(magic, b"NCZP" | b"NCZQ" | b"NCZC" | b"NCZD") { return None; }
        let q8 = magic == b"NCZQ" || magic == b"NCZD"; let cb = magic == b"NCZC" || magic == b"NCZD";
        let mut r = Reader { b, at: 4 };
        let (c, ng, wd) = (r.u16()?, r.u16()?, r.u16()?);
        let sizes: Vec<usize> = (0..ng).map(|_| r.u16()).collect::<Option<_>>()?;
        let order: Vec<usize> = (0..c).map(|_| r.u16()).collect::<Option<_>>()?;
        if sizes.iter().sum::<usize>() != c || order.iter().any(|&o| o >= c) { return None; }
        let mut nets = Vec::new(); let mut nets2 = Vec::new(); let mut groups = Vec::new(); let mut st = 0;
        let conv = |r: &mut Reader, cin: usize, g: usize| -> Option<Net> {
            let red = Conv { ci: cin, co: wd, k: 1, w: r.mat(wd, cin, q8)?, b: r.f32s(wd)? };
            let a = Conv { ci: wd, co: wd, k: 3, w: r.mat(wd, wd * 9, q8)?, b: r.f32s(wd)? };
            let bb = Conv { ci: wd, co: wd, k: 3, w: r.mat(wd, wd * 9, q8)?, b: r.f32s(wd)? };
            let out = Conv { ci: wd, co: 2 * g, k: 3, w: r.mat(2 * g, wd * 9, q8)?, b: r.f32s(2 * g)? };
            Some(Net::Conv { red, a, b: bb, out })
        };
        if cb {
            for &g in &sizes { groups.push(order[st..st + g].to_vec()); nets.push(conv(&mut r, st + 1, g)?); st += g; }
            for &g in &sizes { nets2.push(conv(&mut r, c + 1, g)?); }
        }
        for &g in if cb { &[][..] } else { &sizes[..] } {
            groups.push(order[st..st + g].to_vec());
            if st == 0 { nets.push(Net::Bias(r.f32s(2 * g)?)); }
            else {
                let red = Conv { ci: st, co: wd, k: 1, w: r.mat(wd, st, q8)?, b: r.f32s(wd)? };
                let a = Conv { ci: wd, co: wd, k: 3, w: r.mat(wd, wd * 9, q8)?, b: r.f32s(wd)? };
                let bb = Conv { ci: wd, co: wd, k: 3, w: r.mat(wd, wd * 9, q8)?, b: r.f32s(wd)? };
                let out = Conv { ci: wd, co: 2 * g, k: 3, w: r.mat(2 * g, wd * 9, q8)?, b: r.f32s(2 * g)? };
                nets.push(Net::Conv { red, a, b: bb, out });
            }
            st += g;
        }
        if r.at != b.len() { return None; }
        let mut id = 0x811c_9dc5u32; for &x in b { id = (id ^ x as u32).wrapping_mul(0x0100_0193); }
        Some(Predictor { c, groups, order, nets, nets2, cb, id })
    }
    pub fn channels(&self) -> usize { self.c }
    /// Raw (mean, log-scale) for group g: 2 x g planes.
    fn raw(&self, lat: &[i32], h: usize, w: usize, g: usize, pass: usize) -> Vec<f32> {
        let hw = h * w; let st: usize = self.groups[..g].iter().map(|v| v.len()).sum(); let n = self.groups[g].len();
        let net = if pass == 0 { &self.nets[g] } else { &self.nets2[g] };
        match net {
            Net::Bias(b) => { let mut o = vec![0f32; 2 * n * hw]; for k in 0..2 * n { for v in &mut o[k * hw..(k + 1) * hw] { *v = b[k]; } } o }
            Net::Conv { red, a, b, out } => {
                let anchor = |p: usize| (p / w + p % w) % 2 == 0;
                let x = if !self.cb {
                    let mut x = vec![0f32; st * hw];
                    for (j, &ch) in self.order[..st].iter().enumerate() { for p in 0..hw { x[j * hw + p] = lat[ch * hw + p] as f32 / 4.0; } }
                    x
                } else {
                    // pass 0: anchors of groups < g, mask; pass 1: groups < g whole, the rest's anchors, mask
                    let nin = if pass == 0 { st } else { self.c };
                    let mut x = vec![0f32; (nin + 1) * hw];
                    for (j, &ch) in self.order[..nin].iter().enumerate() {
                        for p in 0..hw { if (pass == 1 && j < st) || anchor(p) { x[j * hw + p] = lat[ch * hw + p] as f32 / 4.0; } }
                    }
                    for p in 0..hw { if anchor(p) { x[nin * hw + p] = 0.25; } }
                    x
                };
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
    fn passes(&self) -> usize { if self.cb { 2 } else { 1 } }
    fn predict(&self, lat: &[i32], _c: usize, h: usize, w: usize, g: usize, pass: usize) -> (Vec<i16>, Vec<u8>, Vec<u8>) {
        let hw = h * w; let n = self.groups[g].len(); let r = self.raw(lat, h, w, g, pass);
        let mu = r[..n * hw].iter().map(|&m| (m * 16.0).round().clamp(-32767.0, 32767.0) as i16).collect();
        let qs = r[n * hw..].iter().map(|&l| bucket(l)).collect();
        let qs2 = r[n * hw..].iter().map(|&l| bucket(l + LN_1_5)).collect();
        (mu, qs, qs2)
    }
}

/// Compress latents (C x H x W, i16) of a `pw` x `ph` picture coded at quality `q` (the latents cover the picture
/// padded up to a multiple of 64 on each side).
pub fn compress(lat: &[i16], c: usize, h: usize, w: usize, pw: usize, ph: usize, q: u8, p: &Predictor) -> Option<Vec<u8>> { compress_grain(lat, c, h, w, pw, ph, q, p, &[]) }
/// With a grain fingerprint (grain::measure of the original picture; empty: none).
#[allow(clippy::too_many_arguments)]
pub fn compress_grain(lat: &[i16], c: usize, h: usize, w: usize, pw: usize, ph: usize, q: u8, p: &Predictor, grain: &[u8]) -> Option<Vec<u8>> {
    compress_full(lat, c, h, w, pw, ph, q, p, grain, &[])
}
/// With a grain fingerprint and a detail map (the original's fine-detail energy per 32 x 32 block, for the texture
/// decoder; opaque bytes here, empty: none).
#[allow(clippy::too_many_arguments)]
pub fn compress_full(lat: &[i16], c: usize, h: usize, w: usize, pw: usize, ph: usize, q: u8, p: &Predictor, grain: &[u8], dmap: &[u8]) -> Option<Vec<u8>> {
    compress_swatch(lat, c, h, w, pw, ph, q, p, grain, dmap, &[])
}
/// With a grain fingerprint, a detail map and a swatch section (swatch::encode; empty: none).
#[allow(clippy::too_many_arguments)]
pub fn compress_swatch(lat: &[i16], c: usize, h: usize, w: usize, pw: usize, ph: usize, q: u8, p: &Predictor, grain: &[u8], dmap: &[u8], sw: &[u8]) -> Option<Vec<u8>> {
    if c != p.channels() || !fits(c, pw, ph) || pw > w * 16 || ph > h * 16 || pw + 64 <= w * 16 || ph + 64 <= h * 16 { return None; }
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC); out.push(VERSION); out.push(q);
    out.extend_from_slice(&(pw as u16).to_le_bytes()); out.extend_from_slice(&(ph as u16).to_le_bytes());
    out.extend_from_slice(&p.id.to_le_bytes());
    out.extend_from_slice(&(grain.len() as u32).to_le_bytes()); out.extend_from_slice(grain);
    out.extend_from_slice(&(dmap.len() as u32).to_le_bytes()); out.extend_from_slice(dmap);
    out.extend_from_slice(&(sw.len() as u32).to_le_bytes()); out.extend_from_slice(sw);
    out.extend_from_slice(&lat::compress_groups(lat, c, h, w, p));
    let crc = crate::pngc::crc32(&out); out.extend_from_slice(&crc.to_le_bytes()); Some(out)
}
/// Version 7, the teacher's own entropy model (tm.rs): float latents y (C x H x W from the encoder network) and the
/// hyperprior's symbols; the latent section is an NCZA stream and the id field holds the model's weights id. Returns
/// the stream and y^, the mean-centred latents the decoder will rebuild (the picture decoder's input).
#[allow(clippy::too_many_arguments)]
pub fn compress_tm(y: &[f32], c: usize, h: usize, w: usize, pw: usize, ph: usize, q: u8, tm: &crate::tm::Tm, zq: &[i16], grain: &[u8], dmap: &[u8], sw: &[u8]) -> Option<(Vec<u8>, Vec<f32>)> {
    if c != tm.n || !fits(c, pw, ph) || pw > w * 16 || ph > h * 16 || pw + 64 <= w * 16 || ph + 64 <= h * 16 { return None; }
    let (ys, yh) = crate::tm::compress(y, h, w, zq, tm)?;
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC); out.push(VERSION); out.push(q);
    out.extend_from_slice(&(pw as u16).to_le_bytes()); out.extend_from_slice(&(ph as u16).to_le_bytes());
    out.extend_from_slice(&tm.id.to_le_bytes());
    out.extend_from_slice(&(grain.len() as u32).to_le_bytes()); out.extend_from_slice(grain);
    out.extend_from_slice(&(dmap.len() as u32).to_le_bytes()); out.extend_from_slice(dmap);
    out.extend_from_slice(&(sw.len() as u32).to_le_bytes()); out.extend_from_slice(sw);
    out.extend_from_slice(&ys);
    let crc = crate::pngc::crc32(&out); out.extend_from_slice(&crc.to_le_bytes()); Some((out, yh))
}
/// The same stream with its swatch section replaced (the encoder builds swatches from the picture the latents decode
/// to, so they come after the latents are coded). Written as the stream's own version, CRC recomputed (version 6 on).
pub fn with_swatches(b: &[u8], sw: &[u8]) -> Option<Vec<u8>> {
    let (grain, dmap, _, lat) = parts(b)?; let v = b[4]; if v < 5 { return None; }
    let mut out = b[..HEAD2].to_vec();
    out.extend_from_slice(&(grain.len() as u32).to_le_bytes()); out.extend_from_slice(grain);
    out.extend_from_slice(&(dmap.len() as u32).to_le_bytes()); out.extend_from_slice(dmap);
    out.extend_from_slice(&(sw.len() as u32).to_le_bytes()); out.extend_from_slice(sw);
    out.extend_from_slice(lat);
    if v >= 6 { let crc = crate::pngc::crc32(&out); out.extend_from_slice(&crc.to_le_bytes()); }
    Some(out)
}
/// Is the latent section an NCZA stream (decode with decompress_tm)?
pub fn uses_tm(b: &[u8]) -> bool { parts(b).is_some_and(|p| p.3.starts_with(crate::tm::MAGIC)) }
/// y^ (C x H x W) and (C, H, W) from a version-7 NCZA stream. None if damaged or made with other weights.
pub fn decompress_tm(b: &[u8], tm: &crate::tm::Tm) -> Option<(Vec<f32>, usize, usize, usize)> {
    let (_, _, _, id) = info(b)?; if id != tm.id { return None; }
    crate::tm::decompress(parts(b)?.3, tm)
}
/// Can a `pw` x `ph` picture with `c` latent channels be stored (and so opened again)? The decoder refuses more than
/// lat::MAX_VALUES latents; the picture is padded to a multiple of 64 px, a latent per 16 x 16 px.
pub fn fits(c: usize, pw: usize, ph: usize) -> bool {
    pw > 0 && ph > 0 && pw <= 65535 && ph <= 65535 && c * pw.div_ceil(64) * 4 * ph.div_ceil(64) * 4 <= lat::MAX_VALUES
}
/// What a reader can say about a stream before decoding it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Check { Ok = 0, NotMine = 1, Newer = 2, Damaged = 3 }
/// Is this an NCZW stream this reader can open, and is it whole (sections in bounds; version 6: the CRC matches)?
pub fn check(b: &[u8]) -> Check {
    if b.len() < 5 || &b[..4] != MAGIC || b[4] < 2 { return Check::NotMine; }
    if b[4] > VERSION { return Check::Newer; }
    if parts(b).is_none() { return Check::Damaged; }
    Check::Ok
}
/// (quality, picture width, height, predictor id) of an NCZW stream, without decoding it.
pub fn info(b: &[u8]) -> Option<(u8, usize, usize, u32)> {
    if b.len() < HEAD2 || &b[..4] != MAGIC || !(2..=VERSION).contains(&b[4]) || (b[4] >= 6 && b.len() < HEAD2 + 4) { return None; }
    Some((b[5], u16::from_le_bytes([b[6], b[7]]) as usize, u16::from_le_bytes([b[8], b[9]]) as usize, u32::from_le_bytes(b[10..14].try_into().ok()?)))
}
/// (grain fingerprint, detail map, swatch section, latent stream) parts of an NCZW stream. Version 6: None unless the
/// CRC matches; the parts are laid out as in version 5.
fn parts(b: &[u8]) -> Option<(&[u8], &[u8], &[u8], &[u8])> {
    info(b)?; let e = &b[..0];
    let b = if b[4] < 6 { b } else {
        let (body, crc) = b.split_at(b.len() - 4);
        if crate::pngc::crc32(body) != u32::from_le_bytes(crc.try_into().ok()?) { return None; }
        body
    };
    if b[4] == 2 { return Some((e, e, e, &b[HEAD2..])); }
    let n = u32::from_le_bytes(b.get(HEAD2..HEAD2 + 4)?.try_into().ok()?) as usize; let g0 = HEAD2 + 4;
    let grain = b.get(g0..g0 + n)?;
    if b[4] == 3 { return Some((grain, e, e, b.get(g0 + n..)?)); }
    let m0 = g0 + n; let k = u32::from_le_bytes(b.get(m0..m0 + 4)?.try_into().ok()?) as usize;
    let dmap = b.get(m0 + 4..m0 + 4 + k)?;
    if b[4] == 4 { return Some((grain, dmap, e, b.get(m0 + 4 + k..)?)); }
    let s0 = m0 + 4 + k; let j = u32::from_le_bytes(b.get(s0..s0 + 4)?.try_into().ok()?) as usize;
    Some((grain, dmap, b.get(s0 + 4..s0 + 4 + j)?, b.get(s0 + 4 + j..)?))
}
/// The grain fingerprint stored in a stream (empty if none).
pub fn grain(b: &[u8]) -> Option<&[u8]> { parts(b).map(|p| p.0) }
/// The detail map stored in a stream (empty if none).
pub fn detail_map(b: &[u8]) -> Option<&[u8]> { parts(b).map(|p| p.1) }
/// The swatch section stored in a stream (empty if none).
pub fn swatches(b: &[u8]) -> Option<&[u8]> { parts(b).map(|p| p.2) }
/// Returns (latents, c, h, w). None if damaged, or made with a different predictor.
pub fn decompress(b: &[u8], p: &Predictor) -> Option<(Vec<i16>, usize, usize, usize)> {
    let (_, pw, ph, id) = info(b)?; if id != p.id { return None; }
    let (lat, c, h, w) = lat::decompress_groups(parts(b)?.3, p)?;
    if pw > w * 16 || ph > h * 16 { return None; }        // the latents must cover the picture (else: a damaged header)
    Some((lat, c, h, w))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// A one-group predictor (a bias: mean 0, log-scale 1 everywhere) over `c` channels.
    fn tiny_predictor(c: u16) -> Predictor {
        let mut b = b"NCZP".to_vec();
        for v in [c, 1, 1, c] { b.extend_from_slice(&v.to_le_bytes()); }
        for ch in 0..c { b.extend_from_slice(&ch.to_le_bytes()); }
        for k in 0..2 * c { b.extend_from_slice(&(if k < c { 0.0f32 } else { 1.0 }).to_le_bytes()); }
        Predictor::load(&b).unwrap()
    }
    fn sample() -> (Predictor, Vec<i16>, Vec<u8>) {
        let p = tiny_predictor(4); let (c, h, w) = (4, 8, 8);
        let mut x = 12345u32;
        let lat: Vec<i16> = (0..c * h * w).map(|_| { x = x.wrapping_mul(1_103_515_245).wrapping_add(12345); ((x >> 16) % 9) as i16 - 4 }).collect();
        let z = compress_swatch(&lat, c, h, w, 120, 100, 1, &p, b"grain", &[], b"swatches").unwrap();
        (p, lat, z)
    }
    #[test] fn v6_round_trip() {
        let (p, lat, z) = sample();
        assert_eq!(z[4], 6); assert_eq!(check(&z), Check::Ok);
        assert_eq!(info(&z), Some((1, 120, 100, p.id)));
        assert_eq!(grain(&z), Some(&b"grain"[..])); assert_eq!(swatches(&z), Some(&b"swatches"[..]));
        assert_eq!(decompress(&z, &p), Some((lat, 4, 8, 8)));
    }
    #[test] fn v6_damaged_byte_is_refused() {
        let (p, _, z) = sample();
        for i in [14, 20, z.len() / 2, z.len() - 6, z.len() - 1] {      // grain length, grain, latents, latent tail, the CRC
            let mut bad = z.clone(); bad[i] ^= 0x10;
            assert_eq!(check(&bad), Check::Damaged, "byte {i}"); assert!(decompress(&bad, &p).is_none(), "byte {i}");
        }
    }
    #[test] fn truncated_is_refused() {
        let (p, lat, z) = sample();
        // version 5 (no CRC): the same file without its last 4 bytes still decodes, and cut short it is refused
        let mut v5 = z[..z.len() - 4].to_vec(); v5[4] = 5;
        assert_eq!(check(&v5), Check::Ok); assert_eq!(decompress(&v5, &p), Some((lat, 4, 8, 8)));
        for cut in 1..12 {
            assert!(decompress(&z[..z.len() - cut], &p).is_none(), "v6 cut {cut}");
            assert!(decompress(&v5[..v5.len() - cut], &p).is_none(), "v5 cut {cut}");
        }
    }
    #[test] fn too_big_is_refused_when_writing() {
        assert!(fits(192, 4096, 4096) && fits(192, 9000, 9000) && !fits(192, 12000, 12000) && !fits(4, 65536, 64));
        let p = tiny_predictor(8);           // 8 x 4096 x 4096 latents: twice what the decoder takes
        assert!(compress_swatch(&[], 8, 4096, 4096, 65535, 65535, 1, &p, &[], &[], &[]).is_none());
    }
    #[test] fn other_files_are_not_mine() {
        assert_eq!(check(b"NCZL\x02\x00"), Check::NotMine); assert_eq!(check(b"NCZW\x07rest"), Check::Newer);
    }
}
