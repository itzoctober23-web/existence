//! Net-vs-net match. This is the only thing that decides whether a trained net is better; the
//! training loss is not evidence (MASTER_PLAN: SPRT decides, surrogate proposes).
//!
//! Colours ALTERNATE and each opening is played from BOTH sides, so a first-move advantage or
//! a lucky opening cannot show up as strength.

use board::{Outcome, Position};
use grammar::Program;
use interp::Interp;
use nnue::Net;

use crate::datagen::Rng;
use crate::search::Searcher;

#[derive(Default, Debug, Clone, Copy)]
pub struct Score {
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    /// Pentanomial pair counts, indexed by the PAIR's total score in half-points:
    /// 0 = LL, 1 = LD/DL, 2 = LW/DD/WL, 3 = DW/WD, 4 = WW.
    ///
    /// FITNESS 7.3 requires pentanomial UNCONDITIONALLY from the first gate. The gate already
    /// PLAYED pairs (each opening from both sides) but SCORED them as independent games, so
    /// the pairing -- the entire point -- was discarded and every interval was computed from a
    /// binomial that assumes independence the design deliberately removed.
    pub pent: [u32; 5],
}

impl Score {
    pub fn points(&self) -> f64 {
        self.wins as f64 + 0.5 * self.draws as f64
    }
    pub fn games(&self) -> u32 {
        self.wins + self.draws + self.losses
    }
    pub fn rate(&self) -> f64 {
        self.points() / self.games().max(1) as f64
    }
    /// 95% CI half-width on the score rate, from the PAIR distribution. Pairing cancels
    /// opening bias, so the variance is the spread of pair outcomes rather than a binomial on
    /// individual games.
    pub fn ci95(&self) -> f64 {
        let n: u32 = self.pent.iter().sum();
        if n < 2 {
            // no pairs recorded: fall back to the binomial rather than report a fake interval
            let g = self.games().max(1) as f64;
            let p = self.rate();
            return 1.96 * (p * (1.0 - p) / g).sqrt();
        }
        let n = n as f64;
        let mean: f64 = self.pent.iter().enumerate()
            .map(|(i, c)| i as f64 * 0.5 * *c as f64).sum::<f64>() / n;
        let var: f64 = self.pent.iter().enumerate()
            .map(|(i, c)| { let d = i as f64 * 0.5 - mean; d * d * *c as f64 }).sum::<f64>()
            / (n - 1.0);
        if var <= 0.0 {
            // ZERO OBSERVED VARIANCE IS NOT ZERO UNCERTAINTY. When every pair lands in the same
            // bucket -- which is what a match between two near-random nets looks like, 24 games
            // all drawn -- the sample variance is 0 and this used to report "0.500 +/- 0.000":
            // a perfectly measured dead heat, from a match that measured nothing at all. Any
            // caller testing `ci95 < 0.05` for "can this gate resolve?" was told YES by the
            // least informative result the gate can produce.
            //
            // Rule of three: having seen zero non-drawn pairs in n, the 95% upper bound on the
            // per-pair rate of a different outcome is ~3/n, and one such pair moves the rate by
            // at most half a point. So the interval is 1.5/n, and it correctly says "no idea"
            // at small n instead of "certain".
            return 1.5 / n;
        }
        // pair score is out of 2; halve to put the interval on the per-game rate scale
        1.96 * (var / n).sqrt() / 2.0
    }

    /// Score rate from the pair distribution when available.
    pub fn pent_rate(&self) -> f64 {
        let n: u32 = self.pent.iter().sum();
        if n == 0 { return self.rate(); }
        let mean: f64 = self.pent.iter().enumerate()
            .map(|(i, c)| i as f64 * 0.5 * *c as f64).sum::<f64>() / n as f64;
        mean / 2.0
    }
}

/// Play `pairs` opening positions, each twice with colours swapped. Returns A's score.
pub fn match_nets(a: &Net, b: &Net, depth: u32, pairs: usize, seed: u64) -> Score {
    match_nets_open(a, b, depth, pairs, seed, 4)
}

