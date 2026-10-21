//! nczcli — compress/decompress files, for cross-checking against the Python reference.
use std::{env, fs, process};

fn main() {
    let a: Vec<String> = env::args().collect();
    if a.len() != 4 || !(a[1] == "compress" || a[1] == "decompress") {
        eprintln!("usage: nczcli compress|decompress <in> <out>");
        process::exit(2);
    }
    let data = fs::read(&a[2]).expect("read input");
    let out = if a[1] == "compress" { ncz::compress(&data) } else { ncz::decompress(&data).unwrap_or_else(|| { eprintln!("not an NCZ2 stream"); process::exit(1) }) };
    fs::write(&a[3], out).expect("write output");
}
