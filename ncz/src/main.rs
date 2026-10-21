//! nczcli — compress/decompress files, for cross-checking against the Python reference.
use std::{env, fs, process};

fn main() {
    let a: Vec<String> = env::args().collect();
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