/// `open_plies` controls how far the shared random opening walks before the two nets take
/// over. Deeper walks create material imbalance, which is what makes games decisive and the
/// gate able to resolve anything at all.
pub fn match_nets_open(a: &Net, b: &Net, depth: u32, pairs: usize, seed: u64, open_plies: usize) -> Score {
    let mut sc = Score::default();
    let mut rng = Rng(seed | 1);
    for _ in 0..pairs {
        let pair_seed = rng.next();
        // one random opening, played from both sides
        let mut opening = Position::startpos();
        for _ in 0..open_plies {
            let l = opening.legal_moves();
            if l.is_empty() { break; }
            opening.make_move(l.as_slice()[rng.below(l.len())]);
        }
        // Play the SAME opening from both sides and score the PAIR, not the two games.
        let mut pair_half = 0usize;
        for a_is_white in [true, false] {
            // SAME shuffle seed for both halves of the pair, so the pairing cancels child-order
            // luck the way it cancels opening bias. A fresh seed per game would put the two
            // halves on different move orders and reintroduce the variance pairing removes.
            let r = play(a, b, a_is_white, &opening, depth, pair_seed);
            match r {
                Some(true) => { sc.wins += 1; pair_half += 2; }
                Some(false) => { sc.losses += 1; }
                None => { sc.draws += 1; pair_half += 1; }
            }
        }
        sc.pent[pair_half.min(4)] += 1;
    }
    sc
}

/// PROGRAM vs PROGRAM on the board, same net, same budget. THE SEARCH TRACK'S GAME GATE.
///
/// MASTER_PLAN:276 makes the gate the arbiter for evolved programs -- "the gate is what would
/// confirm a winner on the clock" -- and until now the search track had none. It promoted on
/// mates-per-cost alone, and that surrogate has been exploited TWICE by programs that were
/// strictly worse at chess: one searched a ply shallower, one raised the initial alpha to +8 and
/// pruned every move worth under 8 centipawns. Both kept every mate on the set while being
/// unplayable, because the set contains no position where the answer is worth tens of
/// centipawns. Guards were added for both, and a guard only ever blocks the exploit it was
/// written for -- the third one will be found the same way, after the fact.
///
/// Games cannot be gamed in that way. A program that prunes real moves loses to one that does
/// not, and no property of the position set can hide it.
///
/// Both sides get the SAME net and the SAME budget, so this measures the PROGRAM and nothing
/// else -- the search-track analogue of the equal-cost gate the net track uses.
pub fn match_progs(
    a: &Program, b: &Program, net: &Net, tables: Vec<i64>, budget: i64, pairs: usize, seed: u64,
    open_plies: usize, cost_per_move: u64,
) -> Score {
    let mut sc = Score::default();
    let mut rng = Rng(seed | 1);
    for p in 0..pairs {
        // Same opening played from both sides, scored as a PAIR -- identical protocol to the net
        // gate, so the two tracks' numbers mean the same thing and pentanomial pairing cancels
        // opening luck rather than being discarded.
        let mut opening = Position::startpos();
        for _ in 0..open_plies {
            let l = opening.legal_moves();
            if l.is_empty() { break; }
            opening.make_move(l.as_slice()[rng.below(l.len())]);
        }
        let mut pair_half = 0usize;
        for a_is_white in [true, false] {
            let r = play_progs(a, b, net, &tables, budget, a_is_white, &opening,
                               seed ^ (p as u64) << 16, cost_per_move);
            match r {
                Some(true) => { sc.wins += 1; pair_half += 2; }
                Some(false) => { sc.losses += 1; }
                None => { sc.draws += 1; pair_half += 1; }
            }
        }
        sc.pent[pair_half.min(4)] += 1;
    }
    sc
}

