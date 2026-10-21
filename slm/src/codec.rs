//! NCZL: ncz with the 3M language model as a side input. One stream, decodable anywhere the bits match.
//!
//! Layout: "NCZL" | version u8 | mode u8 | model id u32 LE | total bytes u64 LE | payload.
//! Modes: 0 = stored, 1 = ncz only (payload is an NCZ3 payload), 2 = ncz with the model as side input.
//! The model helps text and hurts everything else, so mode 2 is used when the input is UTF-8 text.
//! Per byte in mode 2: if inside a token, one flag bit "the token ends here" at the model's probability
//! (straight through the coder, no mixer), then the byte's eight bits through ncz's mixer with the
//! bridge's stretch predictions as the extra input. The encoder tokenises with the model's BPE; the
//! decoder learns the segmentation from the flags. The model id refuses weights that would predict
//! different bits.
use crate::bridge::{Bridge, Flag, Vocab};
use crate::bpe::Bpe;
use crate::{Model, Weights};
use ncz::v3::{Decoder, Encoder};
use crate::flags::FlagModel;

fn flag_model() -> FlagModel { match std::env::var("SLM_FLAG_LR").ok().and_then(|v| v.parse().ok()) { Some(lr) => FlagModel::with_lr(lr), None => FlagModel::new() } }

pub const MAGIC: &[u8; 4] = b"NCZL";
pub const VERSION: u8 = 2;
/// Below this P(end) the speculative model step is skipped (SLM_SPEC_MIN overrides).
pub const SPEC_MIN: f64 = 0.02;
fn spec_min() -> f64 { std::env::var("SLM_SPEC_MIN").ok().and_then(|v| v.parse().ok()).unwrap_or(SPEC_MIN) }
pub(crate) const HEADER: usize = 4 + 1 + 1 + 4 + 8;
pub const MODE_STORE: u8 = 0; pub const MODE_NCZ: u8 = 1; pub const MODE_LM: u8 = 2;

pub struct Assets { pub weights: Weights, pub vocab: Vocab, pub bpe: Bpe, pub model_id: u32 }

impl Assets {
    /// From the raw files: manifest.txt, weights.bin (f32), vocab.bin, merges.bin. The id is the hash of the pack.
    pub fn from_files(manifest: &str, weights: &[u8], vocab: &[u8], merges: &[u8]) -> Assets {
        let w = Weights::load(manifest, weights);
        let id = crate::pack::id(&crate::pack::pack(&w, vocab, merges));
        Assets::build(w, vocab, merges, id)
    }
    /// From a pack (see `pack.rs`).
    pub fn from_pack(pack: &[u8]) -> Option<Assets> {
        let (w, vocab, merges) = crate::pack::unpack(pack)?;
        Some(Assets::build(w, &vocab, &merges, crate::pack::id(pack)))
    }
    fn build(weights: Weights, vocab: &[u8], merges: &[u8], model_id: u32) -> Assets {
        let v = Vocab::load(vocab);
        let bpe = Bpe::load(merges, &v.bytes);
        Assets { weights, vocab: v, bpe, model_id }
    }
    pub fn from_dir(dir: &str) -> Assets {
        let rd = |n: &str| std::fs::read(format!("{dir}/{n}")).unwrap_or_else(|e| panic!("{dir}/{n}: {e}"));
        Assets::from_files(&String::from_utf8(rd("manifest.txt")).unwrap(), &rd("weights.bin"), &rd("vocab.bin"), &rd("merges.bin"))
    }
}

#[inline] fn p12(p: f64) -> u32 { ((p * 4096.0).round() as i64).clamp(1, 4095) as u32 }

/// Text test: the first 64 KB decode as UTF-8 with few control characters.
pub fn looks_like_text(data: &[u8]) -> bool {
    let n = data.len().min(1 << 16); if n == 0 { return false; }
    let mut cut = n; while cut > n.saturating_sub(4) && std::str::from_utf8(&data[..cut]).is_err() { cut -= 1; }
    match std::str::from_utf8(&data[..cut]) {
        Err(_) => false,
        Ok(s) => { let ctrl = s.bytes().filter(|&b| b < 0x20 && b != b'\n' && b != b'\r' && b != b'\t').count(); ctrl * 100 < cut.max(1) }
    }
}

pub(crate) fn header(mode: u8, model_id: u32, total: usize, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER + payload.len());
    out.extend_from_slice(MAGIC); out.push(VERSION); out.push(mode);
    out.extend_from_slice(&model_id.to_le_bytes());
    out.extend_from_slice(&(total as u64).to_le_bytes());
    out.extend_from_slice(payload);
    out
}

