//! ncz v2 — neural lossless compressor (context mixing, PAQ/lpaq lineage).
//! Byte-for-byte port of the Python reference (`~/ncz/ncz.py`): same `NCZ2` stream,
//! either side decodes the other. All integer math, so encoder and decoder agree
//! exactly. Streaming: feed input in chunks, pull output in chunks.

use crate::tables::{SQUASH, STRETCH};

const SLIMIT: i32 = 2047;
const ORDERS: [u32; 6] = [0, 1, 2, 3, 4, 6];
const NCTXM: usize = 6;
const TBITS: u32 = 21;
const TMASK: u32 = (1 << TBITS) - 1;
const HISTMASK: u64 = (1u64 << 48) - 1;
const PRIMES: [u64; 6] = [1, 0x85EB_CA77, 0xC2B2_AE3D, 0x27D4_EB2F, 0x1656_67B1, 0xD3A2_646C];
const C0_PRIME: u32 = 2_654_435_761;
const SM_LIMIT: u32 = 255;
const SM_INIT: u32 = 32768 << 16;
const MIN_MATCH: usize = 6;
const MTBITS: u32 = 22;
const MMASK: u32 = (1 << MTBITS) - 1;
const MHASH: u64 = 0x9E37_79B9_7F4A_7C15;
const MLEN_CAP: u32 = 31;
const N_IN: usize = NCTXM + 2;
const NMIXCTX: usize = 16;
const MIX_LR: i32 = 4;
const W_INIT: i32 = 65536 >> 3;
pub const MAGIC: &[u8; 4] = b"NCZ2";

fn recip(n: u32) -> i64 { (65536 / (n as i64 + 2)).min(32768) }

struct Apm { t: Vec<i32>, idx: usize }
impl Apm {
    fn new(n: usize) -> Apm {
        let mut t = vec![0i32; n * 33];
        for j in 0..33 {
            let k = ((j as i32 - 16) * 128 + 2048).clamp(1, 4095) as usize;
            t[j] = SQUASH[k] * 16;
        }
        for i in 1..n { for j in 0..33 { t[i * 33 + j] = t[j]; } }
        Apm { t, idx: 0 }
    }
    #[inline] fn refine(&mut self, pr: i32, cxt: usize) -> i32 {
        let s = STRETCH[pr as usize] + 2048;
        let w = s & 127;
        let i = (s >> 7) as usize + cxt * 33;
        self.idx = i;
        (self.t[i] * (128 - w) + self.t[i + 1] * w) >> 11
    }
    #[inline] fn update(&mut self, y: u32) {
        let g = if y != 0 { 65535 } else { 0 };
        let i = self.idx;
        self.t[i] += (g - self.t[i]) >> 7;
        self.t[i + 1] += (g - self.t[i + 1]) >> 7;
    }
}

pub struct Model {
    sm: Vec<Vec<u32>>, msm: Vec<u32>, mt: Vec<u32>,
    buf: Vec<u8>, weights: Vec<i32>, apm1: Apm, apm2: Apm,
    c0: u32, kbits: u32, cur_byte: u32, hist: u64, bases: [u32; NCTXM],
    idxs: [usize; NCTXM], st: [i32; N_IN], pmix: i32, mxctx: usize, m_active: bool, midx: usize,
    match_ptr: usize, match_len: u32, match_byte: i32,
}

