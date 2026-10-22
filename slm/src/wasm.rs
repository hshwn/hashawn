//! C ABI for the browser (wasm32, no wasm-bindgen). One global model; streaming encode/decode so a
//! worker can report progress. Without a model loaded, text still compresses, ncz-only.
//!
//!   nczl_alloc(n) / nczl_free(p, n)
//!   nczl_load(p, n) -> model id (0 = bad pack)
//!   nczl_enc_new(p, n) -> enc ; nczl_enc_step(enc, n) -> coded ; nczl_enc_done(enc) ; nczl_enc_finish(enc) -> ptr (len: nczl_out_len)
//!   nczl_dec_new(p, n) -> dec (0 = bad header / wrong model) ; nczl_dec_step(dec, n) ; nczl_dec_total(dec) ; nczl_dec_finish(dec) -> ptr or 0
//!   nczl_enc_new_mode(p, n, mode): PNGs only, mode 0 exact, 1 within +-1, 2 grain (looks the same) ; nczl_png_mode(p, n)
//!   learned mode: nczl_w_load(p, n, q) predictor for quality q ; nczl_w_enc(lat i16*, c, h, w, pw, ph, q) -> ptr ;
//!   nczl_w_dec(p, n) -> i16 latents ptr (0 = bad) ; nczl_w_dim(i) = c, h, w, picture width, height, q of the last decode ;
//!   nczl_is_nczw(p, n). The learned encoder and decoder networks run outside (ONNX in the page's worker).
#![allow(clippy::missing_safety_doc)]
use crate::codec::{Assets, LmDecoder, LmEncoder, looks_like_text, header, MODE_NCZ, MODE_STORE, HEADER};
use ncz::v3::Encoder;

static mut ASSETS: Option<Box<Assets>> = None;
static mut OUT_LEN: usize = 0;
static mut PREDICTORS: Vec<(u8, ncz::wonder::Predictor)> = Vec::new();
static mut WDIM: [u32; 6] = [0; 6];
#[allow(static_mut_refs)]
fn predictor(q: u8) -> Option<&'static ncz::wonder::Predictor> { unsafe { PREDICTORS.iter().find(|p| p.0 == q).map(|p| &p.1) } }
#[allow(static_mut_refs)]
fn assets() -> Option<&'static Assets> { unsafe { ASSETS.as_deref() } }

