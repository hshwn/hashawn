//! nczcli — compress/decompress files, for cross-checking against the Python reference.
use std::{env, fs, process};

fn main() {
    let a: Vec<String> = env::args().collect();
    if a.len() == 5 && a[1] == "add-grain" {
        // nczcli add-grain <original.ppm> <decoded.png> <out.ppm>: grain fingerprint of the original, regrown on the decoded picture
        let (rgb, w, h) = ncz::img::read_ppm(&fs::read(&a[2]).expect("read")).expect("not a P6 PPM");
        let (mut px, nch, dw, dh) = ncz::pngc::pixels(&fs::read(&a[3]).expect("read")).expect("not a PNG");
        assert!(nch == 3 && dw == w && dh == h, "decoded picture must be RGB, same size");
        let t = std::time::Instant::now();
        match ncz::grain::measure(&rgb, w, h) {
            Some(fpz) => { let tm = t.elapsed().as_secs_f64(); ncz::grain::apply_grain(&mut px, w, h, &fpz).unwrap(); println!("fingerprint {} B, measure {:.2}s", fpz.len(), tm); }
            None => println!("no measurable grain: unchanged"),
        }
        let mut v = format!("P6\n{w} {h}\n255\n").into_bytes(); v.extend_from_slice(&px); fs::write(&a[4], v).expect("write");
        return;
    }
    if a.len() >= 4 && a[1] == "wonder" {
        // nczcli wonder <q.pred> <a.lat>...: learned-mode streams (NCZW) from latent files, round trip checked,
        // bits per pixel and time; WONDER_OUT=<dir> writes the streams
        let p = ncz::wonder::Predictor::load(&fs::read(&a[2]).expect("pred")).expect("bad predictor");
        for f in &a[3..] {
            let b = fs::read(f).expect("read"); let g = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]) as usize;
            let (c, h, w) = (g(0), g(2), g(4));
            let lat: Vec<i16> = b[6..].chunks_exact(2).map(|x| i16::from_le_bytes([x[0], x[1]])).collect();
            // WONDER_TILE=k: repeat the latents k x k times (timing big pictures)
            let k: usize = std::env::var("WONDER_TILE").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
            let (lat, h, w) = if k == 1 { (lat, h, w) } else {
                let (h2, w2) = (h * k, w * k); let mut v = vec![0i16; c * h2 * w2];
                for q in 0..c { for y in 0..h2 { for x in 0..w2 { v[(q * h2 + y) * w2 + x] = lat[(q * h + y % h) * w + x % w]; } } } (v, h2, w2) };
            let t = std::time::Instant::now(); let z = ncz::wonder::compress(&lat, c, h, w, w * 16, h * 16, 4, &p).expect("compress"); let te = t.elapsed().as_secs_f64();
            let t = std::time::Instant::now(); let back = ncz::wonder::decompress(&z, &p).expect("decode"); let td = t.elapsed().as_secs_f64();
            if let Ok(d) = std::env::var("WONDER_OUT") { fs::write(format!("{}/{}.nczw", d, f.rsplit('/').next().unwrap().trim_end_matches(".lat")), &z).unwrap(); }
            println!("{} {} B  {:.4} bpp  enc {:.2}s dec {:.2}s  {}", f.rsplit('/').next().unwrap(), z.len(), 8.0 * z.len() as f64 / (h * 16 * w * 16) as f64, te, td, if back.0 == lat { "exact" } else { "MISMATCH" });
        }
        return;
    }
    if a.len() >= 3 && a[1] == "lat" {
        // nczcli lat <a.lat>...: latent files (u16 C,H,W + i16 channel-major) -> NCZY; round trip checked;
        // bits per image pixel (latents at 1/16 per side) and where the bits go
        // LAT_PRIME="a.lat b.lat ...": examples the model learns from first (encoder and decoder alike)
        let read = |p: &str| { let b = fs::read(p).expect("read"); let g = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]) as usize;
            let lat: Vec<i16> = b[6..].chunks_exact(2).map(|x| i16::from_le_bytes([x[0], x[1]])).collect(); (lat, g(0), g(2), g(4)) };
        let prime: Vec<String> = std::env::var("LAT_PRIME").map(|v| v.split_whitespace().map(String::from).collect()).unwrap_or_default();
        let prior = if prime.is_empty() { None } else {
            let mut pr: Option<ncz::lat::Prior> = None;
            for p in &prime { let (l, c, h, w) = read(p); pr.get_or_insert_with(|| ncz::lat::Prior::new(c, h, w)).learn(&l, c, h, w); }
            pr };
        for p in &a[2..] {
            let b = fs::read(p).expect("read"); let g = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]) as usize;
            let (c, h, w) = (g(0), g(2), g(4));
            let lat: Vec<i16> = b[6..].chunks_exact(2).map(|x| i16::from_le_bytes([x[0], x[1]])).collect();
            let t = std::time::Instant::now(); let (z, bits) = ncz::lat::compress_stats(&lat, c, h, w, prior.as_ref()); let te = t.elapsed().as_secs_f64();
            let back = ncz::lat::decompress(&z, prior.as_ref()).expect("decode"); let ok = back.0 == lat;
            if let Ok(dir) = std::env::var("LAT_AUX") {   // <dir>/<name minus .lat>.par: i16 mean x16, u16 scale x64
                let pb = fs::read(format!("{}/{}.par", dir, p.rsplit('/').next().unwrap().trim_end_matches(".lat"))).expect("par"); let k = c * h * w;
                let mu: Vec<i16> = pb[6..6 + 2 * k].chunks_exact(2).map(|x| i16::from_le_bytes([x[0], x[1]])).collect();
                let sc: Vec<u16> = pb[6 + 2 * k..].chunks_exact(2).map(|x| u16::from_le_bytes([x[0], x[1]])).collect();
                let (n, bb) = ncz::lat::compress_aux(&lat, c, h, w, mu, sc); let px = (h * 16 * w * 16) as f64;
                println!("  with outside mean/scale: {} B {:.4} bpp (zero {:.4} sign {:.4} mag {:.4} esc {:.4})", n, 8.0 * n as f64 / px, bb[0] / px, bb[1] / px, bb[2] / px, bb[3] / px);
            }
            if let Ok(dir) = std::env::var("LAT_CHAN") { let cb = ncz::lat::channel_bits(&lat, c, h, w);
                fs::write(format!("{}/{}.chan", dir, p.rsplit('/').next().unwrap()), cb.iter().map(|v| format!("{v:.1}")).collect::<Vec<_>>().join(" ")).unwrap(); }
            let px = (h * 16 * w * 16) as f64;
            println!("{} {} B  {:.4} bpp  (zero {:.4} sign {:.4} mag {:.4} esc {:.4})  {:.2}s  {}", p.rsplit('/').next().unwrap(), z.len(), 8.0 * z.len() as f64 / px,
                bits[0] / px, bits[1] / px, bits[2] / px, bits[3] / px, te, if ok { "exact" } else { "MISMATCH" });
        }
        return;
    }
    if a.len() == 6 && a[1] == "png-mode" {
        // nczcli png-mode <0 exact | 1 near +-1 | 2 grain> <in.png> <out.ncz> <out.png>: what the browser tool writes
        // (fast mode); prints sizes, the mode actually used, and the max per-sample error of the rebuilt PNG
        let mode = match a[2].as_str() { "0" => ncz::pngc::Mode::Exact, "1" => ncz::pngc::Mode::Near(1), _ => ncz::pngc::Mode::Grain };
        let b = fs::read(&a[3]).expect("read");
        let c = ncz::pngc::compress_mode(&b, 0, mode).expect("not handled or not smaller");
        let back = ncz::pngc::decompress(&c).expect("decode");
        fs::write(&a[4], &c).unwrap(); fs::write(&a[5], &back).unwrap();
        let (p0, n0, _, _) = ncz::pngc::pixels(&b).unwrap(); let (p1, n1, _, _) = ncz::pngc::pixels(&back).unwrap();
        let maxerr = if n0 == n1 { p0.iter().zip(&p1).map(|(x, y)| (*x as i32 - *y as i32).abs()).max().unwrap_or(0) } else { -1 };
        println!("{} -> {} bytes  mode {:?}  max error {}", b.len(), c.len(), ncz::pngc::mode_of(&c).unwrap(), maxerr);
        return;
    }
    if a.len() >= 3 && a[1] == "png-roundtrip" {
        // nczcli png-roundtrip <a.png>...: PNG -> .ncz -> PNG, pixels compared exactly; sizes vs the original
        let flags = std::env::var("IMG_FLAGS").ok().and_then(|v| v.parse().ok()).unwrap_or(0u8);
        let (mut tin, mut tout) = (0usize, 0usize);
        for f in &a[2..] {
            let b = fs::read(f).expect("read");
            let t0 = std::time::Instant::now();
            match ncz::pngc::compress(&b, flags) {
                None => println!("{:40} not worth it or not handled (16-bit, animated): generic byte-exact path", f),
                Some(c) => {
                    let t1 = t0.elapsed().as_secs_f64();
                    let back = ncz::pngc::decompress(&c).expect("decode");
                    let (p0, n0, w0, h0) = ncz::pngc::pixels(&b).unwrap(); let (p1, n1, w1, h1) = ncz::pngc::pixels(&back).expect("rebuilt PNG unreadable");
                    let ok = p0 == p1 && n0 == n1 && w0 == w1 && h0 == h1;
                    let mut hh = 0xcbf29ce484222325u64; for &x in &c { hh = (hh ^ x as u64).wrapping_mul(0x100000001b3); }
                    println!("{:40} {:>9} -> {:>9} bytes ({:5.1}%)  rebuilt png {:>9}  {:5.2} MB/s  {:016x}  {}", std::path::Path::new(f).file_name().unwrap().to_string_lossy(), b.len(), c.len(), 100.0 * c.len() as f64 / b.len() as f64, back.len(), (w0 * h0 * n0) as f64 / t1 / 1e6, hh, if ok { "PIXELS EXACT" } else { "MISMATCH" });
                    if !ok { process::exit(1); }
                    tin += b.len(); tout += c.len();
                }
            }
        }
        if tin > 0 { println!("TOTAL {tin} -> {tout} bytes ({:.1}% of the PNGs)", 100.0 * tout as f64 / tin as f64); }
        return;
    }
    if a.len() >= 3 && a[1] == "grain" {
        // nczcli grain <k> <a.ppm>...: grain mode (LOOKS THE SAME, not exact). Sizes vs exact and +-k on the original;
        // decodes twice to check the regenerated grain is identical. GRAIN_OUT=<dir> writes <name>.clean.ppm / .recon.ppm.
        let k: u8 = a[2].parse().expect("k");
        let out = std::env::var("GRAIN_OUT").ok();
        let ppm = |p: &[u8], w: usize, h: usize| { let mut v = format!("P6\n{w} {h}\n255\n").into_bytes(); v.extend_from_slice(p); v };
        let (mut px, mut b_ex, mut b_k, mut b_g, mut b_fp) = (0f64, 0f64, 0f64, 0f64, 0f64);
        for f in &a[3..] {
            let (rgb, w, h) = ncz::img::read_ppm(&fs::read(f).expect("read")).expect("not a P6 PPM");
            let n = (w * h) as f64;
            let ex = ncz::img::compress_rgb(&rgb, w, h, 0).len() as f64; let pk = ncz::img::compress_rgb(&rgb, w, h, k).len() as f64;
            let t0 = std::time::Instant::now();
            let e = ncz::grain::compress(&rgb, w, h, k).expect("picture under 64x64");
            let te = t0.elapsed().as_secs_f64();
            let t1 = std::time::Instant::now();
            let (r1, _, _) = ncz::grain::decompress(&e.stream).expect("decode");
            let td = t1.elapsed().as_secs_f64();
            let (r2, _, _) = ncz::grain::decompress(&e.stream).expect("decode");
            let fpb = ncz::grain::fingerprint_bytes(&e.stream) as f64;
            let mut hh = 0xcbf29ce484222325u64; for &x in &r1 { hh = (hh ^ x as u64).wrapping_mul(0x100000001b3); }
            let name = std::path::Path::new(f).file_stem().unwrap().to_string_lossy().to_string();
            println!("{:12} exact {:6.3}  +-{k} {:6.3}  grain mode {:6.3} bpp ({:4.0} B fingerprint, env {:.2})  {:+5.1}% vs exact  enc {:4.1}s dec {:4.1}s  {:016x} {}",
                name, 8.0 * ex / n, 8.0 * pk / n, 8.0 * e.stream.len() as f64 / n, fpb, e.fp.env, 100.0 * (e.stream.len() as f64 / ex - 1.0), te, td, hh, if r1 == r2 { "repeatable" } else { "NOT REPEATABLE" });
            if r1 != r2 { process::exit(1); }
            if let Some(d) = &out {
                let n0 = 18 + fpb as usize; let (clean, _, _, _) = ncz::img::decompress_rgb(&e.stream[n0..]).unwrap();
                fs::write(format!("{d}/{name}.clean.ppm"), ppm(&clean, w, h)).unwrap(); fs::write(format!("{d}/{name}.sep.ppm"), ppm(&e.clean, w, h)).unwrap(); fs::write(format!("{d}/{name}.recon.ppm"), ppm(&r1, w, h)).unwrap();
            }
            px += n; b_ex += ex; b_k += pk; b_g += e.stream.len() as f64; b_fp += fpb;
        }
        println!("TOTAL exact {:.3}  +-{k} {:.3}  grain mode {:.3} bpp (fingerprints {:.3} bpp)  {:+.1}% vs exact, {:+.1}% vs +-{k}", 8.0 * b_ex / px, 8.0 * b_k / px, 8.0 * b_g / px, 8.0 * b_fp / px, 100.0 * (b_g / b_ex - 1.0), 100.0 * (b_g / b_k - 1.0));
        return;
    }
    if (a.len() == 5 || a.len() == 7 || a.len() == 10) && a[1] == "img-rt" {
        // nczcli img-rt <k> <in.ppm> <out.ppm> [<kc> <flags>]: pixel mode round trip (red/blue within +-kc if given),
        // writes the decoded picture, prints the stream size in bytes
        let k: u8 = a[2].parse().expect("k");
        let (kc, flags) = if a.len() == 7 { (a[5].parse().expect("kc"), a[6].parse().expect("flags")) } else { (k, 0u8) };
        let (rgb, w, h) = ncz::img::read_ppm(&fs::read(&a[3]).expect("read")).expect("not a P6 PPM");
        // optional adaptive allowance: <kmax> <kcmax> <t>
        let ad: [u8; 3] = if a.len() == 10 { [a[7].parse().unwrap(), a[8].parse().unwrap(), a[9].parse().unwrap()] } else { [k, kc, 0] };
        let c = ncz::img::compress_rgb_allow(&rgb, w, h, k, kc, ad, flags);
        let (back, _, _, _) = ncz::img::decompress_rgb(&c).expect("decode");
        let mut v = format!("P6\n{w} {h}\n255\n").into_bytes(); v.extend_from_slice(&back); fs::write(&a[4], v).expect("write");
        println!("{}", c.len());
        return;
    }
    if a.len() == 7 && a[1] == "grain-rt" {
        // nczcli grain-rt <k> <kc> <flags> <in.ppm> <out.ppm>: grain mode round trip, writes the output, prints bytes
        let (k, kc, flags): (u8, u8, u8) = (a[2].parse().unwrap(), a[3].parse().unwrap(), a[4].parse().unwrap());
        let (rgb, w, h) = ncz::img::read_ppm(&fs::read(&a[5]).expect("read")).expect("not a P6 PPM");
        let e = ncz::grain::compress_with(&rgb, w, h, k, kc, flags).expect("picture under 64x64");
        let (back, _, _) = ncz::grain::decompress(&e.stream).expect("decode");
        let mut v = format!("P6\n{w} {h}\n255\n").into_bytes(); v.extend_from_slice(&back); fs::write(&a[6], v).expect("write");
        println!("{}", e.stream.len());
        return;
    }
    if a.len() >= 3 && a[1] == "img-hash" {
        // nczcli img-hash <flags> <a.ppm>...: FNV hash of each compressed stream (regression check)
        let flags: u8 = a[2].parse().unwrap();
        for f in &a[3..] { let (rgb, w, h) = ncz::img::read_ppm(&fs::read(f).unwrap()).unwrap(); let c = ncz::img::compress_rgb_flags(&rgb, w, h, 0, flags);
            let mut hh = 0xcbf29ce484222325u64; for &b in &c { hh = (hh ^ b as u64).wrapping_mul(0x100000001b3); } println!("{hh:016x} {} {}", c.len(), f); }
        return;
    }
    if a.len() >= 3 && a[1] == "img-bench" {
        // nczcli img-bench <k> <a.ppm> [b.ppm ...]: compress, decompress, verify, report bits per pixel
        // IMG_FLAGS: 1 green plane first, 2 learned colour (implies 1), 4 learned green; 7 = everything
        let k: u8 = a[2].parse().expect("k");
        if std::env::var("IMG_STATS").is_ok() { ncz::img::enable_stats(); }
        let (mut tot_bits, mut tot_px, mut tot_s) = (0f64, 0f64, 0f64);
        for f in &a[3..] {
            let (rgb, w, h) = ncz::img::read_ppm(&fs::read(f).expect("read")).expect("not a P6 PPM");
            let t0 = std::time::Instant::now();
            ncz::img::take_stats();
            let flags = std::env::var("IMG_FLAGS").ok().and_then(|v| v.parse().ok()).unwrap_or(0u8);
            let c = ncz::img::compress_rgb_flags(&rgb, w, h, k, flags);
            let t1 = t0.elapsed().as_secs_f64();
            if std::env::var("IMG_STATS").is_ok() { let s = ncz::img::take_stats(); let n = (w * h) as f64; for (ci, cn) in ["R", "G", "B"].iter().enumerate() { eprintln!("   {cn}: zero {:.3} size {:.3} mant {:.3} sign {:.3}  = {:.3} bpp", s[ci*4]/n, s[ci*4+1]/n, s[ci*4+2]/n, s[ci*4+3]/n, (s[ci*4]+s[ci*4+1]+s[ci*4+2]+s[ci*4+3])/n); } }
            let (back, w2, h2, _) = ncz::img::decompress_rgb(&c).expect("decode");
            let t2 = t0.elapsed().as_secs_f64() - t1;
            let maxerr = rgb.iter().zip(&back).map(|(x, y)| (*x as i32 - *y as i32).abs()).max().unwrap_or(0);
            let ok = w2 == w && h2 == h && maxerr <= k as i32;
            let bpp = 8.0 * c.len() as f64 / (w * h) as f64;
            println!("{:40} {:7.3} bpp  enc {:5.2} MB/s  dec {:5.2} MB/s  maxerr {}  {}", std::path::Path::new(f).file_name().unwrap().to_string_lossy(), bpp, (w * h * 3) as f64 / t1 / 1e6, (w * h * 3) as f64 / t2 / 1e6, maxerr, if ok { "OK" } else { "FAIL" });
            if !ok { process::exit(1); }
            tot_bits += 8.0 * c.len() as f64; tot_px += (w * h) as f64; tot_s += t1;
        }
        println!("TOTAL {:.3} bpp  ({:.2} MB/s encode)", tot_bits / tot_px, tot_px * 3.0 / tot_s / 1e6);
        return;
    }
    if a.len() == 5 && a[1] == "compress-side" {
        // experimental: an external per-bit prediction file (i16 LE per bit) as an extra mixer input
        let data = fs::read(&a[2]).expect("read input");
        let raw = fs::read(&a[3]).expect("read side");
        let side: Vec<i16> = raw.chunks_exact(2).map(|c| i16::from_le_bytes([c[0], c[1]])).collect();
        assert!(side.len() >= data.len() * 8, "side file too short");
        let mut e = ncz::Encoder::new(); e.push_side(&data, Some(&side));
        fs::write(&a[4], e.finish()).expect("write output");
        return;
    }
    if a.len() != 4 || !(a[1] == "compress" || a[1] == "decompress") {
        eprintln!("usage: nczcli compress|decompress <in> <out>   |   nczcli compress-side <in> <side> <out>");
        process::exit(2);
    }
    let data = fs::read(&a[2]).expect("read input");
    let out = if a[1] == "compress" { ncz::compress(&data) } else { ncz::decompress(&data).unwrap_or_else(|| { eprintln!("not an ncz stream"); process::exit(1) }) };
    fs::write(&a[3], out).expect("write output");
}
