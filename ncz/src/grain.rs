//! Grain mode: a LOOKS-THE-SAME mode, not an exact copy.
//!
//! The encoder separates the photo into a clean picture and its grain, measures the grain's fingerprint, and
//! stores the clean picture (pixel model, every sample within +-k of the clean picture) plus the fingerprint. The
//! decoder rebuilds the clean picture and grows NEW grain with the same character on top. The output does not
//! match the original pixel for pixel; it is meant to look the same. Hence its own magic, NCZG.
//!
//! Encoder (quality matters, bit-exactness does not): grain spectrum from the flattest 6% of 32x32 patches
//! (plane-detrended, median), then a two-stage 16x16 block-DCT filter driven by that spectrum (hard threshold,
//! then empirical Wiener with the strength adapted to brightness): BM3D's transform-domain core without block
//! matching. Fingerprint, in an orthonormal opponent colour space (brightness, red-blue, green-magenta):
//! strength per channel in 16 brightness bands, the normalised grain's 32x32 power spectrum, the 3x3 channel
//! coupling (stored as its Cholesky factor), 65 quantiles on a normal-score grid (film grain is not Gaussian), and
//! the strength of a slow random envelope fitted to the grain's clumping.
//!
//! Decoder (must give the same pixels everywhere): integer random numbers, own exp / cos / inverse-normal, f64 in a
//! fixed order, no FFT: noise -> 17x17 kernel from the spectrum -> envelope -> coupling -> rank-map to the quantiles
//! -> strength by brightness -> added to the clean picture.
//!
//! Stream: "NCZG" | version | k | w u32 | h u32 | fingerprint length u32 | fingerprint (NCZ3) | clean picture (NCZI).
use crate::img;

pub const MAGIC: &[u8; 4] = b"NCZG";
pub const VERSION: u8 = 1;
const NB: usize = 16;            // brightness bands
const PS: usize = 32;            // spectrum patch size
const NZ: usize = 65;            // quantiles, on z = -4..4
const CLIP: f64 = 6.0;           // normalised grain clipped to +-6
const FLOOR: f64 = 0.25;         // strength floor when normalising (8-bit levels)
const NO_GRAIN: f64 = 0.5;       // mean brightness-grain rms below this: treated as grain-free
const KR: usize = 8;             // synthesis kernel radius: 17x17 taps
const ENV_SIGMA: f64 = 8.0;      // envelope smoothness (px)
const B: usize = 16;             // denoiser block size
const STEP: usize = 4;           // denoiser block step

// orthonormal opponent transform (rows): Y, C1, C2; inverse = transpose
const S3: f64 = 0.577_350_269_189_625_8; const S2: f64 = 0.707_106_781_186_547_6; const S6: f64 = 0.408_248_290_463_863;
const OPP: [[f64; 3]; 3] = [[S3, S3, S3], [S2, 0.0, -S2], [S6, -2.0 * S6, S6]];

// ---------------------------------------------------------------------------
// deterministic maths for the decoder (f64; IEEE add/mul/div/sqrt are exact everywhere, no libm)
// ---------------------------------------------------------------------------
fn dexp(x: f64) -> f64 {
    let x = x.clamp(-700.0, 700.0);
    let k = (x * std::f64::consts::LOG2_E).round_ties_even();
    let r = (x - k * 0.693_147_180_369_123_8) - k * 1.908_214_929_270_587_7e-10;
    let mut t = 1.0; let mut s = 1.0;
    for i in 1..18 { t *= r / i as f64; s += t; }
    s * f64::from_bits(((k as i64 + 1023) as u64) << 52)
}
fn dcos(x: f64) -> f64 {
    // reduce to [-pi, pi], then fold to [0, pi/2] with sign, Taylor
    let tau = 6.283_185_307_179_586;
    let mut y = x - (x / tau).round_ties_even() * tau;
    y = y.abs(); let mut sign = 1.0;
    if y > std::f64::consts::FRAC_PI_2 { y = std::f64::consts::PI - y; sign = -1.0; }
    let y2 = y * y; let mut t = 1.0; let mut s = 1.0;
    for i in 1..14 { t *= -y2 / ((2 * i - 1) * (2 * i)) as f64; s += t; }
    sign * s
}
/// Inverse standard normal CDF (Acklam's rational approximation, |rel err| < 1.2e-9).
fn inv_norm(p: f64) -> f64 {
    let p = p.clamp(1e-12, 1.0 - 1e-12);
    const A: [f64; 6] = [-3.969_683_028_665_376e1, 2.209_460_984_245_205e2, -2.759_285_104_469_687e2, 1.383_577_518_672_69e2, -3.066_479_806_614_716e1, 2.506_628_277_459_239];
    const BB: [f64; 5] = [-5.447_609_879_822_406e1, 1.615_858_368_580_409e2, -1.556_989_798_598_866e2, 6.680_131_188_771_972e1, -1.328_068_155_288_572e1];
    const C: [f64; 6] = [-7.784_894_002_430_293e-3, -3.223_964_580_411_365e-1, -2.400_758_277_161_838, -2.549_732_539_343_734, 4.374_664_141_464_968, 2.938_163_982_698_783];
    const D: [f64; 4] = [7.784_695_709_041_462e-3, 3.224_671_290_700_398e-1, 2.445_134_137_142_996, 3.754_408_661_907_416];
    let pl = 0.024_25;
    if p < pl { let q = (-2.0 * dln(p)).sqrt(); return (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5]) / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0); }
    if p > 1.0 - pl { let q = (-2.0 * dln(1.0 - p)).sqrt(); return -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5]) / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0); }
    let q = p - 0.5; let r = q * q;
    (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q / (((((BB[0] * r + BB[1]) * r + BB[2]) * r + BB[3]) * r + BB[4]) * r + 1.0)
}
fn dln(x: f64) -> f64 {
    let bits = x.to_bits(); let mut e = ((bits >> 52) & 0x7ff) as i64 - 1023;
    let mut m = f64::from_bits((bits & 0x000f_ffff_ffff_ffff) | 0x3ff0_0000_0000_0000);
    if m > std::f64::consts::SQRT_2 { m *= 0.5; e += 1; }
    let t = (m - 1.0) / (m + 1.0); let t2 = t * t; let mut s = 0.0; let mut pw = t;
    for i in 0..20 { s += pw / (2 * i + 1) as f64; pw *= t2; }
    e as f64 * std::f64::consts::LN_2 + 2.0 * s
}
/// SplitMix64.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 { self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15); let mut z = self.0; z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9); z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB); z ^ (z >> 31) }
    fn uni(&mut self) -> f64 { (self.next() >> 11) as f64 / (1u64 << 53) as f64 * 2.0 - 1.0 }
    /// Near-Gaussian: sum of four uniforms, unit variance (the 17x17 filter makes the field Gaussian anyway).
    fn gauss(&mut self) -> f64 { (self.uni() + self.uni() + self.uni() + self.uni()) * 0.866_025_403_784_438_6 }
}

