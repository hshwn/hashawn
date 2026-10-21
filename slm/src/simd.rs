//! Eight f32 lanes with lane-wise IEEE operations only (add, sub, mul, min, max, round-to-even,
//! float<->int). No fused multiply-add, no reassociation: every backend produces identical bits,
//! and the `scalar` feature builds the plain-array backend so that can be checked by hash.
//!
//! Backends: NEON (aarch64), WASM simd128, plain arrays (everything else or `--features scalar`).

#[cfg(all(target_arch = "aarch64", not(feature = "scalar")))]
mod imp {
    use std::arch::aarch64::*;
    #[derive(Clone, Copy)] pub struct F8(float32x4_t, float32x4_t);
    impl F8 {
        #[inline(always)] pub fn zero() -> F8 { unsafe { F8(vdupq_n_f32(0.0), vdupq_n_f32(0.0)) } }
        #[inline(always)] pub fn splat(x: f32) -> F8 { unsafe { F8(vdupq_n_f32(x), vdupq_n_f32(x)) } }
        #[inline(always)] pub fn load(p: &[f32]) -> F8 { debug_assert!(p.len() >= 8); unsafe { F8(vld1q_f32(p.as_ptr()), vld1q_f32(p.as_ptr().add(4))) } }
        #[inline(always)] pub fn store(self, p: &mut [f32]) { debug_assert!(p.len() >= 8); unsafe { vst1q_f32(p.as_mut_ptr(), self.0); vst1q_f32(p.as_mut_ptr().add(4), self.1); } }
        #[inline(always)] pub fn add(self, o: F8) -> F8 { unsafe { F8(vaddq_f32(self.0, o.0), vaddq_f32(self.1, o.1)) } }
        #[inline(always)] pub fn sub(self, o: F8) -> F8 { unsafe { F8(vsubq_f32(self.0, o.0), vsubq_f32(self.1, o.1)) } }
        #[inline(always)] pub fn mul(self, o: F8) -> F8 { unsafe { F8(vmulq_f32(self.0, o.0), vmulq_f32(self.1, o.1)) } }
        #[inline(always)] pub fn div(self, o: F8) -> F8 { unsafe { F8(vdivq_f32(self.0, o.0), vdivq_f32(self.1, o.1)) } }
        #[inline(always)] pub fn max(self, o: F8) -> F8 { unsafe { F8(vmaxq_f32(self.0, o.0), vmaxq_f32(self.1, o.1)) } }
        #[inline(always)] pub fn min(self, o: F8) -> F8 { unsafe { F8(vminq_f32(self.0, o.0), vminq_f32(self.1, o.1)) } }
        #[inline(always)] pub fn abs(self) -> F8 { unsafe { F8(vabsq_f32(self.0), vabsq_f32(self.1)) } }
        #[inline(always)] pub fn neg(self) -> F8 { unsafe { F8(vnegq_f32(self.0), vnegq_f32(self.1)) } }
        /// round to nearest, ties to even
        #[inline(always)] pub fn round(self) -> F8 { unsafe { F8(vrndnq_f32(self.0), vrndnq_f32(self.1)) } }
        /// 2^k for integer-valued lanes k in [-126, 127]
        #[inline(always)] pub fn pow2i(self) -> F8 { unsafe {
            let b = vdupq_n_s32(127);
            F8(vreinterpretq_f32_s32(vshlq_n_s32::<23>(vaddq_s32(vcvtq_s32_f32(self.0), b))), vreinterpretq_f32_s32(vshlq_n_s32::<23>(vaddq_s32(vcvtq_s32_f32(self.1), b))))
        } }
        /// ((l0+l4)+(l1+l5))+((l2+l6)+(l3+l7))
        #[inline(always)] pub fn hsum(self) -> f32 { unsafe { let t = vaddq_f32(self.0, self.1); let p = vpaddq_f32(t, t); vgetq_lane_f32::<0>(p) + vgetq_lane_f32::<1>(p) } }
        #[inline(always)] pub fn hmax(self) -> f32 { unsafe { vmaxvq_f32(vmaxq_f32(self.0, self.1)) } }
        /// lanes truncated to i32, written as 8 ints
        #[inline(always)] pub fn to_i32(self, out: &mut [i32]) { unsafe { vst1q_s32(out.as_mut_ptr(), vcvtq_s32_f32(self.0)); vst1q_s32(out.as_mut_ptr().add(4), vcvtq_s32_f32(self.1)); } }
        /// unbiased exponent as f32 (x must be positive normal)
        #[inline(always)] pub fn exponent(self) -> F8 { unsafe {
            let e = |v: float32x4_t| vcvtq_f32_s32(vsubq_s32(vshrq_n_s32::<23>(vreinterpretq_s32_f32(v)), vdupq_n_s32(127)));
            F8(e(self.0), e(self.1)) } }
        /// mantissa in [1, 2)
        #[inline(always)] pub fn mantissa(self) -> F8 { unsafe {
            let m = |v: float32x4_t| vreinterpretq_f32_u32(vorrq_u32(vandq_u32(vreinterpretq_u32_f32(v), vdupq_n_u32(0x007f_ffff)), vdupq_n_u32(0x3f80_0000)));
            F8(m(self.0), m(self.1)) } }
        /// lanes where self > o pick a, else b
        #[inline(always)] pub fn select_gt(self, o: F8, a: F8, b: F8) -> F8 { unsafe {
            F8(vbslq_f32(vcgtq_f32(self.0, o.0), a.0, b.0), vbslq_f32(vcgtq_f32(self.1, o.1), a.1, b.1)) } }
    }
}

