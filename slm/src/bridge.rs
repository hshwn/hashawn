//! Token model -> byte model, decodable, with the tokenisation transmitted.
//!
//! A codec decodes bytes one at a time, but the model predicts tokens. Guessing token ends on the decoder
//! side (greedy longest match) was measured and is bad twice over: the model loses 18% on non-BPE cuts, and
//! the end-of-token byte is predicted from the wrong distribution. So the encoder keeps the real BPE
//! tokenisation and codes it: before every byte inside a token, one binary flag "the token ends here" with
//! the model's own probability P(token = tail) / P(token starts with tail). Given the flag, the next byte is
//! predicted conditioned on "continues": mass of tokens below the trie node tail+b over the mass of strict
//! extensions of tail. Byte bits plus flag bits telescope to exactly the token cost, so the flags are free
//! in total; the byte bits are what ncz can improve on.
//!
//! Vocabulary ids are laid out in trie DFS order, so any subtree's mass is a difference of two prefix sums.
//! Sums are f64 in fixed order: identical on every machine.
use crate::{Model, V};

const FLOOR: f64 = 1e-7;          // uniform mix so no byte is ever impossible
pub const EOT: u32 = 0;           // <|endoftext|>: start token

struct Node { children: Vec<(u8, u32)>, tok: i32, lo: usize, hi: usize }   // tokens in this subtree = dfs ranks [lo, hi)

pub struct Vocab { nodes: Vec<Node>, rank: Vec<usize> /* token id -> dfs rank */, pub bytes: Vec<Vec<u8>> }

impl Vocab {
    /// `vocab.bin`: for each id, u16 LE length then the bytes.
    pub fn load(bin: &[u8]) -> Vocab {
        let mut bytes = Vec::with_capacity(V); let mut i = 0;
        while i + 2 <= bin.len() { let n = u16::from_le_bytes([bin[i], bin[i + 1]]) as usize; i += 2; bytes.push(bin[i..i + n].to_vec()); i += n; }
        assert_eq!(bytes.len(), V);
        let mut nodes = vec![Node { children: Vec::new(), tok: -1, lo: 0, hi: 0 }];
        for (id, b) in bytes.iter().enumerate() {
            if id as u32 == EOT || b.is_empty() { continue; }
            let mut n = 0usize;
            for &c in b {
                n = match nodes[n].children.iter().find(|(cb, _)| *cb == c) { Some(&(_, ch)) => ch as usize, None => { nodes.push(Node { children: Vec::new(), tok: -1, lo: 0, hi: 0 }); let ch = nodes.len() - 1; nodes[n].children.push((c, ch as u32)); ch } };
            }
            nodes[n].tok = id as i32;
        }
        for n in nodes.iter_mut() { n.children.sort_by_key(|(c, _)| *c); }
        let mut rank = vec![usize::MAX; V]; let mut next = 0usize;
        fn dfs(nodes: &mut Vec<Node>, n: usize, next: &mut usize, rank: &mut [usize]) {
            nodes[n].lo = *next;
            if nodes[n].tok >= 0 { rank[nodes[n].tok as usize] = *next; *next += 1; }
            let ch: Vec<u32> = nodes[n].children.iter().map(|(_, c)| *c).collect();
            for c in ch { dfs(nodes, c as usize, next, rank); }
            nodes[n].hi = *next;
        }
        dfs(&mut nodes, 0, &mut next, &mut rank);
        rank[EOT as usize] = next;   // last slot, outside every subtree
        Vocab { nodes, rank, bytes }
    }
    #[inline] fn child(&self, n: usize, b: u8) -> Option<usize> { self.nodes[n].children.binary_search_by_key(&b, |(c, _)| *c).ok().map(|i| self.nodes[n].children[i].1 as usize) }
}

/// Streaming byte predictor on top of a token model, following a tokenisation the encoder transmits.
pub struct Bridge<'a> {
    pub m: Model<'a>,
    v: &'a Vocab,
    pre: Vec<f64>,          // prefix sums of token probabilities over dfs ranks, len V+1
    by_rank: Vec<f64>,
    tail_node: usize, tail_len: usize,
    pub tokens: usize,      // model steps so far
    pub p: [f64; 256],      // last next-byte distribution
    tree: [f64; 512],
}

