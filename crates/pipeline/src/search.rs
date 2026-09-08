//! The seed search, reused by datagen and the gate. Same semantics as engine/src/search.rs:
//! bare alpha-beta, nothing layered on it.
use board::types::{MOVE_NONE, Move, Outcome};
use board::Position;
use nnue::{Acc, Net, Score};

pub const MATE: Score = 30_000;
pub const INF: Score = 32_000;

pub struct Searcher {
    pub nodes: u64,
    pub incremental: bool,
    scratch: Vec<f32>,
    /// Incremental accumulator plus a stack of saved states, one per ply. A move changes at
    /// most a handful of features, so only those rows are touched; the saved copy is 32 floats
    /// and restoring it on unmake is cheaper than recomputing. Verified equal to a full
    /// refresh in nnue/tests/incremental.rs.
    acc: Acc,
    /// Flat, preallocated stack: one accumulator snapshot per ply, laid out contiguously so a
    /// push is a memcpy and never a heap allocation. The first version cloned a Vec per node.
    stack: Vec<f32>,
    ply: usize,
    /// Membership bitset over the feature space, so the active-set diff is O(n) rather than
    /// the O(n^2) Vec::contains the first version used (38x38 comparisons per move).
    mark: Vec<u64>,
    feat_a: Vec<u16>,
    feat_b: Vec<u16>,
    on: Vec<u16>,
    off: Vec<u16>,
}

impl Searcher {
    pub fn new() -> Self {
        Searcher {
            nodes: 0,
            // Default chosen by MEASUREMENT, not assumption. Node counts are identical across
            // both paths at every width (same tree), so these ratios are real:
            //   hidden  32   refresh 1718969   incr 1560497   0.91x  LOSS
            //   hidden 128   refresh  889406   incr 1014240   1.14x  win
            //   hidden 512   refresh  240701   incr  363458   1.51x  win
            // The saving scales with width (~38 rows rebuilt -> ~4 touched); the bookkeeping
            // (two active() scans, bitset diff, memcpy) does not. Crossover is near 64.
            incremental: true, // set per-search by best_move from net.n_hidden
            scratch: Vec::new(),
            acc: Acc { vals: Vec::new() },
            stack: Vec::new(),
            ply: 0,
            mark: vec![0u64; (nnue::N_INPUTS + 63) / 64],
            feat_a: Vec::new(),
            feat_b: Vec::new(),
            on: Vec::new(),
            off: Vec::new(),
        }
    }

    /// Apply `m`, updating the accumulator by the feature delta. Computing the two active sets
    /// is cheap (bitboard iteration); what it saves is the ROW ADDS -- ~38 rows rebuilt becomes
    /// 2-6 rows touched.
    #[inline]
    fn push_move(&mut self, pos: &mut Position, m: Move, net: &Net) -> board::chess::Undo {
        if !self.incremental {
            return pos.make_move(m);
        }
        let h = net.n_hidden;
        Net::active(pos, &mut self.feat_a);
        let u = pos.make_move(m);
        Net::active(pos, &mut self.feat_b);

        // save: memcpy into the preallocated stack slot for this ply
        let base = self.ply * h;
        if self.stack.len() < base + h {
            self.stack.resize(base + h, 0.0);
        }
        self.stack[base..base + h].copy_from_slice(&self.acc.vals);
        self.ply += 1;

        // O(n) diff via a bitset instead of O(n^2) Vec::contains
        for w in self.mark.iter_mut() { *w = 0; }
        for &f in &self.feat_a { self.mark[f as usize >> 6] |= 1u64 << (f & 63); }
        self.on.clear();
        for &f in &self.feat_b {
            if self.mark[f as usize >> 6] & (1u64 << (f & 63)) == 0 { self.on.push(f); }
        }
        for w in self.mark.iter_mut() { *w = 0; }
        for &f in &self.feat_b { self.mark[f as usize >> 6] |= 1u64 << (f & 63); }
        self.off.clear();
        for &f in &self.feat_a {
            if self.mark[f as usize >> 6] & (1u64 << (f & 63)) == 0 { self.off.push(f); }
        }

        let (on, off) = (std::mem::take(&mut self.on), std::mem::take(&mut self.off));
        self.acc.update(net, &on, &off);
        self.on = on;
        self.off = off;
        u
    }

    #[inline]
    fn pop_move(&mut self, pos: &mut Position, m: Move, u: board::chess::Undo, net: &Net) {
        pos.unmake_move(m, u);
        if !self.incremental { return; }
        let h = net.n_hidden;
        self.ply -= 1;
        let base = self.ply * h;
        self.acc.vals.copy_from_slice(&self.stack[base..base + h]);
    }
    pub fn best_move(&mut self, pos: &mut Position, depth: u32, net: &Net) -> (Move, Score) {
        self.incremental = std::env::var("EXISTENCE_FULL_REFRESH").is_err() && net.n_hidden >= 64;
        self.acc.refresh(net, pos);
        self.ply = 0;
        let list = pos.legal_moves();
        if list.is_empty() {
            return (MOVE_NONE, if pos.in_check(pos.stm) { -MATE } else { 0 });
        }
        let (mut best, mut best_s, mut alpha) = (list.as_slice()[0], -INF, -INF);
        for &m in list.as_slice() {
            let u = self.push_move(pos, m, net);
            let s = -self.ab(pos, depth.saturating_sub(1), -INF, -alpha, net);
            self.pop_move(pos, m, u, net);
            if s > best_s {
                best_s = s;
                best = m;
                if s > alpha { alpha = s; }
            }
        }
        (best, best_s)
    }
    fn ab(&mut self, pos: &mut Position, depth: u32, mut alpha: Score, beta: Score, net: &Net) -> Score {
        self.nodes += 1;
        let list = pos.legal_moves();
        if list.is_empty() {
            return match pos.outcome() {
                Outcome::Loss => -MATE + (64 - depth as Score),
                _ => 0,
            };
        }
        if depth == 0 {
            // A/B switch: the incremental path only pays once the hidden width makes the ~38
            // saved row-adds outweigh its fixed bookkeeping (two active() scans, the bitset
            // diff, the memcpy). Measured, not assumed -- see examples/search_bench.rs.
            return if self.incremental {
                self.acc.score(net, pos)
            } else {
                net.eval(pos, &mut self.scratch)
            };
        }
        let mut best = -INF;
        for &m in list.as_slice() {
            let u = self.push_move(pos, m, net);
            let s = -self.ab(pos, depth - 1, -beta, -alpha, net);
            self.pop_move(pos, m, u, net);
            if s > best { best = s; }
            if best > alpha { alpha = best; }
            if alpha >= beta { break; }
        }
        best
    }
}
impl Default for Searcher { fn default() -> Self { Self::new() } }
