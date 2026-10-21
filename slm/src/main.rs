//! slmcli check            — compare against PyTorch reference logits (model/ref_*.bin)
//! slmcli bpb <ids.bin> <nbytes>  — bits per byte over a token file: one streaming pass (default), or SLM_WINDOWED=1 for
//!                                 1024-token windows at stride 512 (same as bench/lm_bpb.py); SLM_THREADS, SLM_GDN_DECAY, SLM_ATT_WIN knobs
use std::{env, fs, time::Instant};

fn load() -> slm::Weights {
    let dir = env::var("SLM_MODEL").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/model").to_string());
    slm::Weights::load(&fs::read_to_string(format!("{dir}/manifest.txt")).expect("manifest.txt"), &fs::read(format!("{dir}/weights.bin")).expect("weights.bin"))
}
fn ids(path: &str) -> Vec<u32> { fs::read(path).expect("ids").chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect() }

fn threads() -> usize { env::var("SLM_THREADS").ok().and_then(|v| v.parse().ok()).unwrap_or(slm::NH) }
fn model_dir() -> String { env::var("SLM_MODEL").unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/model").to_string()) }

fn main() {
    let a: Vec<String> = env::args().collect();
    match a.get(1).map(String::as_str) {
        Some("tok-check") => {
            // slmcli tok-check <file> <nbytes> <ref.ids>: our BPE vs Hugging Face's ids
            let dir = model_dir();
            let v = slm::bridge::Vocab::load(&fs::read(format!("{dir}/vocab.bin")).unwrap());
            let bpe = slm::bpe::Bpe::load(&fs::read(format!("{dir}/merges.bin")).unwrap(), &v.bytes);
            let n: usize = a[3].parse().unwrap(); let data = fs::read(&a[2]).unwrap(); let data = &data[..n.min(data.len())];
            let t0 = Instant::now(); let ours = bpe.encode(data); let dt = t0.elapsed().as_secs_f64();
            let back: Vec<u8> = ours.iter().flat_map(|&i| v.bytes[i as usize].clone()).collect(); assert_eq!(back, data, "round trip");
            let r = ids(&a[4]);
            // align by byte offsets: count tokens whose (start, id) pair exists in both
            let offs = |t: &[u32]| { let mut o = std::collections::HashSet::new(); let mut p = 0usize; for &i in t { o.insert((p, i)); p += v.bytes[i as usize].len(); } o };
            let (oa, ob) = (offs(&ours), offs(&r)); let same = oa.intersection(&ob).count();
            println!("bytes {}  ours {} tokens  hf {} tokens  identical tokens {:.2}% of hf  ({:.1} MB/s)", data.len(), ours.len(), r.len(), 100.0 * same as f64 / r.len() as f64, data.len() as f64 / dt / 1e6);
            if ours != r { let k = ours.iter().zip(&r).position(|(x, y)| x != y).unwrap_or(ours.len().min(r.len())); let show = |t: &[u32]| t[k.saturating_sub(3)..(k + 5).min(t.len())].iter().map(|&i| String::from_utf8_lossy(&v.bytes[i as usize]).to_string()).collect::<Vec<_>>(); println!("  first difference at token {k}: ours {:?}  hf {:?}", show(&ours), show(&r)); }
            return;
        }
        Some("pack") => {
            // slmcli pack <out.slmq>: quantised weights + tokenizer in one file for the browser; prints the model id
            let dir = model_dir();
            let rd = |n: &str| fs::read(format!("{dir}/{n}")).unwrap();
            let w = slm::Weights::load(&String::from_utf8(rd("manifest.txt")).unwrap(), &rd("weights.bin"));
            let pack = slm::pack::pack(&w, &rd("vocab.bin"), &rd("merges.bin"));
            fs::write(&a[2], &pack).unwrap();
            let back = slm::codec::Assets::from_pack(&pack).expect("pack does not read back");
            let direct = slm::codec::Assets::from_dir(&dir);
            println!("{}: {} bytes  model id {:08x}  (from weights.bin {:08x}){}", a[2], pack.len(), back.model_id, direct.model_id, if back.model_id == direct.model_id { "" } else { "  MISMATCH" });
            return;
        }
        Some("compress") | Some("decompress") | Some("roundtrip") => {
            let assets = slm::codec::Assets::from_dir(&model_dir());
            let data = fs::read(&a[2]).expect("input");
            let t0 = Instant::now();
            match a[1].as_str() {
                "compress" => { let out = slm::codec::compress(&assets, &data, threads()); fs::write(&a[3], &out).unwrap(); let dt = t0.elapsed().as_secs_f64(); println!("{} -> {} bytes  {:.4} bpb  {:.1} KB/s", data.len(), out.len(), 8.0 * out.len() as f64 / data.len().max(1) as f64, data.len() as f64 / dt / 1000.0); }
                "decompress" => { let out = slm::codec::decompress(&assets, &data, threads()).expect("not an NCZL stream for this model"); fs::write(&a[3], &out).unwrap(); let dt = t0.elapsed().as_secs_f64(); println!("{} -> {} bytes  {:.1} KB/s", data.len(), out.len(), out.len() as f64 / dt / 1000.0); }
                _ => {
                    let c = slm::codec::compress(&assets, &data, threads()); let t1 = Instant::now();
                    let d = slm::codec::decompress(&assets, &c, threads()).expect("decode failed");
                    let ok = d == data;
                    println!("{}: {} -> {} bytes  {:.4} bpb  enc {:.1} KB/s  dec {:.1} KB/s  {}", a[2], data.len(), c.len(), 8.0 * c.len() as f64 / data.len().max(1) as f64, data.len() as f64 / (t1 - t0).as_secs_f64() / 1000.0, data.len() as f64 / t1.elapsed().as_secs_f64() / 1000.0, if ok { "round trip OK" } else { "MISMATCH" });
                    if !ok { std::process::exit(1); }
                }
            }
            return;
        }
        _ => {}
    }
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
            let stream = env::var("SLM_WINDOWED").is_err();   // default: one streaming pass; SLM_WINDOWED=1 for the 1024/512 windows
            if stream {
                // one pass, no resets: GDN state runs on, attention sees the last CTX keys
                let gr: usize = env::var("SLM_GDN_RESET").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
                let ar: usize = env::var("SLM_ATT_RESET").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
                if let Some(w) = env::var("SLM_ATT_WIN").ok().and_then(|v| v.parse().ok()) { m.set_attn_window(w); }
                for i in 0..toks.len() {
                    if gr > 0 && i > 0 {
                        if env::var("SLM_GDN_STAGGER").is_ok() {
                            // six GDN layers reset at six evenly spaced offsets inside the interval
                            let mut gi = 0; for l in 0..slm::NL { if !slm::is_attn(l) { if (i + gi * gr / 6) % gr == 0 { m.reset_gdn_layer(l); } gi += 1; } }
                        } else if i % gr == 0 { if let Some(f) = env::var("SLM_GDN_SOFT").ok().and_then(|v| v.parse::<f32>().ok()) { m.soften_gdn(f); } else { m.reset_gdn(); } }
                    }
                    if ar > 0 && i > 0 && i % ar == 0 { m.reset_attn(); }
                    m.step(toks[i]);
                    for &l in &m.logits { hash = (hash ^ l.to_bits() as u64).wrapping_mul(0x100000001b3); }
                    if i + 1 < toks.len() { bits += -(slm::prof::t(7, || slm::log_prob(&m.logits, toks[i + 1] as usize)) as f64) / 2f64.ln(); scored += 1; }
                }
                let dt = t0.elapsed().as_secs_f64();
                slm::prof::report();
                println!("logits hash {hash:016x}");
                println!("STREAM tokens {}  scored {scored}  bits per byte {:.4}  ({:.0} tok/s, {:.1} KB/s, {} threads)", toks.len(), bits / nbytes, toks.len() as f64 / dt, nbytes / dt / 1000.0, m.threads());
                return;
            }
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
        _ => eprintln!("usage: slmcli check | bpb <ids.bin> <nbytes> | micro | tok-check <file> <nbytes> <ref.ids> | compress <in> <out> | decompress <in> <out> | roundtrip <file>"),
    }
}
/// number of model steps the windowed pass performs (windows overlap by ctx-stride)
fn end_steps(n: usize, ctx: usize, stride: usize) -> usize { let mut s = 0; let mut start = 0; loop { let end = (start + ctx).min(n); s += end - start; if end == n { break; } start += stride; } s }