#[cfg(all(target_arch = "wasm32", target_feature = "simd128", not(feature = "scalar")))]
mod imp {
    use std::arch::wasm32::*;
    #[derive(Clone, Copy)] pub struct F8(v128, v128);
    impl F8 {
        #[inline(always)] pub fn zero() -> F8 { F8(f32x4_splat(0.0), f32x4_splat(0.0)) }
        #[inline(always)] pub fn splat(x: f32) -> F8 { F8(f32x4_splat(x), f32x4_splat(x)) }
        #[inline(always)] pub fn load(p: &[f32]) -> F8 { debug_assert!(p.len() >= 8); unsafe { F8(v128_load(p.as_ptr() as *const v128), v128_load(p.as_ptr().add(4) as *const v128)) } }
        #[inline(always)] pub fn store(self, p: &mut [f32]) { debug_assert!(p.len() >= 8); unsafe { v128_store(p.as_mut_ptr() as *mut v128, self.0); v128_store(p.as_mut_ptr().add(4) as *mut v128, self.1); } }
        #[inline(always)] pub fn add(self, o: F8) -> F8 { F8(f32x4_add(self.0, o.0), f32x4_add(self.1, o.1)) }
        #[inline(always)] pub fn sub(self, o: F8) -> F8 { F8(f32x4_sub(self.0, o.0), f32x4_sub(self.1, o.1)) }
        #[inline(always)] pub fn mul(self, o: F8) -> F8 { F8(f32x4_mul(self.0, o.0), f32x4_mul(self.1, o.1)) }
        #[inline(always)] pub fn div(self, o: F8) -> F8 { F8(f32x4_div(self.0, o.0), f32x4_div(self.1, o.1)) }
        #[inline(always)] pub fn max(self, o: F8) -> F8 { F8(f32x4_pmax(o.0, self.0), f32x4_pmax(o.1, self.1)) }
        #[inline(always)] pub fn min(self, o: F8) -> F8 { F8(f32x4_pmin(o.0, self.0), f32x4_pmin(o.1, self.1)) }
        #[inline(always)] pub fn abs(self) -> F8 { F8(f32x4_abs(self.0), f32x4_abs(self.1)) }
        #[inline(always)] pub fn neg(self) -> F8 { F8(f32x4_neg(self.0), f32x4_neg(self.1)) }
        #[inline(always)] pub fn round(self) -> F8 { F8(f32x4_nearest(self.0), f32x4_nearest(self.1)) }
        #[inline(always)] pub fn pow2i(self) -> F8 {
            let b = i32x4_splat(127);
            F8(i32x4_shl(i32x4_add(i32x4_trunc_sat_f32x4(self.0), b), 23), i32x4_shl(i32x4_add(i32x4_trunc_sat_f32x4(self.1), b), 23))
        }
        #[inline(always)] pub fn hsum(self) -> f32 {
            let t = f32x4_add(self.0, self.1);
            (f32x4_extract_lane::<0>(t) + f32x4_extract_lane::<1>(t)) + (f32x4_extract_lane::<2>(t) + f32x4_extract_lane::<3>(t))
        }
        #[inline(always)] pub fn hmax(self) -> f32 {
            let t = f32x4_pmax(self.1, self.0);
            f32x4_extract_lane::<0>(t).max(f32x4_extract_lane::<1>(t)).max(f32x4_extract_lane::<2>(t)).max(f32x4_extract_lane::<3>(t))
        }
        #[inline(always)] pub fn to_i32(self, out: &mut [i32]) { unsafe { v128_store(out.as_mut_ptr() as *mut v128, i32x4_trunc_sat_f32x4(self.0)); v128_store(out.as_mut_ptr().add(4) as *mut v128, i32x4_trunc_sat_f32x4(self.1)); } }
        #[inline(always)] pub fn exponent(self) -> F8 {
            let e = |v: v128| f32x4_convert_i32x4(i32x4_sub(i32x4_shr(v, 23), i32x4_splat(127)));
            F8(e(self.0), e(self.1)) }
        #[inline(always)] pub fn mantissa(self) -> F8 {
            let m = |v: v128| v128_or(v128_and(v, u32x4_splat(0x007f_ffff)), u32x4_splat(0x3f80_0000));
            F8(m(self.0), m(self.1)) }
        #[inline(always)] pub fn select_gt(self, o: F8, a: F8, b: F8) -> F8 {
            F8(v128_bitselect(a.0, b.0, f32x4_gt(self.0, o.0)), v128_bitselect(a.1, b.1, f32x4_gt(self.1, o.1))) }
    }
}