// ---------------------------------------------------------------------------
// streaming encoder: tokenises up front, then `step(n)` codes n more bytes
// ---------------------------------------------------------------------------
pub struct LmEncoder<'a> {
    a: &'a Assets, data: Vec<u8>, toks: Vec<u32>,
    ti: usize, bi: usize, pos: usize, prev: Option<u32>,
    br: Bridge<'a>, enc: Encoder, fm: FlagModel, spec_min: f64,
}
impl<'a> LmEncoder<'a> {
    pub fn new(a: &'a Assets, data: &[u8], threads: usize) -> LmEncoder<'a> {
        LmEncoder { a, data: data.to_vec(), toks: a.bpe.encode(data), ti: 0, bi: 0, pos: 0, prev: None, br: Bridge::new(Model::with_threads(&a.weights, threads), &a.vocab), enc: Encoder::new(), fm: flag_model(), spec_min: spec_min() }
    }
    pub fn done(&self) -> usize { self.pos }
    pub fn total(&self) -> usize { self.data.len() }
    /// Code up to `n` more bytes; returns how many were coded (0 when finished).
    pub fn step(&mut self, n: usize) -> usize {
        let mut side = [0i16; 8]; let mut k = 0;
        while k < n && self.ti < self.toks.len() {
            let tok = self.toks[self.ti]; let tb = &self.a.vocab.bytes[tok as usize];
            let b = tb[self.bi];
            let ended = self.bi == 0 && self.br.in_token();
            let asked = self.br.in_token() && self.br.tail_token().is_some();
            let q = if asked { self.fm.p(self.br.p_end(), self.br.tail(), self.br.prev_byte()) } else { 0.0 };
            self.br.begin(q, q >= self.spec_min);
            self.br.bits(b, &mut side);
            self.enc.encode_byte(b, Some(&side));
            match self.br.flag(b) { Flag::Prob(p) => self.enc.encode_bit(ended as u32, p12(p)), Flag::MustEnd => debug_assert!(ended), Flag::MustContinue => debug_assert!(!ended) }
            if asked { self.fm.update(ended); }
            let ok = if ended { self.br.commit_end(b) } else { self.br.commit_continue(b) };
            assert!(ok, "tokenizer produced a token the vocab trie does not know");
            self.bi += 1; self.pos += 1; k += 1;
            if self.bi == tb.len() { self.prev = Some(tok); self.ti += 1; self.bi = 0; }
        }
        k
    }
    pub fn finish(self) -> Vec<u8> {
        let (a, total, data) = (self.a, self.data.len(), self.data);
        let out = header(MODE_LM, a.model_id, total, &self.enc.finish_payload());
        if out.len() >= HEADER + total { header(MODE_STORE, a.model_id, total, &data) } else { out }
    }
}

pub struct LmDecoder<'a> { mode: u8, total: usize, out: Vec<u8>, dec: Option<Decoder>, br: Option<Bridge<'a>>, fm: FlagModel, spec_min: f64, failed: bool }
impl<'a> LmDecoder<'a> {
    /// None when the header is wrong, or the stream needs a model that is missing or different.
    pub fn new(a: Option<&'a Assets>, blob: &[u8], threads: usize) -> Option<LmDecoder<'a>> {
        if blob.len() < HEADER || &blob[..4] != MAGIC || blob[4] != VERSION { return None; }
        let mode = blob[5];
        let id = u32::from_le_bytes(blob[6..10].try_into().ok()?);
        let total = u64::from_le_bytes(blob[10..18].try_into().ok()?) as usize;
        let mut d = LmDecoder { mode, total, out: Vec::with_capacity(total), dec: None, br: None, fm: flag_model(), spec_min: spec_min(), failed: false };
        match mode {
            MODE_STORE => { if blob.len() != HEADER + total { return None; } d.out.extend_from_slice(&blob[HEADER..]); }
            MODE_NCZ => { d.dec = Some(Decoder::from_payload(blob[HEADER..].to_vec(), total as u64)); }
            MODE_LM => { let a = a?; if a.model_id != id { return None; } d.dec = Some(Decoder::from_payload(blob[HEADER..].to_vec(), total as u64)); d.br = Some(Bridge::new(Model::with_threads(&a.weights, threads), &a.vocab)); }
            _ => return None,
        }
        Some(d)
    }
    pub fn done(&self) -> usize { self.out.len() }
    pub fn total(&self) -> usize { self.total }
    /// Decode up to `n` more bytes; returns how many (0 when finished or failed).
    pub fn step(&mut self, n: usize) -> usize {
        if self.failed { return 0; }
        let n = n.min(self.total - self.out.len());
        match self.mode {
            MODE_NCZ => { let dec = self.dec.as_mut().unwrap(); let chunk = dec.next(n); self.out.extend_from_slice(&chunk); n }
            MODE_LM => {
                let (dec, br, fm, spec_min) = (self.dec.as_mut().unwrap(), self.br.as_mut().unwrap(), &mut self.fm, self.spec_min);
                for k in 0..n {
                    let asked = br.in_token() && br.tail_token().is_some();
                    let q = if asked { fm.p(br.p_end(), br.tail(), br.prev_byte()) } else { 0.0 };
                    br.begin(q, q >= spec_min);
                    br.begin_byte();
                    let b = dec.decode_byte(|j, prefix| br.bit_stretch(j, prefix));
                    let ended = match br.flag(b) { Flag::Prob(p) => dec.decode_bit(p12(p)) == 1, Flag::MustEnd => true, Flag::MustContinue => false };
                    if asked { fm.update(ended); }
                    self.out.push(b);
                    let ok = if ended { br.commit_end(b) } else { br.commit_continue(b) };
                    if !ok { self.failed = true; return k + 1; }
                }
                n
            }
            _ => 0,
        }
    }
    /// The decoded bytes, or None if the stream was inconsistent.
    pub fn finish(self) -> Option<Vec<u8>> { if self.failed || self.out.len() != self.total { None } else { Some(self.out) } }
}

pub fn compress(a: &Assets, data: &[u8], threads: usize) -> Vec<u8> {
    if looks_like_text(data) {
        let mut e = LmEncoder::new(a, data, threads);
        while e.step(1 << 16) > 0 {}
        e.finish()
    } else {
        let mut e = Encoder::new(); e.push(data);
        let out = header(MODE_NCZ, a.model_id, data.len(), &e.finish_payload());
        if out.len() >= HEADER + data.len() { header(MODE_STORE, a.model_id, data.len(), data) } else { out }
    }
}

/// None when the header is wrong, the model differs, or the stream is inconsistent.
pub fn decompress(a: &Assets, blob: &[u8], threads: usize) -> Option<Vec<u8>> {
    let mut d = LmDecoder::new(Some(a), blob, threads)?;
    while d.step(1 << 16) > 0 {}
    d.finish()
}
