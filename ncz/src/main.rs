//! nczcli — compress/decompress files, for cross-checking against the Python reference.
use std::{env, fs, process};

fn main() {
    let a: Vec<String> = env::args().collect();
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