impl<'a> Bridge<'a> {
    pub fn new(m: Model<'a>, v: &'a Vocab) -> Bridge<'a> {
        let mut b = Bridge { m, v, pre: vec![0f64; V + 1], by_rank: vec![0f64; V], tail_node: 0, tail_len: 0, tokens: 0, p: [0.0; 256], tree: [0.0; 512] };
        b.step(EOT);
        b
    }
    fn step(&mut self, tok: u32) {
        self.m.step(tok); self.tokens += 1;
        let lg = &self.m.logits;
        let mx = crate::simd::max(lg);
        let mut sum = 0f64; let mut e = vec![0f32; V];
        for i in 0..V { e[i] = crate::simd::fexp(lg[i] - mx); sum += e[i] as f64; }
        let inv = 1.0 / sum;
        for i in 0..V { self.by_rank[self.v.rank[i]] = e[i] as f64 * inv; }
        self.pre[0] = 0.0; for r in 0..V { self.pre[r + 1] = self.pre[r] + self.by_rank[r]; }
    }
    #[inline] fn mass(&self, n: usize) -> f64 { let nd = &self.v.nodes[n]; self.pre[nd.hi] - self.pre[nd.lo] }
    #[inline] fn own(&self, n: usize) -> f64 { let t = self.v.nodes[n].tok; if t < 0 { 0.0 } else { self.by_rank[self.v.rank[t as usize]] } }
    pub fn in_token(&self) -> bool { self.tail_len > 0 }

    /// P(the current token ends here), defined when the tail is non-empty. Deterministic given past bytes.
    pub fn p_end(&self) -> f64 {
        let (m, o) = (self.mass(self.tail_node), self.own(self.tail_node));
        if m <= 0.0 { 0.5 } else { (o / m).clamp(1e-9, 1.0 - 1e-9) }
    }
    /// The encoder says the token ended: step the model on it.
    pub fn end_token(&mut self, tok: u32) {
        debug_assert_eq!(self.v.nodes[self.tail_node].tok, tok as i32, "tokenisation does not match the vocab trie");
        self.tail_node = 0; self.tail_len = 0;
        self.step(tok);
    }
    /// Fill `self.p` with P(next byte | the token continues past the tail).
    pub fn predict(&mut self) {
        let mut p = [0f64; 256];
        let nd = &self.v.nodes[self.tail_node];
        for &(b, c) in &nd.children { p[b as usize] = self.mass(c as usize); }
        let mut tot = 0f64; for b in 0..256 { tot += p[b]; }
        let (w, u) = if tot > 0.0 { (1.0 - FLOOR * 256.0, FLOOR) } else { (0.0, 1.0 / 256.0) };
        let inv = if tot > 0.0 { w / tot } else { 0.0 };
        for b in 0..256 { p[b] = p[b] * inv + u; }
        self.p = p;
    }
    /// The token the tail spells (valid when the end flag fires: p_end > 0 implies the tail is a token).
    pub fn tail_token(&self) -> Option<u32> { let t = self.v.nodes[self.tail_node].tok; if t < 0 { None } else { Some(t as u32) } }
    /// Build the binary tree over `self.p` so bits can be predicted one at a time (after `predict()`).
    pub fn begin_byte(&mut self) {
        let t = &mut self.tree;
        for b in 0..256 { t[256 + b] = self.p[b]; }
        for n in (1..256).rev() { t[n] = t[2 * n] + t[2 * n + 1]; }
    }
    /// Stretch for bit j (MSB first) given the j bits already known as `prefix`, in ncz units.
    #[inline] pub fn bit_stretch(&self, j: usize, prefix: u32) -> i16 {
        let n = (1usize << j) + prefix as usize;
        let p1 = (self.tree[2 * n + 1] / self.tree[n]).clamp(1e-6, 1.0 - 1e-6);
        (256.0 * (p1 / (1.0 - p1)).ln()).round().clamp(-2047.0, 2047.0) as i16
    }
    /// Per-bit predictions for byte `actual` (MSB first) as ncz stretch units, after `predict()`.
    pub fn bits(&mut self, actual: u8, out: &mut [i16; 8]) {
        self.begin_byte();
        let mut prefix = 0u32;
        for j in 0..8 { out[j] = self.bit_stretch(j, prefix); prefix = (prefix << 1) | ((actual >> (7 - j)) & 1) as u32; }
    }
    /// Consume the actual byte (the token continues through it). False if no token continues this way.
    pub fn observe(&mut self, b: u8) -> bool {
        match self.v.child(self.tail_node, b) { Some(n) => { self.tail_node = n; self.tail_len += 1; true } None => false }
    }
}
