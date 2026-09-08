//! The seed search, reused by datagen and the gate. Same semantics as engine/src/search.rs:
//! bare alpha-beta, nothing layered on it.
use board::types::{MOVE_NONE, Move, Outcome};
use board::Position;
use nnue::{Net, Score};

pub const MATE: Score = 30_000;
pub const INF: Score = 32_000;

pub struct Searcher {
    pub nodes: u64,
    scratch: Vec<f32>,
}

impl Searcher {
    pub fn new() -> Self {
        Searcher { nodes: 0, scratch: Vec::new() }
    }
    pub fn best_move(&mut self, pos: &mut Position, depth: u32, net: &Net) -> (Move, Score) {
        let list = pos.legal_moves();
        if list.is_empty() {
            return (MOVE_NONE, if pos.in_check(pos.stm) { -MATE } else { 0 });
        }
        let (mut best, mut best_s, mut alpha) = (list.as_slice()[0], -INF, -INF);
        for &m in list.as_slice() {
            let u = pos.make_move(m);
            let s = -self.ab(pos, depth.saturating_sub(1), -INF, -alpha, net);
            pos.unmake_move(m, u);
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
            return net.eval(pos, &mut self.scratch);
        }
        let mut best = -INF;
        for &m in list.as_slice() {
            let u = pos.make_move(m);
            let s = -self.ab(pos, depth - 1, -beta, -alpha, net);
            pos.unmake_move(m, u);
            if s > best { best = s; }
            if best > alpha { alpha = best; }
            if alpha >= beta { break; }
        }
        best
    }
}
impl Default for Searcher { fn default() -> Self { Self::new() } }