fn play_progs(
    a: &Program, b: &Program, net: &Net, tables: &[i64], budget: i64, a_is_white: bool,
    start: &Position, _seed: u64, cost_per_move: u64,
) -> Option<bool> {
    let mut pos = start.clone();
    // One interpreter per side, reused across the game. `run` clears the hash table itself, so
    // reuse carries no state between moves and costs one allocation instead of 200.
    // EQUAL COST PER MOVE, not equal budget -- and this is the whole point of the gate.
    //
    // Both sides used to get the same `budget` and no cost ceiling, so a program that was CHEAPER
    // per move got no credit for it: it simply did less work and returned sooner. The surrogate
    // meanwhile scores mates-per-COST. The two metrics were therefore measuring different things,
    // and a candidate could max the surrogate while the games registered nothing.
    //
    // MEASURED, which is what exposed it: the MCTS lineage drove its surrogate from 0.001145 to
    // 0.005034 -- 4.4x -- while five consecutive game gates returned EXACTLY 0.500 +/- 0.250. The
    // candidate was much cheaper and played the identical games, because being cheap bought it
    // nothing at a fixed budget.
    //
    // A cost ceiling makes efficiency convertible into strength: a program that costs half as much
    // per node searches twice as much before the ceiling, which is precisely the claim the
    // surrogate is making on its behalf and which the gate exists to check. Same principle as the
    // equal-TIME net gate and the per-lineage MCTS budget.
    let mut ia = Interp::new(net, tables.to_vec());
    let mut ib = Interp::new(net, tables.to_vec());
    ia.cost_cap = cost_per_move;
    ib.cost_cap = cost_per_move;
    for _ in 0..200 {
        let l = pos.legal_moves();
        if l.is_empty() {
            return match pos.outcome() {
                Outcome::Loss => {
                    let white_won = pos.stm == board::Color::Black;
                    Some(white_won == a_is_white)
                }
                _ => None,
            };
        }
        if pos.halfmove >= 100 { return None; }
        let is_a = (pos.stm == board::Color::White) == a_is_white;
        let prog = if is_a { a } else { b };
        let it = if is_a { &mut ia } else { &mut ib };
        let m = it.run(prog, &pos, budget);
        // A program that returns no move FORFEITS rather than drawing. An evolved program can
        // legitimately fail to answer -- that is a defect in the program, and scoring it as a
        // draw would let a candidate that stops choosing moves gate as "equal".
        if m == board::types::MOVE_NONE {
            return Some(!is_a);
        }
        // And it must return a LEGAL move. Trusting the interpreter here would let a malformed
        // candidate corrupt the position rather than lose the game.
        match l.as_slice().iter().copied().find(|x| *x == m) {
            Some(mv) => { pos.make_move(mv); }
            None => return Some(!is_a),
        }
    }
    None
}

/// Equal-COST match: each side gets its own node budget per move. Pass equal budgets for the
/// fixed-cost-budget gate (FITNESS 6, "is it better per unit of search?"); pass budgets from
/// `arch::equal_time_caps` for the clock (FITNESS 7, "is it better per unit of TIME?").
///
/// A depth-matched gate answers NEITHER question when the two nets differ in width: it hands
/// the more expensive net exactly as much computation as the cheap one and charges it nothing
/// for the difference, so it would wave through every widening step on the menu.
pub fn match_nets_capped(
    a: &Net, b: &Net, depth: u32, cap_a: u64, cap_b: u64, pairs: usize, seed: u64,
    open_plies: usize,
) -> Score {
    let mut sc = Score::default();
    let mut rng = Rng(seed | 1);
    for p in 0..pairs {
        let mut opening = Position::startpos();
        for _ in 0..open_plies {
            let l = opening.legal_moves();
            if l.is_empty() { break; }
            opening.make_move(l.as_slice()[rng.below(l.len())]);
        }
        let mut pair_half = 0usize;
        for a_is_white in [true, false] {
            let r = play_capped(a, b, a_is_white, &opening, depth, cap_a, cap_b,
                                seed ^ (p as u64) << 16 ^ a_is_white as u64);
            match r {
                Some(true) => { sc.wins += 1; pair_half += 2; }
                Some(false) => { sc.losses += 1; }
                None => { sc.draws += 1; pair_half += 1; }
            }
        }
        sc.pent[pair_half.min(4)] += 1;
    }
    sc
}

fn play_capped(
    a: &Net, b: &Net, a_is_white: bool, start: &Position, depth: u32, cap_a: u64, cap_b: u64,
    seed: u64,
) -> Option<bool> {
    let mut pos = start.clone();
    let mut s = Searcher::with_seed(seed);
    for ply in 0..200u64 {
        let l = pos.legal_moves();
        if l.is_empty() {
            return match pos.outcome() {
                Outcome::Loss => {
                    let white_won = pos.stm == board::Color::Black;
                    Some(white_won == a_is_white)
                }
                _ => None,
            };
        }
        if pos.halfmove >= 100 { return None; }
        let white_to_move = pos.stm == board::Color::White;
        let is_a = white_to_move == a_is_white;
        let (net, cap) = if is_a { (a, cap_a) } else { (b, cap_b) };
        let (m, _) = s.best_move_capped(&mut pos, depth, net, cap, seed ^ ply.wrapping_mul(0x9E37));
        if m == board::types::MOVE_NONE { return None; }
        pos.make_move(m);
    }
    None
}