// ---------------------------------------------------------------------------
// fingerprint
// ---------------------------------------------------------------------------
#[derive(Clone)]
pub struct Fingerprint {
    pub strength: [[f64; 3]; NB],     // grain rms (8-bit levels) per brightness band, per opponent channel
    pub psd: Vec<[f64; PS * PS]>,     // normalised grain power spectrum, per channel
    pub chol: [[f64; 3]; 3],          // lower-triangular coupling factor
    pub quant: [[f64; NZ]; 3],        // normalised grain value at z = -4..4, per channel
    pub env: f64,                     // envelope strength
    pub seed: u64,
}

fn band_of(y_mean_rgb: f64) -> f64 { (y_mean_rgb / 256.0 * NB as f64 - 0.5).clamp(0.0, (NB - 1) as f64) }   // continuous band coordinate

fn strength_at(fp: &Fingerprint, yb: f64, c: usize) -> f64 {
    let i = yb.floor() as usize; let f = yb - i as f64; let j = (i + 1).min(NB - 1);
    fp.strength[i][c] * (1.0 - f) + fp.strength[j][c] * f
}

impl Fingerprint {
    fn encode(&self) -> Vec<u8> {
        let mut v = Vec::new();
        for b in 0..NB { for c in 0..3 { v.extend_from_slice(&((self.strength[b][c] * 100.0).round().clamp(0.0, 65535.0) as u16).to_le_bytes()); } }
        // the spectrum of a real field is point-symmetric: half of it (v <= PS/2) is enough
        for c in 0..3 { for u in 0..PS { for vv in 0..=PS / 2 { let q = ((self.psd[c][u * PS + vv].max(1e-5).log10() + 5.0) * 40.0).round().clamp(0.0, 255.0) as u8; v.push(q); } } }
        for r in 0..3 { for c in 0..=r { v.extend_from_slice(&((self.chol[r][c] * 10000.0).round().clamp(-32767.0, 32767.0) as i16).to_le_bytes()); } }
        for c in 0..3 { for i in 0..NZ { v.extend_from_slice(&((self.quant[c][i] * 1000.0).round().clamp(-32767.0, 32767.0) as i16).to_le_bytes()); } }
        v.push((self.env * 100.0).round().clamp(0.0, 255.0) as u8);
        v.extend_from_slice(&self.seed.to_le_bytes());
        v
    }
    fn decode(b: &[u8]) -> Option<Fingerprint> {
        let mut i = 0; let mut u16_ = || -> Option<u16> { let v = u16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?); i += 2; Some(v) };
        let mut strength = [[0.0; 3]; NB];
        for bb in 0..NB { for c in 0..3 { strength[bb][c] = u16_()? as f64 / 100.0; } }
        let mut psd = vec![[0.0; PS * PS]; 3];
        for c in 0..3 { for u in 0..PS { for v in 0..=PS / 2 { let q = *b.get(i)? as f64; i += 1; let p = dexp((q / 40.0 - 5.0) * std::f64::consts::LN_10); psd[c][u * PS + v] = p; psd[c][((PS - u) % PS) * PS + (PS - v) % PS] = p; } } }
        let mut chol = [[0.0; 3]; 3];
        for r in 0..3 { for c in 0..=r { chol[r][c] = i16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?) as f64 / 10000.0; i += 2; } }
        let mut quant = [[0.0; NZ]; 3];
        for c in 0..3 { for k in 0..NZ { quant[c][k] = i16::from_le_bytes(b.get(i..i + 2)?.try_into().ok()?) as f64 / 1000.0; i += 2; } }
        let env = *b.get(i)? as f64 / 100.0; i += 1;
        let seed = u64::from_le_bytes(b.get(i..i + 8)?.try_into().ok()?);
        Some(Fingerprint { strength, psd, chol, quant, env, seed })
    }
}

// ---------------------------------------------------------------------------
// decoder-side synthesis (deterministic)
// ---------------------------------------------------------------------------
/// 17x17 amplitude kernel from a 32x32 power spectrum: inverse DFT of sqrt(psd) (real, symmetric), centred, cropped.
fn kernel(psd: &[f64; PS * PS]) -> Vec<f64> {
    let mut cosv = [0.0f64; PS]; for k in 0..PS { cosv[k] = dcos(2.0 * std::f64::consts::PI * k as f64 / PS as f64); }
    let amp: Vec<f64> = psd.iter().map(|p| p.max(0.0).sqrt()).collect();
    let n = 2 * KR + 1; let mut ker = vec![0.0; n * n];
    for dy in 0..n { for dx in 0..n {
        let (y, x) = ((dy + PS - KR) % PS, (dx + PS - KR) % PS);
        let mut s = 0.0;
        for u in 0..PS { for v in 0..PS { s += amp[u * PS + v] * cosv[(u * y + v * x) % PS]; } }
        ker[dy * n + dx] = s / (PS * PS) as f64;
    } }
    ker
}

