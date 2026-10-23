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
//! The encoder has the original: it tries the options per block and keeps the one whose result has texture statistics
//! (energy in two bands, by direction, peakiness) closest to the original's there. Blocks whose original detail repeats
//! (ribs, springs: random texture over them looks wrong) get nothing; swatches used by fewer than 4 blocks are dropped.
//! Reference: bench/wonder/swatch2.py. Pictures above 4 MP search swatch spots on a 2 px grid and the encoder tries
//! only the 3 swatches whose kind is nearest the block's (the section says which).
//!
//! Everything runs in stripes of 16 block rows (256 px) with 32 px margins for the filters, so memory stays flat at any
//! picture size; patches cross-fade across stripe borders as anywhere else.
//!
//! The picture it works on comes from a learned decoder that need not be bit-exact, so neither is this layer; the
//! section (swatches, labels, levels) decodes the same everywhere.
//!
//! Section: "NCZS" | version | swatch count | swatch size | grain softness index | DCT step | blocks across u16 |
//! blocks down u16 | atlas length u32 | [v2: spot stride] | atlas (DCT levels: the NCZY latent coder, 64 frequency
//! channels) | side info (ncz v3 stream: label + 2 per 16 x 16 block, level + 4 per 32 x 32 block).

pub const MAGIC: &[u8; 4] = b"NCZS";
pub const VERSION: u8 = 2;      // 2: + spot stride byte; 1 (stride 1) still decodes
const B: usize = 16;
const OV: usize = 4;
const P: usize = B + OV;
const PP: usize = P * P;
const NONE: i32 = -1;
const GRAIN: i32 = -2;
const SOFTS: [f32; 6] = [0.0, 0.3, 0.45, 0.6, 0.75, 0.9];
const SW: usize = 48;
const KINDS: usize = 8;
const STEP: u8 = 10;
const SIG: f32 = 2.0;
const SR: usize = 16;            // block rows per stripe
const MG: usize = 32;            // filter margin, px
const BIG: usize = 4 << 20;      // above this many pixels: 2 px spot grid, 3 swatches tried per block

// ---------------------------------------------------------------------------------------------------------------
// planes

