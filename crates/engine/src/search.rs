//! The SEED search program: bare alpha-beta, and nothing else.
//!
//! MASTER_PLAN "Iteration zero" specifies exactly this and lists what must be ABSENT:
//! no ordering, no hash reuse, no iterative deepening, no quiescence, no extensions, no
//! reductions, no pruning. Every one of those has to be DISCOVERED later as a program edit
//! that beats this on the clock. Adding any of them here by hand would quietly move a row
//! from Learned back into Given and break the thesis.
//!
//! This file has a second job: it is the reference that `xtask bench-interp` measures the
//! compiled seed against. The interpreter must reach >= 50% of this NPS or the bytecode design
//! is revisited before anything is built on it (GRAMMAR 8, CRATE 11c).

use board::types::{MOVE_NONE, Move, Outcome};
use board::Position;
use nnue::{Net, Score};

pub const MATE: Score = 30_000;
/// Wide enough to be "no window"; the seed opens with the full window by construction.
pub const INF: Score = 32_000;

pub struct Search {
    pub net: Net,
    pub nodes: u64,
    scratch: Vec<f32>,
    /// Shuffled once per search. Move ordering carries no opinion at the seed
    /// (MASTER_PLAN: "children in emission order (shuffled)"), so the engine cannot
    /// accidentally inherit a hand-written ordering heuristic.
    rng: u64,
}

impl Search {
    pub fn new(net: Net, seed: u64) -> Self {
        Search { net, nodes: 0, scratch: Vec::new(), rng: seed | 1 }
    }

    fn rand(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    /// Root: return the best move and its score at fixed depth.
    pub fn best_move(&mut self, pos: &mut Position, depth: u32) -> (Move, Score) {
        self.nodes = 0;
        let list = pos.legal_moves();
        if list.is_empty() {
            return (MOVE_NONE, if pos.in_check(pos.stm) { -MATE } else { 0 });
        }
        let mut moves: Vec<Move> = list.as_slice().to_vec();
        self.shuffle(&mut moves);

        let mut best = moves[0];
        let mut best_score = -INF;
        let mut alpha = -INF;
        for m in moves {
            let u = pos.make_move(m);
            let s = -self.alphabeta(pos, depth.saturating_sub(1), -INF, -alpha);
            pos.unmake_move(m, u);
            if s > best_score {
                best_score = s;
                best = m;
                if s > alpha {
                    alpha = s;
                }
            }
        }
        (best, best_score)
    }

    fn shuffle(&mut self, v: &mut [Move]) {
        for i in (1..v.len()).rev() {
            let j = (self.rand() % (i as u64 + 1)) as usize;
            v.swap(i, j);
        }
    }

    /// Bare alpha-beta. The ONLY early exit is the window cutoff, which is what makes the
    /// returned value equal to the full-width minimax value (alpha-beta's soundness theorem)
    /// and is why FITNESS 2.3 exempts exactly this pattern when deriving the exactness taint.
    fn alphabeta(&mut self, pos: &mut Position, depth: u32, mut alpha: Score, beta: Score) -> Score {
        self.nodes += 1;

        let list = pos.legal_moves();
        if list.is_empty() {
            // Terminal. Symbolic outcome -> score. At iteration zero this mapping is fixed
            // here; it becomes the learned `score_of` table when the grammar lands.
            return match pos.outcome() {
                Outcome::Loss => -MATE + (64 - depth as Score),
                _ => 0,
            };
        }
        if depth == 0 {
            return self.net.eval(pos, &mut self.scratch);
        }

        let mut moves: Vec<Move> = list.as_slice().to_vec();
        self.shuffle(&mut moves);

        let mut best = -INF;
        for m in moves {
            let u = pos.make_move(m);
            let s = -self.alphabeta(pos, depth - 1, -beta, -alpha);
            pos.unmake_move(m, u);
            if s > best {
                best = s;
            }
            if best > alpha {
                alpha = best;
            }
            if alpha >= beta {
                break; // the window cutoff, and the only one
            }
        }
        best
    }
}