fn std_of(f: &[f64]) -> f64 { let n = f.len() as f64; let m = f.iter().sum::<f64>() / n; (f.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / n).sqrt().max(1e-12) }

/// Unit-variance spectrum-shaped field (h x w) for channel c.
fn shaped_field(rng: &mut Rng, ker: &[f64], w: usize, h: usize) -> Vec<f64> {
    let n = 2 * KR + 1; let (pw, ph) = (w + 2 * KR, h + 2 * KR);
    let noise: Vec<f64> = (0..pw * ph).map(|_| rng.gauss()).collect();
    let mut f = vec![0.0; w * h];
    for y in 0..h { for x in 0..w {
        let mut s = 0.0;
        for ky in 0..n { let row = (y + ky) * pw + x; let kr = ky * n; for kx in 0..n { s += ker[kr + kx] * noise[row + kx]; } }
        f[y * w + x] = s;
    } }
    let sd = std_of(&f); for v in f.iter_mut() { *v /= sd; }
    f
}

/// Slow envelope field m (unit variance): Gaussian blur (sigma ENV_SIGMA) of noise, separable, fixed order.
fn envelope(rng: &mut Rng, w: usize, h: usize) -> Vec<f64> {
    let r = (3.0 * ENV_SIGMA) as usize; let k: Vec<f64> = (0..=2 * r).map(|i| { let d = i as f64 - r as f64; dexp(-d * d / (2.0 * ENV_SIGMA * ENV_SIGMA)) }).collect();
    let (pw, ph) = (w + 2 * r, h + 2 * r);
    let noise: Vec<f64> = (0..pw * ph).map(|_| rng.gauss()).collect();
    let mut t = vec![0.0; w * ph];
    for y in 0..ph { for x in 0..w { let mut s = 0.0; for i in 0..k.len() { s += k[i] * noise[y * pw + x + i]; } t[y * w + x] = s; } }
    let mut m = vec![0.0; w * h];
    for y in 0..h { for x in 0..w { let mut s = 0.0; for i in 0..k.len() { s += k[i] * t[(y + i) * w + x]; } m[y * w + x] = s; } }
    let sd = std_of(&m); for v in m.iter_mut() { *v /= sd; }
    m
}

/// Rank-map a field onto the stored quantiles (deterministic: ties broken by index).
fn shape_to(f: &mut [f64], quant: &[f64; NZ]) {
    let n = f.len(); let mut idx: Vec<u32> = (0..n as u32).collect();
    idx.sort_by(|&a, &b| f[a as usize].total_cmp(&f[b as usize]).then(a.cmp(&b)));
    let step = 8.0 / (NZ - 1) as f64;
    for (r, &i) in idx.iter().enumerate() {
        let z = inv_norm((r as f64 + 0.5) / n as f64).clamp(-4.0, 4.0);
        let t = (z + 4.0) / step; let j = (t.floor() as usize).min(NZ - 2); let fr = t - j as f64;
        f[i as usize] = quant[j] * (1.0 - fr) + quant[j + 1] * fr;
    }
}

/// Normalised grain (before strength), per channel, h x w. Used by the decoder and by the encoder's envelope fit.
fn unit_grain(fp: &Fingerprint, w: usize, h: usize, env: f64) -> [Vec<f64>; 3] {
    let mut rng = Rng(fp.seed);
    let mut f: Vec<Vec<f64>> = (0..3).map(|c| { let k = kernel(&fp.psd[c]); shaped_field(&mut rng, &k, w, h) }).collect();
    if env > 0.0 { let m = envelope(&mut rng, w, h); for c in 0..3 { for i in 0..w * h { f[c][i] *= dexp(env * m[i]); } } }
    let mut out = [vec![0.0; w * h], vec![0.0; w * h], vec![0.0; w * h]];
    for i in 0..w * h {
        let (a, b, c) = (f[0][i], f[1][i], f[2][i]); let l = &fp.chol;
        out[0][i] = l[0][0] * a; out[1][i] = l[1][0] * a + l[1][1] * b; out[2][i] = l[2][0] * a + l[2][1] * b + l[2][2] * c;
    }
    for c in 0..3 { shape_to(&mut out[c], &fp.quant[c]); }
    out
}