/// A grey plane, row-major; `y0` is the picture row of its first row (stripes).
#[derive(Clone)]
struct Plane { w: usize, h: usize, y0: usize, v: Vec<f32> }
impl Plane {
    fn new(w: usize, h: usize) -> Plane { Plane { w, h, y0: 0, v: vec![0.0; w * h] } }
    #[inline] fn at(&self, x: usize, y: usize) -> f32 { self.v[y * self.w + x] }
    /// Value at picture coordinates, edge values repeated past the right and bottom ends of the plane.
    #[inline] fn get(&self, x: usize, gy: usize) -> f32 { self.v[(gy - self.y0).min(self.h - 1) * self.w + x.min(self.w - 1)] }
    /// Brightness of picture rows [a, b).
    fn luma(rgb: &[u8], w: usize, a: usize, b: usize) -> Plane {
        let v = (a * w..b * w).map(|i| 0.299 * rgb[3 * i] as f32 + 0.587 * rgb[3 * i + 1] as f32 + 0.114 * rgb[3 * i + 2] as f32).collect();
        Plane { w, h: b - a, y0: a, v }
    }
    fn sub(&self, o: &Plane) -> Plane { Plane { w: self.w, h: self.h, y0: self.y0, v: self.v.iter().zip(&o.v).map(|(a, b)| a - b).collect() } }
    /// Gaussian blur, scipy.ndimage.gaussian_filter semantics: radius int(4 sigma + 0.5), half-sample mirror edges.
    fn blur(&self, s: f32) -> Plane {
        if s <= 0.0 { return self.clone(); }
        let r = (4.0 * s + 0.5) as isize; let mut k: Vec<f32> = (-r..=r).map(|i| (-0.5 * (i * i) as f32 / (s * s)).exp()).collect();
        let t: f32 = k.iter().sum(); for x in k.iter_mut() { *x /= t; }
        let (w, h) = (self.w, self.h);
        let mut a = vec![0.0f32; w * h];
        for y in 0..h { let row = &self.v[y * w..(y + 1) * w];
            for x in 0..w { let mut acc = 0.0; if x as isize >= r && (x as isize) + r < w as isize { let s0 = x - r as usize; for (j, kk) in k.iter().enumerate() { acc += kk * row[s0 + j]; } }
                else { for (j, kk) in k.iter().enumerate() { acc += kk * row[mirror(x as isize + j as isize - r, w)]; } } a[y * w + x] = acc; } }
        let mut b = vec![0.0f32; w * h];
        for y in 0..h { let rows: Vec<usize> = (0..k.len()).map(|j| mirror(y as isize + j as isize - r, h)).collect();
            let o = &mut b[y * w..(y + 1) * w]; for (j, kk) in k.iter().enumerate() { let src = &a[rows[j] * w..rows[j] * w + w]; for x in 0..w { o[x] += kk * src[x]; } } }
        Plane { w, h, y0: self.y0, v: b }
    }
    fn band(&self) -> Plane { self.sub(&self.blur(SIG)) }
    /// The P x P window at picture (x0, gy0), edge-repeated.
    fn window(&self, x0: usize, gy0: usize, n: usize) -> Vec<f32> { let mut o = Vec::with_capacity(n * n); for y in 0..n { for x in 0..n { o.push(self.get(x0 + x, gy0 + y)); } } o }
    /// rms of the k x k block at picture (x0, gy0).
    fn rms_at(&self, x0: usize, gy0: usize, k: usize) -> f32 {
        let mut s = 0.0f64; for y in 0..k { let r = (gy0 + y - self.y0) * self.w + x0; for x in 0..k { let a = self.v[r + x] as f64; s += a * a; } }
        (s / (k * k) as f64).sqrt() as f32
    }
}
/// scipy "reflect": d c b a | a b c d | d c b a
fn mirror(mut i: isize, n: usize) -> usize {
    let n = n as isize; if n == 1 { return 0; }
    loop { if i < 0 { i = -i - 1; } else if i >= n { i = 2 * n - i - 1; } else { return i as usize; } }
}
/// Rows [a, b) of the stripe holding block rows [r0, r1), with margins so filters see real neighbours.
fn span(r0: usize, r1: usize, h: usize) -> (usize, usize) { ((r0 * B).saturating_sub(MG), (r1 * B + OV + MG).min(h)) }
fn blur_patch(p: &[f32], n: usize, s: f32) -> Vec<f32> { Plane { w: n, h: n, y0: 0, v: p.to_vec() }.blur(s).v }
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
/// Sum of a[i] * b[i] over PP values, 8 running sums (so it vectorises; the order is fixed).
#[inline] fn dot(a: &[f32], b: &[f32]) -> f32 {
    let mut s = [0f32; 8]; for (ca, cb) in a[..PP].chunks_exact(8).zip(b[..PP].chunks_exact(8)) { for l in 0..8 { s[l] += ca[l] * cb[l]; } }
    (s[0] + s[1]) + (s[2] + s[3]) + ((s[4] + s[5]) + (s[6] + s[7]))
}
/// Sum of m[i] * (a[i] - b[i])^2 over PP values.
#[inline] fn masked_sq(a: &[f32], b: &[f32], m: &[f32]) -> f32 {
    let mut s = [0f32; 8];
    for ((ca, cb), cm) in a[..PP].chunks_exact(8).zip(b[..PP].chunks_exact(8)).zip(m[..PP].chunks_exact(8)) { for l in 0..8 { let d = ca[l] - cb[l]; s[l] += cm[l] * d * d; } }
    (s[0] + s[1]) + (s[2] + s[3]) + ((s[4] + s[5]) + (s[6] + s[7]))
}

/// Grain field at picture rows [a, b): position-hashed noise (sum of 4 hash bytes), blurred by the photo's grain
/// softness, its fine-detail band; divided by `sd` (the band's spread, from a fixed 256 x 256 field).
fn hnoise_rows(w: usize, a: usize, b: usize, soft: f32, sd: f32) -> Plane {
    let mut n = Plane { w, h: b - a, y0: a, v: vec![0.0; w * (b - a)] };
    for y in a..b { for x in 0..w {
        let mut k = (x as u32).wrapping_mul(0x9E3779B1) ^ (y as u32).wrapping_mul(0x85EBCA77);
        k ^= k >> 16; k = k.wrapping_mul(0x7feb352d); k ^= k >> 15; k = k.wrapping_mul(0x846ca68b); k ^= k >> 16;
        n.v[(y - a) * w + x] = (((k & 255) + ((k >> 8) & 255) + ((k >> 16) & 255) + (k >> 24)) as f32 - 510.0) / 147.8;
    } }
    let mut g = n.blur(soft).band(); for v in g.v.iter_mut() { *v /= sd; } g
}
fn grain_sd(soft: f32) -> f32 { ms(&hnoise_rows(256, 0, 256, soft, 1.0).v).sqrt().max(1e-6) }
fn fine_share(p: &Plane) -> f32 { let f = p.sub(&p.blur(0.7)); ms(&f.v) / (ms(&p.v) + 1e-6) }

