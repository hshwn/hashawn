//! Swatches: real texture for tiny mode, instead of invented texture.
//!
//! At ~0.1 bits per pixel the latents keep a photo's shapes and colours but not its fine texture, and a decoder that
//! invents texture draws its statistics (speckle). Instead the file carries a few small grey samples of the photo's
//! own textures ("swatches", 48 x 48, transform coded) and, per 16 x 16 block, what fills in the detail lost there: a
//! swatch, grain, or nothing. The decoder lays the swatch's fine detail (picture minus its blur, sigma 2) onto the
//! decoded picture patch by patch: in the swatch (or its mirror image) it takes the spot whose coarse shape matches the
//! picture there and whose detail joins the patches already laid (4 px overlaps, cross-faded), scaled to the detail the
//! picture lacks (the original's level per 32 x 32 block, stored). The detail goes on all three channels.
//!
//! The encoder has the original: it tries every option per block and keeps the one whose result has texture
//! statistics (energy in two bands, by direction, peakiness) closest to the original's there. Blocks whose original
//! detail repeats (ribs, springs: random texture over them looks wrong) get nothing; swatches used by fewer than 4
//! blocks are dropped. Reference: bench/wonder/swatch2.py.
//!
//! The picture it works on comes from a learned decoder that need not be bit-exact, so neither is this layer; the
//! section (swatches, labels, levels) decodes the same everywhere.
//!
//! Section: "NCZS" | version | swatch count | swatch size | grain softness index | DCT step | blocks across u16 |
//! blocks down u16 | atlas length u32 | atlas (DCT levels: the NCZY latent coder, 64 frequency channels) | side info
//! (ncz v3 stream: label + 2 per 16 x 16 block, level + 4 per 32 x 32 block).

pub const MAGIC: &[u8; 4] = b"NCZS";
pub const VERSION: u8 = 1;
const B: usize = 16;
const OV: usize = 4;
const P: usize = B + OV;
const NONE: i32 = -1;
const GRAIN: i32 = -2;
const SOFTS: [f32; 6] = [0.0, 0.3, 0.45, 0.6, 0.75, 0.9];
const SW: usize = 48;
const KINDS: usize = 8;
const STEP: u8 = 10;
const SIG: f32 = 2.0;

// ---------------------------------------------------------------------------------------------------------------
// planes

