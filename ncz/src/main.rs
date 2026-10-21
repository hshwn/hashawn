//! nczcli — compress/decompress files, for cross-checking against the Python reference.
use std::{env, fs, process};

fn main() {
    let a: Vec<String> = env::args().collect();
    if a.len() >= 3 && a[1] == "img-bench" {
        // nczcli img-bench <k> <a.ppm> [b.ppm ...]: compress, decompress, verify, report bits per pixel
        let k: u8 = a[2].parse().expect("k");
        if std::env::var("IMG_STATS").is_ok() { ncz::img::enable_stats(); }
        let (mut tot_bits, mut tot_px, mut tot_s) = (0f64, 0f64, 0f64);
        for f in &a[3..] {
            let (rgb, w, h) = ncz::img::read_ppm(&fs::read(f).expect("read")).expect("not a P6 PPM");
            let t0 = std::time::Instant::now();
            ncz::img::take_stats();
            let c = ncz::img::compress_rgb(&rgb, w, h, k);
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