impl Model {
    pub fn new() -> Model {
        Model {
            sm: (0..NCTXM).map(|_| vec![SM_INIT; TMASK as usize + 1]).collect(),
            msm: vec![SM_INIT; (MLEN_CAP as usize + 1) * 2],
            mt: vec![0u32; MMASK as usize + 1],
            buf: Vec::new(),
            weights: vec![W_INIT; NMIXCTX * N_IN],
            apm1: Apm::new(256), apm2: Apm::new(256),
            c0: 1, kbits: 0, cur_byte: 0, hist: 0, bases: Self::base_hashes(0),
            idxs: [0; NCTXM], st: [0; N_IN], pmix: 2048, mxctx: 0, m_active: false, midx: 0,
            match_ptr: 0, match_len: 0, match_byte: -1,
        }
    }
    fn base_hashes(hist: u64) -> [u32; NCTXM] {
        let mut b = [0u32; NCTXM];
        for i in 0..NCTXM {
            let order = ORDERS[i];
            if order == 0 { b[i] = 0; continue; }
            let cxt = hist & ((1u64 << (8 * order)) - 1);
            let mut h = (cxt.wrapping_mul(PRIMES[i]) & 0xFFFF_FFFF) as u32;
            h ^= h >> 15;
            b[i] = h & TMASK;
        }
        b
    }
    #[inline] pub fn predict(&mut self) -> i32 {
        let c0 = self.c0;
        let hashc0 = c0.wrapping_mul(C0_PRIME);
        for i in 0..NCTXM {
            let ix = ((self.bases[i] ^ hashc0) & TMASK) as usize;
            self.idxs[i] = ix;
            let p12 = ((self.sm[i][ix] >> 16) >> 4) as usize;
            self.st[i] = STRETCH[if p12 != 0 { p12 } else { 1 }];
        }
        let mut m_active = false;
        let mb = self.match_byte;
        if mb >= 0 {
            let k = self.kbits;
            if c0 == ((1 << k) | ((mb as u32) >> (8 - k))) {
                let exp_bit = ((mb as u32) >> (7 - k)) & 1;
                let mlen = self.match_len.min(MLEN_CAP);
                let midx = (mlen * 2 + exp_bit) as usize;
                self.midx = midx;
                let p12 = ((self.msm[midx] >> 16) >> 4) as usize;
                self.st[NCTXM] = STRETCH[if p12 != 0 { p12 } else { 1 }];
                m_active = true;
            }
        }
        if !m_active { self.st[NCTXM] = 0; }
        self.m_active = m_active;
        self.st[NCTXM + 1] = 256;
        let mxctx = ((m_active as usize) << 3) | ((c0 >> 5) & 7) as usize;
        self.mxctx = mxctx;
        let base = mxctx * N_IN;
        let mut dot: i64 = 0;
        for i in 0..N_IN { dot += self.weights[base + i] as i64 * self.st[i] as i64; }
        dot >>= 16;
        let dot = dot.clamp(-SLIMIT as i64, SLIMIT as i64) as i32;
        let pmix = SQUASH[(dot + 2048) as usize];
        self.pmix = pmix;
        let prev = (self.hist & 0xFF) as usize;
        let pr = (pmix + self.apm1.refine(pmix, prev) * 3) >> 2;
        let pr = (pr + self.apm2.refine(pr, c0 as usize) * 3) >> 2;
        pr.clamp(1, 4095)
    }
    #[inline] pub fn update(&mut self, y: u32) {
        let err = (((y as i32) << 12) - self.pmix) * MIX_LR;
        let base = self.mxctx * N_IN;
        for i in 0..N_IN { self.weights[base + i] += (self.st[i] * err) >> 16; }
        let target: i64 = if y != 0 { 65535 } else { 0 };
        for i in 0..NCTXM {
            let ix = self.idxs[i];
            let v = self.sm[i][ix];
            let mut p = (v >> 16) as i64; let mut n = v & 0xFFFF;
            p += ((target - p) * recip(n)) >> 16;
            p = p.clamp(1, 65534);
            if n < SM_LIMIT { n += 1; }
            self.sm[i][ix] = ((p as u32) << 16) | n;
        }
        if self.m_active {
            let mi = self.midx;
            let v = self.msm[mi];
            let mut p = (v >> 16) as i64; let mut n = v & 0xFFFF;
            p += ((target - p) * recip(n)) >> 16;
            p = p.clamp(1, 65534);
            if n < SM_LIMIT { n += 1; }
            self.msm[mi] = ((p as u32) << 16) | n;
        }
        self.apm1.update(y);
        self.apm2.update(y);
        self.cur_byte = (self.cur_byte << 1) | y;
        self.c0 = (self.c0 << 1) | y;
        self.kbits += 1;
        if self.kbits == 8 { self.byte_boundary((self.cur_byte & 0xFF) as u8); }
    }
    fn byte_boundary(&mut self, byte: u8) {
        self.buf.push(byte);
        if self.match_len > 0 && self.match_byte == byte as i32 { self.match_ptr += 1; self.match_len += 1; }
        else { self.match_len = 0; }
        self.hist = ((self.hist << 8) | byte as u64) & HISTMASK;
        self.bases = Self::base_hashes(self.hist);
        let n = self.buf.len();
        if n >= MIN_MATCH {
            let cxt = self.hist & ((1u64 << (8 * MIN_MATCH)) - 1);
            let hm = ((cxt.wrapping_mul(MHASH)) >> (64 - MTBITS)) as u32 & MMASK;
            let cand = self.mt[hm as usize];
            self.mt[hm as usize] = n as u32;
            if self.match_len == 0 && cand > 0 { self.match_ptr = cand as usize; self.match_len = 1; }
        }
        let mp = self.match_ptr;
        self.match_byte = if self.match_len > 0 && mp < n { self.buf[mp] as i32 } else { -1 };
        self.c0 = 1; self.kbits = 0; self.cur_byte = 0;
    }
}