/// A grey plane, row-major.
#[derive(Clone)]
struct Plane { w: usize, h: usize, v: Vec<f32> }
impl Plane {
    fn new(w: usize, h: usize) -> Plane { Plane { w, h, v: vec![0.0; w * h] } }
    #[inline] fn at(&self, x: usize, y: usize) -> f32 { self.v[y * self.w + x] }
    fn luma(rgb: &[u8], w: usize, h: usize) -> Plane {
        Plane { w, h, v: (0..w * h).map(|i| 0.299 * rgb[3 * i] as f32 + 0.587 * rgb[3 * i + 1] as f32 + 0.114 * rgb[3 * i + 2] as f32).collect() }
    }
    fn sub(&self, o: &Plane) -> Plane { Plane { w: self.w, h: self.h, v: self.v.iter().zip(&o.v).map(|(a, b)| a - b).collect() } }
    /// Gaussian blur, scipy.ndimage.gaussian_filter semantics: radius int(4 sigma + 0.5), half-sample mirror edges.
    fn blur(&self, s: f32) -> Plane {
        if s <= 0.0 { return self.clone(); }
        let r = (4.0 * s + 0.5) as isize; let mut k: Vec<f32> = (-r..=r).map(|i| (-0.5 * (i * i) as f32 / (s * s)).exp()).collect();
        let t: f32 = k.iter().sum(); for x in k.iter_mut() { *x /= t; }
        let (w, h) = (self.w, self.h);
        let mut a = vec![0.0f32; w * h];
        for y in 0..h { for x in 0..w { let mut acc = 0.0; for (j, kk) in k.iter().enumerate() { acc += kk * self.v[y * w + mirror(x as isize + j as isize - r, w)]; } a[y * w + x] = acc; } }
        let mut b = vec![0.0f32; w * h];
        for y in 0..h { for x in 0..w { let mut acc = 0.0; for (j, kk) in k.iter().enumerate() { acc += kk * a[mirror(y as isize + j as isize - r, h) * w + x]; } b[y * w + x] = acc; } }
        Plane { w, h, v: b }
    }
    fn band(&self) -> Plane { self.sub(&self.blur(SIG)) }
    /// Padded by `p` at the right and bottom, edge values repeated.
    fn pad_edge(&self, p: usize) -> Plane {
        let (w, h) = (self.w + p, self.h + p); let mut o = Plane::new(w, h);
        for y in 0..h { for x in 0..w { o.v[y * w + x] = self.at(x.min(self.w - 1), y.min(self.h - 1)); } }
        o
    }
    fn window(&self, x0: usize, y0: usize, n: usize) -> Vec<f32> { let mut o = Vec::with_capacity(n * n); for y in 0..n { o.extend_from_slice(&self.v[(y0 + y) * self.w + x0..(y0 + y) * self.w + x0 + n]); } o }
    /// rms per k x k block (whole blocks only), as (blocks across, down, values).
    fn block_rms(&self, k: usize) -> (usize, usize, Vec<f32>) {
        let (bw, bh) = (self.w / k, self.h / k); let mut o = vec![0.0f32; bw * bh];
        for by in 0..bh { for bx in 0..bw { let mut s = 0.0f64; for y in 0..k { for x in 0..k { let a = self.at(bx * k + x, by * k + y) as f64; s += a * a; } } o[by * bw + bx] = (s / (k * k) as f64).sqrt() as f32; } }
        (bw, bh, o)
    }
}
/// scipy "reflect": d c b a | a b c d | d c b a
fn mirror(mut i: isize, n: usize) -> usize {
    let n = n as isize; if n == 1 { return 0; }
    loop { if i < 0 { i = -i - 1; } else if i >= n { i = 2 * n - i - 1; } else { return i as usize; } }
}
/// Blur of a small n x n patch (same edge rule).
fn blur_patch(p: &[f32], n: usize, s: f32) -> Vec<f32> { Plane { w: n, h: n, v: p.to_vec() }.blur(s).v }
/// numpy.gradient of an n x n patch: (d/dy, d/dx), central inside, one-sided at the edges.
fn gradient(p: &[f32], n: usize) -> (Vec<f32>, Vec<f32>) {
    let mut gy = vec![0.0; n * n]; let mut gx = vec![0.0; n * n];
    for y in 0..n { for x in 0..n {
        gx[y * n + x] = if x == 0 { p[y * n + 1] - p[y * n] } else if x == n - 1 { p[y * n + x] - p[y * n + x - 1] } else { (p[y * n + x + 1] - p[y * n + x - 1]) / 2.0 };
        gy[y * n + x] = if y == 0 { p[n + x] - p[x] } else if y == n - 1 { p[y * n + x] - p[(y - 1) * n + x] } else { (p[(y + 1) * n + x] - p[(y - 1) * n + x]) / 2.0 };
    } }
    (gy, gx)
}
fn mean(v: &[f32]) -> f32 { (v.iter().map(|&x| x as f64).sum::<f64>() / v.len().max(1) as f64) as f32 }
fn ms(v: &[f32]) -> f32 { (v.iter().map(|&x| (x as f64) * (x as f64)).sum::<f64>() / v.len().max(1) as f64) as f32 }

/// Grain field: position-hashed noise (sum of 4 hash bytes), blurred by the photo's grain softness, its fine-detail
/// band, unit rms.
fn hnoise(w: usize, h: usize, soft: f32) -> Plane {
    let mut n = Plane::new(w, h);
    for y in 0..h { for x in 0..w {
        let mut k = (x as u32).wrapping_mul(0x9E3779B1) ^ (y as u32).wrapping_mul(0x85EBCA77);
        k ^= k >> 16; k = k.wrapping_mul(0x7feb352d); k ^= k >> 15; k = k.wrapping_mul(0x846ca68b); k ^= k >> 16;
        n.v[y * w + x] = (((k & 255) + ((k >> 8) & 255) + ((k >> 16) & 255) + (k >> 24)) as f32 - 510.0) / 147.8;
    } }
    let b = n.blur(soft).band(); let sd = ms(&b.v).sqrt().max(1e-6); Plane { w, h, v: b.v.iter().map(|x| x / sd).collect() }
}
fn fine_share(p: &Plane) -> f32 { let f = p.sub(&p.blur(0.7)); ms(&f.v) / (ms(&p.v) + 1e-6) }