pub enum AnyEncoder { Lm(LmEncoder<'static>), Ncz { data: Vec<u8>, pos: usize, enc: Encoder, id: u32 }, Png { e: ncz::pngc::PngEncoder, data: Vec<u8> } }
impl AnyEncoder {
    pub fn new(data: &[u8], threads: usize) -> AnyEncoder { AnyEncoder::new_mode(data, threads, ncz::pngc::Mode::Exact) }
    /// `mode` applies to PNGs (see ncz::pngc); everything else is always byte-exact.
    pub fn new_mode(data: &[u8], threads: usize, mode: ncz::pngc::Mode) -> AnyEncoder {
        // PNG: pixel model (fast mode); falls back to the byte-exact path in finish() when it would not be smaller
        if ncz::pngc::is_png(data) { if let Some(e) = ncz::pngc::PngEncoder::new_mode(data, 0, mode) { return AnyEncoder::Png { e, data: data.to_vec() }; } }
        match assets() {
            Some(a) if looks_like_text(data) => AnyEncoder::Lm(LmEncoder::new(a, data, threads)),
            _ => AnyEncoder::Ncz { data: data.to_vec(), pos: 0, enc: Encoder::new(), id: 0 },
        }
    }
    pub fn step(&mut self, n: usize) -> usize {
        match self {
            AnyEncoder::Lm(e) => e.step(n),
            AnyEncoder::Ncz { data, pos, enc, .. } => { let k = n.min(data.len() - *pos); enc.push(&data[*pos..*pos + k]); *pos += k; k }
            AnyEncoder::Png { e, .. } => e.step(n),                        // n = rows
        }
    }
    /// Progress in input bytes (PNG: rows done scaled to the file size).
    pub fn done(&self) -> usize { match self { AnyEncoder::Lm(e) => e.done(), AnyEncoder::Ncz { pos, .. } => *pos, AnyEncoder::Png { e, data } => data.len() * e.done() / e.units().max(1) } }
    pub fn finish(self) -> Vec<u8> {
        match self {
            AnyEncoder::Lm(e) => e.finish(),
            AnyEncoder::Ncz { data, enc, id, .. } => { let out = header(MODE_NCZ, id, data.len(), &enc.finish_payload()); if out.len() >= HEADER + data.len() { header(MODE_STORE, id, data.len(), &data) } else { out } }
            AnyEncoder::Png { e, data } => match e.finish() {
                Some(out) => out,
                None => { let mut enc = Encoder::new(); enc.push(&data); let out = header(MODE_NCZ, 0, data.len(), &enc.finish_payload()); if out.len() >= HEADER + data.len() { header(MODE_STORE, 0, data.len(), &data) } else { out } }
            },
        }
    }
}

#[no_mangle] pub extern "C" fn nczl_alloc(n: usize) -> *mut u8 { let mut v = Vec::<u8>::with_capacity(n.max(1)); let p = v.as_mut_ptr(); std::mem::forget(v); p }
#[no_mangle] pub unsafe extern "C" fn nczl_free(p: *mut u8, n: usize) { if !p.is_null() { drop(Vec::from_raw_parts(p, 0, n.max(1))); } }
#[no_mangle] pub extern "C" fn nczl_out_len() -> usize { unsafe { OUT_LEN } }

#[no_mangle] pub unsafe extern "C" fn nczl_load(p: *const u8, n: usize) -> u32 {
    let pack = std::slice::from_raw_parts(p, n);
    match Assets::from_pack(pack) { Some(a) => { let id = a.model_id; ASSETS = Some(Box::new(a)); id } None => 0 }
}
#[no_mangle] pub unsafe extern "C" fn nczl_enc_new(p: *const u8, n: usize) -> *mut AnyEncoder { Box::into_raw(Box::new(AnyEncoder::new(std::slice::from_raw_parts(p, n), 1))) }
#[no_mangle] pub unsafe extern "C" fn nczl_enc_new_mode(p: *const u8, n: usize, mode: u32) -> *mut AnyEncoder {
    let m = match mode { 1 => ncz::pngc::Mode::Near(1), 2 => ncz::pngc::Mode::Grain, _ => ncz::pngc::Mode::Exact };
    Box::into_raw(Box::new(AnyEncoder::new_mode(std::slice::from_raw_parts(p, n), 1, m)))
}
#[no_mangle] pub unsafe extern "C" fn nczl_enc_step(e: *mut AnyEncoder, n: usize) -> usize { (*e).step(n) }
#[no_mangle] pub unsafe extern "C" fn nczl_enc_done(e: *mut AnyEncoder) -> usize { (*e).done() }
#[no_mangle] pub unsafe extern "C" fn nczl_enc_finish(e: *mut AnyEncoder) -> *mut u8 { let out = Box::from_raw(e).finish(); leak(out) }

