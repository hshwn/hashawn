//! Token model -> byte model, decodable, with the tokenisation transmitted.
//!
//! Byte-first order (v2): at a tail u the next byte is coded under the mixture
//! (1-q) P(b | the token continues) + q P(b | a new token starts), q = the flag model's P(end);
//! the new-token distribution comes from a speculative model step on the tail's token (state snapshot,
//! restored if the token continues). Then the flag is coded given the byte: free when the byte cannot
//! extend the tail or the tail is not a token, cheap otherwise. Under the model alone this costs the same
//! as flag-first (chain rule); it moves the uncertainty into the byte bits, where ncz's mixer helps.
//! Below a threshold on q the speculative step is skipped and the current first-byte marginal stands in.
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

/// What the flag can be, given the byte just coded.
pub enum Flag { MustContinue, MustEnd, Prob(f64) }

/// Streaming byte predictor on top of a token model, following a tokenisation the encoder transmits.
pub struct Bridge<'a> {
    pub m: Model<'a>,
    v: &'a Vocab,
    pre: Vec<f64>, by_rank: Vec<f64>,            // prefix sums over dfs ranks (len V+1), token probs by rank
    pre_s: Vec<f64>, by_rank_s: Vec<f64>,        // same, for the speculative step
    snap: crate::Snapshot, spec: bool, q: f64,
    pc: [f64; 256], pn: [f64; 256],              // continue / new-token byte distributions behind `p`
    tail_node: usize, tail_len: usize, tail: Vec<u8>, prev_byte: u8,
    pub tokens: usize,      // model steps so far (speculative ones included)
    pub spec_steps: usize,
    pub p: [f64; 256],      // last next-byte distribution
    tree: [f64; 512],
}

