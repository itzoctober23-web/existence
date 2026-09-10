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
use nnue::{Acc, FeatSnap, Net, Score};

pub const MATE: Score = 30_000;
/// Wide enough to be "no window"; the seed opens with the full window by construction.
pub const INF: Score = 32_000;

pub struct Search {
    pub net: Net,
    pub nodes: u64,
    /// Stop when `nodes` reaches this. `u64::MAX` disables it, which is the fixed-depth behaviour
    /// every prior measurement used.
    ///
    /// THIS IS THE GRAMMAR'S "ITERATE TO BUDGET", not a new search technique. MASTER_PLAN's Given
    /// column lists `iterate to budget` among the search primitives, and `pipeline::search` has
    /// carried `best_move_capped` for exactly this reason. It adds no ordering, no hash reuse, no
    /// deepening, no pruning -- the absent-list is untouched. It only decides WHEN TO STOP.
    pub node_cap: u64,
    aborted: bool,
    scratch: Vec<f32>,
    /// Shuffled once per search. Move ordering carries no opinion at the seed
    /// (MASTER_PLAN: "children in emission order (shuffled)"), so the engine cannot
    /// accidentally inherit a hand-written ordering heuristic.
    rng: u64,
    /// Incrementally maintained hidden sums. This is an EVAL IMPLEMENTATION detail and carries no
    /// chess opinion: it computes the same number `Net::eval` computes, from ~4 row updates per move
    /// instead of ~38 per leaf. It is behaviour-preserving by construction and asserted so by
    /// `nnue/tests/incremental.rs`, so it does not move a row from Learned back into Given -- the
    /// MASTER_PLAN "Iteration zero" absent-list is about SEARCH decisions (ordering, hash reuse,
    /// deepening, quiescence, extensions, reductions, pruning) and this changes none of them.
    acc: Acc,
    /// Feature delta scratch, reused per node so the hot path does not allocate.
    on: Vec<u16>,
    off: Vec<u16>,
    /// Saved accumulator states, one frame per ply, restored on unmake.
    ///
    /// RESTORE IS BY COPY, NOT BY INVERSE DELTA. Undoing an update by re-adding the opposite sign
    /// looks equivalent and is not: float add/sub does not round-trip, so `(a + w) - w != a` in
    /// general and the error compounds along every path in the tree. A copy is exact, and at
    /// n_hidden floats per ply it is cheaper than the row updates it protects.
    saved: Vec<f32>,
}

impl Search {
    pub fn new(net: Net, seed: u64) -> Self {
        let acc = Acc::new(&net);
        Search {
            net,
            nodes: 0,
            node_cap: u64::MAX,
            aborted: false,
            scratch: Vec::new(),
            rng: seed | 1,
            acc,
            on: Vec::new(),
            off: Vec::new(),
            saved: Vec::new(),
        }
    }

    /// Make `m`, advancing the accumulator to match, and return the state needed to undo both.
    /// Pairs with `undo`; the two exist so the make/unmake sites cannot forget half of it.
    #[inline]
    fn advance(&mut self, pos: &mut Position, m: Move) -> (board::chess::Undo, usize) {
        let mark = self.saved.len();
        self.saved.extend_from_slice(&self.acc.vals);
        let before = FeatSnap::of(pos);
        let u = pos.make_move(m);
        let after = FeatSnap::of(pos);
        before.delta(&after, &mut self.on, &mut self.off);
        self.acc.update(&self.net, &self.on, &self.off);
        (u, mark)
    }