/// NCZL streams, and the site's earlier plain NCZ2/NCZ3 files.
pub enum AnyDecoder { Lm(LmDecoder<'static>), Old { d: ncz::Decoder, out: Vec<u8>, total: usize }, Png(ncz::pngc::PngDecoder) }
impl AnyDecoder {
    fn new(blob: &[u8]) -> Option<AnyDecoder> {
        if blob.starts_with(ncz::pngc::MAGIC) { return ncz::pngc::PngDecoder::new(blob).map(AnyDecoder::Png); }
        if blob.starts_with(crate::codec::MAGIC) { return LmDecoder::new(assets(), blob, 1).map(AnyDecoder::Lm); }
        let d = ncz::Decoder::new(blob)?; let total = d.total() as usize;
        Some(AnyDecoder::Old { d, out: Vec::with_capacity(total), total })
    }
    fn step(&mut self, n: usize) -> usize { match self { AnyDecoder::Lm(d) => d.step(n), AnyDecoder::Old { d, out, .. } => { let c = d.next(n); out.extend_from_slice(&c); c.len() }, AnyDecoder::Png(d) => d.step(n) } }
    /// Progress (PNG: in rows; the page only uses done/total as a fraction).
    fn done(&self) -> usize { match self { AnyDecoder::Lm(d) => d.done(), AnyDecoder::Old { out, .. } => out.len(), AnyDecoder::Png(d) => d.done() } }
    fn total(&self) -> usize { match self { AnyDecoder::Lm(d) => d.total(), AnyDecoder::Old { total, .. } => *total, AnyDecoder::Png(d) => d.units() } }
    fn finish(self) -> Option<Vec<u8>> { match self { AnyDecoder::Lm(d) => d.finish(), AnyDecoder::Old { out, total, .. } => if out.len() == total { Some(out) } else { None }, AnyDecoder::Png(d) => d.finish() } }
}
#[no_mangle] pub unsafe extern "C" fn nczl_dec_new(p: *const u8, n: usize) -> *mut AnyDecoder {
    match AnyDecoder::new(std::slice::from_raw_parts(p, n)) { Some(d) => Box::into_raw(Box::new(d)), None => std::ptr::null_mut() }
}
#[no_mangle] pub unsafe extern "C" fn nczl_dec_step(d: *mut AnyDecoder, n: usize) -> usize { (*d).step(n) }
#[no_mangle] pub unsafe extern "C" fn nczl_dec_done(d: *mut AnyDecoder) -> usize { (*d).done() }
#[no_mangle] pub unsafe extern "C" fn nczl_dec_total(d: *mut AnyDecoder) -> usize { (*d).total() }
#[no_mangle] pub unsafe extern "C" fn nczl_dec_finish(d: *mut AnyDecoder) -> *mut u8 { match Box::from_raw(d).finish() { Some(out) => leak(out), None => std::ptr::null_mut() } }
/// Does this stream need the model (NCZL mode 2)? Lets the page fetch the pack only when it must.
#[no_mangle] pub unsafe extern "C" fn nczl_needs_model(p: *const u8, n: usize) -> u32 { let b = std::slice::from_raw_parts(p, n); (b.len() >= 6 && b.starts_with(crate::codec::MAGIC) && b[5] == crate::codec::MODE_LM) as u32 }
/// Is this a PNG (coded by the pixel model; step in rows, no gzip comparison)?
#[no_mangle] pub unsafe extern "C" fn nczl_is_png(p: *const u8, n: usize) -> u32 { ncz::pngc::is_png(std::slice::from_raw_parts(p, n)) as u32 }
/// Is this an NCZP stream (decodes to a PNG)?
#[no_mangle] pub unsafe extern "C" fn nczl_is_nczp(p: *const u8, n: usize) -> u32 { std::slice::from_raw_parts(p, n).starts_with(ncz::pngc::MAGIC) as u32 }
/// What an NCZP stream gives back: 0 every pixel exact (also: not an NCZP stream), k = every sample within +-k,
/// 255 = grain mode (looks the same; the grain is regrown, the pixels are not the originals).
#[no_mangle] pub unsafe extern "C" fn nczl_png_mode(p: *const u8, n: usize) -> u32 {
    match ncz::pngc::mode_of(std::slice::from_raw_parts(p, n)) { Some(ncz::pngc::Mode::Near(k)) => k as u32, Some(ncz::pngc::Mode::Grain) => 255, _ => 0 }
}
/// Would compressing this input use the model (UTF-8 text)?
#[no_mangle] pub unsafe extern "C" fn nczl_is_text(p: *const u8, n: usize) -> u32 { looks_like_text(std::slice::from_raw_parts(p, n)) as u32 }

