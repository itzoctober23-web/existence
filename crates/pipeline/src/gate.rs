//! Net-vs-net match. This is the only thing that decides whether a trained net is better; the
//! training loss is not evidence (MASTER_PLAN: SPRT decides, surrogate proposes).
//!
//! Colours ALTERNATE and each opening is played from BOTH sides, so a first-move advantage or
//! a lucky opening cannot show up as strength.

use board::{Outcome, Position};
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
