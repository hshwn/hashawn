//! One file for the web: the quantised weights exactly as the model uses them, plus the tokenizer.
//! "SLMQ" | version u8 | sections in a fixed order. The model id is the hash of this file, so a stream
//! made natively from weights.bin and one made in the browser from the pack agree on the id and the bits.
use crate::{AttnRes, Attn, Gdn, Layer, Mixer, QMat, Weights, NL, V, is_attn};

pub const MAGIC: &[u8; 4] = b"SLMQ";
pub const VERSION: u8 = 1;

struct W(Vec<u8>);
impl W {
    fn u32(&mut self, v: u32) { self.0.extend_from_slice(&v.to_le_bytes()); }
    fn f32s(&mut self, v: &[f32]) { self.u32(v.len() as u32); for x in v { self.0.extend_from_slice(&x.to_le_bytes()); } }
    fn bytes(&mut self, v: &[u8]) { self.u32(v.len() as u32); self.0.extend_from_slice(v); }
    fn qmat(&mut self, m: &QMat) { self.u32(m.n_in as u32); self.u32(m.n_out as u32); self.0.extend_from_slice(unsafe { std::slice::from_raw_parts(m.w.as_ptr() as *const u8, m.w.len()) }); self.f32s(&m.scale); }
    fn res(&mut self, r: &AttnRes) { self.f32s(&r.q); }
}
struct R<'a> { b: &'a [u8], p: usize }
impl<'a> R<'a> {
    fn u32(&mut self) -> u32 { let v = u32::from_le_bytes(self.b[self.p..self.p + 4].try_into().unwrap()); self.p += 4; v }
    fn f32s(&mut self) -> Vec<f32> { let n = self.u32() as usize; let v = self.b[self.p..self.p + 4 * n].chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect(); self.p += 4 * n; v }
    fn bytes(&mut self) -> Vec<u8> { let n = self.u32() as usize; let v = self.b[self.p..self.p + n].to_vec(); self.p += n; v }
    fn qmat(&mut self) -> QMat { let n_in = self.u32() as usize; let n_out = self.u32() as usize; let w: Vec<i8> = self.b[self.p..self.p + n_in * n_out].iter().map(|&x| x as i8).collect(); self.p += n_in * n_out; let scale = self.f32s(); QMat { n_in, n_out, w, scale } }
    fn res(&mut self) -> AttnRes { AttnRes { q: self.f32s() } }
}

/// Weights + vocab.bin + merges.bin -> pack bytes.
pub fn pack(w: &Weights, vocab: &[u8], merges: &[u8]) -> Vec<u8> {
    assert!(w.tok_row.iter().enumerate().all(|(i, &r)| r as usize == i), "pack the weights before permuting them");
    let mut o = W(Vec::with_capacity(4 << 20));
    o.0.extend_from_slice(MAGIC); o.0.push(VERSION);
    o.qmat(&w.wte); o.f32s(&w.ln_f); o.res(&w.res_f);
    o.f32s(&w.rope_cos); o.f32s(&w.rope_sin); o.f32s(&w.rope_ccos); o.f32s(&w.rope_csin);
    for l in &w.layers {
        o.f32s(&l.ln1); o.f32s(&l.ln2); o.res(&l.res1); o.res(&l.res2); o.qmat(&l.gu); o.qmat(&l.down);
        match &l.mixer {
            Mixer::Gdn(g) => { o.qmat(&g.proj); o.f32s(&g.g_bias); o.f32s(&g.conv_q); o.f32s(&g.conv_k); o.f32s(&g.conv_v); o.f32s(&g.a_log); o.f32s(&g.dt_bias); o.f32s(&g.o_norm); o.qmat(&g.o); }
            Mixer::Attn(a) => { o.qmat(&a.proj); o.qmat(&a.o); o.f32s(&a.q_norm1); o.f32s(&a.k_norm1); }
        }
    }
    o.bytes(vocab); o.bytes(merges);
    o.0
}

/// Pack bytes -> (weights, vocab.bin, merges.bin). None if the header is wrong.
pub fn unpack(b: &[u8]) -> Option<(Weights, Vec<u8>, Vec<u8>)> {
    if b.len() < 5 || &b[..4] != MAGIC || b[4] != VERSION { return None; }
    let mut r = R { b, p: 5 };
    let wte = r.qmat(); let ln_f = r.f32s(); let res_f = r.res();
    let rope_cos = r.f32s(); let rope_sin = r.f32s(); let rope_ccos = r.f32s(); let rope_csin = r.f32s();
    let mut layers = Vec::with_capacity(NL);
    for l in 0..NL {
        let ln1 = r.f32s(); let ln2 = r.f32s(); let res1 = r.res(); let res2 = r.res(); let gu = r.qmat(); let down = r.qmat();
        let mixer = if is_attn(l) { let proj = r.qmat(); let o = r.qmat(); Mixer::Attn(Attn { proj, o, q_norm1: r.f32s(), k_norm1: r.f32s() }) }
            else { let proj = r.qmat(); Mixer::Gdn(Gdn { proj, g_bias: r.f32s(), conv_q: r.f32s(), conv_k: r.f32s(), conv_v: r.f32s(), a_log: r.f32s(), dt_bias: r.f32s(), o_norm: r.f32s(), o: r.qmat() }) };
        layers.push(Layer { ln1, ln2, res1, res2, mixer, gu, down });
    }
    let vocab = r.bytes(); let merges = r.bytes();
    Some((Weights { wte, tok_row: (0..V as u32).collect(), layers, ln_f, res_f, rope_cos, rope_sin, rope_ccos, rope_csin }, vocab, merges))
}

/// FNV-1a over the pack, folded to 32 bits: the model id in every NCZL header.
pub fn id(pack: &[u8]) -> u32 {
    let mut h = 0xcbf29ce484222325u64;
    for &x in pack { h = (h ^ x as u64).wrapping_mul(0x100000001b3); }
    (h ^ (h >> 32)) as u32
}