/// Texture statistics of a P x P patch of fine detail.
fn feats(h: &[f32]) -> [f32; 7] {
    let m = blur_patch(h, P, 0.7); let f: Vec<f32> = h.iter().zip(&m).map(|(a, b)| a - b).collect();
    let (gy, gx) = gradient(h, P);
    let e = [ms(&gx), ms(&gy), gx.iter().zip(&gy).map(|(a, b)| (a + b) * (a + b)).sum::<f32>() / (P * P) as f32 / 2.0,
             gx.iter().zip(&gy).map(|(a, b)| (a - b) * (a - b)).sum::<f32>() / (P * P) as f32 / 2.0];
    let tot = e.iter().sum::<f32>() + 1e-3; let m2 = ms(h); let m4 = mean(&h.iter().map(|x| x * x * x * x).collect::<Vec<_>>());
    [(ms(&f).sqrt() + 0.5).ln(), (ms(&m).sqrt() + 0.5).ln(), e[0] / tot * 4.0, e[1] / tot * 4.0, e[2] / tot * 4.0, e[3] / tot * 4.0, (m4 / (m2 * m2 + 1e-3) + 0.1).ln()]
}

// ---------------------------------------------------------------------------------------------------------------
// swatch atlas: 8 x 8 DCT, step growing with frequency, rounding toward zero (dead zone)

fn dct_basis() -> [[f32; 8]; 8] {
    let mut c = [[0f32; 8]; 8];
    for u in 0..8 { for x in 0..8 { let a = if u == 0 { (1.0f64 / 8.0).sqrt() } else { (2.0f64 / 8.0).sqrt() }; c[u][x] = (a * ((2 * x + 1) as f64 * u as f64 * std::f64::consts::PI / 16.0).cos()) as f32; } }
    c
}
fn qstep(u: usize, v: usize, step: f32) -> f32 { if u == 0 && v == 0 { 4.0 } else { step * (1.0 + 0.15 * (u + v) as f32) } }
/// Grey atlas (w x h, multiples of 8) -> DCT levels as 64 channels x (h/8) x (w/8).
fn dct_levels(px: &[f32], w: usize, h: usize, step: f32) -> Vec<i16> {
    let c = dct_basis(); let (bw, bh) = (w / 8, h / 8); let mut lv = vec![0i16; 64 * bw * bh];
    for by in 0..bh { for bx in 0..bw {
        let mut t = [[0f32; 8]; 8];
        for u in 0..8 { for x in 0..8 { let mut s = 0.0; for y in 0..8 { s += c[u][y] * (px[(by * 8 + y) * w + bx * 8 + x] - 128.0); } t[u][x] = s; } }
        for u in 0..8 { for v in 0..8 {
            let mut s = 0.0; for x in 0..8 { s += t[u][x] * c[v][x]; }
            let q = s / qstep(u, v, step); let q = q + if s > 0.0 { 0.3 } else if s < 0.0 { -0.3 } else { 0.0 };
            lv[((u * 8 + v) * bh + by) * bw + bx] = q.trunc().clamp(-32000.0, 32000.0) as i16;
        } }
    } }
    lv
}
fn dct_rebuild(lv: &[i16], w: usize, h: usize, step: f32) -> Vec<f32> {
    let c = dct_basis(); let (bw, bh) = (w / 8, h / 8); let mut px = vec![0f32; w * h];
    for by in 0..bh { for bx in 0..bw {
        let mut k = [[0f32; 8]; 8];
        for u in 0..8 { for v in 0..8 { k[u][v] = lv[((u * 8 + v) * bh + by) * bw + bx] as f32 * qstep(u, v, step); } }
        let mut t = [[0f32; 8]; 8];
        for y in 0..8 { for v in 0..8 { let mut s = 0.0; for u in 0..8 { s += c[u][y] * k[u][v]; } t[y][v] = s; } }
        for y in 0..8 { for x in 0..8 { let mut s = 0.0; for v in 0..8 { s += t[y][v] * c[v][x]; } px[(by * 8 + y) * w + bx * 8 + x] = (s + 128.0).clamp(0.0, 255.0); } }
    } }
    px
}

// ---------------------------------------------------------------------------------------------------------------
// laying

/// One swatch's candidate patches (both orientations): fine detail and mean-removed coarse shape, P x P each.
struct Source { hb: Vec<Vec<f32>>, ll: Vec<Vec<f32>> }
fn source(sw: &Plane) -> Source {
    let mut hb = Vec::new(); let mut ll = Vec::new();
    for flip in [false, true] {
        let mut p = sw.clone(); if flip { for y in 0..p.h { p.v[y * p.w..(y + 1) * p.w].reverse(); } }
        let lo = p.blur(SIG); let bd = p.sub(&lo);
        for y in 0..=p.h - P { for x in 0..=p.w - P {
            hb.push(bd.window(x, y, P)); let l = lo.window(x, y, P); let m = mean(&l); ll.push(l.iter().map(|v| v - m).collect());
        } }
    }
    Source { hb, ll }
}