#[cfg(not(any(all(target_arch = "aarch64", not(feature = "scalar")), all(target_arch = "wasm32", target_feature = "simd128", not(feature = "scalar")))))]
mod imp {
    #[derive(Clone, Copy)] pub struct F8([f32; 8]);
    impl F8 {
        #[inline(always)] pub fn zero() -> F8 { F8([0.0; 8]) }
        #[inline(always)] pub fn splat(x: f32) -> F8 { F8([x; 8]) }
        #[inline(always)] pub fn load(p: &[f32]) -> F8 { let mut a = [0f32; 8]; a.copy_from_slice(&p[..8]); F8(a) }
        #[inline(always)] pub fn store(self, p: &mut [f32]) { p[..8].copy_from_slice(&self.0) }
        #[inline(always)] fn map2(self, o: F8, f: impl Fn(f32, f32) -> f32) -> F8 { let mut a = [0f32; 8]; for i in 0..8 { a[i] = f(self.0[i], o.0[i]); } F8(a) }
        #[inline(always)] fn map(self, f: impl Fn(f32) -> f32) -> F8 { let mut a = [0f32; 8]; for i in 0..8 { a[i] = f(self.0[i]); } F8(a) }
        #[inline(always)] pub fn add(self, o: F8) -> F8 { self.map2(o, |a, b| a + b) }
        #[inline(always)] pub fn sub(self, o: F8) -> F8 { self.map2(o, |a, b| a - b) }
        #[inline(always)] pub fn mul(self, o: F8) -> F8 { self.map2(o, |a, b| a * b) }
        #[inline(always)] pub fn div(self, o: F8) -> F8 { self.map2(o, |a, b| a / b) }
        #[inline(always)] pub fn max(self, o: F8) -> F8 { self.map2(o, |a, b| if a > b { a } else { b }) }
        #[inline(always)] pub fn min(self, o: F8) -> F8 { self.map2(o, |a, b| if a < b { a } else { b }) }
        #[inline(always)] pub fn abs(self) -> F8 { self.map(|a| a.abs()) }
        #[inline(always)] pub fn neg(self) -> F8 { self.map(|a| -a) }
        #[inline(always)] pub fn round(self) -> F8 { self.map(|a| a.round_ties_even()) }
        #[inline(always)] pub fn pow2i(self) -> F8 { self.map(|k| f32::from_bits(((k as i32 + 127) << 23) as u32)) }
        #[inline(always)] pub fn hsum(self) -> f32 { let a = self.0; ((a[0] + a[4]) + (a[1] + a[5])) + ((a[2] + a[6]) + (a[3] + a[7])) }
        #[inline(always)] pub fn hmax(self) -> f32 { self.0.iter().cloned().fold(f32::NEG_INFINITY, f32::max) }
        #[inline(always)] pub fn to_i32(self, out: &mut [i32]) { for i in 0..8 { out[i] = self.0[i] as i32; } }
        #[inline(always)] pub fn exponent(self) -> F8 { self.map(|v| (((v.to_bits() >> 23) & 0xff) as i32 - 127) as f32) }
        #[inline(always)] pub fn mantissa(self) -> F8 { self.map(|v| f32::from_bits((v.to_bits() & 0x007f_ffff) | 0x3f80_0000)) }
        #[inline(always)] pub fn select_gt(self, o: F8, a: F8, b: F8) -> F8 { let mut r = [0f32; 8]; for i in 0..8 { r[i] = if self.0[i] > o.0[i] { a.0[i] } else { b.0[i] }; } F8(r) }
    }
}
pub use imp::F8;

