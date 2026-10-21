//! PNG in, pixels coded by the pixel model, PNG out: every pixel exact, every non-pixel chunk kept.
//!
//! A PNG is a sequence of chunks. Only the image data (IDAT) is decoded and re-coded; everything else
//! (IHDR, PLTE, tRNS, colour profile, gamma, text, EXIF, ...) is kept byte for byte, in its original order, as a
//! "skeleton". Decompressing re-encodes the pixels into new IDAT chunks (same colour type and bit depth,
//! not interlaced) and splices them into the skeleton. So the pixels and metadata come back exactly; the file's
//! bytes differ only inside IDAT, because PNG's own zlib stream cannot be replayed byte for byte.
//!
//! Handled: grey, grey+alpha, RGB, RGBA at 8 bits; palette at 1/2/4/8 bits; grey at 1/2/4 bits. Not handled
//! (returns None, caller falls back to byte-exact generic compression): 16-bit, animated PNG, broken files.
//!
//! Container: "NCZP" | version | kind | bit depth | coding | skeleton length u32 | skeleton (NCZ3) | payload.
//! Coding 0: the pixel model (NCZI) over RGB(A) samples. Coding 1: the image's own samples (palette indices or
//! low-bit grey, one byte each, unfiltered) through ncz's byte model; few-colour images are far cheaper this way.
use crate::img;

pub const MAGIC: &[u8; 4] = b"NCZP";
pub const VERSION: u8 = 1;
const SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
const C_PIXEL: u8 = 0; const C_SAMPLES: u8 = 1;
const K_GRAY: u8 = 0; const K_GRAYA: u8 = 1; const K_RGB: u8 = 2; const K_RGBA: u8 = 3; const K_PAL: u8 = 4;

pub fn is_png(b: &[u8]) -> bool { b.len() > 8 && b[..8] == SIG }

fn crc32(data: &[u8]) -> u32 {
    static T: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let t = T.get_or_init(|| { let mut t = [0u32; 256]; for n in 0..256 { let mut c = n as u32; for _ in 0..8 { c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 }; } t[n] = c; } t });
    let mut c = 0xFFFF_FFFFu32; for &b in data { c = t[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8); } c ^ 0xFFFF_FFFF
}

/// (type, data) for every chunk; None if the structure is broken.
fn chunks(b: &[u8]) -> Option<Vec<([u8; 4], &[u8])>> {
    if !is_png(b) { return None; }
    let mut i = 8; let mut v = Vec::new();
    while i + 12 <= b.len() {
        let len = u32::from_be_bytes(b[i..i + 4].try_into().ok()?) as usize;
        let ty: [u8; 4] = b[i + 4..i + 8].try_into().ok()?;
        if i + 12 + len > b.len() { return None; }
        v.push((ty, &b[i + 8..i + 8 + len]));
        i += 12 + len;
        if &ty == b"IEND" { break; }
    }
    if v.first().map(|c| &c.0) != Some(b"IHDR") || v.last().map(|c| &c.0) != Some(b"IEND") { return None; }
    Some(v)
}

fn put_chunk(out: &mut Vec<u8>, ty: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let start = out.len(); out.extend_from_slice(ty); out.extend_from_slice(data);
    let c = crc32(&out[start..]); out.extend_from_slice(&c.to_be_bytes());
}

/// Unpack one row of `depth`-bit samples (MSB first) into bytes.
fn unpack(row: &[u8], depth: u8, n: usize, out: &mut Vec<u8>) {
    if depth == 8 { out.extend_from_slice(&row[..n]); return; }
    let per = 8 / depth as usize; let mask = (1u8 << depth) - 1;
    for i in 0..n { let b = row[i / per]; let shift = 8 - depth as usize * (i % per + 1); out.push((b >> shift) & mask); }
}
fn pack(vals: &[u8], depth: u8, out: &mut Vec<u8>) {
    if depth == 8 { out.extend_from_slice(vals); return; }
    let per = 8 / depth as usize;
    for chunk in vals.chunks(per) { let mut b = 0u8; for (i, &v) in chunk.iter().enumerate() { b |= v << (8 - depth as usize * (i + 1)); } out.push(b); }
}

