//! slmcli check            — compare against PyTorch reference logits (model/ref_*.bin)
//! slmcli bpb <ids.bin> <nbytes>  — bits per byte over a token file, 1024-token windows, stride 512 (same as bench/lm_bpb.py)
use std::{env, fs, time::Instant};

fn load() -> slm::Weights {
    let dir = env::var("SLM_MODEL").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/model").to_string());
    slm::Weights::load(&fs::read_to_string(format!("{dir}/manifest.txt")).expect("manifest.txt"), &fs::read(format!("{dir}/weights.bin")).expect("weights.bin"))
}
fn ids(path: &str) -> Vec<u32> { fs::read(path).expect("ids").chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect() }

fn main() {
    let a: Vec<String> = env::args().collect();
    let w = load();
    match a.get(1).map(String::as_str) {
        Some("check") => {
            let dir = env::var("SLM_MODEL").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/model").to_string());
            let toks = ids(&format!("{dir}/ref_ids.bin"));
            let refl: Vec<f32> = fs::read(format!("{dir}/ref_logits.bin")).unwrap().chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
            let mut m = slm::Model::new(&w);
            let (mut maxd, mut at) = (0f32, (0, 0));
            let mut bits_ref = 0f64; let mut bits_me = 0f64;
            for (i, &t) in toks.iter().enumerate() {
                m.step(t);
                let r = &refl[i * slm::V..(i + 1) * slm::V];
                for j in 0..slm::V { let d = (m.logits[j] - r[j]).abs(); if d > maxd { maxd = d; at = (i, j); } }
                if i + 1 < toks.len() { let nt = toks[i + 1] as usize; bits_ref += -(slm::log_prob(r, nt) as f64) / 2f64.ln(); bits_me += -(slm::log_prob(&m.logits, nt) as f64) / 2f64.ln(); }
            }
            println!("{} tokens: max |logit diff| = {maxd:.3e} at token {} vocab {}", toks.len(), at.0, at.1);
            println!("total bits over the sequence: pytorch {bits_ref:.3}  rust {bits_me:.3}");
        }
        Some("bpb") => {
            let toks = ids(&a[2]); let nbytes: f64 = a[3].parse().unwrap();
            let (ctx, stride) = (slm::CTX, env::var("SLM_STRIDE").ok().and_then(|v| v.parse().ok()).unwrap_or(slm::CTX / 2));
            let mut m = slm::Model::new(&w);
            let mut bits = 16f64; let mut scored = 0usize; let t0 = Instant::now(); let mut hash = 0xcbf29ce484222325u64;
            let mut start = 0;
            loop {
                let end = (start + ctx).min(toks.len());
                m.reset();
                let first = if start == 0 { 1 } else { ctx - stride };
                for i in start..end {
                    m.step(toks[i]);
                    for &l in &m.logits { hash = (hash ^ l.to_bits() as u64).wrapping_mul(0x100000001b3); }
                    let k = i - start;
                    if k + 1 >= first && i + 1 < end { bits += -(slm::prof::t(7, || slm::log_prob(&m.logits, toks[i + 1] as usize)) as f64) / 2f64.ln(); scored += 1; }
                }
                if end == toks.len() { break; }
                start += stride;
            }
            let dt = t0.elapsed().as_secs_f64();
            slm::prof::report();
            println!("logits hash {hash:016x}");
            println!("tokens {}  scored {scored}  bits per byte {:.4}  ({:.0} tok/s, {:.1} KB/s, {} threads)", toks.len(), bits / nbytes, (end_steps(toks.len(), ctx, stride)) as f64 / dt, nbytes / dt / 1000.0, m.threads());
        }
        Some("micro") => {
            // hot-cache int8 matvec throughput per matrix shape
            let l = &w.layers[0];
            let mut x = slm::QVec::zero(); let xs: Vec<f32> = (0..slm::FF).map(|i| ((i * 37) % 101) as f32 / 50.0 - 1.0).collect();
            let mut out = vec![0f32; slm::V];
            let mats: Vec<(&str, &slm::QMat)> = vec![("gu 704x144", &l.gu), ("down 144x352", &l.down), ("wte 4096x144", &w.wte),
                ("gdn proj 1008x144", match &l.mixer { slm::Mixer::Gdn(g) => &g.proj, _ => unreachable!() })];
            for (name, m) in mats {
                x.set(&xs[..m.n_in]);
                let iters = 20000; let t0 = Instant::now();
                for _ in 0..iters { slm::qmatvec(m, &x, &mut out[..m.n_out]); std::hint::black_box(&out); }
                let dt = t0.elapsed().as_secs_f64() / iters as f64;
                println!("{name:>20}: {:7.0} ns  {:5.1} GMAC/s  {:5.1} GB/s weights", dt * 1e9, (m.n_in * m.n_out) as f64 / dt / 1e9, (m.n_in * m.n_out) as f64 / dt / 1e9);
            }
        }
        _ => eprintln!("usage: slmcli check | slmcli bpb <ids.bin> <nbytes> | slmcli micro"),
    }
}
/// number of model steps the windowed pass performs (windows overlap by ctx-stride)
fn end_steps(n: usize, ctx: usize, stride: usize) -> usize { let mut s = 0; let mut start = 0; loop { let end = (start + ctx).min(n); s += end - start; if end == n { break; } start += stride; } s }
