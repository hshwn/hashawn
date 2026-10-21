//! ncz v3 model — lpaq-style context mixing.
//!
//! Changes from v2:
//!   * Contexts live in a shared hash table of 16-byte buckets, one bucket per
//!     (context, nibble): [checksum][15 bit-history states]. Two lookups per byte
//!     per order instead of eight, and the states of a nibble sit in one cache line.
//!   * Each slot is a nonstationary bit-history STATE (n0, n1 with the opposite
//!     count discounted), and a per-order StateMap learns the probability of each
//!     state from the data. v2 stored a raw probability per slot.
//!   * Mixer weight set selected by the partial byte (256 sets).
//!   * Two APM stages keyed by the previous one and two bytes.
//!   * Optional word model (hash of the current and previous word) as a 7th order.
//! All integer math; encoder and decoder replay the same updates.
use crate::tables::{SQUASH, STRETCH};

const SLIMIT: i32 = 2047;
pub const MAGIC: &[u8; 4] = b"NCZ3";

/// byte-history orders feeding the hash table (plus the word model if enabled)
const ORDERS: [u32; 6] = [0, 1, 2, 3, 4, 6];
pub const WORD_MODEL: bool = true;
const NORD: usize = ORDERS.len() + if WORD_MODEL { 1 } else { 0 };
const TBITS: u32 = 22;                      // 2^22 buckets * 16 B = 64 MB, shared by all orders
const TMASK: u32 = (1 << TBITS) - 1;
const HISTMASK: u64 = (1u64 << 48) - 1;
const PRIMES: [u64; 7] = [0x9E37_79B1, 0x85EB_CA77, 0xC2B2_AE3D, 0x27D4_EB2F, 0x1656_67B1, 0xD3A2_646C, 0x3C6E_F372];
const MIN_MATCH: usize = 6;
const MTBITS: u32 = 22;
const MMASK: u32 = (1 << MTBITS) - 1;
const MHASH: u64 = 0x9E37_79B9_7F4A_7C15;
const MLEN_CAP: u32 = 31;
const N_IN: usize = NORD + 3;               // orders + match prob + match confidence + bias
const NMIXCTX: usize = 256;
const MIX_LR: i32 = 3;
const W_INIT: i32 = 65536 >> 2;
const SM_LIMIT: u32 = 1023;

static RECIP: [i32; SM_LIMIT as usize + 1] = { let mut t = [0i32; SM_LIMIT as usize + 1]; let mut n = 0; while n <= SM_LIMIT as usize { let r = 65536 / (n as i32 + 2); t[n] = if r < 32 { 32 } else { r }; n += 1; } t };
#[inline] fn recip(n: u32) -> i64 { RECIP[n as usize] as i64 }

// ---------------------------------------------------------------------------
// bit history: state byte = n0<<4 | n1, counts 0..15, nonstationary update
// ---------------------------------------------------------------------------
#[inline] fn next_state(s: u8, y: u32) -> u8 {
    let (mut n0, mut n1) = ((s >> 4) as u32, (s & 15) as u32);
    if y != 0 { n1 = (n1 + 1).min(15); if n0 > 2 { n0 = n0 / 2 + 1; } }
    else      { n0 = (n0 + 1).min(15); if n1 > 2 { n1 = n1 / 2 + 1; } }
    ((n0 << 4) | n1) as u8
}

/// Learns P(bit=1 | state) per order. value = p(16 bit) << 16 | count. Flat: NORD * 256 entries.
#[inline] fn sm_prior(s: u32) -> u32 {
    let (n0, n1) = (s >> 4, s & 15);
    (((n1 * 2 + 1) * 65536 / ((n0 + n1) * 2 + 2)).clamp(1, 65534)) << 16
}
#[inline] fn sm_update(v: u32, y: u32) -> u32 {
    let (mut p, mut n) = ((v >> 16) as i64, v & 0xFFFF);
    let target: i64 = if y != 0 { 65535 } else { 0 };
    p += ((target - p) * recip(n)) >> 16;
    p = p.clamp(1, 65534);
    if n < SM_LIMIT { n += 1; }
    ((p as u32) << 16) | n
}

