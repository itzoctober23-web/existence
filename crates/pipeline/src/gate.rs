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
    /// 95% CI half-width on the score rate (normal approximation).
    pub fn ci95(&self) -> f64 {
        let n = self.games().max(1) as f64;
        let p = self.rate();
        1.96 * (p * (1.0 - p) / n).sqrt()
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
        // one random opening, played from both sides
        let mut opening = Position::startpos();
        for _ in 0..open_plies {
            let l = opening.legal_moves();
            if l.is_empty() { break; }
            opening.make_move(l.as_slice()[rng.below(l.len())]);
        }
        for a_is_white in [true, false] {
            let r = play(a, b, a_is_white, &opening, depth);
            match r {
                Some(true) => sc.wins += 1,
                Some(false) => sc.losses += 1,
                None => sc.draws += 1,
            }
        }
    }
    sc
}

/// Returns Some(true) if A won, Some(false) if B won, None for a draw.
fn play(a: &Net, b: &Net, a_is_white: bool, start: &Position, depth: u32) -> Option<bool> {
    let mut pos = start.clone();
    let mut s = Searcher::new();
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