struct Layer<'a> {
    w: usize, h: usize, bw: usize, bh: usize,
    acc: Vec<f32>, wsum: Vec<f32>, sl: Plane, sbp: Plane, rs: Vec<f32>, grain: Plane, src: &'a [Source], lvl: &'a [u8], lw: usize, lh: usize, fw: [f32; P * P],
}
impl<'a> Layer<'a> {
    fn new(s: &Plane, src: &'a [Source], lvl: &'a [u8], lw: usize, lh: usize, soft: f32) -> Layer<'a> {
        let (w, h) = (s.w, s.h); let sb = s.band(); let (bw, bh, rs) = sb.block_rms(B);
        let mut ramp = [1f32; P]; for i in 0..OV { ramp[i] = (i as f32 + 0.5) / OV as f32; ramp[P - 1 - i] = ramp[i]; }
        let mut fw = [0f32; P * P]; for y in 0..P { for x in 0..P { fw[y * P + x] = ramp[y] * ramp[x]; } }
        Layer { w, h, bw, bh, acc: vec![0.0; (w + OV) * (h + OV)], wsum: vec![0.0; (w + OV) * (h + OV)], sl: s.blur(SIG).pad_edge(OV), sbp: sb.pad_edge(OV), rs,
            grain: hnoise(w + OV, h + OV, soft), src, lvl, lw, lh, fw }
    }
    fn reg(&self, p: &Plane, by: usize, bx: usize) -> Vec<f32> { p.window(bx * B, by * B, P) }
    fn cur(&self, by: usize, bx: usize) -> (Vec<f32>, Vec<f32>) {
        let pw = self.w + OV; let mut c = vec![0.0; P * P]; let mut ws = vec![0.0; P * P];
        for y in 0..P { for x in 0..P { let i = (by * B + y) * pw + bx * B + x; ws[y * P + x] = self.wsum[i]; c[y * P + x] = self.acc[i] / self.wsum[i].max(1e-6); } }
        (c, ws)
    }
    fn miss(&self, by: usize, bx: usize) -> f32 {
        let l = self.lvl[(by / 2).min(self.lh - 1) * self.lw + (bx / 2).min(self.lw - 1)] as f32 - 4.0;
        let t = (2f32).powf(l / 2.0); (t * t - self.rs[by * self.bw + bx].powi(2)).max(0.0).sqrt()
    }
    /// The detail block (by, bx) gets from option k, scaled.
    fn patch(&self, k: i32, by: usize, bx: usize, cur: &[f32], ws: &[f32]) -> Vec<f32> {
        let miss = self.miss(by, bx);
        if k == GRAIN { return self.reg(&self.grain, by, bx).iter().map(|v| v * miss).collect(); }
        let s = &self.src[k as usize]; let t0 = self.reg(&self.sl, by, bx); let tm = mean(&t0); let t: Vec<f32> = t0.iter().map(|v| v - tm).collect(); let tv = ms(&t) + 4.0;
        let placed: Vec<usize> = (0..P * P).filter(|&i| ws[i] > 0.0).collect();
        let (pc, pv) = if placed.is_empty() { (Vec::new(), 1.0) } else {
            let pc: Vec<f32> = placed.iter().map(|&i| cur[i]).collect(); let m = mean(&pc); let v = pc.iter().map(|x| (x - m) * (x - m)).sum::<f32>() / pc.len() as f32; (pc, v + 1.0) };
        let mut best = f32::INFINITY; let mut bj = 0;
        for j in 0..s.hb.len() {
            let a = &s.ll[j]; let mut c = 0.0f32; for i in 0..P * P { let d = a[i] - t[i]; c += d * d; } let mut cost = c / (P * P) as f32 / tv;
            if !placed.is_empty() { let hb = &s.hb[j]; let mut e = 0.0f32; for (n, &i) in placed.iter().enumerate() { let d = hb[i] - pc[n]; e += d * d; } cost += e / placed.len() as f32 / pv; }
            if cost < best { best = cost; bj = j; }
        }
        let p = &s.hb[bj]; let g = (miss / (ms(p).sqrt() + 1e-3)).clamp(0.0, 2.0); p.iter().map(|v| v * g).collect()
    }
    fn commit(&mut self, by: usize, bx: usize, p: &[f32]) {
        let pw = self.w + OV;
        for y in 0..P { for x in 0..P { let i = (by * B + y) * pw + bx * B + x; self.acc[i] += p[y * P + x] * self.fw[y * P + x]; self.wsum[i] += self.fw[y * P + x]; } }
    }
    /// Walk the blocks in raster order. Encoder (`o` = the original's fine detail, padded; `cand`): choose each label.
    /// Decoder: `lab` given.
    fn walk(&mut self, cand: &[bool], lab: Option<&[i32]>, ob: Option<&Plane>) -> Vec<i32> {
        let mut out = vec![NONE; self.bw * self.bh]; let nsw = self.src.len() as i32;
        for by in 0..self.bh { for bx in 0..self.bw {
            let i = by * self.bw + bx;
            if !cand[i] { continue; }
            let (cur, ws) = self.cur(by, bx);
            let k = match (lab, ob) {
                (Some(l), _) => l[i],
                (None, Some(ob)) => {
                    let fo = feats(&self.reg(ob, by, bx)); let base = self.reg(&self.sbp, by, bx);
                    let mut best = f32::INFINITY; let mut bk = NONE;
                    for k in std::iter::once(NONE).chain(std::iter::once(GRAIN)).chain(0..nsw) {
                        let add = if k == NONE { vec![0.0; P * P] } else { self.patch(k, by, bx, &cur, &ws) };
                        let r: Vec<f32> = (0..P * P).map(|j| base[j] + if k == NONE { if ws[j] > 0.0 { cur[j] } else { 0.0 } }
                            else if ws[j] > 0.0 { (cur[j] * ws[j] + add[j] * self.fw[j]) / (ws[j] + self.fw[j]) } else { add[j] }).collect();
                        let f = feats(&r); let mut d: f32 = f.iter().zip(&fo).map(|(a, b)| (a - b) * (a - b)).sum();
                        if k >= 0 { d += 0.05; }                     // a swatch block must clearly win
                        if d < best { best = d; bk = k; }
                    }
                    bk
                }
                _ => NONE,
            };
            out[i] = k;
            if k == NONE { continue; }
            let p = self.patch(k, by, bx, &cur, &ws); self.commit(by, bx, &p);
        } }
        out
    }
    /// The picture with the laid detail added (all three channels).
    fn finish(&self, rgb: &mut [u8]) {
        let pw = self.w + OV;
        for y in 0..self.h { for x in 0..self.w {
            let i = y * pw + x; let m = self.wsum[i]; if m <= 0.0 { continue; }
            let d = self.acc[i] / m * m.min(1.0);
            for c in 0..3 { let o = (y * self.w + x) * 3 + c; rgb[o] = (rgb[o] as f32 + d).round().clamp(0.0, 255.0) as u8; }
        } }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// encoder

fn kmeans(x: &[[f32; 8]], k: usize) -> Vec<usize> {
    // deterministic start: the point nearest the mean, then each next the point farthest from those chosen
    let n = x.len(); let d2 = |a: &[f32; 8], b: &[f32; 8]| a.iter().zip(b).map(|(p, q)| (p - q) * (p - q)).sum::<f32>();
    let mut mu = [0f32; 8]; for p in x { for j in 0..8 { mu[j] += p[j] / n as f32; } }
    let mut c: Vec<[f32; 8]> = vec![x[(0..n).min_by(|&a, &b| d2(&x[a], &mu).total_cmp(&d2(&x[b], &mu))).unwrap()]];
    while c.len() < k { let far = (0..n).max_by(|&a, &b| { let da = c.iter().map(|q| d2(&x[a], q)).fold(f32::INFINITY, f32::min); let db = c.iter().map(|q| d2(&x[b], q)).fold(f32::INFINITY, f32::min); da.total_cmp(&db) }).unwrap(); c.push(x[far]); }
    let mut lab = vec![0usize; n];
    for _ in 0..30 {
        for i in 0..n { lab[i] = (0..k).min_by(|&a, &b| d2(&x[i], &c[a]).total_cmp(&d2(&x[i], &c[b]))).unwrap(); }
        for j in 0..k { let m: Vec<&[f32; 8]> = (0..n).filter(|&i| lab[i] == j).map(|i| &x[i]).collect(); if m.is_empty() { continue; } let mut s = [0f32; 8]; for p in &m { for q in 0..8 { s[q] += p[q] / m.len() as f32; } } c[j] = s; }
    }
    lab
}

/// Per block: how strongly the original's fine detail repeats (normalised autocorrelation peak at 3-10 px offsets,
/// Hann-windowed 32 x 32 window). Ribs, springs: 0.4-0.8; foliage 0.1-0.25; plaster 0.25-0.4.
fn regularity(ob: &Plane, by: usize, bx: usize) -> f32 {
    const R: isize = 16; let mut win = vec![0f32; 32 * 32];
    let np_reflect = |i: isize, n: usize| -> usize { let n = n as isize; let mut i = i; loop { if i < 0 { i = -i; } else if i >= n { i = 2 * n - 2 - i; } else { return i as usize; } } };
    let (cy, cx) = ((by * B + B / 2) as isize, (bx * B + B / 2) as isize);
    for y in 0..32 { for x in 0..32 { win[y * 32 + x] = ob.at(np_reflect(cx - R + x as isize, ob.w), np_reflect(cy - R + y as isize, ob.h)); } }
    let m = mean(&win); let hann: Vec<f32> = (0..32).map(|n| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * n as f32 / 31.0).cos()).collect();
    for y in 0..32 { for x in 0..32 { win[y * 32 + x] = (win[y * 32 + x] - m) * hann[y] * hann[x]; } }
    let ac = |dy: isize, dx: isize| -> f32 { let mut s = 0.0; for y in 0..32isize { let yy = y + dy; if !(0..32).contains(&yy) { continue; } for x in 0..32isize { let xx = x + dx; if (0..32).contains(&xx) { s += win[(y * 32 + x) as usize] * win[(yy * 32 + xx) as usize]; } } } s };
    let a0 = ac(0, 0) + 1e-9; let mut best = f32::NEG_INFINITY;
    for dy in -10isize..=10 { for dx in -10isize..=10 { let r = ((dy * dy + dx * dx) as f32).sqrt(); if (3.0..=10.0).contains(&r) { best = best.max(ac(dy, dx)); } } }
    best / a0
}