const LOG2E: f32 = 1.442_695_f32;
const LN2_HI: f32 = 0.693_145_75_f32;      // ln2 split so k*LN2_HI is exact for |k| < 2^8
const LN2_LO: f32 = 1.428_606_8e-6_f32;
const LN2: f32 = 0.693_147_18_f32;

/// exp on 8 lanes; ~1 ulp; same operations in the same order as `fexp`.
#[inline(always)] pub fn exp8(x: F8) -> F8 {
    let x = x.max(F8::splat(-87.0)).min(F8::splat(88.0));
    let k = x.mul(F8::splat(LOG2E)).round();
    let r = x.sub(k.mul(F8::splat(LN2_HI))).sub(k.mul(F8::splat(LN2_LO)));
    let p = F8::splat(0.001_388_889).mul(r).add(F8::splat(0.008_333_334)).mul(r).add(F8::splat(0.041_666_668)).mul(r)
        .add(F8::splat(0.166_666_67)).mul(r).add(F8::splat(0.5)).mul(r).add(F8::splat(1.0)).mul(r).add(F8::splat(1.0));
    k.pow2i().mul(p)
}
/// Scalar twin of `exp8`: identical bits lane for lane.
#[inline] pub fn fexp(x: f32) -> f32 {
    let x = x.clamp(-87.0, 88.0);
    let k = (x * LOG2E).round_ties_even();
    let r = (x - k * LN2_HI) - k * LN2_LO;
    let p = ((((((0.001_388_889 * r + 0.008_333_334) * r + 0.041_666_668) * r + 0.166_666_67) * r + 0.5) * r + 1.0) * r) + 1.0;
    f32::from_bits(((k as i32 + 127) << 23) as u32) * p
}
/// ln on 8 lanes (x > 0, normal), ~1e-7 relative; same operations as `fln`.
#[inline(always)] pub fn ln8(x: F8) -> F8 {
    let e0 = x.exponent(); let m0 = x.mantissa();                                 // [1, 2)
    let thr = F8::splat(1.414_213_6);
    let m = m0.select_gt(thr, m0.mul(F8::splat(0.5)), m0);                        // [0.707, 1.414)
    let e = m0.select_gt(thr, e0.add(F8::splat(1.0)), e0);
    let one = F8::splat(1.0);
    let t = m.sub(one).div(m.add(one));                                           // |t| <= 0.172
    let t2 = t.mul(t);
    let s = F8::splat(0.181_818_18).mul(t2).add(F8::splat(0.222_222_22)).mul(t2).add(F8::splat(0.285_714_3)).mul(t2)
        .add(F8::splat(0.4)).mul(t2).add(F8::splat(0.666_666_7)).mul(t2).add(F8::splat(2.0)).mul(t);
    e.mul(F8::splat(LN2)).add(s)
}
/// Scalar twin of `ln8`.
#[inline] pub fn fln(x: f32) -> f32 {
    let bits = x.to_bits();
    let mut e = (((bits >> 23) & 0xff) as i32 - 127) as f32;
    let mut m = f32::from_bits((bits & 0x007f_ffff) | 0x3f80_0000);
    if m > 1.414_213_6 { m *= 0.5; e += 1.0; }
    let t = (m - 1.0) / (m + 1.0);
    let t2 = t * t;
    let s = (((((0.181_818_18 * t2 + 0.222_222_22) * t2 + 0.285_714_3) * t2 + 0.4) * t2 + 0.666_666_7) * t2 + 2.0) * t;
    e * LN2 + s
}
/// softplus(x) = max(x,0) + ln(1 + exp(-|x|)) on 8 lanes
#[inline(always)] pub fn softplus8(x: F8) -> F8 { x.max(F8::zero()).add(ln8(F8::splat(1.0).add(exp8(x.abs().neg())))) }
#[inline(always)] pub fn sigmoid8(x: F8) -> F8 { recip8(F8::splat(1.0).add(exp8(x.neg()))) }
/// 1/x lane-wise; IEEE division is exact, so identical everywhere.
#[inline(always)] pub fn recip8(x: F8) -> F8 { F8::splat(1.0).div(x) }
#[inline] pub fn sigmoid(x: f32) -> f32 { 1.0 / (1.0 + fexp(-x)) }