fn leak(mut v: Vec<u8>) -> *mut u8 { v.shrink_to_fit(); unsafe { OUT_LEN = v.len(); } let p = v.as_mut_ptr(); std::mem::forget(v); p }

/// Segment plan for big inputs (same function as native, so both write the same container):
/// writes up to `max` exclusive end offsets as u32 to `out`, returns how many.
#[no_mangle] pub unsafe extern "C" fn nczl_split(p: *const u8, n: usize, out: *mut u32, max: usize) -> usize {
    if ncz::pngc::is_png(std::slice::from_raw_parts(p, n)) { if max > 0 { *out = n as u32; } return 1; }   // images are never split
    let ends = crate::codec::split_points(std::slice::from_raw_parts(p, n), crate::codec::seg_len(n, crate::codec::SEG_MIN));
    let k = ends.len().min(max);
    for i in 0..k { *out.add(i) = ends[i] as u32; }
    k
}
#[no_mangle] pub extern "C" fn nczl_seg_size() -> usize { crate::codec::SEG_MIN }
#[no_mangle] pub extern "C" fn nczl_version() -> u32 { crate::codec::VERSION as u32 }
#[no_mangle] pub extern "C" fn nczl_mode_seg() -> u32 { crate::codec::MODE_SEG as u32 }

#[no_mangle] #[allow(static_mut_refs)] pub unsafe extern "C" fn nczl_w_load(p: *const u8, n: usize, q: u32) -> u32 {
    match ncz::wonder::Predictor::load(std::slice::from_raw_parts(p, n)) {
        Some(pr) => { PREDICTORS.retain(|x| x.0 != q as u8); PREDICTORS.push((q as u8, pr)); 1 }
        None => 0,
    }
}
#[no_mangle] pub unsafe extern "C" fn nczl_w_enc(lat: *const i16, c: usize, h: usize, w: usize, pw: usize, ph: usize, q: u32) -> *mut u8 {
    let Some(pr) = predictor(q as u8) else { return std::ptr::null_mut() };
    match ncz::wonder::compress(std::slice::from_raw_parts(lat, c * h * w), c, h, w, pw, ph, q as u8, pr) { Some(v) => leak(v), None => std::ptr::null_mut() }
}
#[no_mangle] pub unsafe extern "C" fn nczl_is_nczw(p: *const u8, n: usize) -> u32 { ncz::wonder::info(std::slice::from_raw_parts(p, n)).map_or(0, |i| i.0 as u32 | 256) }
/// Was this NCZW stream made with the predictor loaded for its quality? (1 yes, 0 no or not loaded)
#[no_mangle] pub unsafe extern "C" fn nczl_w_match(p: *const u8, n: usize) -> u32 {
    let Some((q, _, _, id)) = ncz::wonder::info(std::slice::from_raw_parts(p, n)) else { return 0 };
    predictor(q).map_or(0, |pr| (pr.id == id) as u32)
}
#[no_mangle] pub unsafe extern "C" fn nczl_w_dec(p: *const u8, n: usize) -> *mut u8 {
    let b = std::slice::from_raw_parts(p, n);
    let Some((q, pw, ph, _)) = ncz::wonder::info(b) else { return std::ptr::null_mut() };
    let Some(pr) = predictor(q) else { return std::ptr::null_mut() };
    match ncz::wonder::decompress(b, pr) {
        Some((lat, c, h, w)) => { WDIM = [c as u32, h as u32, w as u32, pw as u32, ph as u32, q as u32]; leak(lat.iter().flat_map(|v| v.to_le_bytes()).collect()) }
        None => std::ptr::null_mut(),
    }
}
#[no_mangle] pub unsafe extern "C" fn nczl_w_dim(i: usize) -> u32 { WDIM[i.min(5)] }