/// Decode to interleaved samples: (pixels, nch 3 or 4, w, h, kind, depth, palette, trns). None if unsupported.
#[allow(clippy::type_complexity)]
fn decode(b: &[u8]) -> Option<(Vec<u8>, usize, usize, usize, u8, u8, Vec<u8>)> {
    let mut dec = png::Decoder::new(std::io::Cursor::new(b));
    dec.set_transformations(png::Transformations::IDENTITY);
    let mut r = dec.read_info().ok()?;
    let info = r.info().clone();
    if info.animation_control.is_some() { return None; }
    let (w, h) = (info.width as usize, info.height as usize);
    let depth = info.bit_depth as u8;
    if depth == 16 { return None; }
    let mut buf = vec![0u8; r.output_buffer_size()?];
    let out = r.next_frame(&mut buf).ok()?;
    let line = out.line_size;
    let (kind, nch_in) = match info.color_type {
        png::ColorType::Grayscale => (K_GRAY, 1), png::ColorType::GrayscaleAlpha => (K_GRAYA, 2),
        png::ColorType::Rgb => (K_RGB, 3), png::ColorType::Rgba => (K_RGBA, 4), png::ColorType::Indexed => (K_PAL, 1),
    };
    if (kind == K_GRAYA || kind == K_RGB || kind == K_RGBA) && depth != 8 { return None; }
    let mut raw = Vec::with_capacity(w * h * nch_in);
    for y in 0..h { unpack(&buf[y * line..(y + 1) * line], depth, w * nch_in, &mut raw); }
    let (pal, trns) = (info.palette.as_deref().unwrap_or(&[]).to_vec(), info.trns.as_deref().unwrap_or(&[]).to_vec());
    let alpha = match kind { K_GRAYA | K_RGBA => true, K_PAL => !trns.is_empty(), _ => false };
    let nch = if alpha { 4 } else { 3 };
    let mut px = Vec::with_capacity(w * h * nch);
    let scale = if depth < 8 { 255 / ((1u32 << depth) - 1) } else { 1 } as u8;
    for i in 0..w * h {
        match kind {
            K_GRAY => { let g = raw[i] * scale; px.extend_from_slice(&[g, g, g]); }
            K_GRAYA => { let (g, a) = (raw[2 * i], raw[2 * i + 1]); px.extend_from_slice(&[g, g, g, a]); }
            K_RGB => px.extend_from_slice(&raw[3 * i..3 * i + 3]),
            K_RGBA => px.extend_from_slice(&raw[4 * i..4 * i + 4]),
            _ => {
                let k = raw[i] as usize; if 3 * k + 2 >= pal.len() { return None; }
                px.extend_from_slice(&pal[3 * k..3 * k + 3]);
                if alpha { px.push(*trns.get(k).unwrap_or(&255)); }
            }
        }
    }
    Some((px, nch, w, h, kind, depth, raw))
}

