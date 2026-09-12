//! The seed search, reused by datagen and the gate. Same semantics as engine/src/search.rs:
//! bare alpha-beta, nothing layered on it.
use board::types::{MOVE_NONE, Move, Outcome};
use board::Position;
use nnue::{Acc, Net, Score};

pub const MATE: Score = 30_000;
pub const INF: Score = 32_000;

pub struct Searcher {
    pub nodes: u64,
    /// Hard ceiling on nodes for this search; `u64::MAX` means unlimited. This is the "iterate
    /// to budget" primitive from the Given grammar row, and it is what makes an EQUAL-COST gate
    /// possible: two nets of different width can be given the same amount of work rather than
    /// the same depth. Without it, a wider net is handed strictly more computation and every
    /// architecture comparison is rigged in favour of the bigger net.
    pub node_cap: u64,
    /// Set when the cap was hit. The root move being searched at that moment has an incomplete
    /// score and is DISCARDED, not compared.
    pub aborted: bool,
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
    /// Shuffle state. MASTER_PLAN "Iteration zero" declares children in emission order
    /// (SHUFFLED), and `crates/engine/src/search.rs` -- the binary that actually ships -- does
    /// shuffle, at its root and inside alphabeta.
    ///
    /// This file did not, while its own header claimed "Same semantics as
    /// engine/src/search.rs". That was two bugs at once. Unshuffled, the search inherits the
    /// MOVEGEN'S emission order, which is by piece type -- gen_pawns runs first -- so alpha-beta
    /// was being handed a free "try pawn moves first" heuristic that nobody declared and nobody
    /// earned. Move ordering is worth a large fraction of alpha-beta's strength; that is exactly
    /// why the plan puts it on the DISCOVERY list and shuffles the seed to deny it.
    /// And because datagen and every gate run THIS searcher while the engine ships the other
    /// one, every label and every verdict was produced by a different search from the one under
    /// test.
    rng: u64,
    /// Per-depth scratch for the shuffled child list, so shuffling costs no allocation after
    /// the first visit to each depth.
    order: Vec<Vec<Move>>,
    /// A/B switch, kept so the SIZE of the ordering prior stays measurable rather than being
    /// a claim in a comment. Setting it false reproduces the old behaviour (movegen emission
    /// order), and examples/ordering_prior.rs reports the node-count difference.
    pub shuffle_children: bool,
    /// Which delta implementation push_move uses. Set per-search from the environment so the two
    /// are A/B-comparable on identical trees rather than one replacing the other on an argument.
    xor_delta: bool,
    /// Explicit path override for tests. `best_move` re-reads the environment on EVERY call, so a
    /// test cannot pin the path by setting a field, and setting process env vars from a test is racy
    /// under cargo's default thread-parallel harness. When this is `Some`, it wins over the env.
    override_paths: Option<(bool, bool)>,
}

impl Searcher {
    pub fn new() -> Self {
        Searcher {
            nodes: 0,
            node_cap: u64::MAX,
            aborted: false,
            // Default chosen by MEASUREMENT, not assumption. Node counts are identical across
            // every path at every width (same tree), so these ratios are real work-for-work.
            //
            // The ORIGINAL finding, which gated incremental behind `n_hidden >= 64`:
            //   hidden  32   refresh 1718969   incr 1560497   0.91x  LOSS
            //   hidden 128   refresh  889406   incr 1014240   1.14x  win
            //   hidden 512   refresh  240701   incr  363458   1.51x  win
            // Diagnosis in that comment was right: the ROW ADDS scale with width, the bookkeeping
            // does not, so at small widths the fixed cost wins. The conclusion drawn from it -- that
            // small widths must use refresh -- was wrong, because the bookkeeping was not a constant
            // of nature. It was two active() scans, four passes over a 13-word bitset and two
            // ~38-element Vec builds, i.e. O(features ACTIVE).
            //
            // 2026-09-10: replaced by FeatSnap::delta, a per-plane bitboard XOR that is O(features
            // CHANGED) -- typically 2-6. Re-measured on a contended box, 3 reps, node counts equal:
            //   hidden   8   refresh  967k   old-delta    --     XOR 1188k   1.23x
            //   hidden  16   refresh  906k   old-delta    --     XOR 1125k   1.24x
            //   hidden  32   refresh  747k   old-delta  674k     XOR  956k   1.28x   (was 0.90x)
            //   hidden 128   refresh  403k   old-delta  442k     XOR  561k   1.39x
            //   hidden 512   refresh  122k   old-delta    --     XOR  185k   1.52x
            // The control reproduces the original ratios (0.90x at 32, 1.10x at 128), so the harness
            // is the same one. THERE IS NO LONGER A CROSSOVER: the XOR delta wins at every width
            // measured, so the `>= 64` gate is gone and the default width (32) now benefits.
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
            rng: 0x9E3779B97F4A7C15,
            order: Vec::new(),
            shuffle_children: true,
            xor_delta: false, // set per-search by best_move
            override_paths: None,
        }
    }