/// Texture statistics of a P x P patch of fine detail.
fn feats(h: &[f32]) -> [f32; 7] {
    let m = blur_patch(h, P, 0.7); let f: Vec<f32> = h.iter().zip(&m).map(|(a, b)| a - b).collect();
    let (gy, gx) = gradient(h, P);
    let e = [ms(&gx), ms(&gy), gx.iter().zip(&gy).map(|(a, b)| (a + b) * (a + b)).sum::<f32>() / PP as f32 / 2.0,
             gx.iter().zip(&gy).map(|(a, b)| (a - b) * (a - b)).sum::<f32>() / PP as f32 / 2.0];
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

/// One swatch's candidate patches (both orientations, spots `stride` px apart): fine detail, mean-removed coarse shape
/// and its energy.
struct Source { hb: Vec<Vec<f32>>, ll: Vec<Vec<f32>>, aa: Vec<f32> }
fn source(sw: &Plane, stride: usize) -> Source {
    let (mut hb, mut ll, mut aa) = (Vec::new(), Vec::new(), Vec::new());
    for flip in [false, true] {
        let mut p = sw.clone(); if flip { for y in 0..p.h { p.v[y * p.w..(y + 1) * p.w].reverse(); } }
        let lo = p.blur(SIG); let bd = p.sub(&lo);
        for y in (0..=p.h - P).step_by(stride) { for x in (0..=p.w - P).step_by(stride) {
            hb.push(bd.window(x, y, P)); let l = lo.window(x, y, P); let m = mean(&l); let a: Vec<f32> = l.iter().map(|v| v - m).collect();
            aa.push(dot(&a, &a)); ll.push(a);
        } }
    }
    Source { hb, ll, aa }
}

/// The detail laid so far: rows [y0, y0 + rows) of the picture, W + OV wide.
struct Canvas { w: usize, y0: usize, acc: Vec<f32>, wsum: Vec<f32> }
impl Canvas {
    fn rows(&self) -> usize { self.acc.len() / (self.w + OV) }
    /// Keep rows from picture row `y` on (the overlap the next stripe also writes), grow to `n` rows.
    fn shift(&mut self, y: usize, n: usize) {
        let pw = self.w + OV; let k = (y - self.y0) * pw;
        self.acc.drain(..k.min(self.acc.len())); self.wsum.drain(..k.min(self.wsum.len())); self.y0 = y;
        self.acc.resize(n * pw, 0.0); self.wsum.resize(n * pw, 0.0);
    }
    /// Write finished rows [a, b) onto the picture: detail / weight, faded where the weight is below 1.
    fn finish(&self, rgb: &mut [u8], a: usize, b: usize) {
        let pw = self.w + OV;
        for y in a..b { for x in 0..self.w {
            let i = (y - self.y0) * pw + x; let m = self.wsum[i]; if m <= 0.0 { continue; }
            let d = self.acc[i] / m * m.min(1.0);
            for c in 0..3 { let o = (y * self.w + x) * 3 + c; rgb[o] = (rgb[o] as f32 + d).round().clamp(0.0, 255.0) as u8; }
        } }
    }
}

/// The planes one stripe needs.
struct Strip { sl: Plane, sb: Plane, grain: Plane, ob: Option<Plane> }
fn strip(pic: &[u8], orig: Option<&[u8]>, w: usize, h: usize, r0: usize, r1: usize, soft: f32, gsd: f32) -> Strip {
    let (a, b) = span(r0, r1, h); let s = Plane::luma(pic, w, a, b); let sl = s.blur(SIG);
    Strip { sb: s.sub(&sl), sl, grain: hnoise_rows(w, a, b, soft, gsd), ob: orig.map(|o| Plane::luma(o, w, a, b).band()) }
}

struct Layer<'a> { w: usize, bw: usize, src: &'a [Source], lvl: &'a [u8], lw: usize, lh: usize, fw: [f32; PP] }
impl<'a> Layer<'a> {
    fn new(w: usize, src: &'a [Source], lvl: &'a [u8], lw: usize, lh: usize) -> Layer<'a> {
        let mut ramp = [1f32; P]; for i in 0..OV { ramp[i] = (i as f32 + 0.5) / OV as f32; ramp[P - 1 - i] = ramp[i]; }
        let mut fw = [0f32; PP]; for y in 0..P { for x in 0..P { fw[y * P + x] = ramp[y] * ramp[x]; } }
        Layer { w, bw: w / B, src, lvl, lw, lh, fw }
    }
    fn cur(&self, cv: &Canvas, by: usize, bx: usize) -> (Vec<f32>, Vec<f32>) {
        let pw = self.w + OV; let mut c = vec![0.0; PP]; let mut ws = vec![0.0; PP];
        for y in 0..P { for x in 0..P { let i = (by * B + y - cv.y0) * pw + bx * B + x; ws[y * P + x] = cv.wsum[i]; c[y * P + x] = cv.acc[i] / cv.wsum[i].max(1e-6); } }
        (c, ws)
    }
    fn miss(&self, st: &Strip, by: usize, bx: usize) -> f32 {
        let l = self.lvl[(by / 2).min(self.lh - 1) * self.lw + (bx / 2).min(self.lw - 1)] as f32 - 4.0;
        let t = (2f32).powf(l / 2.0); let rs = st.sb.rms_at(bx * B, by * B, B); (t * t - rs * rs).max(0.0).sqrt()
    }
    /// The detail block (by, bx) gets from option k, scaled.
    fn patch(&self, st: &Strip, k: i32, by: usize, bx: usize, cur: &[f32], ws: &[f32]) -> Vec<f32> {
        let miss = self.miss(st, by, bx);
        if k == GRAIN { return st.grain.window(bx * B, by * B, P).iter().map(|v| v * miss).collect(); }
        let s = &self.src[k as usize]; let t0 = st.sl.window(bx * B, by * B, P); let tm = mean(&t0); let t: Vec<f32> = t0.iter().map(|v| v - tm).collect();
        let tt = dot(&t, &t); let tv = ms(&t) + 4.0;
        let mask: Vec<f32> = ws.iter().map(|&v| (v > 0.0) as u8 as f32).collect(); let np = mask.iter().sum::<f32>();
        let pv = if np > 0.0 { let pc: Vec<f32> = (0..PP).filter(|&i| ws[i] > 0.0).map(|i| cur[i]).collect(); let m = mean(&pc); pc.iter().map(|x| (x - m) * (x - m)).sum::<f32>() / pc.len() as f32 + 1.0 } else { 1.0 };
        let mut best = f32::INFINITY; let mut bj = 0;
        for j in 0..s.hb.len() {
            let mut cost = (s.aa[j] - 2.0 * dot(&s.ll[j], &t) + tt).max(0.0) / PP as f32 / tv;
            if np > 0.0 { cost += masked_sq(&s.hb[j], cur, &mask) / np / pv; }
            if cost < best { best = cost; bj = j; }
        }
        let p = &s.hb[bj]; let g = (miss / (ms(p).sqrt() + 1e-3)).clamp(0.0, 2.0); p.iter().map(|v| v * g).collect()
    }
    fn commit(&self, cv: &mut Canvas, by: usize, bx: usize, p: &[f32]) {
        let pw = self.w + OV;
        for y in 0..P { for x in 0..P { let i = (by * B + y - cv.y0) * pw + bx * B + x; cv.acc[i] += p[y * P + x] * self.fw[y * P + x]; cv.wsum[i] += self.fw[y * P + x]; } }
    }
    /// One stripe's blocks, raster order. Encoder (`st.ob` set): choose each label among NONE, GRAIN and `tries(i)`;
    /// decoder: `lab` given.
    fn walk(&self, st: &Strip, cv: &mut Canvas, r0: usize, r1: usize, lab: &mut [i32], choose: Option<&dyn Fn(usize) -> Vec<i32>>) {
        for by in r0..r1 { for bx in 0..self.bw {
            let i = by * self.bw + bx;
            if lab[i] == NONE && choose.is_none() { continue; }
            let k = match (choose, &st.ob) {
                (Some(tries), Some(ob)) => {
                    if lab[i] == NONE { continue; }              // not a candidate
                    let (cur, ws) = self.cur(cv, by, bx);
                    let fo = feats(&ob.window(bx * B, by * B, P)); let base = st.sb.window(bx * B, by * B, P);
                    let mut best = f32::INFINITY; let mut bk = NONE;
                    for k in [NONE, GRAIN].into_iter().chain(tries(i)) {
                        let add = if k == NONE { vec![0.0; PP] } else { self.patch(st, k, by, bx, &cur, &ws) };
                        let r: Vec<f32> = (0..PP).map(|j| base[j] + if k == NONE { if ws[j] > 0.0 { cur[j] } else { 0.0 } }
                            else if ws[j] > 0.0 { (cur[j] * ws[j] + add[j] * self.fw[j]) / (ws[j] + self.fw[j]) } else { add[j] }).collect();
                        let f = feats(&r); let mut d: f32 = f.iter().zip(&fo).map(|(a, b)| (a - b) * (a - b)).sum();
                        if k >= 0 { d += 0.05; }                     // a swatch block must clearly win
                        if d < best { best = d; bk = k; }
                    }
                    bk
                }
                _ => lab[i],
            };
            lab[i] = k;
            if k == NONE { continue; }
            let (cur, ws) = self.cur(cv, by, bx); let p = self.patch(st, k, by, bx, &cur, &ws); self.commit(cv, by, bx, &p);
        } }
    }
}

