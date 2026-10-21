//! The model's tokenizer in Rust: HF `Digits(individual)` + `ByteLevel(use_regex)` pre-tokenisation,
//! then byte-level BPE over `merges.bin` (left, right, merged ids per rank).
//!
//! Correctness of the codec never depends on matching Hugging Face exactly: the decoder follows whatever
//! segmentation the encoder transmits, and every byte is a token, so any input round-trips. Matching
//! matters only for ratio (the model was trained on HF's cuts). The GPT-2 regex is hand-rolled over
//! chars with std's Unicode classes (`is_alphabetic` for \p{L}, `is_numeric` for \p{N}).
use std::collections::HashMap;

pub struct Bpe { ranks: HashMap<(u16, u16), (u32, u16)>, byte_tok: [u16; 256] }

impl Bpe {
    /// `merges`: merges.bin; `vocab_bytes`: the id -> bytes table (to find the 256 single-byte tokens).
    pub fn load(merges: &[u8], vocab_bytes: &[Vec<u8>]) -> Bpe {
        let mut ranks = HashMap::with_capacity(merges.len() / 6);
        for (r, m) in merges.chunks_exact(6).enumerate() {
            let a = u16::from_le_bytes([m[0], m[1]]); let b = u16::from_le_bytes([m[2], m[3]]); let c = u16::from_le_bytes([m[4], m[5]]);
            ranks.entry((a, b)).or_insert((r as u32, c));
        }
        let mut byte_tok = [u16::MAX; 256];
        for (id, b) in vocab_bytes.iter().enumerate() { if b.len() == 1 && byte_tok[b[0] as usize] == u16::MAX { byte_tok[b[0] as usize] = id as u16; } }
        assert!(byte_tok.iter().all(|&t| t != u16::MAX), "vocab lacks a single-byte token");
        Bpe { ranks, byte_tok }
    }

    /// Token ids whose bytes concatenate to `data`.
    pub fn encode(&self, data: &[u8]) -> Vec<u32> {
        let mut out = Vec::with_capacity(data.len() / 2);
        // work on valid UTF-8 runs as chars; stray bytes become pieces of their own
        let mut i = 0;
        while i < data.len() {
            match std::str::from_utf8(&data[i..]) {
                Ok(s) => { self.encode_str(s, &mut out); break; }
                Err(e) => {
                    let good = e.valid_up_to();
                    if good > 0 { self.encode_str(std::str::from_utf8(&data[i..i + good]).unwrap(), &mut out); }
                    let bad = e.error_len().unwrap_or(data.len() - i - good);
                    for k in 0..bad { self.bpe_piece(&data[i + good + k..i + good + k + 1], &mut out); }
                    i += good + bad;
                }
            }
        }
        out
    }

    fn encode_str(&self, s: &str, out: &mut Vec<u32>) {
        // Digits(individual_digits): every numeric char is its own piece; the rest between them are pieces
        let mut start = 0;
        for (idx, ch) in s.char_indices() {
            if ch.is_numeric() {
                if idx > start { self.regex_pieces(&s[start..idx], out); }
                self.bpe_piece(&s[idx..idx + ch.len_utf8()].as_bytes(), out);
                start = idx + ch.len_utf8();
            }
        }
        if start < s.len() { self.regex_pieces(&s[start..], out); }
    }

    /// GPT-2's pattern: 's|'t|'re|'ve|'m|'ll|'d| ?\p{L}+| ?\p{N}+| ?[^\s\p{L}\p{N}]+|\s+(?!\S)|\s+
    fn regex_pieces(&self, s: &str, out: &mut Vec<u32>) {
        let cs: Vec<(usize, char)> = s.char_indices().collect();
        let n = cs.len();
        let at = |k: usize| if k < n { Some(cs[k].1) } else { None };
        let end_of = |k: usize| if k < n { cs[k].0 } else { s.len() };
        let mut k = 0;
        while k < n {
            let c = cs[k].1;
            let mut len = 0usize;   // in chars
            if c == '\'' {
                let rest: String = cs[k + 1..(k + 3).min(n)].iter().map(|x| x.1).collect();
                for suf in ["s", "t", "re", "ve", "m", "ll", "d"] { if rest.starts_with(suf) { len = 1 + suf.chars().count(); break; } }
                // note: 're' before 'r'? the alternation lists only these exact suffixes; order matters for 's' vs 'se' (no such alt)
            }
            if len == 0 {
                let (sp, j) = if c == ' ' { (1, k + 1) } else { (0, k) };
                if let Some(d) = at(j) {
                    let class = if d.is_alphabetic() { 1 } else if d.is_numeric() { 2 } else if !d.is_whitespace() { 3 } else { 0 };
                    if class != 0 {
                        let mut m = j;
                        while let Some(x) = at(m) {
                            let same = match class { 1 => x.is_alphabetic(), 2 => x.is_numeric(), _ => !x.is_whitespace() && !x.is_alphabetic() && !x.is_numeric() };
                            if same { m += 1; } else { break; }
                        }
                        len = sp + (m - j);
                    }
                }
            }
            if len == 0 && c.is_whitespace() {
                let mut m = k; while let Some(x) = at(m) { if x.is_whitespace() { m += 1; } else { break; } }
                let run = m - k;
                len = if m == n { run } else if run >= 2 { run - 1 } else { 1 };
            }
            if len == 0 { len = 1; }   // cannot happen for valid text; keep moving
            let piece = &s[cs[k].0..end_of(k + len)];
            self.bpe_piece(piece.as_bytes(), out);
            k += len;
        }
    }

    /// Byte-level BPE on one pre-token: start from single-byte tokens, merge the lowest-ranked adjacent pair until none.
    fn bpe_piece(&self, bytes: &[u8], out: &mut Vec<u32>) {
        let mut sym: Vec<u16> = bytes.iter().map(|&b| self.byte_tok[b as usize]).collect();
        loop {
            let mut best: Option<(u32, usize, u16)> = None;
            for i in 0..sym.len().saturating_sub(1) {
                if let Some(&(r, id)) = self.ranks.get(&(sym[i], sym[i + 1])) {
                    if best.map_or(true, |(br, _, _)| r < br) { best = Some((r, i, id)); }
                }
            }
            match best { Some((_, i, id)) => { sym[i] = id; sym.remove(i + 1); } None => break }
        }
        out.extend(sym.iter().map(|&s| s as u32));
    }
}