struct Apm { t: Vec<u16>, idx: usize }     // 16-bit cells: values are 0..65535, so this halves the footprint for free
impl Apm {
    fn new(n: usize) -> Apm {
        let mut t = vec![0u16; n * 33];
        for j in 0..33 { let k = ((j as i32 - 16) * 128 + 2048).clamp(1, 4095) as usize; t[j] = (SQUASH[k] * 16) as u16; }
        for i in 1..n { for j in 0..33 { t[i * 33 + j] = t[j]; } }
        Apm { t, idx: 0 }
    }
    #[inline(always)] fn refine(&mut self, pr: i32, cxt: usize) -> i32 {
        let s = STRETCH[pr as usize] + 2048;
        let w = s & 127;
        let i = (s >> 7) as usize + cxt * 33;
        self.idx = i;
        unsafe { (*self.t.get_unchecked(i) as i32 * (128 - w) + *self.t.get_unchecked(i + 1) as i32 * w) >> 11 }
    }
    #[inline(always)] fn update(&mut self, y: u32) {
        let g: i32 = if y != 0 { 65535 } else { 0 };
        let i = self.idx;
        unsafe {
            let a = self.t.get_unchecked_mut(i); *a = (*a as i32 + ((g - *a as i32) >> 7)) as u16;
            let b = self.t.get_unchecked_mut(i + 1); *b = (*b as i32 + ((g - *b as i32) >> 7)) as u16;
        }
    }
}

pub struct Model {
    table: Vec<u8>,                         // shared buckets, 16 bytes each: [chk, states for nibble nodes 1..15]
    bucket: [usize; NORD],                  // byte offset of the current bucket per order (this nibble)
    ctxh: [u32; NORD],                      // context hash per order (this byte)
    sm: Vec<u32>,                           // NORD * 256 state maps
    smi: [usize; NORD],                     // state-map index used by the last predict, per order
    msm: [u32; (MLEN_CAP as usize + 1) * 2], mt: Vec<u32>, buf: Vec<u8>,
    weights: Vec<i32>, apm1: Apm, apm2: Apm,
    c0: u32, nc0: u32, kbits: u32, cur_byte: u32, hist: u64,
    word: u32, prev_word: u32, a2base: u32,
    st: [i32; N_IN], pmix: i32, mxctx: usize, m_active: bool, midx: usize,
    match_ptr: usize, match_len: u32, match_byte: i32,
}

impl Model {
    pub fn new() -> Model {
        let mut sm = vec![0u32; NORD * 256];
        for i in 0..NORD { for s in 0..256u32 { sm[i * 256 + s as usize] = sm_prior(s); } }
        let mut m = Model {
            table: vec![0u8; (TMASK as usize + 1) * 16],
            bucket: [0; NORD], ctxh: [0; NORD],
            sm, smi: [0; NORD],
            msm: [32768 << 16; (MLEN_CAP as usize + 1) * 2],
            mt: vec![0u32; MMASK as usize + 1], buf: Vec::new(),
            weights: vec![W_INIT; NMIXCTX * N_IN],
            apm1: Apm::new(65536), apm2: Apm::new(65536),
            c0: 1, nc0: 1, kbits: 0, cur_byte: 0, hist: 0, word: 0, prev_word: 0, a2base: 0,
            st: [0; N_IN], pmix: 2048, mxctx: 0, m_active: false, midx: 0,
            match_ptr: 0, match_len: 0, match_byte: -1,
        };
        m.new_byte_contexts();
        m.lookup_nibble();
        m
    }