/// Walk all stripes. Encoder (`orig`, `choose`): `lab` holds candidates (anything but NONE) and gets the choices.
/// Decoder (`rgb` given): the texture is laid onto it.
#[allow(clippy::too_many_arguments)]
fn run(pic: &[u8], orig: Option<&[u8]>, w: usize, h: usize, src: &[Source], lvl: &[u8], soft: f32, lab: &mut [i32], choose: Option<&dyn Fn(usize) -> Vec<i32>>, mut rgb: Option<&mut [u8]>) {
    let (bw, bh) = (w / B, h / B); let (lw, lh) = (w / 32, h / 32); let gsd = grain_sd(soft);
    let ly = Layer::new(w, src, lvl, lw, lh); let _ = bw;
    let mut cv = Canvas { w, y0: 0, acc: Vec::new(), wsum: Vec::new() };
    let mut r0 = 0;
    while r0 < bh {
        let r1 = (r0 + SR).min(bh);
        let st = strip(pic, orig, w, h, r0, r1, soft, gsd);
        cv.shift(r0 * B, (r1 - r0) * B + OV);
        ly.walk(&st, &mut cv, r0, r1, lab, choose);
        if let Some(out) = rgb.as_deref_mut() { let end = if r1 == bh { (r1 * B + OV).min(h) } else { r1 * B }; cv.finish(out, r0 * B, end); }
        r0 = r1;
    }
    let _ = cv.rows();
}

