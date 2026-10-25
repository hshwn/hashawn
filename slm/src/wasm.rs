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
//!   nczl_is_nczw(p, n) ; nczl_w_check(p, n) 0 ok, 1 not NCZW, 2 newer version, 3 damaged ; nczl_w_fits(q, pw, ph) can a
//!   picture that size be stored (and opened again) at quality q. The learned encoder and decoder networks run outside (ONNX in the page's worker).
#![allow(clippy::missing_safety_doc)]
use crate::codec::{Assets, LmDecoder, LmEncoder, looks_like_text, header, MODE_NCZ, MODE_STORE, HEADER};
use ncz::v3::Encoder;

static mut ASSETS: Option<Box<Assets>> = None;
static mut OUT_LEN: usize = 0;
static mut PREDICTORS: Vec<(u8, ncz::wonder::Predictor)> = Vec::new();
static mut WDIM: [u32; 6] = [0; 6];
#[allow(static_mut_refs)]
fn predictor(q: u8) -> Option<&'static ncz::wonder::Predictor> { unsafe { PREDICTORS.iter().find(|p| p.0 == q).map(|p| &p.1) } }
static mut TMS: Vec<(u8, ncz::tm::Tm)> = Vec::new();
#[allow(static_mut_refs)] fn tm_for(q: u8) -> Option<&'static ncz::tm::Tm> { unsafe { TMS.iter().find(|p| p.0 == q).map(|p| &p.1) } }
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
    nczl_w_enc_grain(lat, c, h, w, pw, ph, q, std::ptr::null(), 0)
}
/// With a grain fingerprint (from nczl_grain_measure; n 0: none).
#[no_mangle] #[allow(clippy::too_many_arguments)] pub unsafe extern "C" fn nczl_w_enc_grain(lat: *const i16, c: usize, h: usize, w: usize, pw: usize, ph: usize, q: u32, g: *const u8, gn: usize) -> *mut u8 {
    nczl_w_enc_full(lat, c, h, w, pw, ph, q, g, gn, std::ptr::null(), 0)
}
/// With a grain fingerprint and a detail map (opaque bytes, from the page); n 0: none.
#[no_mangle] #[allow(clippy::too_many_arguments)] pub unsafe extern "C" fn nczl_w_enc_full(lat: *const i16, c: usize, h: usize, w: usize, pw: usize, ph: usize, q: u32, g: *const u8, gn: usize, m: *const u8, mn: usize) -> *mut u8 {
    let Some(pr) = predictor(q as u8) else { return std::ptr::null_mut() };
    let grain = if gn == 0 { &[][..] } else { std::slice::from_raw_parts(g, gn) };
    let dmap = if mn == 0 { &[][..] } else { std::slice::from_raw_parts(m, mn) };
    match ncz::wonder::compress_full(std::slice::from_raw_parts(lat, c * h * w), c, h, w, pw, ph, q as u8, pr, grain, dmap) { Some(v) => leak(v), None => std::ptr::null_mut() }
}
/// With a grain fingerprint, a detail map and a swatch section (nczl_sw_enc); n 0: none.
#[no_mangle] #[allow(clippy::too_many_arguments)] pub unsafe extern "C" fn nczl_w_enc_sw(lat: *const i16, c: usize, h: usize, w: usize, pw: usize, ph: usize, q: u32, g: *const u8, gn: usize, m: *const u8, mn: usize, s: *const u8, sn: usize) -> *mut u8 {
    let Some(pr) = predictor(q as u8) else { return std::ptr::null_mut() };
    let part = |p: *const u8, n: usize| if n == 0 { &[][..] } else { std::slice::from_raw_parts(p, n) };
    match ncz::wonder::compress_swatch(std::slice::from_raw_parts(lat, c * h * w), c, h, w, pw, ph, q as u8, pr, part(g, gn), part(m, mn), part(s, sn)) { Some(v) => leak(v), None => std::ptr::null_mut() }
}
/// Swatch section for a picture (original and decoded-with-grain RGB, w x h): ptr (len nczl_out_len), 0 = none.
#[no_mangle] pub unsafe extern "C" fn nczl_sw_enc(o: *const u8, d: *const u8, w: usize, h: usize) -> *mut u8 {
    match ncz::swatch::encode(std::slice::from_raw_parts(o, w * h * 3), std::slice::from_raw_parts(d, w * h * 3), w, h) { Some((v, _)) => leak(v), None => { OUT_LEN = 0; std::ptr::null_mut() } }
}
/// Lay a swatch section's texture onto an RGB picture in place: 1 done, 0 not (damaged or not for this size).
#[no_mangle] pub unsafe extern "C" fn nczl_sw_apply(rgb: *mut u8, w: usize, h: usize, s: *const u8, sn: usize) -> u32 {
    ncz::swatch::apply(std::slice::from_raw_parts_mut(rgb, w * h * 3), w, h, std::slice::from_raw_parts(s, sn)).is_some() as u32
}
/// The swatch section stored in an NCZW stream: ptr (len nczl_out_len), 0 = none.
#[no_mangle] pub unsafe extern "C" fn nczl_w_swatch(p: *const u8, n: usize) -> *mut u8 {
    match ncz::wonder::swatches(std::slice::from_raw_parts(p, n)) { Some(g) if !g.is_empty() => leak(g.to_vec()), _ => { OUT_LEN = 0; std::ptr::null_mut() } }
}
/// The detail map stored in an NCZW stream: ptr (len nczl_out_len), 0 = none.
#[no_mangle] pub unsafe extern "C" fn nczl_w_dmap(p: *const u8, n: usize) -> *mut u8 {
    match ncz::wonder::detail_map(std::slice::from_raw_parts(p, n)) { Some(g) if !g.is_empty() => leak(g.to_vec()), _ => { OUT_LEN = 0; std::ptr::null_mut() } }
}
/// Grain fingerprint of an RGB picture, measured on a centre crop of at most est x est: ptr (len nczl_out_len), 0 = no grain.
#[no_mangle] pub unsafe extern "C" fn nczl_grain_measure(rgb: *const u8, w: usize, h: usize, est: usize) -> *mut u8 {
    match ncz::grain::measure_on(std::slice::from_raw_parts(rgb, w * h * 3), w, h, est) { Some(v) => leak(v), None => { OUT_LEN = 0; std::ptr::null_mut() } }
}
/// The grain fingerprint stored in an NCZW stream: ptr (len nczl_out_len), 0 = none.
#[no_mangle] pub unsafe extern "C" fn nczl_w_grain(p: *const u8, n: usize) -> *mut u8 {
    match ncz::wonder::grain(std::slice::from_raw_parts(p, n)) { Some(g) if !g.is_empty() => leak(g.to_vec()), _ => { OUT_LEN = 0; std::ptr::null_mut() } }
}
/// Regrow grain on an RGB picture in place. 1 = done.
#[no_mangle] pub unsafe extern "C" fn nczl_grain_apply(rgb: *mut u8, w: usize, h: usize, g: *const u8, gn: usize) -> u32 {
    ncz::grain::apply_grain(std::slice::from_raw_parts_mut(rgb, w * h * 3), w, h, std::slice::from_raw_parts(g, gn)).is_some() as u32
}
#[no_mangle] pub unsafe extern "C" fn nczl_is_nczw(p: *const u8, n: usize) -> u32 { ncz::wonder::info(std::slice::from_raw_parts(p, n)).map_or(0, |i| i.0 as u32 | 256) }
/// Can this NCZW stream be opened at all: 0 yes, 1 not NCZW, 2 made by a newer version, 3 damaged (cut short, or the CRC is wrong).
#[no_mangle] pub unsafe extern "C" fn nczl_w_check(p: *const u8, n: usize) -> u32 { ncz::wonder::check(std::slice::from_raw_parts(p, n)) as u32 }
/// Can a pw x ph picture be stored at quality q (its predictor loaded)? 1 yes, 0 too big (or no predictor).
#[no_mangle] pub extern "C" fn nczl_w_fits(q: u32, pw: usize, ph: usize) -> u32 {
    predictor(q as u8).map(|pr| pr.channels()).or(tm_for(q as u8).map(|t| t.n)).is_some_and(|c| ncz::wonder::fits(c, pw, ph)) as u32
}
/// The stream with its swatch section replaced (sn 0: none): ptr (len nczl_out_len), 0 = not an NCZW v5+ stream.
#[no_mangle] pub unsafe extern "C" fn nczl_w_set_swatch(p: *const u8, n: usize, s: *const u8, sn: usize) -> *mut u8 {
    let sw = if sn == 0 { &[][..] } else { std::slice::from_raw_parts(s, sn) };
    match ncz::wonder::with_swatches(std::slice::from_raw_parts(p, n), sw) { Some(v) => leak(v), None => std::ptr::null_mut() }
}
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
/// The teacher's entropy model (NCZT / NCZU weights, ncz::tm) for quality q: 1 loaded, 0 bad.
#[no_mangle] #[allow(static_mut_refs)] pub unsafe extern "C" fn nczl_tm_load(p: *const u8, n: usize, q: u32) -> u32 {
    match ncz::tm::Tm::load(std::slice::from_raw_parts(p, n)) { Some(t) => { TMS.retain(|x| x.0 != q as u8); TMS.push((q as u8, t)); 1 } None => 0 }
}
/// Version-7 NCZW from float latents y (c x h x w) and the hyperprior encoder's float z (c x h/4 x w/4), with the
/// grain fingerprint, detail map and swatch section (n 0: none): ptr (len nczl_out_len). y^ (what the decoder will
/// rebuild) is kept for nczl_w_yhat.
#[no_mangle] #[allow(clippy::too_many_arguments, static_mut_refs)] pub unsafe extern "C" fn nczl_w_enc_tm(y: *const f32, c: usize, h: usize, w: usize, z: *const f32, pw: usize, ph: usize, q: u32,
        g: *const u8, gn: usize, m: *const u8, mn: usize, s: *const u8, sn: usize) -> *mut u8 {
    let Some(t) = tm_for(q as u8) else { return std::ptr::null_mut() };
    let part = |p: *const u8, n: usize| if n == 0 { &[][..] } else { std::slice::from_raw_parts(p, n) };
    let zq = ncz::tm::z_symbols(std::slice::from_raw_parts(z, c * h * w / 16), h / 4, w / 4, t);
    match ncz::wonder::compress_tm(std::slice::from_raw_parts(y, c * h * w), c, h, w, pw, ph, q as u8, t, &zq, part(g, gn), part(m, mn), part(s, sn)) {
        Some((v, yh)) => { YHAT = yh; leak(v) }
        None => std::ptr::null_mut(),
    }
}
static mut YHAT: Vec<f32> = Vec::new();
/// y^ of the last nczl_w_enc_tm, f32 LE: ptr (len nczl_out_len).
#[no_mangle] #[allow(static_mut_refs)] pub unsafe extern "C" fn nczl_w_yhat() -> *mut u8 { leak(std::mem::take(&mut YHAT).iter().flat_map(|v| v.to_le_bytes()).collect()) }
/// Is this NCZW stream's latent section the teacher's entropy model (decode with nczl_w_dec_tm)? 1 yes, 0 no.
#[no_mangle] pub unsafe extern "C" fn nczl_w_uses_tm(p: *const u8, n: usize) -> u32 { ncz::wonder::uses_tm(std::slice::from_raw_parts(p, n)) as u32 }
/// Was this NCZA-carrying stream made with the entropy model loaded for its quality? 1 yes, 0 no or not loaded.
#[no_mangle] pub unsafe extern "C" fn nczl_w_tm_match(p: *const u8, n: usize) -> u32 {
    let Some((q, _, _, id)) = ncz::wonder::info(std::slice::from_raw_parts(p, n)) else { return 0 };
    tm_for(q).map_or(0, |t| (t.id == id) as u32)
}
/// y^ (f32 LE, c x h x w) of a version-7 NCZA stream: ptr (len nczl_out_len), 0 = bad; dimensions in nczl_w_dim.
#[no_mangle] pub unsafe extern "C" fn nczl_w_dec_tm(p: *const u8, n: usize) -> *mut u8 {
    let b = std::slice::from_raw_parts(p, n);
    let Some((q, pw, ph, _)) = ncz::wonder::info(b) else { return std::ptr::null_mut() };
    let Some(t) = tm_for(q) else { return std::ptr::null_mut() };
    match ncz::wonder::decompress_tm(b, t) {
        Some((yh, c, h, w)) => { WDIM = [c as u32, h as u32, w as u32, pw as u32, ph as u32, q as u32]; leak(yh.iter().flat_map(|v| v.to_le_bytes()).collect()) }
        None => std::ptr::null_mut(),
    }
}