/// Returns Some(true) if A won, Some(false) if B won, None for a draw.
fn play(a: &Net, b: &Net, a_is_white: bool, start: &Position, depth: u32, shuffle_seed: u64)
    -> Option<bool>
{
    let mut pos = start.clone();
    let mut s = Searcher::with_seed(shuffle_seed);
    for _ in 0..200 {
        let l = pos.legal_moves();
        if l.is_empty() {
            return match pos.outcome() {
                // side to move is mated -> the OTHER side won
                Outcome::Loss => {
                    let white_won = pos.stm == board::Color::Black;
                    Some(white_won == a_is_white)
                }
                _ => None,
            };
        }
        if pos.halfmove >= 100 {
            return None;
        }
        let white_to_move = pos.stm == board::Color::White;
        let net = if white_to_move == a_is_white { a } else { b };
        let (m, _) = s.best_move(&mut pos, depth, net);
        if m == board::types::MOVE_NONE { return None; }
        pos.make_move(m);
    }
    None
}

/// SEQUENTIAL testing. FITNESS 7.2: "a candidate near a bound gets thousands of pairs, an
/// obvious dud a few hundred; nobody picks the count, the evidence does."
///
/// Every gate here was FIXED-COUNT, which is wrong in both directions at once. At 12 pairs the
/// pentanomial interval is about +/-0.14, so `rate - ci95 > 0.5` demands a ~64% score before it
/// will accept anything — real but modest gains are invisible. Meanwhile an obviously broken
/// candidate costs exactly as many games as a promising one. Sequential testing fixes both:
/// it spends games only where the evidence is still ambiguous.
///
/// Bounds: alpha = beta = 0.05, so the log-likelihood-ratio thresholds are
/// ln((1-beta)/alpha) = +2.944 to accept and ln(beta/(1-alpha)) = -2.944 to reject. Those error
/// rates are FIXED and human-declared (FITNESS 7.2 part 1) — an instrument calibrated by its
/// subject measures nothing.
pub const LLR_BOUND: f64 = 2.944;

/// Expected score for an Elo difference.
pub fn elo_to_score(elo: f64) -> f64 {
    1.0 / (1.0 + 10f64.powf(-elo / 400.0))
}

impl Score {
    /// Generalised SPRT log-likelihood ratio on the PAIR distribution.
    ///
    /// Normal approximation on the pair-score mean, which is what fishtest's pentanomial GSPRT
    /// uses: with the pair outcome as the observation, the mean and variance are estimated from
    /// the five buckets directly, so the pairing that cancels opening bias is preserved in the
    /// statistic rather than thrown away by re-scoring games independently.
    pub fn llr(&self, elo0: f64, elo1: f64) -> f64 {
        let n: u32 = self.pent.iter().sum();
        if n < 2 { return 0.0; }
        let n = n as f64;
        // pair score on 0..1
        let mean: f64 = self.pent.iter().enumerate()
            .map(|(i, c)| (i as f64 / 4.0) * *c as f64).sum::<f64>() / n;
        let var: f64 = self.pent.iter().enumerate()
            .map(|(i, c)| { let d = i as f64 / 4.0 - mean; d * d * *c as f64 }).sum::<f64>()
            / (n - 1.0);
        if var <= 0.0 {
            // Zero observed variance: every pair identical. Fall back to the rule-of-three
            // logic used by ci95 rather than dividing by zero and reporting infinite evidence.
            return 0.0;
        }
        let p0 = elo_to_score(elo0);
        let p1 = elo_to_score(elo1);
        (n / (2.0 * var)) * ((mean - p0).powi(2) - (mean - p1).powi(2))
    }
}

/// Verdict of a sequential test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sprt { Accept, Reject, Inconclusive }