/// Re-encode pixels in the original colour type / bit depth; returns the IHDR data and the IDAT chunks' data.
fn encode(px: &[u8], nch: usize, w: usize, h: usize, kind: u8, depth: u8, pal: &[u8], trns: &[u8]) -> Option<(Vec<u8>, Vec<Vec<u8>>)> {
    let ct = match kind { K_GRAY => png::ColorType::Grayscale, K_GRAYA => png::ColorType::GrayscaleAlpha, K_RGB => png::ColorType::Rgb, K_RGBA => png::ColorType::Rgba, _ => png::ColorType::Indexed };
    let bd = match depth { 1 => png::BitDepth::One, 2 => png::BitDepth::Two, 4 => png::BitDepth::Four, _ => png::BitDepth::Eight };
    // pixels -> raw samples
    let scale = if depth < 8 { 255 / ((1u32 << depth) - 1) } else { 1 } as u8;
    let mut index = std::collections::HashMap::new();
    if kind == K_PAL { for k in (0..pal.len() / 3).rev() { let a = if nch == 4 { *trns.get(k).unwrap_or(&255) } else { 255 }; index.insert([pal[3 * k], pal[3 * k + 1], pal[3 * k + 2], a], k as u8); } }
    let mut data = Vec::new(); let mut row = Vec::with_capacity(w * 4);
    for y in 0..h {
        row.clear();
        for x in 0..w {
            let p = &px[(y * w + x) * nch..(y * w + x + 1) * nch];
            match kind {
                K_GRAY => row.push(p[1] / scale),
                K_GRAYA => row.extend_from_slice(&[p[1], p[3]]),
                K_RGB => row.extend_from_slice(&p[..3]),
                K_RGBA => row.extend_from_slice(&p[..4]),
                _ => row.push(*index.get(&[p[0], p[1], p[2], if nch == 4 { p[3] } else { 255 }])?),
            }
        }
        pack(&row, if kind == K_PAL || kind == K_GRAY { depth } else { 8 }, &mut data);
    }
    let mut out = Vec::new();
    {
        let mut e = png::Encoder::new(&mut out, w as u32, h as u32);
        e.set_color(ct); e.set_depth(bd);
        if kind == K_PAL { e.set_palette(pal.to_vec()); }
        let mut wr = e.write_header().ok()?; wr.write_image_data(&data).ok()?; wr.finish().ok()?;
    }
    let cs = chunks(&out)?;
    let ihdr = cs.iter().find(|c| &c.0 == b"IHDR")?.1.to_vec();
    let idat = cs.iter().filter(|c| &c.0 == b"IDAT").map(|c| c.1.to_vec()).collect();
    Some((ihdr, idat))
}

/// Streaming PNG compressor: the decode, the skeleton and few-colour images happen in `new`; photo-like images are
/// coded row by row in `step`. `finish` returns None when the result would not beat the original file.
pub struct PngEncoder { head: Vec<u8>, img: Option<img::ImgEncoder>, done: Option<Vec<u8>>, orig_len: usize }
impl PngEncoder {
    pub fn new(b: &[u8], flags: u8) -> Option<PngEncoder> {
        let cs = chunks(b)?;
        let (px, nch, w, h, kind, depth, raw) = decode(b)?;
        // skeleton: every chunk but IDAT, IDAT marked once where it first appears
        let mut sk = Vec::new(); let mut idat_seen = false;
        for (ty, data) in &cs {
            if ty == b"IDAT" { if idat_seen { continue; } idat_seen = true; sk.extend_from_slice(ty); sk.extend_from_slice(&0u32.to_le_bytes()); continue; }
            sk.extend_from_slice(ty); sk.extend_from_slice(&(data.len() as u32).to_le_bytes()); sk.extend_from_slice(data);
        }
        let skz = crate::compress(&sk);
        let few = kind == K_PAL || depth < 8;
        let mut head = Vec::with_capacity(16 + skz.len());
        head.extend_from_slice(MAGIC); head.push(VERSION); head.push(kind); head.push(depth); head.push(if few { C_SAMPLES } else { C_PIXEL });
        head.extend_from_slice(&(skz.len() as u32).to_le_bytes()); head.extend_from_slice(&skz);
        if few {
            // few-colour images (palette, 1-4 bit grey): their own sample bytes through the byte model
            let mut out = head; out.extend_from_slice(&(w as u32).to_le_bytes()); out.extend_from_slice(&(h as u32).to_le_bytes()); out.extend_from_slice(&crate::compress(&raw));
            return Some(PngEncoder { head: Vec::new(), img: None, done: Some(out), orig_len: b.len() });
        }
        let img = img::ImgEncoder::new(px, w, h, 0, if nch == 4 { flags | img::F_ALPHA } else { flags & !img::F_ALPHA });
        Some(PngEncoder { head, img: Some(img), done: None, orig_len: b.len() })
    }
    /// Work units (rows) in total and done so far.
    pub fn units(&self) -> usize { self.img.as_ref().map_or(1, |e| e.units()) }
    pub fn done(&self) -> usize { self.img.as_ref().map_or(1, |e| e.done()) }
    pub fn step(&mut self, n: usize) -> usize { self.img.as_mut().map_or(0, |e| e.step(n)) }
    pub fn finish(self) -> Option<Vec<u8>> {
        let out = match (self.done, self.img) { (Some(o), _) => o, (None, Some(e)) => { let mut o = self.head; o.extend_from_slice(&e.finish()); o } _ => return None };
        if out.len() >= self.orig_len { None } else { Some(out) }     // not worth it: use the byte-exact path
    }
}