    /// Context hashes for the coming byte, from the history.
    fn new_byte_contexts(&mut self) {
        for i in 0..ORDERS.len() {
            let order = ORDERS[i];
            let cxt = if order == 0 { 0 } else { self.hist & ((1u64 << (8 * order)) - 1) };
            let mut h = (cxt.wrapping_mul(PRIMES[i]).wrapping_add(i as u64 * 0x9E37_79B9) & 0xFFFF_FFFF) as u32;
            h ^= h >> 15; h = h.wrapping_mul(0x2C1B_3C6D); h ^= h >> 12;
            self.ctxh[i] = h;
        }
        if WORD_MODEL {
            let w = (self.word as u64 ^ ((self.prev_word as u64) << 20)).wrapping_mul(PRIMES[6]);
            let mut h = (w & 0xFFFF_FFFF) as u32; h ^= h >> 15; h = h.wrapping_mul(0x2C1B_3C6D); h ^= h >> 12;
            self.ctxh[NORD - 1] = h.wrapping_add(0x7777_7777);
        }
        let h = self.hist;
        self.a2base = (((h >> 8) & 0xFF) as u32 * 0x9E37 + ((h & 0xFF) as u32) * 0x85EB) & 0xFF00;
    }

    /// Find (or claim) the bucket for each order for the current nibble.
    fn lookup_nibble(&mut self) {
        let nib = if self.kbits < 4 { 0 } else { 16 + (self.c0 & 15) };
        for i in 0..NORD {
            let h = self.ctxh[i].wrapping_add(nib.wrapping_mul(0x61C8_8647)).wrapping_mul(0x9E37_79B1);
            let idx = (h & TMASK) as usize;
            let chk = ((h >> 24) as u8) | 1;
            let b0 = idx * 16; let b1 = (idx ^ 1) * 16;
            let t = &mut self.table;
            let b = unsafe {
                if *t.get_unchecked(b0) == chk { b0 }
                else if *t.get_unchecked(b1) == chk { b1 }
                else {
                    let pri = |s: u8| (s >> 4) + (s & 15);
                    let v = if pri(*t.get_unchecked(b0 + 1)) <= pri(*t.get_unchecked(b1 + 1)) { b0 } else { b1 };
                    for k in 0..16 { *t.get_unchecked_mut(v + k) = 0; }
                    *t.get_unchecked_mut(v) = chk; v
                }
            };
            self.bucket[i] = b;
        }
    }

