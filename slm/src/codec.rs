//! NCZL: ncz with the 3M language model as a side input. One stream, decodable anywhere the bits match.
//!
//! Layout: "NCZL" | version u8 | mode u8 | model id u32 LE | total bytes u64 LE | payload.
//! Modes: 0 = stored, 1 = ncz only (payload is an NCZ3 payload), 2 = ncz with the model as side input.
//! The model helps text and hurts everything else, so mode 2 is used when the input is UTF-8 text.
//! Per byte in mode 2: if inside a token, one flag bit "the token ends here" at the model's probability (straight
//! through the coder, no mixer), then the byte's eight bits through ncz's mixer with the bridge's stretch
//! predictions as the extra input. The encoder tokenises with the model's BPE; the decoder learns the
//! segmentation from the flags. The model id refuses weights that would predict different bits.
use crate::bridge::{Bridge, Vocab};
use crate::bpe::Bpe;
use crate::{Model, Weights};
use ncz::v3::{Decoder, Encoder};

pub const MAGIC: &[u8; 4] = b"NCZL";
pub const VERSION: u8 = 1;
const HEADER: usize = 4 + 1 + 1 + 4 + 8;
pub const MODE_STORE: u8 = 0; pub const MODE_NCZ: u8 = 1; pub const MODE_LM: u8 = 2;

pub struct Assets { pub weights: Weights, pub vocab: Vocab, pub bpe: Bpe, pub model_id: u32 }

impl Assets {
    /// From the raw files: manifest.txt, weights.bin (f32), vocab.bin, merges.bin.
    pub fn from_files(manifest: &str, weights: &[u8], vocab: &[u8], merges: &[u8]) -> Assets {
        let w = Weights::load(manifest, weights);
        let v = Vocab::load(vocab);
        let bpe = Bpe::load(merges, &v.bytes);
        // id = FNV-1a over the weight bytes and the vocab, truncated
        let mut h = 0xcbf29ce484222325u64;
        for &b in weights.iter().chain(vocab.iter()) { h = (h ^ b as u64).wrapping_mul(0x100000001b3); }
        Assets { weights: w, vocab: v, bpe, model_id: (h ^ (h >> 32)) as u32 }
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

fn header(mode: u8, model_id: u32, total: usize, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(HEADER + payload.len());
    out.extend_from_slice(MAGIC); out.push(VERSION); out.push(mode);
    out.extend_from_slice(&model_id.to_le_bytes());
    out.extend_from_slice(&(total as u64).to_le_bytes());
    out.extend_from_slice(payload);
    out
}

pub fn compress(a: &Assets, data: &[u8], threads: usize) -> Vec<u8> {
    let best = if looks_like_text(data) { compress_lm(a, data, threads) } else { let mut e = Encoder::new(); e.push(data); header(MODE_NCZ, a.model_id, data.len(), &e.finish_payload()) };
    if best.len() >= HEADER + data.len() { header(MODE_STORE, a.model_id, data.len(), data) } else { best }
}

fn compress_lm(a: &Assets, data: &[u8], threads: usize) -> Vec<u8> {
    let toks = a.bpe.encode(data);
    let mut br = Bridge::new(Model::with_threads(&a.weights, threads), &a.vocab);
    let mut enc = Encoder::new();
    let mut side = [0i16; 8];
    let mut prev: Option<u32> = None;
    for &tok in &toks {
        for (k, &b) in a.vocab.bytes[tok as usize].iter().enumerate() {
            if br.in_token() {
                let p = p12(br.p_end());
                if k == 0 { enc.encode_bit(1, p); br.end_token(prev.unwrap()); } else { enc.encode_bit(0, p); }
            }
            br.predict();
            br.bits(b, &mut side);
            enc.encode_byte(b, Some(&side));
            assert!(br.observe(b), "tokenizer produced a token the vocab trie does not know");
        }
        prev = Some(tok);
    }
    header(MODE_LM, a.model_id, data.len(), &enc.finish_payload())
}

/// None when the header is wrong, the model differs, or the stream is inconsistent.
pub fn decompress(a: &Assets, blob: &[u8], threads: usize) -> Option<Vec<u8>> {
    if blob.len() < HEADER || &blob[..4] != MAGIC || blob[4] != VERSION { return None; }
    let mode = blob[5];
    if u32::from_le_bytes(blob[6..10].try_into().ok()?) != a.model_id { return None; }
    let total = u64::from_le_bytes(blob[10..18].try_into().ok()?) as usize;
    match mode {
        MODE_STORE => return if blob.len() == HEADER + total { Some(blob[HEADER..].to_vec()) } else { None },
        MODE_NCZ => { let mut d = Decoder::from_payload(blob[HEADER..].to_vec(), total as u64); return Some(d.next(total)); }
        MODE_LM => {}
        _ => return None,
    }
    let mut dec = Decoder::from_payload(blob[HEADER..].to_vec(), total as u64);
    let mut br = Bridge::new(Model::with_threads(&a.weights, threads), &a.vocab);
    let mut out = Vec::with_capacity(total);
    for _ in 0..total {
        if br.in_token() {
            let p = p12(br.p_end());
            if dec.decode_bit(p) == 1 { let tok = br.tail_token()?; br.end_token(tok); }
        }
        br.predict(); br.begin_byte();
        let b = dec.decode_byte(|j, prefix| br.bit_stretch(j, prefix));
        out.push(b);
        if !br.observe(b) { return None; }
    }
    Some(out)
}