impl<'a> Bridge<'a> {
    pub fn new(m: Model<'a>, v: &'a Vocab) -> Bridge<'a> {
        let mut b = Bridge { m, v, pre: vec![0f64; V + 1], by_rank: vec![0f64; V], pre_s: vec![0f64; V + 1], by_rank_s: vec![0f64; V], snap: crate::Snapshot::default(), spec: false, q: 0.0,
            pc: [0.0; 256], pn: [0.0; 256], tail_node: 0, tail_len: 0, tail: Vec::with_capacity(16), prev_byte: 0, tokens: 0, spec_steps: 0, p: [0.0; 256], tree: [0.0; 512] };
        b.step(EOT, false);
        b
    }
    fn step(&mut self, tok: u32, speculative: bool) {
        self.m.step(tok); self.tokens += 1;
        let lg = &self.m.logits;
        let mx = crate::simd::max(lg);
        let mut sum = 0f64; let mut e = vec![0f32; V];
        for i in 0..V { e[i] = crate::simd::fexp(lg[i] - mx); sum += e[i] as f64; }
        let inv = 1.0 / sum;
        let (pre, by_rank) = if speculative { (&mut self.pre_s, &mut self.by_rank_s) } else { (&mut self.pre, &mut self.by_rank) };
        for i in 0..V { by_rank[self.v.rank[i]] = e[i] as f64 * inv; }
        pre[0] = 0.0; for r in 0..V { pre[r + 1] = pre[r] + by_rank[r]; }
    }
    #[inline] fn mass_in(pre: &[f64], nd: &Node) -> f64 { pre[nd.hi] - pre[nd.lo] }
    #[inline] fn mass(&self, n: usize) -> f64 { Self::mass_in(&self.pre, &self.v.nodes[n]) }
    #[inline] fn own(&self, n: usize) -> f64 { let t = self.v.nodes[n].tok; if t < 0 { 0.0 } else { self.by_rank[self.v.rank[t as usize]] } }
    pub fn in_token(&self) -> bool { self.tail_len > 0 }
    /// Bytes of the current token so far, and the byte just before it (0 at the start).
    pub fn tail(&self) -> &[u8] { &self.tail }
    pub fn prev_byte(&self) -> u8 { self.prev_byte }
    /// The token the tail spells, if it is one.
    pub fn tail_token(&self) -> Option<u32> { let t = self.v.nodes[self.tail_node].tok; if t < 0 { None } else { Some(t as u32) } }

    /// P(the current token ends here) under the model alone (the flag model's main input).
    pub fn p_end(&self) -> f64 {
        let (m, o) = (self.mass(self.tail_node), self.own(self.tail_node));
        if m <= 0.0 { 0.5 } else { (o / m).clamp(1e-9, 1.0 - 1e-9) }
    }

    /// Set up the next byte: `q` = P(end) to mix with (0 if the tail is not a token or empty). With `speculate`
    /// the model takes a snapshot and steps on the tail's token to get the real new-token distribution;
    /// otherwise the current first-byte marginal stands in. Fills `self.p`.
    pub fn begin(&mut self, q: f64, speculate: bool) {
        let q = if self.tail_token().is_some() { q.clamp(0.0, 1.0) } else { 0.0 };
        self.q = q; self.spec = false;
        let mut pc = [0f64; 256]; let mut pn = [0f64; 256];
        // continue: strict extensions of the tail
        if self.tail_len > 0 {
            let nd = &self.v.nodes[self.tail_node];
            let mut tot = 0f64; for &(b, c) in &nd.children { let m = self.mass(c as usize); pc[b as usize] = m; tot += m; }
            if tot > 0.0 { let inv = 1.0 / tot; for b in 0..256 { pc[b] *= inv; } }
        }
        // new token: first-byte marginal of the speculative (or current) distribution
        if q > 0.0 && speculate {
            let tok = self.tail_token().unwrap();
            self.m.snapshot_into(&mut self.snap);
            self.step(tok, true); self.spec = true; self.spec_steps += 1;
        }
        let pre_new = if self.spec { &self.pre_s } else { &self.pre };
        if q > 0.0 || self.tail_len == 0 {
            let root = &self.v.nodes[0];
            let mut tot = 0f64; for &(b, c) in &root.children { let m = Self::mass_in(pre_new, &self.v.nodes[c as usize]); pn[b as usize] = m; tot += m; }
            if tot > 0.0 { let inv = 1.0 / tot; for b in 0..256 { pn[b] *= inv; } }
        }
        let mut p = [0f64; 256];
        if self.tail_len == 0 { p = pn; } else { for b in 0..256 { p[b] = (1.0 - q) * pc[b] + q * pn[b]; } }
        let mut tot = 0f64; for b in 0..256 { tot += p[b]; }
        let (w, u) = if tot > 0.0 { (1.0 - FLOOR * 256.0, FLOOR) } else { (0.0, 1.0 / 256.0) };
        let inv = if tot > 0.0 { w / tot } else { 0.0 };
        for b in 0..256 { p[b] = p[b] * inv + u; }
        self.pc = pc; self.pn = pn; self.p = p;
    }
    /// The flag, given the byte: deterministic when only one reading is possible.
    pub fn flag(&self, b: u8) -> Flag {
        if self.tail_len == 0 { return Flag::MustContinue; }
        let can_cont = self.v.child(self.tail_node, b).is_some();
        let can_end = self.q > 0.0;
        match (can_cont, can_end) {
            (false, false) => Flag::MustEnd,       // cannot happen with a consistent tokeniser; treat as end
            (false, true) => Flag::MustEnd,
            (true, false) => Flag::MustContinue,
            (true, true) => {
                let e = self.q * self.pn[b as usize]; let c = (1.0 - self.q) * self.pc[b as usize];
                let tot = e + c; Flag::Prob(if tot > 0.0 { (e / tot).clamp(1.0 / 4096.0, 1.0 - 1.0 / 4096.0) } else { 0.5 })
            }
        }
    }
    /// The token ended before `b`; `b` starts the next one.
    pub fn commit_end(&mut self, b: u8) -> bool {
        if !self.spec { match self.tail_token() { Some(t) => self.step(t, false), None => return false } }
        else { std::mem::swap(&mut self.pre, &mut self.pre_s); std::mem::swap(&mut self.by_rank, &mut self.by_rank_s); }
        self.spec = false;
        self.prev_byte = *self.tail.last().unwrap_or(&0);
        self.tail.clear(); self.tail_len = 0; self.tail_node = 0;
        self.observe(b)
    }
    /// The token continues through `b`.
    pub fn commit_continue(&mut self, b: u8) -> bool {
        if self.spec { self.m.restore(&self.snap); self.spec = false; }
        self.observe(b)
    }
    /// Build the binary tree over `self.p` so bits can be predicted one at a time (after `begin()`).
    pub fn begin_byte(&mut self) {
        let t = &mut self.tree;
        for b in 0..256 { t[256 + b] = self.p[b]; }
        for n in (1..256).rev() { t[n] = t[2 * n] + t[2 * n + 1]; }
    }
    /// Stretch for bit j (MSB first) given the j bits already known as `prefix`, in ncz units.
    #[inline] pub fn bit_stretch(&self, j: usize, prefix: u32) -> i16 {
        let n = (1usize << j) + prefix as usize;
        let p1 = (self.tree[2 * n + 1] / self.tree[n]).clamp(1e-6, 1.0 - 1e-6) as f32;
        (256.0 * crate::simd::fln(p1 / (1.0 - p1))).round_ties_even().clamp(-2047.0, 2047.0) as i16
    }
    /// Per-bit predictions for byte `actual` (MSB first) as ncz stretch units, after `begin()`.
    pub fn bits(&mut self, actual: u8, out: &mut [i16; 8]) {
        self.begin_byte();
        let mut prefix = 0u32;
        for j in 0..8 { out[j] = self.bit_stretch(j, prefix); prefix = (prefix << 1) | ((actual >> (7 - j)) & 1) as u32; }
    }
    fn observe(&mut self, b: u8) -> bool {
        match self.v.child(self.tail_node, b) { Some(n) => { self.tail_node = n; self.tail_len += 1; self.tail.push(b); true } None => false }
    }
}