// ---------------------------------------------------------------------------------------------------------------
// encoder

fn d2(a: &[f32; 8], b: &[f32; 8]) -> f32 { a.iter().zip(b).map(|(p, q)| (p - q) * (p - q)).sum::<f32>() }
fn kmeans(x: &[[f32; 8]], k: usize) -> (Vec<usize>, Vec<[f32; 8]>) {
    // deterministic start: the point nearest the mean, then each next the point farthest from those chosen
    let n = x.len(); let mut mu = [0f32; 8]; for p in x { for j in 0..8 { mu[j] += p[j] / n as f32; } }
    let mut c: Vec<[f32; 8]> = vec![x[(0..n).min_by(|&a, &b| d2(&x[a], &mu).total_cmp(&d2(&x[b], &mu))).unwrap()]];
    let mut near: Vec<f32> = x.iter().map(|p| d2(p, &c[0])).collect();
    while c.len() < k { let far = (0..n).max_by(|&a, &b| near[a].total_cmp(&near[b])).unwrap(); c.push(x[far]); for i in 0..n { near[i] = near[i].min(d2(&x[i], &x[far])); } }
    let mut lab = vec![0usize; n];
    for _ in 0..30 {
        for i in 0..n { lab[i] = (0..k).min_by(|&a, &b| d2(&x[i], &c[a]).total_cmp(&d2(&x[i], &c[b]))).unwrap(); }
        let mut s = vec![[0f64; 8]; k]; let mut cnt = vec![0usize; k];
        for i in 0..n { cnt[lab[i]] += 1; for q in 0..8 { s[lab[i]][q] += x[i][q] as f64; } }
        for j in 0..k { if cnt[j] > 0 { for q in 0..8 { c[j][q] = (s[j][q] / cnt[j] as f64) as f32; } } }
    }
    (lab, c)
}

/// How strongly the original's fine detail repeats around block (by, bx): normalised autocorrelation peak at 3-10 px
/// offsets, Hann-windowed 32 x 32 window. Ribs, springs: 0.4-0.8; foliage 0.1-0.25; plaster 0.25-0.4.
fn regularity(ob: &Plane, h: usize, by: usize, bx: usize) -> f32 {
    const R: isize = 16; let mut win = [0f32; 64 * 32];        // 32 rows, padded to 64 columns of zeros
    let np_reflect = |i: isize, n: usize| -> usize { let n = n as isize; let mut i = i; loop { if i < 0 { i = -i; } else if i >= n { i = 2 * n - 2 - i; } else { return i as usize; } } };
    let (cy, cx) = ((by * B + B / 2) as isize, (bx * B + B / 2) as isize);
    for y in 0..32 { for x in 0..32 { win[y * 64 + x] = ob.get(np_reflect(cx - R + x as isize, ob.w), np_reflect(cy - R + y as isize, h)); } }
    let mut m = 0.0; for y in 0..32 { for x in 0..32 { m += win[y * 64 + x]; } } m /= 1024.0;
    let hann: Vec<f32> = (0..32).map(|n| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * n as f32 / 31.0).cos()).collect();
    for y in 0..32 { for x in 0..32 { win[y * 64 + x] = (win[y * 64 + x] - m) * hann[y] * hann[x]; } }
    // autocorrelation at (dy, dx); (-dy, -dx) is the same, so half the ring
    let ac = |dy: usize, dx: isize| -> f32 {
        let mut s = [0f32; 8];
        for y in 0..32 - dy { let ra = &win[y * 64..y * 64 + 64]; let rb = &win[(y + dy) * 64..(y + dy) * 64 + 64];
            let (x0, x1) = if dx >= 0 { (0usize, 32 - dx as usize) } else { ((-dx) as usize, 32) };
            let n = x1 - x0; let (pa, pb) = (&ra[x0..x1], &rb[(x0 as isize + dx) as usize..(x0 as isize + dx) as usize + n]);
            let mut i = 0; while i + 8 <= n { for l in 0..8 { s[l] += pa[i + l] * pb[i + l]; } i += 8; } while i < n { s[0] += pa[i] * pb[i]; i += 1; } }
        (s[0] + s[1]) + (s[2] + s[3]) + ((s[4] + s[5]) + (s[6] + s[7]))
    };
    let a0 = ac(0, 0) + 1e-9; let mut best = f32::NEG_INFINITY;
    for dy in 0..=10usize { for dx in -10isize..=10 { if dy == 0 && dx <= 0 { continue; } let r = ((dy * dy) as f32 + (dx * dx) as f32).sqrt(); if (3.0..=10.0).contains(&r) { best = best.max(ac(dy, dx)); } } }
    best / a0
}