/// Compress a PNG file. None = not handled here (16-bit, animated, broken) or not smaller: use byte-exact compression.
/// `flags` are the pixel model's (0 = fast).
pub fn compress(b: &[u8], flags: u8) -> Option<Vec<u8>> { PngEncoder::new(b, flags)?.finish() }

/// Streaming PNG decompressor (the mirror of PngEncoder).
pub struct PngDecoder { kind: u8, depth: u8, sk: Vec<u8>, img: Option<img::ImgDecoder>, done: Option<Vec<u8>> }
impl PngDecoder {
    pub fn new(blob: &[u8]) -> Option<PngDecoder> {
        let (kind, depth, coding, sk, pix) = parse(blob)?;
        if coding == C_SAMPLES {
            if pix.len() < 8 { return None; }
            let (w, h) = (u32::from_le_bytes(pix[0..4].try_into().ok()?) as usize, u32::from_le_bytes(pix[4..8].try_into().ok()?) as usize);
            let raw = crate::decompress(&pix[8..])?;
            let out = rebuild_samples(&raw, w, h, kind, depth, &sk)?;
            return Some(PngDecoder { kind, depth, sk, img: None, done: Some(out) });
        }
        if coding != C_PIXEL { return None; }
        Some(PngDecoder { kind, depth, sk, img: Some(img::ImgDecoder::new(pix)?), done: None })
    }
    pub fn units(&self) -> usize { self.img.as_ref().map_or(1, |d| d.units()) }
    pub fn done(&self) -> usize { self.img.as_ref().map_or(1, |d| d.done()) }
    pub fn step(&mut self, n: usize) -> usize { self.img.as_mut().map_or(0, |d| d.step(n)) }
    pub fn finish(self) -> Option<Vec<u8>> {
        if let Some(o) = self.done { return Some(o); }
        let d = self.img?; let (w, h) = (d.w, d.h); let nch = if d.flags & img::F_ALPHA != 0 { 4 } else { 3 };
        let px = d.finish();
        rebuild(&px, nch, w, h, self.kind, self.depth, &self.sk)
    }
}

/// The parts of an NCZP stream: (kind, depth, coding, skeleton bytes, payload).
pub fn parse(blob: &[u8]) -> Option<(u8, u8, u8, Vec<u8>, &[u8])> {
    if blob.len() < 12 || &blob[..4] != MAGIC || blob[4] != VERSION { return None; }
    let (kind, depth, coding) = (blob[5], blob[6], blob[7]);
    let n = u32::from_le_bytes(blob[8..12].try_into().ok()?) as usize;
    if 12 + n > blob.len() { return None; }
    let sk = crate::decompress(&blob[12..12 + n])?;
    Some((kind, depth, coding, sk, &blob[12 + n..]))
}