    #[inline]
    fn undo(&mut self, pos: &mut Position, m: Move, u: board::chess::Undo, mark: usize) {
        pos.unmake_move(m, u);
        self.acc.vals.clear();
        self.acc.vals.extend_from_slice(&self.saved[mark..]);
        self.saved.truncate(mark);
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
        self.aborted = false;
        let list = pos.legal_moves();
        if list.is_empty() {
            return (MOVE_NONE, if pos.in_check(pos.stm) { -MATE } else { 0 });
        }
        let mut moves: Vec<Move> = list.as_slice().to_vec();
        self.shuffle(&mut moves);

        // The one full rebuild per search. Everything below is a delta off this.
        self.acc.refresh(&self.net, pos);
        self.saved.clear();

        let mut best = moves[0];
        let mut best_score = -INF;
        let mut alpha = -INF;
        for m in moves {
            let (u, mark) = self.advance(pos, m);
            let s = -self.alphabeta(pos, depth.saturating_sub(1), -INF, -alpha);
            self.undo(pos, m, u, mark);
            if self.aborted {
                // This move's score is incomplete. Keep the best COMPLETED one -- the root list is
                // shuffled, so the moves that did get searched are a fair sample rather than
                // whichever ones movegen happens to emit first.
                break;
            }
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
        // BEFORE counting, and the flag stays set on the way out. Checking only on the way down
        // is not enough: the unwind passes back through sibling loops that would each enter here
        // once more, so the search overshoots its cap by one node per open frame.
        if self.nodes >= self.node_cap {
            self.aborted = true;
            return 0;
        }
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
            return self.acc.score(&self.net, pos);
        }

        let mut moves: Vec<Move> = list.as_slice().to_vec();
        self.shuffle(&mut moves);

        let mut best = -INF;
        for m in moves {
            let (u, mark) = self.advance(pos, m);
            let s = -self.alphabeta(pos, depth - 1, -beta, -alpha);
            self.undo(pos, m, u, mark);
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

#[cfg(test)]
mod tests {
    //! The accumulator must not change a single score the engine returns.
    //!
    //! WHY THIS IS NOT COVERED BY `nnue/tests/incremental.rs`. That test proves `refresh() ==
    //! update()` and `FeatSnap::delta == refresh` -- the accumulator tracks the feature set
    //! correctly. It says nothing about the two substitutions made HERE: that `Acc::score` returns
    //! what `Net::eval` returns (a doc comment asserted this, and a doc comment is a hypothesis),
    //! and that maintaining the accumulator across make/unmake through a whole tree leaves every
    //! leaf holding the value a from-scratch eval would have produced.
    //!
    //! So the reference below is a SECOND alpha-beta, calling `Net::eval` at every leaf exactly as
    //! this file did before the accumulator landed.
    //!
    //! IT COMPARES SCORES, NOT MOVES, ON PURPOSE. `Search` shuffles its move list, so which of
    //! several equal-valued moves comes back depends on RNG state. The root VALUE does not:
    //! alpha-beta with a full window returns the exact minimax value regardless of move order, so a
    //! score mismatch is a real divergence and cannot be an ordering artefact.
    use super::*;

    fn reference(net: &Net, pos: &mut Position, depth: u32, mut alpha: Score, beta: Score,
                 scratch: &mut Vec<f32>) -> Score {
        let list = pos.legal_moves();
        if list.is_empty() {
            return match pos.outcome() { Outcome::Loss => -MATE + (64 - depth as Score), _ => 0 };
        }
        if depth == 0 {
            return net.eval(pos, scratch);
        }
        let mut best = -INF;
        for &m in list.as_slice() {
            let u = pos.make_move(m);
            let s = -reference(net, pos, depth - 1, -beta, -alpha, scratch);
            pos.unmake_move(m, u);
            if s > best { best = s; }
            if best > alpha { alpha = best; }
            if alpha >= beta { break; }
        }
        best
    }

    #[test]
    fn accumulator_search_scores_match_from_scratch_eval() {
        let net = Net::random(32, 0xC0FFEE);
        let fens = [
            None,
            Some("r3k2r/1P6/8/8/8/8/6p1/R3K2R w KQkq - 0 1"),
            Some("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1"),
            Some("8/8/8/3pP3/8/8/8/4K2k w - d6 0 1"),
            Some("r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5Q2/PPPP1PPP/RNB1K1NR w KQkq - 4 4"),
        ];
        let mut checked = 0usize;
        for (i, f) in fens.into_iter().enumerate() {
            let pos = match f {
                None => Position::startpos(),
                Some(s) => Position::from_fen(s).unwrap(),
            };
            for depth in 1..=3u32 {
                let mut scratch = Vec::new();
                let want = reference(&net, &mut pos.clone(), depth, -INF, INF, &mut scratch);
                // A fresh Search per probe: the accumulator must be right from a cold start, and a
                // different seed varies the move order it is maintained through.
                let mut s = Search::new(net.clone(), 0x1234 + i as u64 * 77 + depth as u64);
                let (_, got) = s.best_move(&mut pos.clone(), depth);
                assert_eq!(want, got,
                    "fen {i} depth {depth}: reference {want} vs accumulator {got}");
                checked += 1;
            }
        }
        assert!(checked >= 15, "only {checked} probes ran");
        println!("accumulator == from-scratch eval on {checked} (position, depth) probes");
    }
}