/// Clean RGB (8-bit) + fingerprint -> RGB with regenerated grain.
pub fn regrain(clean: &[u8], w: usize, h: usize, fp: &Fingerprint) -> Vec<u8> {
    if fp.strength.iter().all(|s| s.iter().all(|&v| v == 0.0)) { return clean.to_vec(); }   // grain-free picture
    let u = unit_grain(fp, w, h, fp.env);
    let mut out = vec![0u8; w * h * 3];
    for i in 0..w * h {
        let (r, g, b) = (clean[3 * i] as f64, clean[3 * i + 1] as f64, clean[3 * i + 2] as f64);
        let yb = band_of((r + g + b) / 3.0);
        let go = [u[0][i] * strength_at(fp, yb, 0), u[1][i] * strength_at(fp, yb, 1), u[2][i] * strength_at(fp, yb, 2)];
        for k in 0..3 {
            let d = OPP[0][k] * go[0] + OPP[1][k] * go[1] + OPP[2][k] * go[2];       // inverse = transpose
            out[3 * i + k] = (clean[3 * i + k] as f64 + d).round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// encoder side: spectrum, denoiser, fingerprint
// ---------------------------------------------------------------------------
fn to_opp(rgb: &[u8], n: usize) -> [Vec<f64>; 3] {
    let mut o = [vec![0.0; n], vec![0.0; n], vec![0.0; n]];
    for i in 0..n { let p = [rgb[3 * i] as f64, rgb[3 * i + 1] as f64, rgb[3 * i + 2] as f64]; for c in 0..3 { o[c][i] = OPP[c][0] * p[0] + OPP[c][1] * p[1] + OPP[c][2] * p[2]; } }
    o
}
fn from_opp(o: &[Vec<f64>; 3], n: usize) -> Vec<u8> {
    let mut rgb = vec![0u8; n * 3];
    for i in 0..n { for k in 0..3 { rgb[3 * i + k] = (OPP[0][k] * o[0][i] + OPP[1][k] * o[1][i] + OPP[2][k] * o[2][i]).round().clamp(0.0, 255.0) as u8; } }
    rgb
}

/// 32x32 power spectrum of a detrended, Hann-windowed patch (encoder only; plain f64 DFT).
fn patch_psd(x: &[f64], w: usize, r0: usize, c0: usize, detrend: bool, tw: &[(f64, f64)]) -> [f64; PS * PS] {
    let mut p = [0.0; PS * PS];
    for y in 0..PS { for xx in 0..PS { p[y * PS + xx] = x[(r0 + y) * w + c0 + xx]; } }
    // plane fit a + b*x + c*y (centred coordinates make it three independent sums)
    let n = (PS * PS) as f64; let mean = p.iter().sum::<f64>() / n;
    let (mut sx, mut sy, mut sxx) = (0.0, 0.0, 0.0);
    for y in 0..PS { for xx in 0..PS { let (cx, cy) = (xx as f64 - 15.5, y as f64 - 15.5); sx += cx * p[y * PS + xx]; sy += cy * p[y * PS + xx]; sxx += cx * cx; } }
    let (bx, by) = if detrend { (sx / sxx, sy / sxx) } else { (0.0, 0.0) };
    let mut win = [0.0; PS]; for i in 0..PS { win[i] = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / (PS - 1) as f64).cos(); }
    let wn = (win.iter().map(|v| v * v).sum::<f64>() / PS as f64).sqrt();
    for y in 0..PS { for xx in 0..PS { let (cx, cy) = (xx as f64 - 15.5, y as f64 - 15.5); p[y * PS + xx] = (p[y * PS + xx] - mean - bx * cx - by * cy) * win[y] * win[xx] / (wn * wn); } }
    // 2D DFT: rows then columns
    let mut re = [0.0; PS * PS]; let mut im = [0.0; PS * PS];
    for y in 0..PS { for k in 0..PS { let (mut a, mut b) = (0.0, 0.0); for xx in 0..PS { let (c, s) = tw[(k * xx) % PS]; a += p[y * PS + xx] * c; b -= p[y * PS + xx] * s; } re[y * PS + k] = a; im[y * PS + k] = b; } }
    let mut out = [0.0; PS * PS];
    for k2 in 0..PS { for k in 0..PS { let (mut a, mut b) = (0.0, 0.0); for y in 0..PS { let (c, s) = tw[(k2 * y) % PS]; a += re[y * PS + k] * c + im[y * PS + k] * s; b += im[y * PS + k] * c - re[y * PS + k] * s; } out[k2 * PS + k] = (a * a + b * b) / n; } }
    out
}
fn twiddles() -> Vec<(f64, f64)> { (0..PS).map(|k| { let a = 2.0 * std::f64::consts::PI * k as f64 / PS as f64; (a.cos(), a.sin()) }).collect() }

/// Grain spectrum per opponent channel (absolute units): residual (image - pilot) in the flattest 6% of patches
/// (flatness judged on the pilot, so grain does not decide which patches count), median per frequency.
fn grain_spectrum(o: &[Vec<f64>; 3], pilot: &[Vec<f64>], w: usize, h: usize) -> Vec<[f64; PS * PS]> {
    let tw = twiddles();
    let mut patches = Vec::new();
    for r in (0..h.saturating_sub(PS)).step_by(PS) { for c in (0..w.saturating_sub(PS)).step_by(PS) {
        let mut g = 0.0; for y in r..r + PS - 1 { for x in c..c + PS - 1 { let i = y * w + x; g += (pilot[0][i + 1] - pilot[0][i]).abs() + (pilot[0][i + w] - pilot[0][i]).abs(); } }
        patches.push((g, r, c));
    } }
    patches.sort_by(|a, b| a.0.total_cmp(&b.0));
    let take = (patches.len() * 6 / 100).max(8).min(patches.len());
    let mut out = vec![[0.0; PS * PS]; 3];
    for ch in 0..3 {
        let res: Vec<f64> = (0..w * h).map(|i| o[ch][i] - pilot[ch][i]).collect();
        let specs: Vec<[f64; PS * PS]> = patches[..take].iter().map(|&(_, r, c)| patch_psd(&res, w, r, c, true, &tw)).collect();
        for k in 0..PS * PS { let mut v: Vec<f64> = specs.iter().map(|s| s[k]).collect(); v.sort_by(|a, b| a.total_cmp(b)); out[ch][k] = v[v.len() / 2]; }
    }
    out
}

fn dct_matrix() -> [[f64; B]; B] {
    let mut d = [[0.0; B]; B];
    for u in 0..B { let s = if u == 0 { (1.0 / B as f64).sqrt() } else { (2.0 / B as f64).sqrt() }; for x in 0..B { d[u][x] = s * (std::f64::consts::PI * (2 * x + 1) as f64 * u as f64 / (2 * B) as f64).cos(); } }
    d
}
fn dct2(d: &[[f64; B]; B], blk: &[f64; B * B], inverse: bool) -> [f64; B * B] {
    let mut t = [0.0; B * B]; let mut o = [0.0; B * B];
    // forward: D X D^T ; inverse: D^T X D
    for i in 0..B { for j in 0..B { let mut s = 0.0; for k in 0..B { s += if inverse { d[k][i] * blk[k * B + j] } else { d[i][k] * blk[k * B + j] }; } t[i * B + j] = s; } }
    for i in 0..B { for j in 0..B { let mut s = 0.0; for k in 0..B { s += if inverse { t[i * B + k] * d[k][j] } else { t[i * B + k] * d[j][k] }; } o[i * B + j] = s; } }
    o
}
fn block_starts(n: usize) -> Vec<usize> { let mut v: Vec<usize> = (0..=n.saturating_sub(B)).step_by(STEP).collect(); if *v.last().unwrap_or(&0) != n.saturating_sub(B) { v.push(n.saturating_sub(B)); } v }

/// Two-stage transform-domain denoiser for coloured noise of spectrum `psd`, strength scaled by `fac(block)`.
fn denoise_channel(x: &[f64], pilot_in: Option<&[f64]>, w: usize, h: usize, psd: &[f64; PS * PS], fac: &dyn Fn(usize, usize) -> f64) -> Vec<f64> {
    let d = dct_matrix();
    let mut var = [0.0; B * B];
    for u in 0..B { for v in 0..B { let (u2, v2) = ((PS - u) % PS, (PS - v) % PS); var[u * B + v] = ((psd[u * PS + v] + psd[u2 * PS + v] + psd[u * PS + v2] + psd[u2 * PS + v2]) / 4.0).max(1e-4); } }
    let (mut num, mut den) = (vec![0.0; w * h], vec![0.0; w * h]);
    for &r in &block_starts(h) { for &c in &block_starts(w) {
        let f = fac(r, c);
        let mut blk = [0.0; B * B]; for y in 0..B { for xx in 0..B { blk[y * B + xx] = x[(r + y) * w + c + xx]; } }
        let cx = dct2(&d, &blk, false);
        let mut cy = [0.0; B * B]; let wt;
        match pilot_in {
            None => {        // stage 1: hard threshold at 2.7 sigma
                let mut kept = 0;
                for k in 0..B * B { if k == 0 || cx[k].abs() > 2.7 * (var[k] * f * f).sqrt() { cy[k] = cx[k]; kept += 1; } }
                wt = 1.0 / kept as f64;
            }
            Some(p) => {     // stage 2: empirical Wiener from the pilot
                let mut pb = [0.0; B * B]; for y in 0..B { for xx in 0..B { pb[y * B + xx] = p[(r + y) * w + c + xx]; } }
                let cp = dct2(&d, &pb, false); let mut s = 0.0;
                for k in 0..B * B { let nv = var[k] * f * f; let g = if k == 0 { 1.0 } else { cp[k] * cp[k] / (cp[k] * cp[k] + nv) }; cy[k] = g * cx[k]; s += g * g * nv; }
                wt = 1.0 / (s + 1e-6);
            }
        }
        let y = dct2(&d, &cy, true);
        for yy in 0..B { for xx in 0..B { let i = (r + yy) * w + c + xx; num[i] += wt * y[yy * B + xx]; den[i] += wt; } }
    } }
    (0..w * h).map(|i| num[i] / den[i]).collect()
}

/// Split into (clean opponent planes, absolute grain spectrum). Encoder only.
pub fn separate(rgb: &[u8], w: usize, h: usize) -> ([Vec<f64>; 3], Vec<[f64; PS * PS]>) {
    let n = w * h; let o = to_opp(rgb, n);
    let one = |_: usize, _: usize| 1.0;
    // pass 0: white-noise pilot, sigma from the finest diagonal detail (median, robust to edges)
    let pilot0: Vec<Vec<f64>> = (0..3).map(|c| {
        let mut d: Vec<f64> = Vec::with_capacity(n / 4);
        for y in (0..h - 1).step_by(2) { for x in (0..w - 1).step_by(2) { let i = y * w + x; d.push(((o[c][i] - o[c][i + 1] - o[c][i + w] + o[c][i + w + 1]) / 2.0).abs()); } }
        d.sort_by(|a, b| a.total_cmp(b)); let s = d[d.len() / 2] / 0.6745;
        let white = [s * s; PS * PS];
        if std::env::var("GRAIN_DEBUG").is_ok() { eprintln!("ch{c} white sigma {s:.2}"); }
        let p = denoise_channel(&o[c], None, w, h, &white, &one);
        denoise_channel(&o[c], Some(&p), w, h, &white, &one)
    }).collect();
    let psd = grain_spectrum(&o, &pilot0, w, h);
    if std::env::var("GRAIN_DEBUG").is_ok() { for c in 0..3 { let p0: Vec<f64> = (0..w * h).map(|i| o[c][i] - pilot0[c][i]).collect(); eprintln!("ch{c} pilot0 removed {:.2}  spectrum rms {:.2}", std_of(&p0), (psd[c].iter().sum::<f64>() / (PS * PS) as f64).sqrt()); } }
    let pilot: Vec<Vec<f64>> = (0..3).map(|c| denoise_channel(&o[c], None, w, h, &psd[c], &one)).collect();
    // strength by brightness: stage-1 residual in FLAT pixels of each band vs in all flat pixels (the threshold's
    // bias in the residual cancels in the ratio; textured pixels would read as weak grain)
    let ybright = |i: usize| pilot[0][i] / S3 / 3.0;   // mean RGB of the pilot
    let flat = flat_mask(&pilot[0], w, h, 0.25);
    let mut fac_tab = [[1.0f64; NB]; 3];
    for c in 0..3 {
        let (mut s2, mut cnt, mut all2, mut alln) = ([0.0; NB], [0usize; NB], 0.0, 0usize);
        for i in 0..n { if !flat[i] { continue; } let b = band_of(ybright(i)).round() as usize; let r = o[c][i] - pilot[c][i]; s2[b] += r * r; cnt[b] += 1; all2 += r * r; alln += 1; }
        let base = (all2 / alln.max(1) as f64).sqrt().max(1e-6);
        for b in 0..NB { if cnt[b] > 300 { fac_tab[c][b] = ((s2[b] / cnt[b] as f64).sqrt() / base).clamp(0.3, 3.0); } }
    }
    if std::env::var("GRAIN_DEBUG").is_ok() { for c in 0..3 { eprintln!("ch{c} fac {:?}", fac_tab[c].iter().map(|v| (v * 100.0).round() / 100.0).collect::<Vec<_>>()); } }
    let clean: Vec<Vec<f64>> = (0..3).map(|c| {
        let p = &pilot[c]; let ft = fac_tab[c];
        let fac = |r: usize, cc: usize| { let mut s = 0.0; for y in 0..B { for x in 0..B { s += pilot[0][(r + y) * w + cc + x]; } } let yb = band_of(s / (B * B) as f64 / S3 / 3.0).round() as usize; ft[yb] };
        denoise_channel(&o[c], Some(p), w, h, &psd[c], &fac)
    }).collect();
    ([clean[0].clone(), clean[1].clone(), clean[2].clone()], psd)
}

/// Pixels whose 7x7 mean gradient (of plane x) is in the lowest `frac`.
fn flat_mask(x: &[f64], w: usize, h: usize, frac: f64) -> Vec<bool> {
    let mut g = vec![0.0; w * h];
    for y in 0..h - 1 { for xx in 0..w - 1 { let i = y * w + xx; g[i] = (x[i + 1] - x[i]).abs() + (x[i + w] - x[i]).abs(); } }
    let mut ii = vec![0.0; (w + 1) * (h + 1)];
    for y in 0..h { for xx in 0..w { ii[(y + 1) * (w + 1) + xx + 1] = g[y * w + xx] + ii[y * (w + 1) + xx + 1] + ii[(y + 1) * (w + 1) + xx] - ii[y * (w + 1) + xx]; } }
    let m: Vec<f64> = (0..w * h).map(|i| { let (y, xx) = (i / w, i % w); let (y0, y1, x0, x1) = (y.saturating_sub(3), (y + 4).min(h), xx.saturating_sub(3), (xx + 4).min(w));
        (ii[y1 * (w + 1) + x1] - ii[y0 * (w + 1) + x1] - ii[y1 * (w + 1) + x0] + ii[y0 * (w + 1) + x0]) / ((y1 - y0) * (x1 - x0)) as f64 }).collect();
    let mut srt = m.clone(); srt.sort_by(|a, b| a.total_cmp(b)); let t = srt[((srt.len() as f64 * frac) as usize).min(srt.len() - 1)];
    m.iter().map(|&v| v <= t).collect()
}

fn clumping(u: &[f64], w: usize, h: usize, mask: Option<&[bool]>) -> f64 {
    // spread of local energy: 6x6 box mean of u^2 (windows at least 90% inside the mask), std / mean
    let mut e = Vec::new();
    for y in (0..h - 6).step_by(2) { for x in (0..w - 6).step_by(2) {
        let (mut s, mut f) = (0.0, 0); for dy in 0..6 { for dx in 0..6 { let i = (y + dy) * w + x + dx; s += u[i] * u[i]; if let Some(m) = mask { f += m[i] as usize; } } }
        if mask.is_none() || f >= 33 { e.push(s / 36.0); } } }
    if e.is_empty() { return 0.0; }
    let m = e.iter().sum::<f64>() / e.len() as f64; (e.iter().map(|v| (v - m) * (v - m)).sum::<f64>() / e.len() as f64).sqrt() / m.max(1e-12)
}

/// Fingerprint from the original and its clean version (opponent planes). Encoder only.
pub fn fingerprint(rgb: &[u8], clean: &[Vec<f64>; 3], w: usize, h: usize, seed: u64) -> Fingerprint {
    // every statistic comes from FLAT pixels: in texture the separation is conservative (grain stays in the picture),
    // so texture would read as weak, odd-shaped grain
    let n = w * h; let o = to_opp(rgb, n);
    let g: Vec<Vec<f64>> = (0..3).map(|c| (0..n).map(|i| o[c][i] - clean[c][i]).collect()).collect();
    let yb: Vec<f64> = (0..n).map(|i| band_of(clean[0][i] / S3 / 3.0)).collect();
    let flat = flat_mask(&clean[0], w, h, 0.3);
    let fl: Vec<usize> = (0..n).filter(|&i| flat[i]).collect();
    let mut strength = [[FLOOR; 3]; NB];
    for c in 0..3 {
        let (mut s2, mut cnt) = ([0.0; NB], [0usize; NB]);
        for &i in &fl { let b = yb[i].round() as usize; s2[b] += g[c][i] * g[c][i]; cnt[b] += 1; }
        let ok: Vec<usize> = (0..NB).filter(|&b| cnt[b] > 300).collect();
        let at = |k: usize| (s2[k] / cnt[k] as f64).sqrt();
        for b in 0..NB {
            let v = if cnt[b] > 300 { at(b) } else {
                let lo = ok.iter().rev().find(|&&k| k < b); let hi = ok.iter().find(|&&k| k > b);
                match (lo, hi) { (Some(&a), Some(&z)) => at(a) + (at(z) - at(a)) * (b - a) as f64 / (z - a) as f64, (Some(&a), None) => at(a), (None, Some(&z)) => at(z), _ => FLOOR } };
            strength[b][c] = v.max(0.0);      // stored as measured; FLOOR only guards the normalising below
        }
    }
    let fpl = Fingerprint { strength, psd: vec![[0.0; PS * PS]; 3], chol: [[0.0; 3]; 3], quant: [[0.0; NZ]; 3], env: 0.0, seed };
    let unit: Vec<Vec<f64>> = (0..3).map(|c| (0..n).map(|i| (g[c][i] / strength_at(&fpl, yb[i], c).max(FLOOR)).clamp(-CLIP, CLIP)).collect()).collect();
    // spectrum of the normalised grain: mean over the flattest patches (half-overlapping), at least 8
    let tw = twiddles(); let mut psd = vec![[0.0; PS * PS]; 3];
    let mut pats = Vec::new();
    for r in (0..h.saturating_sub(PS)).step_by(PS / 2) { for cc in (0..w.saturating_sub(PS)).step_by(PS / 2) {
        let mut f = 0; for y in r..r + PS { for x in cc..cc + PS { f += flat[y * w + x] as usize; } } pats.push((f, r, cc)); } }
    pats.sort_by(|a, b| b.0.cmp(&a.0));
    let take = pats.iter().filter(|p| p.0 * 10 >= PS * PS * 9).count().max(8).min(pats.len());
    for c in 0..3 { for &(_, r, cc) in &pats[..take] { let s = patch_psd(&unit[c], w, r, cc, false, &tw); for k in 0..PS * PS { psd[c][k] += s[k]; } } for k in 0..PS * PS { psd[c][k] /= take.max(1) as f64; } }
    // coupling: Cholesky of the 3x3 covariance (flat pixels)
    let nf = fl.len().max(1) as f64;
    let mut cov = [[0.0; 3]; 3]; let mean: Vec<f64> = (0..3).map(|c| fl.iter().map(|&i| unit[c][i]).sum::<f64>() / nf).collect();
    for a in 0..3 { for b in 0..3 { cov[a][b] = fl.iter().map(|&i| (unit[a][i] - mean[a]) * (unit[b][i] - mean[b])).sum::<f64>() / nf; } }
    let mut l = [[0.0; 3]; 3];
    for i in 0..3 { for j in 0..=i { let mut s = cov[i][j]; for k in 0..j { s -= l[i][k] * l[j][k]; } l[i][j] = if i == j { s.max(1e-9).sqrt() } else { s / l[j][j] }; } }
    // quantiles on the normal-score grid (flat pixels)
    let mut quant = [[0.0; NZ]; 3];
    for c in 0..3 {
        let mut v: Vec<f64> = fl.iter().map(|&i| unit[c][i]).collect(); v.sort_by(|a, b| a.total_cmp(b)); let m = v.len();
        for k in 0..NZ { let z = -4.0 + 8.0 * k as f64 / (NZ - 1) as f64; let p = 0.5 * (1.0 + erf(z / std::f64::consts::SQRT_2)); let idx = ((p * (m - 1) as f64).round() as usize).min(m - 1); quant[c][k] = v[idx]; }
    }
    let mut fp = Fingerprint { strength, psd, chol: l, quant, env: 0.0, seed };
    // envelope: strength whose synthetic clumping matches the real grain's (flat windows; 256x256 trial)
    let target = clumping(&unit[0], w, h, Some(&flat));
    let (tw_, th_) = (256.min(w), 256.min(h)); let mut best = (0.0, f64::MAX);
    for a in [0.0, 0.05, 0.1, 0.15, 0.2, 0.25, 0.3, 0.4, 0.5] {
        let u = unit_grain(&fp, tw_, th_, a); let cl = clumping(&u[0], tw_, th_, None);
        if (cl - target).abs() < best.1 { best = (a, (cl - target).abs()); }
    }
    if std::env::var("GRAIN_DEBUG").is_ok() { eprintln!("flat px {}  psd patches {take}  clumping target {target:.3}  env {:.2}", fl.len(), best.0); }
    fp.env = best.0;
    fp
}

fn erf(x: f64) -> f64 {   // Abramowitz-Stegun 7.1.26 (encoder only; 1.5e-7)
    let s = x.signum(); let x = x.abs(); let t = 1.0 / (1.0 + 0.327_591_1 * x);
    let y = 1.0 - (((((1.061_405_429 * t - 1.453_152_027) * t) + 1.421_413_741) * t - 0.284_496_736) * t + 0.254_829_592) * t * (-x * x).exp();
    s * y
}

// ---------------------------------------------------------------------------
// stream
// ---------------------------------------------------------------------------
/// Encoder result: the stream plus the clean picture it carries (for inspection).
pub struct Encoded { pub stream: Vec<u8>, pub clean: Vec<u8>, pub fp: Fingerprint }

/// Grain mode: clean picture within +-k (k >= 1 typical: 2) + fingerprint. LOOKS THE SAME, NOT EXACT.
/// None for pictures under 64x64 (too small to measure grain); use the exact or +-k pixel mode for those.
pub fn compress(rgb: &[u8], w: usize, h: usize, k: u8) -> Option<Encoded> { compress_with(rgb, w, h, k, k, 0) }

/// Grain mode with the clean picture's green within +-k, red and blue within +-kc, and pixel-mode `flags` (learned models).
pub fn compress_with(rgb: &[u8], w: usize, h: usize, k: u8, kc: u8, flags: u8) -> Option<Encoded> {
    if w < 64 || h < 64 { return None; }
    let (clean_o, _psd) = separate(rgb, w, h);
    let mut seed = 0xcbf2_9ce4_8422_2325u64; for &b in rgb.iter().step_by(97) { seed = (seed ^ b as u64).wrapping_mul(0x100_0000_01b3); }
    let mut fp = fingerprint(rgb, &clean_o, w, h, seed);
    // no measurable grain (digital art, clean renders): nothing to strip or regrow, the picture itself goes at +-k
    let grainy = fp.strength.iter().map(|s| s[0]).sum::<f64>() / NB as f64 >= NO_GRAIN;
    let clean = if grainy { from_opp(&clean_o, w * h) } else { fp = Fingerprint { strength: [[0.0; 3]; NB], psd: vec![[0.0; PS * PS]; 3], chol: [[0.0; 3]; 3], quant: [[0.0; NZ]; 3], env: 0.0, seed: 0 }; rgb.to_vec() };
    let pix = img::compress_rgb_split(&clean, w, h, k, kc, flags);
    let fpz = crate::compress(&fp.encode());
    let mut out = Vec::with_capacity(18 + fpz.len() + pix.len());
    out.extend_from_slice(MAGIC); out.push(VERSION); out.push(k);
    out.extend_from_slice(&(w as u32).to_le_bytes()); out.extend_from_slice(&(h as u32).to_le_bytes());
    out.extend_from_slice(&(fpz.len() as u32).to_le_bytes()); out.extend_from_slice(&fpz); out.extend_from_slice(&pix);
    Some(Encoded { stream: out, clean, fp })
}

/// Returns (rgb with regenerated grain, w, h).
pub fn decompress(blob: &[u8]) -> Option<(Vec<u8>, usize, usize)> {
    if blob.len() < 18 || &blob[..4] != MAGIC || blob[4] != VERSION { return None; }
    let w = u32::from_le_bytes(blob[6..10].try_into().ok()?) as usize; let h = u32::from_le_bytes(blob[10..14].try_into().ok()?) as usize;
    let n = u32::from_le_bytes(blob[14..18].try_into().ok()?) as usize;
    if 18 + n > blob.len() { return None; }
    let fp = Fingerprint::decode(&crate::decompress(&blob[18..18 + n])?)?;
    let (clean, w2, h2, _) = img::decompress_rgb(&blob[18 + n..])?;
    if w2 != w || h2 != h || clean.len() != w * h * 3 { return None; }
    Some((regrain(&clean, w, h, &fp), w, h))
}

/// Size of the fingerprint part of a stream (bytes), for reporting.
pub fn fingerprint_bytes(blob: &[u8]) -> usize { if blob.len() < 18 { 0 } else { u32::from_le_bytes(blob[14..18].try_into().unwrap()) as usize } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grainy_gradient_round_trip() {
        // smooth gradient + coloured grain: grain mode must decode, repeat exactly, and keep the clean picture within k
        let (w, h) = (128, 96); let mut r = Rng(7); let mut rgb = vec![0u8; w * h * 3];
        for y in 0..h { for x in 0..w { for c in 0..3 { rgb[3 * (y * w + x) + c] = (60.0 + x as f64 + 20.0 * c as f64 + 4.0 * r.gauss()).round().clamp(0.0, 255.0) as u8; } } }
        let e = compress(&rgb, w, h, 2).unwrap();
        let (a, w2, h2) = decompress(&e.stream).unwrap(); let (b, _, _) = decompress(&e.stream).unwrap();
        assert_eq!((w2, h2), (w, h)); assert_eq!(a, b);
        assert!(e.fp.strength[8][0] > 1.0, "grain not measured");
        let n0 = 18 + fingerprint_bytes(&e.stream); let (clean, _, _, _) = img::decompress_rgb(&e.stream[n0..]).unwrap();
        assert!(clean.iter().zip(&e.clean).all(|(x, y)| (*x as i32 - *y as i32).abs() <= 2));
        assert!(compress(&rgb[..48 * 48 * 3], 48, 48, 2).is_none());
    }
    #[test]
    fn adaptive_allowance_round_trip() {
        // flat half then busy half: errors stay within kmax, and busy areas get the larger allowance
        let (w, h) = (64, 64); let mut r = Rng(5); let mut rgb = vec![0u8; w * h * 3];
        for y in 0..h { for x in 0..w { for c in 0..3 { let busy = if x >= 32 { 40.0 * r.gauss() } else { 0.0 };
            rgb[3 * (y * w + x) + c] = (120.0 + 10.0 * c as f64 + busy).round().clamp(0.0, 255.0) as u8; } } }
        let z = img::compress_rgb_allow(&rgb, w, h, 1, 1, [6, 6, 8], 7);
        let (back, _, _, _) = img::decompress_rgb(&z).unwrap();
        let err = |x0: usize, x1: usize| (0..h).flat_map(|y| (x0..x1).flat_map(move |x| (0..3).map(move |c| 3 * (y * w + x) + c))).map(|i| (rgb[i] as i32 - back[i] as i32).abs()).max().unwrap();
        eprintln!("flat {} busy {}", err(0, 28), err(32, 64));
        assert!(err(0, 28) <= 1 && err(32, 64) <= 6 && err(32, 64) > 1);   // columns next to the busy half may borrow its allowance
        assert!(z.len() < img::compress_rgb_flags(&rgb, w, h, 1, 7).len());
    }
    #[test]
    fn split_allowance_round_trip() {
        // green within +-1, red and blue within +-6, learned models; header carries kc
        let (w, h) = (64, 48); let mut r = Rng(3); let mut rgb = vec![0u8; w * h * 3];
        for i in 0..w * h { for c in 0..3 { rgb[3 * i + c] = (100.0 + (i % w) as f64 + 30.0 * c as f64 + 6.0 * r.gauss()).round().clamp(0.0, 255.0) as u8; } }
        let z = img::compress_rgb_split(&rgb, w, h, 1, 6, 7);
        let (back, w2, h2, _) = img::decompress_rgb(&z).unwrap(); assert_eq!((w2, h2), (w, h));
        for i in 0..w * h { for c in 0..3 { let e = (rgb[3 * i + c] as i32 - back[3 * i + c] as i32).abs(); assert!(e <= if c == 1 { 1 } else { 6 }); } }
        assert!(z.len() < img::compress_rgb_flags(&rgb, w, h, 1, 7).len());
    }
}
