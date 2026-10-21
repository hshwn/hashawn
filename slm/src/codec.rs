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
pub const VERSION: u8 = 5;
/// Below this P(end) the speculative model step is skipped (SLM_SPEC_MIN overrides).
pub const SPEC_MIN: f64 = 0.02;
/// Below this P(end) the flag is coded first (no speculative step needed); at or above it, byte first.
pub const FLAG_FIRST_BELOW: f64 = 0.2;
fn ff_below() -> f64 { std::env::var("SLM_FF").ok().and_then(|v| v.parse().ok()).unwrap_or(FLAG_FIRST_BELOW) }
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
    fn build(mut weights: Weights, vocab: &[u8], merges: &[u8], model_id: u32) -> Assets {
        let v = Vocab::load(vocab);
        weights.permute_vocab(&v.rank);
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
    br: Bridge<'a>, enc: Encoder, fm: FlagModel, spec_min: f64, ff: f64,
}
impl<'a> LmEncoder<'a> {
    pub fn new(a: &'a Assets, data: &[u8], threads: usize) -> LmEncoder<'a> {
        LmEncoder { a, data: data.to_vec(), toks: a.bpe.encode(data), ti: 0, bi: 0, pos: 0, prev: None, br: Bridge::new(Model::with_threads(&a.weights, threads), &a.vocab), enc: Encoder::new(), fm: flag_model(), spec_min: spec_min(), ff: ff_below() }
    }
    pub fn done(&self) -> usize { self.pos }
    pub fn total(&self) -> usize { self.data.len() }
    /// (tokens, model steps, speculative steps) so far.
    pub fn steps(&self) -> (usize, usize, usize) { (self.toks.len(), self.br.tokens, self.br.spec_steps) }
    /// Code up to `n` more bytes; returns how many were coded (0 when finished).
    pub fn step(&mut self, n: usize) -> usize {
        let mut side = [0i16; 8]; let mut k = 0;
        while k < n && self.ti < self.toks.len() {
            let tok = self.toks[self.ti]; let tb = &self.a.vocab.bytes[tok as usize];
            let b = tb[self.bi];
            let ended = self.bi == 0 && self.br.in_token();
            let asked = self.br.in_token() && self.br.tail_token().is_some();
            let q = if asked { self.fm.p(self.br.p_end(), self.br.tail(), self.br.prev_byte()) } else { 0.0 };
            if asked && q < self.ff {
                // flag first: no speculation; if it ends, the real step happens now
                self.enc.encode_bit(ended as u32, p12(q)); self.fm.update(ended);
                if ended { assert!(self.br.end_now()); }
                self.br.begin(0.0, false); self.br.bits(b, &mut side);
                self.enc.encode_byte(b, Some(&side));
                assert!(self.br.commit_continue(b), "tokenizer produced a token the vocab trie does not know");
                self.bi += 1; self.pos += 1; k += 1;
                if self.bi == tb.len() { self.prev = Some(tok); self.ti += 1; self.bi = 0; }
                continue;
            }
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
        if out.len() >= HEADER + total { header(MODE_STORE, 0, total, &data) } else { out }
    }
}

pub struct LmDecoder<'a> { mode: u8, total: usize, out: Vec<u8>, dec: Option<Decoder>, br: Option<Bridge<'a>>, fm: FlagModel, spec_min: f64, ff: f64, failed: bool }
impl<'a> LmDecoder<'a> {
    /// None when the header is wrong, or the stream needs a model that is missing or different.
    pub fn new(a: Option<&'a Assets>, blob: &[u8], threads: usize) -> Option<LmDecoder<'a>> {
        if blob.len() < HEADER || &blob[..4] != MAGIC || blob[4] != VERSION { return None; }
        let mode = blob[5];
        let id = u32::from_le_bytes(blob[6..10].try_into().ok()?);
        let total = u64::from_le_bytes(blob[10..18].try_into().ok()?) as usize;
        let mut d = LmDecoder { mode, total, out: Vec::with_capacity(total), dec: None, br: None, fm: flag_model(), spec_min: spec_min(), ff: ff_below(), failed: false };
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
                let (dec, br, fm, spec_min, ff) = (self.dec.as_mut().unwrap(), self.br.as_mut().unwrap(), &mut self.fm, self.spec_min, self.ff);
                for k in 0..n {
                    let asked = br.in_token() && br.tail_token().is_some();
                    let q = if asked { fm.p(br.p_end(), br.tail(), br.prev_byte()) } else { 0.0 };
                    if asked && q < ff {
                        let ended = dec.decode_bit(p12(q)) == 1; fm.update(ended);
                        if ended && !br.end_now() { self.failed = true; return k; }
                        br.begin(0.0, false); br.begin_byte();
                        let b = dec.decode_byte(|j, prefix| br.bit_stretch(j, prefix));
                        self.out.push(b);
                        if !br.commit_continue(b) { self.failed = true; return k + 1; }
                        continue;
                    }
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

/// One independent stream (no segmenting).
pub fn compress_one(a: &Assets, data: &[u8], threads: usize) -> Vec<u8> {
    if looks_like_text(data) {
        let mut e = LmEncoder::new(a, data, threads);
        while e.step(1 << 16) > 0 {}
        e.finish()
    } else {
        // the model id goes only into streams that need the model, so a browser without it writes the same bytes
        let mut e = Encoder::new(); e.push(data);
        let out = header(MODE_NCZ, 0, data.len(), &e.finish_payload());
        if out.len() >= HEADER + data.len() { header(MODE_STORE, 0, data.len(), data) } else { out }
    }
}

/// None when the header is wrong, the model differs, or the stream is inconsistent.
pub fn decompress_one(a: &Assets, blob: &[u8], threads: usize) -> Option<Vec<u8>> {
    let mut d = LmDecoder::new(Some(a), blob, threads)?;
    while d.step(1 << 16) > 0 {}
    d.finish()
}

// ---------------------------------------------------------------------------
// segments: big inputs become independent streams that compress and decompress in parallel
// ---------------------------------------------------------------------------
/// Container: "NCZL" | version | MODE_SEG | model id u32 | total u64 | n u32 | n x stream length u32 | streams.
/// Each stream is a complete NCZL stream of one segment. The split depends only on the input, never on
/// the machine, so every device writes the same bytes.
pub const MODE_SEG: u8 = 3;
/// Every segment starts cold, which costs ~1.4% (prose) to ~5% (code) at 512 KiB and ~1-2% at 1 MiB, so
/// segments are at least `SEG_MIN` and there are at most `MAX_SEGS`: n = clamp(len / SEG_MIN, 1, 8).
pub const SEG_MIN: usize = 1 << 20;
pub const MAX_SEGS: usize = 8;
fn seg_min() -> usize { std::env::var("SLM_SEG").ok().and_then(|v| v.parse().ok()).unwrap_or(SEG_MIN) }
/// Segment length for an input of `len` bytes under the policy (len itself when it stays one stream).
pub fn seg_len(len: usize, seg_min: usize) -> usize { let n = (len / seg_min).clamp(1, MAX_SEGS); (len + n - 1) / n }

/// Segment boundaries (exclusive ends): near every multiple of `seg`, just after a newline within the
/// 4 KB before it, else at a UTF-8 character start.
pub fn split_points(data: &[u8], seg: usize) -> Vec<usize> {
    let mut ends = Vec::new(); let mut prev = 0usize; let mut k = 1usize;
    while k * seg < data.len() {
        let t = k * seg;
        let lo = t.saturating_sub(4096).max(prev + 1);
        let mut cut = (lo..t).rev().find(|&i| data[i] == b'\n').map(|i| i + 1);
        if cut.is_none() { let mut c = t; while c > prev + 1 && (data[c] & 0xC0) == 0x80 { c -= 1; } cut = Some(c); }
        let c = cut.unwrap();
        if c > prev && c < data.len() { ends.push(c); prev = c; }
        k += 1;
    }
    ends.push(data.len());
    ends
}

fn workers() -> usize { std::env::var("SLM_WORKERS").ok().and_then(|v| v.parse().ok()).unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get())) }

/// Run `f` over `items` with up to `w` at a time, results in order.
fn par_map<T: Sync, R: Send>(items: &[T], w: usize, f: impl Fn(&T) -> R + Sync) -> Vec<R> {
    #[cfg(target_arch = "wasm32")] { let _ = w; return items.iter().map(&f).collect(); }
    #[allow(unreachable_code)]
    {
        let next = std::sync::atomic::AtomicUsize::new(0);
        let out: Vec<std::sync::Mutex<Option<R>>> = items.iter().map(|_| std::sync::Mutex::new(None)).collect();
        std::thread::scope(|sc| {
            for _ in 0..w.min(items.len()).max(1) {
                sc.spawn(|| loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if i >= items.len() { break; }
                    let r = f(&items[i]); *out[i].lock().unwrap() = Some(r);
                });
            }
        });
        out.into_iter().map(|m| m.into_inner().unwrap().unwrap()).collect()
    }
}

/// Assemble a segmented container from per-segment streams.
pub fn container(model_id: u32, total: usize, streams: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC); out.push(VERSION); out.push(MODE_SEG);
    out.extend_from_slice(&model_id.to_le_bytes()); out.extend_from_slice(&(total as u64).to_le_bytes());
    out.extend_from_slice(&(streams.len() as u32).to_le_bytes());
    for st in streams { out.extend_from_slice(&(st.len() as u32).to_le_bytes()); }
    for st in streams { out.extend_from_slice(st); }
    out
}
/// The streams inside a segmented container, or None if `blob` is not one.
pub fn segments(blob: &[u8]) -> Option<Vec<&[u8]>> {
    if blob.len() < HEADER + 4 || &blob[..4] != MAGIC || blob[4] != VERSION || blob[5] != MODE_SEG { return None; }
    let n = u32::from_le_bytes(blob[HEADER..HEADER + 4].try_into().ok()?) as usize;
    let mut off = HEADER + 4 + 4 * n; if off > blob.len() { return None; }
    let mut v = Vec::with_capacity(n);
    for i in 0..n {
        let l = u32::from_le_bytes(blob[HEADER + 4 + 4 * i..HEADER + 8 + 4 * i].try_into().ok()?) as usize;
        if off + l > blob.len() { return None; }
        v.push(&blob[off..off + l]); off += l;
    }
    Some(v)
}

/// Inputs above one segment are split and compressed in parallel (one model per segment, `threads` is
/// then unused); smaller ones are a single stream using `threads` inside each token.
pub fn compress(a: &Assets, data: &[u8], threads: usize) -> Vec<u8> {
    // PNGs: pixels through ncz's pixel model, every other chunk kept (pixel-exact, metadata-exact); never split
    if ncz::pngc::is_png(data) { if let Some(c) = ncz::pngc::compress(data, 0) { return c; } }
    let seg = seg_len(data.len(), seg_min());
    if seg >= data.len() { return compress_one(a, data, threads); }
    let ends = split_points(data, seg);
    let parts: Vec<&[u8]> = ends.iter().scan(0usize, |st, &e| { let p = &data[*st..e]; *st = e; Some(p) }).collect();
    let streams = par_map(&parts, workers(), |p| compress_one(a, p, 1));
    container(0, data.len(), &streams)
}

pub fn decompress(a: &Assets, blob: &[u8], threads: usize) -> Option<Vec<u8>> {
    if blob.starts_with(ncz::pngc::MAGIC) { return ncz::pngc::decompress(blob); }
    match segments(blob) {
        None => decompress_one(a, blob, threads),
        Some(parts) => {
            let total = u64::from_le_bytes(blob[10..18].try_into().ok()?) as usize;
            let outs = par_map(&parts, workers(), |p| decompress_one(a, p, 1));
            let mut out = Vec::with_capacity(total);
            for o in outs { out.extend_from_slice(&o?); }
            if out.len() == total { Some(out) } else { None }
        }
    }
}