    /// Pin the eval path explicitly, overriding the environment. Tests only.
    pub fn set_paths(&mut self, incremental: bool, xor_delta: bool) {
        self.override_paths = Some((incremental, xor_delta));
    }

    /// Fix the shuffle stream. Determinism is a gate requirement (FITNESS 10, "stochastic
    /// program that passes by luck -> determinism check; seeds fixed per game"), so the shuffle
    /// is seeded per game rather than drawn from the clock.
    pub fn with_seed(seed: u64) -> Self {
        let mut s = Self::new();
        s.rng = seed | 1;
        s
    }

    #[inline]
    fn rand(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }

    /// Range reduction WITHOUT a division: Lemire's multiply-shift.
    ///
    /// `rand() % n` is a 64-bit integer division, and this runs ~30 times per node -- once per
    /// child, at every node. `node_profile` measures the shuffle at 213.8 ns/node against 83.2 ns
    /// for the identical shuffle done modulo-free, so **130.6 ns of a ~1256 ns node is the division
    /// alone**: 10.4% of all search time, spent on an operation with a two-instruction equivalent.
    ///
    /// The multiply-shift maps a uniform u64 into [0, n) by taking the high half of a 128-bit
    /// product. It is not *exactly* uniform -- the bias is at most n/2^64, which for a move list of
    /// at most a few hundred entries is order 1e-17 and cannot be observed in any number of games
    /// this project will ever play -- and it remains fully DETERMINISTIC, which is the property the
    /// gate actually requires (FITNESS 10: "stochastic program that passes by luck -> determinism
    /// check; seeds fixed per game").
    ///
    /// The shuffle itself stays: `pipeline/src/search.rs` shuffles children deliberately, to deny
    /// alpha-beta a move ordering the search is supposed to DISCOVER. This changes only how the
    /// index is computed.
    #[inline]
    fn below(&mut self, n: u64) -> u64 {
        ((self.rand() as u128 * n as u128) >> 64) as u64
    }