/// What the encoder chose, for reports.
pub struct Stats { pub swatches: usize, pub none: usize, pub grain: usize, pub swatch: usize, pub atlas_bytes: usize, pub side_bytes: usize }

/// The swatch section for a picture: `orig` the original, `pic` what the decoder will show without swatches (the
/// decoded picture, grain regrown), both RGB w x h. None if there is nothing worth carrying.
pub fn encode(orig: &[u8], pic: &[u8], w: usize, h: usize) -> Option<(Vec<u8>, Stats)> {
    if w < SW || h < SW { return None; }
    let o = Plane::luma(orig, w, h); let s = Plane::luma(pic, w, h);
    let ob = o.band(); let sb = s.band(); let (bw, bh, ro) = ob.block_rms(B); let (_, _, rs) = sb.block_rms(B);
    // texture the picture lost: the original has fine detail, the picture kept < 60% of it
    let mut cand: Vec<bool> = (0..bw * bh).map(|i| ro[i] > 3.0 && rs[i] < 0.6 * ro[i]).collect();
    if cand.iter().filter(|&&c| c).count() < 4 { return None; }
    // block descriptors: detail energy in three bands, orientation x coherence, brightness, colour
    let e: Vec<Vec<f32>> = [1.0f32, 2.0, 4.0].iter().map(|&sg| { let (_, _, r) = o.sub(&o.blur(sg)).block_rms(B); r.iter().map(|v| (v + 0.5).ln()).collect() }).collect();
    let g = o.blur(1.0); let mut jxx = vec![0f32; bw * bh]; let mut jyy = jxx.clone(); let mut jxy = jxx.clone();
    let mut yb = jxx.clone(); let mut cb = jxx.clone(); let mut cr = jxx.clone(); let nb = (B * B) as f32;
    for by in 0..bh { for bx in 0..bw { for y in by * B..by * B + B { for x in bx * B..bx * B + B {
        let gx = if x == 0 { g.at(1, y) - g.at(0, y) } else if x == w - 1 { g.at(x, y) - g.at(x - 1, y) } else { (g.at(x + 1, y) - g.at(x - 1, y)) / 2.0 };
        let gy = if y == 0 { g.at(x, 1) - g.at(x, 0) } else if y == h - 1 { g.at(x, y) - g.at(x, y - 1) } else { (g.at(x, y + 1) - g.at(x, y - 1)) / 2.0 };
        let i = by * bw + bx; jxx[i] += gx * gx / nb; jyy[i] += gy * gy / nb; jxy[i] += gx * gy / nb;
        let l = o.at(x, y); yb[i] += l / nb; cb[i] += (orig[(y * w + x) * 3 + 2] as f32 - l) / nb; cr[i] += (orig[(y * w + x) * 3] as f32 - l) / nb;
    } } } }
    let idx: Vec<usize> = (0..bw * bh).filter(|&i| cand[i]).collect();
    let mut x: Vec<[f32; 8]> = idx.iter().map(|&i| { let tr = jxx[i] + jyy[i] + 1e-6; [e[0][i], e[1][i], e[2][i], (jxx[i] - jyy[i]) / tr, 2.0 * jxy[i] / tr, yb[i] / 32.0, cb[i] / 16.0, cr[i] / 16.0] }).collect();
    for j in 0..8 { let m = x.iter().map(|p| p[j]).sum::<f32>() / x.len() as f32; let sd = (x.iter().map(|p| (p[j] - m) * (p[j] - m)).sum::<f32>() / x.len() as f32).sqrt() + 1e-6; for p in x.iter_mut() { p[j] = (p[j] - m) / sd; } }
    let kinds = kmeans(&x, KINDS.min(x.len()));
    let mut kind = vec![-1i32; bw * bh]; for (n, &i) in idx.iter().enumerate() { kind[i] = kinds[n] as i32; }
    // each kind's swatch: the window with most blocks of that kind, every block of it texture (no edges inside)
    let n = SW / B; let mut swatches: Vec<Plane> = Vec::new();
    if bw >= n && bh >= n {
        for k in 0..KINDS as i32 {
            let mut best = 0; let mut at = (0, 0);
            for wy in 0..=bh - n { for wx in 0..=bw - n {
                let mut pure = true; let mut c = 0;
                for y in wy..wy + n { for xx in wx..wx + n { let i = y * bw + xx; pure &= cand[i]; c += (kind[i] == k) as i32; } }
                if pure && c > best { best = c; at = (wx, wy); }
            } }
            if best >= 2 { let mut p = Plane::new(SW, SW); for y in 0..SW { for xx in 0..SW { p.v[y * SW + xx] = o.at(at.0 * B + xx, at.1 * B + y).round().clamp(0.0, 255.0); } } swatches.push(p); }
        }
    }
    // blocks holding a repeating pattern get nothing
    let reg: Vec<f32> = (0..bw * bh).map(|i| if cand[i] { regularity(&ob, i / bw, i % bw) } else { 0.0 }).collect();
    let hi: Vec<bool> = reg.iter().map(|&r| r >= 0.5).collect();
    for by in 0..bh { for bx in 0..bw { let i = by * bw + bx; if !cand[i] { continue; }
        let near = hi[i] || (by > 0 && hi[i - bw]) || (by + 1 < bh && hi[i + bw]) || (bx > 0 && hi[i - 1]) || (bx + 1 < bw && hi[i + 1]);
        if hi[i] || (reg[i] >= 0.35 && near) { cand[i] = false; } } }
    // levels: the original's fine-detail rms per 32 x 32 block, log2 in half steps
    let (lw, lh, lr) = ob.block_rms(32);
    let lvl: Vec<u8> = lr.iter().map(|&r| ((2.0 * r.max(0.25).log2()).round_ties_even() + 4.0).clamp(0.0, 255.0) as u8).collect();
    // grain softness: the one whose fine / whole-band energy matches the original's texture blocks
    let (mut f2, mut a2) = (0f64, 0f64); let fo = ob.sub(&ob.blur(0.7));
    for by in 0..bh { for bx in 0..bw { if !cand[by * bw + bx] { continue; } for y in by * B..by * B + B { for xx in bx * B..bx * B + B { f2 += (fo.at(xx, y) as f64).powi(2); a2 += (ob.at(xx, y) as f64).powi(2); } } } }
    let r = (f2 / (a2 + 1e-6)) as f32;
    let si = (0..SOFTS.len()).min_by(|&a, &b| (fine_share(&hnoise(128, 128, SOFTS[a])) - r).abs().total_cmp(&(fine_share(&hnoise(128, 128, SOFTS[b])) - r).abs())).unwrap();
    // swatches as the decoder will see them (coded), then choose; drop the ones fewer than 4 blocks use, choose again
    let coded = |sw: &[Plane]| -> Vec<Plane> { sw.iter().map(|p| Plane { w: SW, h: SW, v: dct_rebuild(&dct_levels(&p.v, SW, SW, STEP as f32), SW, SW, STEP as f32) }).collect() };
    let obp = ob.pad_edge(OV); let rgb_s = pic;
    let choose = |sw: &[Plane]| -> Vec<i32> { let src: Vec<Source> = sw.iter().map(source).collect(); let mut l = Layer::new(&Plane::luma(rgb_s, w, h), &src, &lvl, lw, lh, SOFTS[si]); l.walk(&cand, None, Some(&obp)) };
    let mut sw = coded(&swatches); let mut lab = choose(&sw);
    let used: Vec<usize> = (0..sw.len()).filter(|&k| lab.iter().filter(|&&l| l == k as i32).count() >= 4).collect();
    if used.len() < sw.len() { swatches = used.iter().map(|&k| swatches[k].clone()).collect(); sw = coded(&swatches); lab = choose(&sw); }
    let _ = sw;
    if lab.iter().all(|&l| l == NONE) { return None; }
    // section
    let ns = swatches.len(); let aw = ns.max(1) * SW;
    let mut atlas = vec![128f32; aw * SW]; for (k, p) in swatches.iter().enumerate() { for y in 0..SW { for xx in 0..SW { atlas[y * aw + k * SW + xx] = p.v[y * SW + xx]; } } }
    let a = if ns > 0 { crate::lat::compress(&dct_levels(&atlas, aw, SW, STEP as f32), 64, SW / 8, aw / 8, None) } else { Vec::new() };
    let mut side: Vec<u8> = lab.iter().map(|&l| (l + 2) as u8).collect(); side.extend_from_slice(&lvl);
    let sz = crate::compress(&side);
    let mut out = Vec::new(); out.extend_from_slice(MAGIC); out.extend_from_slice(&[VERSION, ns as u8, SW as u8, si as u8, STEP]);
    out.extend_from_slice(&(bw as u16).to_le_bytes()); out.extend_from_slice(&(bh as u16).to_le_bytes());
    out.extend_from_slice(&(a.len() as u32).to_le_bytes()); out.extend_from_slice(&a); out.extend_from_slice(&sz);
    let st = Stats { swatches: ns, none: lab.iter().filter(|&&l| l == NONE).count(), grain: lab.iter().filter(|&&l| l == GRAIN).count(), swatch: lab.iter().filter(|&&l| l >= 0).count(), atlas_bytes: a.len(), side_bytes: sz.len() };
    Some((out, st))
}