impl Default for Model { fn default() -> Self { Model::new() } }

/// Streaming encoder: `push` bytes, then `finish`. Output accumulates in `out`.
pub struct Encoder { m: Model, x1: u32, x2: u32, pub out: Vec<u8>, pub total: u64 }
impl Encoder {
    pub fn new() -> Encoder { Encoder { m: Model::new(), x1: 0, x2: 0xFFFF_FFFF, out: Vec::new(), total: 0 } }
    pub fn push(&mut self, data: &[u8]) {
        for &byte in data {
            for shift in (0..8).rev() {
                let y = ((byte >> shift) & 1) as u32;
                let p = self.m.predict() as u32;
                let xmid = self.x1 + ((self.x2 - self.x1) >> 12) * p;
                if y != 0 { self.x2 = xmid; } else { self.x1 = xmid + 1; }
                while (self.x1 ^ self.x2) & 0xFF00_0000 == 0 {
                    self.out.push((self.x2 >> 24) as u8);
                    self.x1 <<= 8;
                    self.x2 = (self.x2 << 8) | 0xFF;
                }
                self.m.update(y);
            }
        }
        self.total += data.len() as u64;
    }
    /// Flush the coder and return the complete NCZ2 stream (header + payload).
    pub fn finish(mut self) -> Vec<u8> {
        self.out.extend_from_slice(&self.x1.to_be_bytes());
        let mut v = Vec::with_capacity(12 + self.out.len());
        v.extend_from_slice(MAGIC);
        v.extend_from_slice(&self.total.to_le_bytes());
        v.extend_from_slice(&self.out);
        v
    }
}
impl Default for Encoder { fn default() -> Self { Encoder::new() } }

/// Streaming decoder over a complete NCZ2 stream: `next(n)` yields up to n bytes.
pub struct Decoder { m: Model, x1: u32, x2: u32, x: u32, payload: Vec<u8>, pos: usize, pub remaining: u64, pub total: u64 }
impl Decoder {
    pub fn new(blob: &[u8]) -> Option<Decoder> {
        if blob.len() < 12 || &blob[..4] != MAGIC { return None; }
        let total = u64::from_le_bytes(blob[4..12].try_into().ok()?);
        let payload = blob[12..].to_vec();
        let mut d = Decoder { m: Model::new(), x1: 0, x2: 0xFFFF_FFFF, x: 0, payload, pos: 0, remaining: total, total };
        for _ in 0..4 { d.x = (d.x << 8) | d.next_in(); }
        Some(d)
    }
    #[inline] fn next_in(&mut self) -> u32 { let b = if self.pos < self.payload.len() { self.payload[self.pos] } else { 0 }; self.pos += 1; b as u32 }
    pub fn next(&mut self, max: usize) -> Vec<u8> {
        let n = (self.remaining.min(max as u64)) as usize;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let mut byte = 0u32;
            for _ in 0..8 {
                let p = self.m.predict() as u32;
                let xmid = self.x1 + ((self.x2 - self.x1) >> 12) * p;
                let y = if self.x <= xmid { self.x2 = xmid; 1 } else { self.x1 = xmid + 1; 0 };
                while (self.x1 ^ self.x2) & 0xFF00_0000 == 0 {
                    self.x1 <<= 8;
                    self.x2 = (self.x2 << 8) | 0xFF;
                    self.x = (self.x << 8) | self.next_in();
                }
                self.m.update(y);
                byte = (byte << 1) | y;
            }
            out.push(byte as u8);
        }
        self.remaining -= n as u64;
        out
    }
}

pub fn compress(data: &[u8]) -> Vec<u8> { let mut e = Encoder::new(); e.push(data); e.finish() }
pub fn decompress(blob: &[u8]) -> Option<Vec<u8>> { let mut d = Decoder::new(blob)?; let n = d.total as usize; Some(d.next(n)) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn roundtrip() {
        for data in [b"".to_vec(), b"a".to_vec(), b"the quick brown fox ".repeat(300), (0..=255u8).cycle().take(5000).collect::<Vec<_>>()] {
            let c = compress(&data); assert_eq!(decompress(&c).unwrap(), data);
        }
    }
    #[test] fn streaming_matches_whole() {
        let data: Vec<u8> = b"hello hello hello world ".repeat(200);
        let whole = compress(&data);
        let mut e = Encoder::new(); for ch in data.chunks(37) { e.push(ch); } assert_eq!(e.finish(), whole);
        let mut d = Decoder::new(&whole).unwrap(); let mut out = Vec::new(); while d.remaining > 0 { out.extend(d.next(53)); }
        assert_eq!(out, data);
    }
}