// ---------------------------------------------------------------------------
// kernels (all lengths multiples of 8)
// ---------------------------------------------------------------------------
#[inline] pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len(); debug_assert!(n % 8 == 0 && b.len() >= n);
    let mut acc = F8::zero();
    let mut i = 0;
    while i < n { acc = acc.add(F8::load(&a[i..]).mul(F8::load(&b[i..]))); i += 8; }
    acc.hsum()
}
/// out[j] = dot(x, rows[j]) * scale, four rows at a time.
#[inline] pub fn dots(x: &[f32], rows: &[f32], scale: f32, out: &mut [f32]) {
    let n = x.len(); let m = out.len(); debug_assert!(n % 8 == 0 && rows.len() >= m * n);
    let mut j = 0;
    while j + 4 <= m {
        let (r0, r1, r2, r3) = (&rows[j * n..], &rows[(j + 1) * n..], &rows[(j + 2) * n..], &rows[(j + 3) * n..]);
        let (mut a0, mut a1, mut a2, mut a3) = (F8::zero(), F8::zero(), F8::zero(), F8::zero());
        let mut i = 0;
        while i < n {
            let xv = F8::load(&x[i..]);
            a0 = a0.add(F8::load(&r0[i..]).mul(xv)); a1 = a1.add(F8::load(&r1[i..]).mul(xv));
            a2 = a2.add(F8::load(&r2[i..]).mul(xv)); a3 = a3.add(F8::load(&r3[i..]).mul(xv));
            i += 8;
        }
        out[j] = a0.hsum() * scale; out[j + 1] = a1.hsum() * scale; out[j + 2] = a2.hsum() * scale; out[j + 3] = a3.hsum() * scale;
        j += 4;
    }
    while j < m { out[j] = dot(x, &rows[j * n..(j + 1) * n]) * scale; j += 1; }
}
/// acc += p * v
#[inline] pub fn axpy(acc: &mut [f32], p: f32, v: &[f32]) {
    let n = acc.len(); debug_assert!(n % 8 == 0 && v.len() >= n);
    let pv = F8::splat(p); let mut i = 0;
    while i < n { F8::load(&acc[i..]).add(pv.mul(F8::load(&v[i..]))).store(&mut acc[i..]); i += 8; }
}
/// x = exp(x - m) in place; returns the fixed-order sum.
#[inline] pub fn exp_sub_sum(x: &mut [f32], m: f32) -> f32 {
    let n = x.len(); debug_assert!(n % 8 == 0);
    let mv = F8::splat(m); let mut s = F8::zero(); let mut i = 0;
    while i < n { let e = exp8(F8::load(&x[i..]).sub(mv)); e.store(&mut x[i..]); s = s.add(e); i += 8; }
    s.hsum()
}
#[inline] pub fn max(x: &[f32]) -> f32 {
    let n = x.len(); debug_assert!(n % 8 == 0);
    let mut m = F8::splat(f32::NEG_INFINITY); let mut i = 0;
    while i < n { m = m.max(F8::load(&x[i..])); i += 8; }
    m.hmax()
}
/// out[i] = sigmoid(x[i])
#[inline] pub fn sigmoid_into(x: &[f32], out: &mut [f32]) {
    let n = x.len(); debug_assert!(n % 8 == 0); let mut i = 0;
    while i < n { sigmoid8(F8::load(&x[i..])).store(&mut out[i..]); i += 8; }
}
/// x[i] = x[i] * sigmoid(x[i]) in place
#[inline] pub fn silu_inplace(x: &mut [f32]) {
    let n = x.len(); debug_assert!(n % 8 == 0); let mut i = 0;
    while i < n { let v = F8::load(&x[i..]); v.mul(sigmoid8(v)).store(&mut x[i..]); i += 8; }
}
/// out[i] = x[i] * r * w[i]
#[inline] pub fn scale_mul(x: &[f32], r: f32, w: &[f32], out: &mut [f32]) {
    let n = x.len(); debug_assert!(n % 8 == 0); let rv = F8::splat(r); let mut i = 0;
    while i < n { F8::load(&x[i..]).mul(rv).mul(F8::load(&w[i..])).store(&mut out[i..]); i += 8; }
}
/// Quantise to int8 with scale amax/127 (round to nearest even). Returns the scale.
#[inline] pub fn quantize(x: &[f32], q: &mut [i8]) -> f32 {
    let n = x.len(); debug_assert!(n % 8 == 0);
    let mut m = F8::zero(); let mut i = 0;
    while i < n { m = m.max(F8::load(&x[i..]).abs()); i += 8; }
    let amax = m.hmax();
    if amax == 0.0 { for v in q[..n].iter_mut() { *v = 0; } return 0.0; }
    let inv = F8::splat(127.0 / amax);
    let mut tmp = [0i32; 8]; i = 0;
    while i < n { F8::load(&x[i..]).mul(inv).round().to_i32(&mut tmp); for l in 0..8 { q[i + l] = tmp[l] as i8; } i += 8; }
    amax / 127.0
}