    #[inline(always)] pub fn predict(&mut self) -> i32 {
        let node = self.nc0 as usize;
        unsafe {
            for i in 0..NORD {
                let s = *self.table.get_unchecked(self.bucket[i] + node) as usize;
                let si = i * 256 + s;
                self.smi[i] = si;
                let p12 = (*self.sm.get_unchecked(si) >> 20) as usize;
                self.st[i] = STRETCH[if p12 != 0 { p12 } else { 1 }];
            }
        }
        let mut m_active = false;
        let mb = self.match_byte;
        if mb >= 0 {
            let k = self.kbits;
            if self.c0 == ((1 << k) | ((mb as u32) >> (8 - k))) {
                let exp_bit = ((mb as u32) >> (7 - k)) & 1;
                let mlen = self.match_len.min(MLEN_CAP);
                let midx = (mlen * 2 + exp_bit) as usize;
                self.midx = midx;
                let p12 = ((self.msm[midx] >> 16) >> 4) as usize;
                self.st[NORD] = STRETCH[if p12 != 0 { p12 } else { 1 }];
                let conf = (mlen.min(16) as i32) * 64;
                self.st[NORD + 1] = if exp_bit != 0 { conf } else { -conf };
                m_active = true;
            }
        }
        if !m_active { self.st[NORD] = 0; self.st[NORD + 1] = 0; }
        self.m_active = m_active;
        self.st[NORD + 2] = 256;
        let mxctx = self.c0 as usize & 255;
        self.mxctx = mxctx;
        let base = mxctx * N_IN;
        let mut dot: i64 = 0;
        unsafe { for i in 0..N_IN { dot += *self.weights.get_unchecked(base + i) as i64 * self.st[i] as i64; } }
        let dot = (dot >> 16).clamp(-SLIMIT as i64, SLIMIT as i64) as i32;
        let pmix = SQUASH[(dot + 2048) as usize];
        self.pmix = pmix;
        let a1 = (self.c0 | (((self.hist & 0xFF) as u32) << 8)) as usize;
        let a2 = (self.c0 ^ self.a2base) as usize;
        let pr = (pmix + self.apm1.refine(pmix, a1) * 3) >> 2;
        let pr = (pr + self.apm2.refine(pr, a2) * 3) >> 2;
        pr.clamp(1, 4095)
    }

    #[inline(always)] pub fn update(&mut self, y: u32) {
        let err = (((y as i32) << 12) - self.pmix) * MIX_LR;
        let base = self.mxctx * N_IN;
        unsafe { for i in 0..N_IN { *self.weights.get_unchecked_mut(base + i) += (self.st[i] * err) >> 16; } }
        let node = self.nc0 as usize;
        unsafe {
            for i in 0..NORD {
                let si = self.smi[i];
                let v = self.sm.get_unchecked_mut(si); *v = sm_update(*v, y);
                let sp = self.table.get_unchecked_mut(self.bucket[i] + node); *sp = next_state(*sp, y);
            }
        }
        if self.m_active {
            let mi = self.midx;
            let v = self.msm[mi];
            let (mut p, mut n) = ((v >> 16) as i64, v & 0xFFFF);
            let target: i64 = if y != 0 { 65535 } else { 0 };
            p += ((target - p) * recip(n)) >> 16;
            p = p.clamp(1, 65534);
            if n < 255 { n += 1; }
            self.msm[mi] = ((p as u32) << 16) | n;
        }
        self.apm1.update(y);
        self.apm2.update(y);
        self.cur_byte = (self.cur_byte << 1) | y;
        self.c0 = (self.c0 << 1) | y;
        self.nc0 = (self.nc0 << 1) | y;
        self.kbits += 1;
        if self.kbits == 8 { self.byte_boundary((self.cur_byte & 0xFF) as u8); }
        else if self.kbits == 4 { self.nc0 = 1; self.lookup_nibble(); }
    }

    fn byte_boundary(&mut self, byte: u8) {
        self.buf.push(byte);
        if self.match_len > 0 && self.match_byte == byte as i32 { self.match_ptr += 1; self.match_len += 1; }
        else { self.match_len = 0; }
        self.hist = ((self.hist << 8) | byte as u64) & HISTMASK;
        if WORD_MODEL {
            let c = byte;
            if c.is_ascii_alphabetic() { self.word = self.word.wrapping_add((c | 32) as u32).wrapping_mul(0x2F0B_4F2D); }
            else if self.word != 0 { self.prev_word = self.word; self.word = 0; }
        }
        self.new_byte_contexts();
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
        self.c0 = 1; self.nc0 = 1; self.kbits = 0; self.cur_byte = 0;
        self.lookup_nibble();
    }
}
impl Default for Model { fn default() -> Self { Model::new() } }

// ---------------------------------------------------------------------------
// coder (same as v2)
// ---------------------------------------------------------------------------
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

pub struct Decoder { m: Model, x1: u32, x2: u32, x: u32, payload: Vec<u8>, pos: usize, pub remaining: u64, pub total: u64 }
impl Decoder {
    pub fn new(blob: &[u8]) -> Option<Decoder> {
        if blob.len() < 12 || &blob[..4] != MAGIC { return None; }
        let total = u64::from_le_bytes(blob[4..12].try_into().ok()?);
        let mut d = Decoder { m: Model::new(), x1: 0, x2: 0xFFFF_FFFF, x: 0, payload: blob[12..].to_vec(), pos: 0, remaining: total, total };
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
        for data in [b"".to_vec(), b"a".to_vec(), b"the quick brown fox ".repeat(300), (0..=255u8).cycle().take(5000).collect::<Vec<_>>(),
                     (0..20000u32).map(|i| (i.wrapping_mul(2654435761) >> 13) as u8).collect::<Vec<_>>()] {
            let c = compress(&data); assert_eq!(decompress(&c).unwrap(), data);
        }
    }
    #[test] fn streaming_matches_whole() {
        let data: Vec<u8> = b"hello hello hello world, said the word model. ".repeat(200);
        let whole = compress(&data);
        let mut e = Encoder::new(); for ch in data.chunks(37) { e.push(ch); } assert_eq!(e.finish(), whole);
        let mut d = Decoder::new(&whole).unwrap(); let mut out = Vec::new(); while d.remaining > 0 { out.extend(d.next(53)); }
        assert_eq!(out, data);
    }
}