/// Play pairs until the LLR crosses a bound or `max_pairs` is spent, checking every `chunk`
/// pairs. Returns the verdict, the accumulated score, and the final LLR.
///
/// `max_pairs` is a CAP, not a target: a run that hits it is INCONCLUSIVE and must be recorded
/// as such. An unrecorded tie is usually a capped run, and treating it as a rejection quietly
/// discards candidates the evidence never actually ruled out.
pub fn sprt_match_capped(
    a: &Net, b: &Net, depth: u32, cap_a: u64, cap_b: u64, seed: u64, open_plies: usize,
    elo0: f64, elo1: f64, chunk: usize, max_pairs: usize,
) -> (Sprt, Score, f64) {
    let mut total = Score::default();
    let mut played = 0usize;
    while played < max_pairs {
        let n = chunk.min(max_pairs - played);
        let s = match_nets_capped(a, b, depth, cap_a, cap_b, n, seed ^ (played as u64) << 8, open_plies);
        total.wins += s.wins; total.draws += s.draws; total.losses += s.losses;
        for i in 0..5 { total.pent[i] += s.pent[i]; }
        played += n;
        let llr = total.llr(elo0, elo1);
        if llr >= LLR_BOUND { return (Sprt::Accept, total, llr); }
        if llr <= -LLR_BOUND { return (Sprt::Reject, total, llr); }
    }
    let llr = total.llr(elo0, elo1);
    (Sprt::Inconclusive, total, llr)
}

/// Sequential net-vs-net at fixed DEPTH (the main learning loop's gate), as opposed to
/// `sprt_match_capped` which is node-budgeted for cross-architecture comparisons.
/// SPRT at a FIXED COST BUDGET -- the gate FITNESS 6 actually specifies.
///
/// FITNESS line 22 lists the NET class as "fixed-cost-budget gate, then STC, LTC", line 82 says
/// "all fixed budgets in this file are counted in COST UNITS", and section 6 is titled
/// "Fixed-cost-budget gate". The NET gate has been running `sprt_match_nets`, which is
/// DEPTH-matched -- while `match_nets_capped` (equal cost, right below) was already implemented
/// and already used by the ARCH arm and the origin control. The correct gate existed and the
/// primary decision path did not call it.
///
/// Why it matters even though a NET step keeps the architecture fixed, so both sides cost the
/// same per node: a fixed DEPTH makes the comparison horizon-sensitive. At depth 2 -- where every
/// measurement today was taken -- the horizon dominates, and a net is charged nothing for how
/// many nodes it needed to get there. A fixed node budget lets each side reach whatever depth its
/// own pruning earns, which is the paradigm-neutral comparison the spec asks for.
pub fn sprt_match_nets_capped(
    a: &Net, b: &Net, depth_cap: u32, cap_a: u64, cap_b: u64, max_pairs: usize, seed: u64,
    chunk: usize, elo0: f64, elo1: f64,
) -> (Sprt, Score, f64) {
    let mut total = Score::default();
    let mut played = 0usize;
    while played < max_pairs {
        let n = chunk.min(max_pairs - played);
        let s = match_nets_capped(a, b, depth_cap, cap_a, cap_b, n, seed ^ (played as u64) << 8, 4);
        total.wins += s.wins; total.draws += s.draws; total.losses += s.losses;
        for i in 0..5 { total.pent[i] += s.pent[i]; }
        played += n;
        let llr = total.llr(elo0, elo1);
        if llr >= LLR_BOUND { return (Sprt::Accept, total, llr); }
        if llr <= -LLR_BOUND { return (Sprt::Reject, total, llr); }
    }
    (Sprt::Inconclusive, total, total.llr(elo0, elo1))
}

pub fn sprt_match_nets(
    a: &Net, b: &Net, depth: u32, max_pairs: usize, seed: u64, chunk: usize,
    elo0: f64, elo1: f64,
) -> (Sprt, Score, f64) {
    let mut total = Score::default();
    let mut played = 0usize;
    while played < max_pairs {
        let n = chunk.min(max_pairs - played);
        let s = match_nets(a, b, depth, n, seed ^ (played as u64) << 8);
        total.wins += s.wins; total.draws += s.draws; total.losses += s.losses;
        for i in 0..5 { total.pent[i] += s.pent[i]; }
        played += n;
        let llr = total.llr(elo0, elo1);
        if llr >= LLR_BOUND { return (Sprt::Accept, total, llr); }
        if llr <= -LLR_BOUND { return (Sprt::Reject, total, llr); }
    }
    let llr = total.llr(elo0, elo1);
    (Sprt::Inconclusive, total, llr)
}