/// What the encoder chose, for reports.
pub struct Stats { pub swatches: usize, pub none: usize, pub grain: usize, pub swatch: usize, pub atlas_bytes: usize, pub side_bytes: usize }

/// The swatch section for a picture: `orig` the original, `pic` what the decoder will show without swatches (the
/// decoded picture, grain regrown), both RGB w x h. None if there is nothing worth carrying.
pub fn encode(orig: &[u8], pic: &[u8], w: usize, h: usize) -> Option<(Vec<u8>, Stats)> {
    if w < SW || h < SW { return None; }
    let (bw, bh) = (w / B, h / B); let (lw, lh) = (w / 32, h / 32); let big = w * h > BIG; let stride = if big { 2 } else { 1 };
    // per block (stripe by stripe): texture the picture lost (the original has fine detail, the picture kept < 60% of
    // it), its descriptor (detail energy in three bands, orientation x coherence, brightness, colour), how regular it
    // is; per 32 x 32 block the original's detail level; the fine / whole-band energy of the original's texture
    let mut cand = vec![false; bw * bh]; let mut desc = vec![[0f32; 8]; bw * bh]; let mut reg = vec![0f32; bw * bh];
    let mut lvl = vec![0u8; lw * lh];
    let mut r0 = 0;
    while r0 < bh {
        let r1 = (r0 + SR).min(bh); let (a, b) = span(r0, r1, h);
        let o = Plane::luma(orig, w, a, b); let s = Plane::luma(pic, w, a, b); let ob = o.band(); let sb = s.band();
        let es: Vec<Plane> = [1.0f32, 2.0, 4.0].iter().map(|&sg| o.sub(&o.blur(sg))).collect(); let g = o.blur(1.0);
        for by in r0..r1 { for bx in 0..bw {
            let i = by * bw + bx; let (x0, y0) = (bx * B, by * B);
            let ro = ob.rms_at(x0, y0, B); let rs = sb.rms_at(x0, y0, B); cand[i] = ro > 3.0 && rs < 0.6 * ro;
            if !cand[i] { continue; }
            let (mut jxx, mut jyy, mut jxy, mut yb, mut cb, mut cr) = (0f32, 0f32, 0f32, 0f32, 0f32, 0f32); let nb = (B * B) as f32;
            for y in y0..y0 + B { for x in x0..x0 + B {
                let gx = if x == 0 { g.get(1, y) - g.get(0, y) } else if x == w - 1 { g.get(x, y) - g.get(x - 1, y) } else { (g.get(x + 1, y) - g.get(x - 1, y)) / 2.0 };
                let gy = if y == 0 { g.get(x, 1) - g.get(x, 0) } else if y == h - 1 { g.get(x, y) - g.get(x, y - 1) } else { (g.get(x, y + 1) - g.get(x, y - 1)) / 2.0 };
                jxx += gx * gx / nb; jyy += gy * gy / nb; jxy += gx * gy / nb;
                let l = o.get(x, y); yb += l / nb; cb += (orig[(y * w + x) * 3 + 2] as f32 - l) / nb; cr += (orig[(y * w + x) * 3] as f32 - l) / nb;
            } }
            let tr = jxx + jyy + 1e-6;
            desc[i] = [(es[0].rms_at(x0, y0, B) + 0.5).ln(), (es[1].rms_at(x0, y0, B) + 0.5).ln(), (es[2].rms_at(x0, y0, B) + 0.5).ln(), (jxx - jyy) / tr, 2.0 * jxy / tr, yb / 32.0, cb / 16.0, cr / 16.0];
            reg[i] = regularity(&ob, h, by, bx);
        } }
        for ly in (r0 * B / 32)..((r1 * B / 32).min(lh)) { if r1 < bh && (ly + 1) * 32 > r1 * B { break; } for lx in 0..lw {
            lvl[ly * lw + lx] = ((2.0 * ob.rms_at(lx * 32, ly * 32, 32).max(0.25).log2()).round_ties_even() + 4.0).clamp(0.0, 255.0) as u8; } }
        r0 = r1;
    }
    let idx: Vec<usize> = (0..bw * bh).filter(|&i| cand[i]).collect();
    if idx.len() < 4 { return None; }
    // kinds: k-means over the standardised descriptors
    let mut x: Vec<[f32; 8]> = idx.iter().map(|&i| desc[i]).collect();
    let mut mu = [0f32; 8]; let mut sd = [0f32; 8];
    for j in 0..8 { let m = x.iter().map(|p| p[j] as f64).sum::<f64>() / x.len() as f64; let v = x.iter().map(|p| (p[j] as f64 - m).powi(2)).sum::<f64>() / x.len() as f64; mu[j] = m as f32; sd[j] = v.sqrt() as f32 + 1e-6; }
    for p in x.iter_mut() { for j in 0..8 { p[j] = (p[j] - mu[j]) / sd[j]; } }
    let (kinds, cent) = kmeans(&x, KINDS.min(x.len()));
    let mut kind = vec![-1i32; bw * bh]; for (n, &i) in idx.iter().enumerate() { kind[i] = kinds[n] as i32; }
    // each kind's swatch: the window with most blocks of that kind, every block of it texture (no edges inside)
    let n = SW / B; let mut swatches: Vec<(Plane, usize)> = Vec::new();
    if bw >= n && bh >= n {
        // blocks of kind k in each n x n window, via running sums
        for k in 0..cent.len() as i32 {
            let mut best = 0; let mut at = (0, 0);
            for wy in 0..=bh - n { for wx in 0..=bw - n {
                let mut pure = true; let mut c = 0;
                'w: for y in wy..wy + n { for xx in wx..wx + n { let i = y * bw + xx; if !cand[i] { pure = false; break 'w; } c += (kind[i] == k) as i32; } }
                if pure && c > best { best = c; at = (wx, wy); }
            } }
            if best >= 2 { let p = Plane::luma(orig, w, at.1 * B, at.1 * B + SW); let mut q = Plane::new(SW, SW);
                for y in 0..SW { for xx in 0..SW { q.v[y * SW + xx] = p.at(at.0 * B + xx, y).round().clamp(0.0, 255.0); } } swatches.push((q, k as usize)); }
        }
    }
    // blocks holding a repeating pattern get nothing
    let hi: Vec<bool> = reg.iter().map(|&r| r >= 0.5).collect();
    for by in 0..bh { for bx in 0..bw { let i = by * bw + bx; if !cand[i] { continue; }
        let near = hi[i] || (by > 0 && hi[i - bw]) || (by + 1 < bh && hi[i + bw]) || (bx > 0 && hi[i - 1]) || (bx + 1 < bw && hi[i + 1]);
        if hi[i] || (reg[i] >= 0.35 && near) { cand[i] = false; } } }
    // grain softness: the one whose fine / whole-band energy matches the original's texture blocks
    let (mut f2, mut a2) = (0f64, 0f64); let mut r0 = 0;
    while r0 < bh {
        let r1 = (r0 + SR).min(bh);
        if (r0 * bw..r1 * bw).any(|i| cand[i]) {
            let (a, b) = span(r0, r1, h); let ob = Plane::luma(orig, w, a, b).band(); let fo = ob.sub(&ob.blur(0.7));
            for by in r0..r1 { for bx in 0..bw { if !cand[by * bw + bx] { continue; }
                for y in by * B..by * B + B { for xx in bx * B..bx * B + B { f2 += (fo.get(xx, y) as f64).powi(2); a2 += (ob.get(xx, y) as f64).powi(2); } } } }
        }
        r0 = r1;
    }
    let r = (f2 / (a2 + 1e-6)) as f32;
    let shares: Vec<f32> = SOFTS.iter().map(|&s| { let n = hnoise_rows(128, 0, 128, s, 1.0); fine_share(&n) }).collect();
    let si = (0..SOFTS.len()).min_by(|&a, &b| (shares[a] - r).abs().total_cmp(&(shares[b] - r).abs())).unwrap();
    // swatches as the decoder will see them (coded); choose; drop the ones fewer than 4 blocks use, choose again
    let coded = |p: &Plane| Plane { w: SW, h: SW, y0: 0, v: dct_rebuild(&dct_levels(&p.v, SW, SW, STEP as f32), SW, SW, STEP as f32) };
    let choose_all = |sw: &[(Plane, usize)]| -> Vec<i32> {
        let src: Vec<Source> = sw.iter().map(|(p, _)| source(&coded(p), stride)).collect();
        // which swatches a block tries: all, or (big pictures) the 3 whose kind is nearest the block's descriptor
        let tries = |i: usize| -> Vec<i32> {
            if !big || sw.len() <= 3 { return (0..sw.len() as i32).collect(); }
            let mut v = [0f32; 8]; for j in 0..8 { v[j] = (desc[i][j] - mu[j]) / sd[j]; }
            let mut o: Vec<usize> = (0..sw.len()).collect(); o.sort_by(|&a, &b| d2(&v, &cent[sw[a].1]).total_cmp(&d2(&v, &cent[sw[b].1])));
            let mut t: Vec<i32> = o[..3].iter().map(|&k| k as i32).collect(); t.sort(); t
        };
        let mut lab: Vec<i32> = cand.iter().map(|&c| if c { 0 } else { NONE }).collect();
        run(pic, Some(orig), w, h, &src, &lvl, SOFTS[si], &mut lab, Some(&tries), None); lab
    };
    let mut lab = choose_all(&swatches);
    let used: Vec<usize> = (0..swatches.len()).filter(|&k| lab.iter().filter(|&&l| l == k as i32).count() >= 4).collect();
    if used.len() < swatches.len() {
        if big {                // the second pass would double the time: those few blocks get nothing instead
            let mut re = vec![NONE; swatches.len()]; for (n, &k) in used.iter().enumerate() { re[k] = n as i32; }
            for l in lab.iter_mut() { if *l >= 0 { *l = re[*l as usize]; } }
            swatches = used.iter().map(|&k| swatches[k].clone()).collect();
        } else { swatches = used.iter().map(|&k| swatches[k].clone()).collect(); lab = choose_all(&swatches); }
    }
    if lab.iter().all(|&l| l == NONE) { return None; }
    // section
    let ns = swatches.len(); let aw = ns.max(1) * SW;
    let mut atlas = vec![128f32; aw * SW]; for (k, (p, _)) in swatches.iter().enumerate() { for y in 0..SW { for xx in 0..SW { atlas[y * aw + k * SW + xx] = p.v[y * SW + xx]; } } }
    let a = if ns > 0 { crate::lat::compress(&dct_levels(&atlas, aw, SW, STEP as f32), 64, SW / 8, aw / 8, None) } else { Vec::new() };
    let mut side: Vec<u8> = lab.iter().map(|&l| (l + 2) as u8).collect(); side.extend_from_slice(&lvl);
    let sz = crate::compress(&side);
    let mut out = Vec::new(); out.extend_from_slice(MAGIC); out.extend_from_slice(&[VERSION, ns as u8, SW as u8, si as u8, STEP]);
    out.extend_from_slice(&(bw as u16).to_le_bytes()); out.extend_from_slice(&(bh as u16).to_le_bytes());
    out.extend_from_slice(&(a.len() as u32).to_le_bytes()); out.push(stride as u8); out.extend_from_slice(&a); out.extend_from_slice(&sz);
    let st = Stats { swatches: ns, none: lab.iter().filter(|&&l| l == NONE).count(), grain: lab.iter().filter(|&&l| l == GRAIN).count(), swatch: lab.iter().filter(|&&l| l >= 0).count(), atlas_bytes: a.len(), side_bytes: sz.len() };
    Some((out, st))
}

