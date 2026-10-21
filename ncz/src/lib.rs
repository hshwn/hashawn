//! ncz — neural lossless compressor. v3 writes; v2 and v3 both read.
//! C ABI below is what the WebAssembly page and the CLI use.
#![allow(clippy::needless_range_loop)]

mod tables;
pub mod v2;
pub mod v3;
pub mod img;

pub use v3::{Encoder, compress};

/// Decode either an NCZ3 or an NCZ2 stream.
pub fn decompress(blob: &[u8]) -> Option<Vec<u8>> {
    if blob.starts_with(v3::MAGIC) { v3::decompress(blob) }
    else if blob.starts_with(v2::MAGIC) { v2::decompress(blob) }
    else { None }
}

/// Streaming decoder over either format.
pub enum Decoder { V2(v2::Decoder), V3(v3::Decoder) }
impl Decoder {
    pub fn new(blob: &[u8]) -> Option<Decoder> {
        if blob.starts_with(v3::MAGIC) { v3::Decoder::new(blob).map(Decoder::V3) }
        else if blob.starts_with(v2::MAGIC) { v2::Decoder::new(blob).map(Decoder::V2) }
        else { None }
    }
    pub fn total(&self) -> u64 { match self { Decoder::V2(d) => d.total, Decoder::V3(d) => d.total } }
    pub fn remaining(&self) -> u64 { match self { Decoder::V2(d) => d.remaining, Decoder::V3(d) => d.remaining } }
    pub fn next(&mut self, max: usize) -> Vec<u8> { match self { Decoder::V2(d) => d.next(max), Decoder::V3(d) => d.next(max) } }
}

// ---------------------------------------------------------------------------
// C ABI for WebAssembly. Handles are raw pointers; the JS side owns lifetimes.
// ---------------------------------------------------------------------------
#[no_mangle] pub extern "C" fn ncz_alloc(n: usize) -> *mut u8 { let mut v = Vec::<u8>::with_capacity(n); let p = v.as_mut_ptr(); std::mem::forget(v); p }
#[no_mangle] pub unsafe extern "C" fn ncz_free(p: *mut u8, n: usize) { if !p.is_null() { drop(Vec::from_raw_parts(p, 0, n)); } }

#[no_mangle] pub extern "C" fn ncz_enc_new() -> *mut Encoder { Box::into_raw(Box::new(Encoder::new())) }
#[no_mangle] pub unsafe extern "C" fn ncz_enc_push(e: *mut Encoder, p: *const u8, n: usize) { (*e).push(std::slice::from_raw_parts(p, n)); }
#[no_mangle] pub unsafe extern "C" fn ncz_enc_out_len(e: *mut Encoder) -> usize { (*e).out.len() }
#[no_mangle] pub unsafe extern "C" fn ncz_enc_finish(e: *mut Encoder, out_len: *mut usize) -> *mut u8 {
    let enc = Box::from_raw(e); let v = enc.finish(); *out_len = v.len();
    let mut v = v.into_boxed_slice(); let p = v.as_mut_ptr(); std::mem::forget(v); p
}
#[no_mangle] pub unsafe extern "C" fn ncz_dec_new(p: *const u8, n: usize) -> *mut Decoder {
    match Decoder::new(std::slice::from_raw_parts(p, n)) { Some(d) => Box::into_raw(Box::new(d)), None => std::ptr::null_mut() }
}
#[no_mangle] pub unsafe extern "C" fn ncz_dec_total(d: *mut Decoder) -> u64 { (*d).total() }
#[no_mangle] pub unsafe extern "C" fn ncz_dec_remaining(d: *mut Decoder) -> u64 { (*d).remaining() }
#[no_mangle] pub unsafe extern "C" fn ncz_dec_next(d: *mut Decoder, out: *mut u8, max: usize) -> usize {
    let v = (*d).next(max); std::ptr::copy_nonoverlapping(v.as_ptr(), out, v.len()); v.len()
}
#[no_mangle] pub unsafe extern "C" fn ncz_dec_free(d: *mut Decoder) { if !d.is_null() { drop(Box::from_raw(d)); } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn reads_both_formats() {
        let data = b"both formats, one decoder ".repeat(100);
        assert_eq!(decompress(&v2::compress(&data)).unwrap(), data);
        assert_eq!(decompress(&v3::compress(&data)).unwrap(), data);
        assert!(decompress(b"nope").is_none());
    }
}