    #[inline]
    fn shuffle(&mut self, v: &mut [Move]) {
        if !self.shuffle_children { return; }
        for i in (1..v.len()).rev() {
            let j = self.below(i as u64 + 1) as usize;
            v.swap(i, j);
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

        // ---- ARM B: O(changed) delta from per-plane bitboard XOR ---------------------------------
        // The doc comment above says computing the two active sets is cheap. The measured crossover
        // at hidden 64 is evidence that at small widths it is NOT: the ROW ADDS scale with width, but
        // two active() scans, four passes over a 13-word bitset and two ~38-element Vec builds are a
        // FIXED per-node cost the saving must overcome before it shows a win.
        //
        // FeatSnap::delta reads the same fields active_with reads and diffs them as bitboards, so it
        // is O(features CHANGED) -- typically 2-6 -- rather than O(features ACTIVE). Behaviour is
        // identical by construction and asserted by nnue/tests/incremental.rs.
        if self.xor_delta {
            let before = nnue::FeatSnap::of(pos);
            let u = pos.make_move(m);
            let after = nnue::FeatSnap::of(pos);

            let base = self.ply * h;
            if self.stack.len() < base + h {
                self.stack.resize(base + h, 0.0);
            }
            self.stack[base..base + h].copy_from_slice(&self.acc.vals);
            self.ply += 1;

            before.delta(&after, &mut self.on, &mut self.off);
            let (on, off) = (std::mem::take(&mut self.on), std::mem::take(&mut self.off));
            self.acc.update(net, &on, &off);
            self.on = on;
            self.off = off;
            return u;
        }

        // ---- ARM A: the original two-active-sets diff, kept as the control -----------------------
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
        match self.override_paths {
            Some((inc, xor)) => { self.incremental = inc; self.xor_delta = xor; }
            None => {
                self.incremental = std::env::var("EXISTENCE_FULL_REFRESH").is_err();
                self.xor_delta = std::env::var("EXISTENCE_OLD_DELTA").is_err();
            }
        }
        self.acc.refresh(net, pos);
        self.ply = 0;
        let list = pos.legal_moves();
        if list.is_empty() {
            return (MOVE_NONE, if pos.in_check(pos.stm) { -MATE } else { 0 });
        }
        let mut moves: Vec<Move> = list.as_slice().to_vec();
        self.shuffle(&mut moves);
        let (mut best, mut best_s, mut alpha) = (moves[0], -INF, -INF);
        for &m in &moves {
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

    /// Root search under a NODE BUDGET. Root moves are searched in a shuffled order and the
    /// loop stops when the budget runs out, keeping the best move found so far.
    ///
    /// Why the shuffle: truncating a fixed-order list would systematically favour whichever
    /// moves the generator happens to emit first, so a net that ran out of budget would be
    /// judged on move 1..k rather than on a fair sample. Shuffling is also what MASTER_PLAN
    /// "Iteration zero" declares for root order anyway ("children in emission order (shuffled)").
    ///
    /// Determinism is preserved: the budget is in NODES, not milliseconds, and the shuffle
    /// takes a seed. The same seed replays the same game (FITNESS 10, determinism check).
    pub fn best_move_capped(
        &mut self, pos: &mut Position, depth: u32, net: &Net, node_cap: u64, seed: u64,
    ) -> (Move, Score) {
        match self.override_paths {
            Some((inc, xor)) => { self.incremental = inc; self.xor_delta = xor; }
            None => {
                self.incremental = std::env::var("EXISTENCE_FULL_REFRESH").is_err();
                self.xor_delta = std::env::var("EXISTENCE_OLD_DELTA").is_err();
            }
        }
        self.acc.refresh(net, pos);
        self.ply = 0;
        self.nodes = 0;
        self.node_cap = node_cap;
        self.aborted = false;
        let list = pos.legal_moves();
        if list.is_empty() {
            return (MOVE_NONE, if pos.in_check(pos.stm) { -MATE } else { 0 });
        }
        let mut moves: Vec<Move> = list.as_slice().to_vec();
        let mut r = crate::datagen::Rng(seed | 1);
        for i in (1..moves.len()).rev() {
            moves.swap(i, r.below(i + 1));
        }
        let (mut best, mut best_s, mut alpha) = (moves[0], -INF, -INF);
        for &m in &moves {
            let u = self.push_move(pos, m, net);
            let s = -self.ab(pos, depth.saturating_sub(1), -INF, -alpha, net);
            self.pop_move(pos, m, u, net);
            if self.aborted {
                break; // this move's score is incomplete; keep the best COMPLETED one
            }
            if s > best_s {
                best_s = s;
                best = m;
                if s > alpha { alpha = s; }
            }
        }
        self.node_cap = u64::MAX;
        (best, best_s)
    }

    /// Iterative deepening to a NODE BUDGET. Returns `(move, score, realised_depth)`.
    ///
    /// WHY THIS IS A SEPARATE FUNCTION AND NOT A CHANGE TO `best_move_capped`.
    /// `node_cap`'s own doc comment above calls it "the iterate to budget primitive", but nothing
    /// iterates: `best_move_capped` runs ONE fixed-depth search and ABORTS when the cap is hit,
    /// keeping whatever root moves happened to finish. So handing it a deep `depth` and a budget
    /// does not spend the budget wisely -- it blows the cap inside the first root move and returns
    /// a result derived from one or two shuffled moves. `control.rs:12` already records exactly
    /// that failure mode.
    ///
    /// `best_move_capped` has ~20 call sites, including `gate.rs:453` (the EQUAL-COST architecture
    /// gate) and the `arch` tests, which depend on its current abort semantics. Changing it would
    /// silently redefine that gate, so it is left untouched.
    ///
    /// WHAT THIS DOES INSTEAD. Search depth 1, then 2, then 3 ... and keep the result of the last
    /// depth that COMPLETED INSIDE the budget. A depth that aborts is discarded whole -- its score
    /// is partial by construction, and comparing a partial score against a complete one is the bug
    /// `datagen.rs:189` guards against. Depth 1 is always run before the budget is consulted, so
    /// this can never return `-INF`: there is always a completed answer to fall back on.
    ///
    /// This is the primitive `structural_next_PREREG.md` Candidate A needs. Fixed depth spends
    /// equal effort on unequal positions; `datagen_node_census_RESULT.md` measures that inequality
    /// at 18-25x (p90/p10) with CV ~0.80. A budget reallocates that effort.
    pub fn best_move_budget(
        &mut self, pos: &mut Position, net: &Net, budget: u64, max_depth: u32, seed: u64,
    ) -> (Move, Score, u32) {
        match self.override_paths {
            Some((inc, xor)) => { self.incremental = inc; self.xor_delta = xor; }
            None => {
                self.incremental = std::env::var("EXISTENCE_FULL_REFRESH").is_err();
                self.xor_delta = std::env::var("EXISTENCE_OLD_DELTA").is_err();
            }
        }
        self.acc.refresh(net, pos);
        self.ply = 0;
        self.nodes = 0;
        self.node_cap = budget;
        self.aborted = false;

        let list = pos.legal_moves();
        if list.is_empty() {
            self.node_cap = u64::MAX;
            return (MOVE_NONE, if pos.in_check(pos.stm) { -MATE } else { 0 }, 0);
        }
        let mut moves: Vec<Move> = list.as_slice().to_vec();
        let mut r = crate::datagen::Rng(seed | 1);
        for i in (1..moves.len()).rev() {
            moves.swap(i, r.below(i + 1));
        }

        let (mut best, mut best_s, mut realised) = (moves[0], -INF, 0u32);
        for d in 1..=max_depth.max(1) {
            // DEPTH 1 RUNS UNCAPPED, ON PURPOSE. An earlier version consulted the budget from the
            // first iteration and a test caught it returning -INF at budget=1: depth 1 aborted
            // part-way, was discarded as incomplete, and nothing was left to fall back on. That
            // -INF becomes `tanh(-32000/600) = -1.0`, a confidently-lost label on a position
            // nobody evaluated. `datagen.rs:189-194` already resolves the same problem the same
            // way ("falling back to a depth-1 search, which always completes, costs a few hundred
            // nodes and keeps the label honest"), so this matches existing practice rather than
            // inventing a second convention. The overspend is bounded by one depth-1 search.
            self.node_cap = if d == 1 { u64::MAX } else { budget };
            let (mut ib, mut ibs, mut alpha) = (moves[0], -INF, -INF);
            for &m in &moves {
                let u = self.push_move(pos, m, net);
                let s = -self.ab(pos, d.saturating_sub(1), -INF, -alpha, net);
                self.pop_move(pos, m, u, net);
                if self.aborted { break; }
                if s > ibs { ibs = s; ib = m; if s > alpha { alpha = s; } }
            }
            if self.aborted {
                break; // this depth is incomplete -- discard it entirely, keep depth d-1
            }
            best = ib;
            best_s = ibs;
            realised = d;
            // Cheapest possible early exit: the next iteration costs strictly more than this one,
            // so if the budget is already gone there is nothing to gain by entering it.
            if self.nodes >= budget { break; }
        }
        self.node_cap = u64::MAX;
        self.aborted = false;
        (best, best_s, realised)
    }

    fn ab(&mut self, pos: &mut Position, depth: u32, mut alpha: Score, beta: Score, net: &Net) -> Score {
        // Check BEFORE counting, and again before every child. Setting the flag on the way down
        // is not enough: the unwind passes back through sibling loops that would each call ab()
        // once more, so the search overshot its cap by one node per open frame (33 over a 1000
        // cap at depth 4). A budget that is only approximately a budget makes an "equal cost"
        // gate approximately fair, which is the same as not fair.
        if self.aborted { return 0; }
        if self.nodes >= self.node_cap {
            self.aborted = true;
            return 0; // discarded by the root; never compared against a real score
        }
        self.nodes += 1;
        // A LEAF NEVER READS THE MOVE LIST, so do not build one.
        //
        // This used to call `legal_moves()` unconditionally and, at depth 0, use the result for
        // exactly one thing: `is_empty()`, to tell mate and stalemate from an ordinary position.
        // The list was then dropped. `node_profile` prices that discarded work at **393.4 ns of a
        // 657.9 ns leaf (59.8%)** -- against eval's 223.7 (34.0%) -- and leaves are ~66% of a
        // depth-4 frontier, so it was the largest single piece of waste in the engine.
        //
        // `has_legal_move()` answers the only question asked, and answers it from work the
        // generator does first anyway (checkers, then the danger map). Semantics are UNCHANGED by
        // construction -- it delegates to `legal_moves()` in every case it cannot settle -- so node
        // counts must be byte-identical before and after. That identity is the correctness proof
        // and it is what makes the speed ratio honest: `throughput_RESULT.md` records a change here
        // that raised nps while LOSING wall-clock, because the two arms did different work.
        if depth == 0 {
            if !pos.has_legal_move() {
                return match pos.outcome() {
                    Outcome::Loss => -MATE + (64 - depth as Score),
                    _ => 0,
                };
            }
            // A/B switch: the incremental path only pays once the hidden width makes the ~38
            // saved row-adds outweigh its fixed bookkeeping (two active() scans, the bitset
            // diff, the memcpy). Measured, not assumed -- see examples/search_bench.rs.
            return if self.incremental {
                self.acc.score(net, pos)
            } else {
                net.eval(pos, &mut self.scratch)
            };
        }
        let list = pos.legal_moves();
        if list.is_empty() {
            return match pos.outcome() {
                Outcome::Loss => -MATE + (64 - depth as Score),
                _ => 0,
            };
        }
        // Shuffle the children, as the seed program declares. Reuses a per-depth buffer, so
        // this costs no allocation after the first visit to each depth.
        let d = depth as usize;
        while self.order.len() <= d { self.order.push(Vec::new()); }
        let mut buf = std::mem::take(&mut self.order[d]);
        buf.clear();
        buf.extend_from_slice(list.as_slice());
        self.shuffle(&mut buf);

        let mut best = -INF;
        for i in 0..buf.len() {
            let m = buf[i];
            let u = self.push_move(pos, m, net);
            let s = -self.ab(pos, depth - 1, -beta, -alpha, net);
            self.pop_move(pos, m, u, net);
            if self.aborted { self.order[d] = buf; return best.max(-INF); }
            if s > best { best = s; }
            if best > alpha { alpha = best; }
            if alpha >= beta { break; }
        }
        self.order[d] = buf;
        best
    }
}
impl Default for Searcher { fn default() -> Self { Self::new() } }