/// Lay a section's texture onto `rgb` (the decoded picture, grain regrown; w x h RGB) in place.
pub fn apply(rgb: &mut [u8], w: usize, h: usize, sec: &[u8]) -> Option<()> {
    if sec.len() < 17 || &sec[..4] != MAGIC || !(1..=VERSION).contains(&sec[4]) { return None; }
    let (ns, sw, si, step) = (sec[5] as usize, sec[6] as usize, sec[7] as usize, sec[8] as f32);
    let bw = u16::from_le_bytes([sec[9], sec[10]]) as usize; let bh = u16::from_le_bytes([sec[11], sec[12]]) as usize;
    let an = u32::from_le_bytes(sec[13..17].try_into().ok()?) as usize;
    let (stride, a0) = if sec[4] >= 2 { (*sec.get(17)? as usize, 18) } else { (1, 17) };
    if sw != SW || si >= SOFTS.len() || bw != w / B || bh != h / B || !(1..=4).contains(&stride) { return None; }
    let swatches: Vec<Plane> = if ns > 0 {
        let (lv, c, ah, aw) = crate::lat::decompress(sec.get(a0..a0 + an)?, None)?; if c != 64 || ah * 8 != SW || aw * 8 != ns * SW { return None; }
        let px = dct_rebuild(&lv, aw * 8, SW, step);
        (0..ns).map(|k| { let mut p = Plane::new(SW, SW); for y in 0..SW { for x in 0..SW { p.v[y * SW + x] = px[y * aw * 8 + k * SW + x]; } } p }).collect()
    } else { Vec::new() };
    let side = crate::decompress(sec.get(a0 + an..)?)?; let (lw, lh) = (w / 32, h / 32);
    if side.len() != bw * bh + lw * lh { return None; }
    let mut lab: Vec<i32> = side[..bw * bh].iter().map(|&b| b as i32 - 2).collect();
    if lab.iter().any(|&l| l < GRAIN || l >= ns as i32) { return None; }
    let src: Vec<Source> = swatches.iter().map(|p| source(p, stride)).collect();
    let pic = rgb.to_vec();
    run(&pic, None, w, h, &src, &side[bw * bh..], SOFTS[si], &mut lab, None, Some(rgb)); Some(())
}