/// Rebuild the PNG from decoded pixels and the skeleton.
pub fn rebuild(px: &[u8], nch: usize, w: usize, h: usize, kind: u8, depth: u8, sk: &[u8]) -> Option<Vec<u8>> {
    let items = skeleton_items(sk)?;
    let pal = items.iter().find(|c| &c.0 == b"PLTE").map(|c| c.1).unwrap_or(&[]);
    let trns = items.iter().find(|c| &c.0 == b"tRNS").map(|c| c.1).unwrap_or(&[]);
    let (ihdr, idats) = encode(px, nch, w, h, kind, depth, pal, trns)?;
    rebuild_with(ihdr, idats, &items)
}
/// Rebuild from the image's own samples (coding 1): pack them back without going through RGB.
pub fn rebuild_samples(raw: &[u8], w: usize, h: usize, kind: u8, depth: u8, sk: &[u8]) -> Option<Vec<u8>> {
    let items = skeleton_items(sk)?;
    let pal = items.iter().find(|c| &c.0 == b"PLTE").map(|c| c.1).unwrap_or(&[]);
    let nch_in = match kind { K_GRAYA => 2, K_RGB => 3, K_RGBA => 4, _ => 1 };
    if raw.len() != w * h * nch_in { return None; }
    let ct = match kind { K_GRAY => png::ColorType::Grayscale, K_GRAYA => png::ColorType::GrayscaleAlpha, K_RGB => png::ColorType::Rgb, K_RGBA => png::ColorType::Rgba, _ => png::ColorType::Indexed };
    let bd = match depth { 1 => png::BitDepth::One, 2 => png::BitDepth::Two, 4 => png::BitDepth::Four, _ => png::BitDepth::Eight };
    let mut data = Vec::new();
    for y in 0..h { pack(&raw[y * w * nch_in..(y + 1) * w * nch_in], depth, &mut data); }
    let mut out = Vec::new();
    {
        let mut e = png::Encoder::new(&mut out, w as u32, h as u32);
        e.set_color(ct); e.set_depth(bd);
        if kind == K_PAL { e.set_palette(pal.to_vec()); }
        let mut wr = e.write_header().ok()?; wr.write_image_data(&data).ok()?; wr.finish().ok()?;
    }
    let cs = chunks(&out)?;
    let ihdr = cs.iter().find(|c| &c.0 == b"IHDR")?.1.to_vec();
    let idat = cs.iter().filter(|c| &c.0 == b"IDAT").map(|c| c.1.to_vec()).collect();
    rebuild_with(ihdr, idat, &items)
}
fn skeleton_items(sk: &[u8]) -> Option<Vec<([u8; 4], &[u8])>> {
    let mut items = Vec::new(); let mut i = 0;
    while i + 8 <= sk.len() {
        let ty: [u8; 4] = sk[i..i + 4].try_into().ok()?; let n = u32::from_le_bytes(sk[i + 4..i + 8].try_into().ok()?) as usize;
        if i + 8 + n > sk.len() { return None; }
        items.push((ty, &sk[i + 8..i + 8 + n])); i += 8 + n;
    }
    Some(items)
}
fn rebuild_with(ihdr: Vec<u8>, idats: Vec<Vec<u8>>, items: &[([u8; 4], &[u8])]) -> Option<Vec<u8>> {
    let mut out = SIG.to_vec();
    for (ty, data) in items {
        match ty {
            b"IHDR" => put_chunk(&mut out, b"IHDR", &ihdr),          // ours: same fields, not interlaced
            b"IDAT" => for d in &idats { put_chunk(&mut out, b"IDAT", d); },
            _ => put_chunk(&mut out, ty, data),
        }
    }
    Some(out)
}

/// Decompress an NCZP stream back to a PNG file.
pub fn decompress(blob: &[u8]) -> Option<Vec<u8>> { PngDecoder::new(blob)?.finish() }

/// Pixels of any PNG this module handles, as (samples, nch, w, h): for checking round trips.
pub fn pixels(b: &[u8]) -> Option<(Vec<u8>, usize, usize, usize)> { decode(b).map(|(p, n, w, h, _, _, _)| (p, n, w, h)) }