/// Lay a section's texture onto `rgb` (the decoded picture, grain regrown; w x h RGB) in place.
pub fn apply(rgb: &mut [u8], w: usize, h: usize, sec: &[u8]) -> Option<()> {
    if sec.len() < 17 || &sec[..4] != MAGIC || sec[4] != VERSION { return None; }
    let (ns, sw, si, step) = (sec[5] as usize, sec[6] as usize, sec[7] as usize, sec[8] as f32);
    let bw = u16::from_le_bytes([sec[9], sec[10]]) as usize; let bh = u16::from_le_bytes([sec[11], sec[12]]) as usize;
    let an = u32::from_le_bytes(sec[13..17].try_into().ok()?) as usize;
    if sw != SW || si >= SOFTS.len() || bw != w / B || bh != h / B { return None; }
    let swatches: Vec<Plane> = if ns > 0 {
        let (lv, c, ah, aw) = crate::lat::decompress(sec.get(17..17 + an)?, None)?; if c != 64 || ah * 8 != SW || aw * 8 != ns * SW { return None; }
        let px = dct_rebuild(&lv, aw * 8, SW, step);
        (0..ns).map(|k| { let mut p = Plane::new(SW, SW); for y in 0..SW { for x in 0..SW { p.v[y * SW + x] = px[y * aw * 8 + k * SW + x]; } } p }).collect()
    } else { Vec::new() };
    let side = crate::decompress(sec.get(17 + an..)?)?; let (lw, lh) = (w / 32, h / 32);
    if side.len() != bw * bh + lw * lh { return None; }
    let lab: Vec<i32> = side[..bw * bh].iter().map(|&b| b as i32 - 2).collect();
    if lab.iter().any(|&l| l < GRAIN || l >= ns as i32) { return None; }
    let cand: Vec<bool> = lab.iter().map(|&l| l != NONE).collect();
    let src: Vec<Source> = swatches.iter().map(source).collect();
    let mut l = Layer::new(&Plane::luma(rgb, w, h), &src, &side[bw * bh..], lw, lh, SOFTS[si]);
    l.walk(&cand, Some(&lab), None); l.finish(rgb); Some(())
}