/// acc += p0 * v0 + p1 * v1 with two independent chains (fixed order: (acc + p0*v0) + p1*v1).
#[inline] pub fn axpy2(acc: &mut [f32], p0: f32, v0: &[f32], p1: f32, v1: &[f32]) {
    let n = acc.len(); debug_assert!(n % 8 == 0);
    let (a, b) = (F8::splat(p0), F8::splat(p1)); let mut i = 0;
    while i < n { F8::load(&acc[i..]).add(a.mul(F8::load(&v0[i..]))).add(b.mul(F8::load(&v1[i..]))).store(&mut acc[i..]); i += 8; }
}

/// Exact int8 dot products of one query against many keys: out[j] = (q . k_j) as f32 * (sq * ks[j]) * scale.
/// Key length must be a multiple of 16.
#[inline] pub fn qdots(q: &[i8], sq: f32, keys: &[i8], ks: &[f32], scale: f32, out: &mut [f32]) {
    let d = q.len(); debug_assert!(d % 16 == 0);
    #[cfg(all(target_arch = "aarch64", not(feature = "scalar")))]
    { unsafe { qdots_neon(q, sq, keys, ks, scale, out) }; return; }
    #[allow(unreachable_code)]
    for (j, k) in keys.chunks_exact(d).enumerate().take(out.len()) {
        let mut acc = [0i32; 4];
        for (c, qc) in k.chunks_exact(4).zip(q.chunks_exact(4)) { for l in 0..4 { acc[l] += c[l] as i32 * qc[l] as i32; } }
        out[j] = ((acc[0] + acc[1]) + (acc[2] + acc[3])) as f32 * (sq * ks[j]) * scale;
    }
}
#[cfg(all(target_arch = "aarch64", not(feature = "scalar")))]
#[target_feature(enable = "neon,dotprod")]
unsafe fn qdots_neon(q: &[i8], sq: f32, keys: &[i8], ks: &[f32], scale: f32, out: &mut [f32]) {
    use std::arch::aarch64::*;
    let d = q.len(); let m = out.len(); let kp = keys.as_ptr(); let qp = q.as_ptr();
    let mut j = 0;
    while j + 4 <= m {
        let (k0, k1, k2, k3) = (kp.add(j * d), kp.add((j + 1) * d), kp.add((j + 2) * d), kp.add((j + 3) * d));
        let (mut a0, mut a1, mut a2, mut a3) = (vdupq_n_s32(0), vdupq_n_s32(0), vdupq_n_s32(0), vdupq_n_s32(0));
        let mut i = 0;
        while i < d {
            let qv = vld1q_s8(qp.add(i));
            a0 = crate::sdot(a0, vld1q_s8(k0.add(i)), qv); a1 = crate::sdot(a1, vld1q_s8(k1.add(i)), qv);
            a2 = crate::sdot(a2, vld1q_s8(k2.add(i)), qv); a3 = crate::sdot(a3, vld1q_s8(k3.add(i)), qv);
            i += 16;
        }
        *out.get_unchecked_mut(j) = vaddvq_s32(a0) as f32 * (sq * ks[j]) * scale;
        *out.get_unchecked_mut(j + 1) = vaddvq_s32(a1) as f32 * (sq * ks[j + 1]) * scale;
        *out.get_unchecked_mut(j + 2) = vaddvq_s32(a2) as f32 * (sq * ks[j + 2]) * scale;
        *out.get_unchecked_mut(j + 3) = vaddvq_s32(a3) as f32 * (sq * ks[j + 3]) * scale;
        j += 4;
    }
    while j < m {
        let k0 = kp.add(j * d); let mut a0 = vdupq_n_s32(0); let mut i = 0;
        while i < d { a0 = crate::sdot(a0, vld1q_s8(k0.add(i)), vld1q_s8(qp.add(i))); i += 16; }
        *out.get_unchecked_mut(j) = vaddvq_s32(a0) as f32 * (sq * ks[j]) * scale;
        j += 1;
    }
}
