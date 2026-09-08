//! Self-play position generation.
//!
//! Objectives are self-referential (MASTER_PLAN Given): the label is the OUTCOME of a game the
//! engine played against itself, plus its own search score. No external judgement, no human
//! games. Openings are randomised because self-play from one start position collapses the
//! state distribution — the plan's own "Openings and draw death" section allows random plies
//! as the early source.

use board::{Move, Outcome, Position};
use nnue::Net;

use crate::search::Searcher;

#[derive(Clone)]
pub struct Sample {
    pub fen: String,
    /// Plies from this position to the end of the game. Rules-derived. In self-play by a
    /// near-random engine the OUTCOME is nearly independent of a position 40 plies earlier --
    /// the players are noise, so the result is not yet determined by the position. Distance to
    /// terminal is how that is testable.
    pub plies_to_end: u32,
    /// Game result from WHITE's point of view: +1 white won, -1 black won, 0 drawn.
    pub z: f32,
    /// The engine's own root search score at this position, mover-relative.
    pub root: i32,
}

pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// Play one self-play game, returning every recorded position labelled with the final result.
pub fn play_game(
    net: &Net,
    depth: u32,
    rng: &mut Rng,
    open_plies: usize,
    max_plies: usize,
    out: &mut Vec<Sample>,
) -> Outcome {
    let mut pos = Position::startpos();
    let mut s = Searcher::new();
    let start = out.len();

    // Random opening plies, unrecorded: coverage the net's own preferences would never reach.
    for _ in 0..open_plies {
        let l = pos.legal_moves();
        if l.is_empty() {
            break;
        }
        let m = l.as_slice()[rng.below(l.len())];
        pos.make_move(m);
    }

    let mut result = Outcome::Draw;
    for ply in 0..max_plies {
        let l = pos.legal_moves();
        if l.is_empty() {
            result = pos.outcome();
            break;
        }
        if pos.halfmove >= 100 {
            result = Outcome::Draw;
            break;
        }
        let (mv, score) = s.best_move(&mut pos, depth, net);
        if mv == board::types::MOVE_NONE {
            break;
        }
        out.push(Sample { fen: pos.to_fen(), z: 0.0, root: score, plies_to_end: 0 });
        // Temperature early: pick a random legal move occasionally so games diverge.
        let m: Move = if ply < 6 && rng.next() % 4 == 0 {
            l.as_slice()[rng.below(l.len())]
        } else {
            mv
        };
        pos.make_move(m);
    }

    // Label every position in THIS game with the final result, from white's point of view.
    let z = match result {
        Outcome::Loss => {
            // side to move at the terminal lost
            if pos.stm == board::Color::White { -1.0 } else { 1.0 }
        }
        _ => 0.0,
    };
    let n = out.len() - start;
    for (i, sample) in out[start..].iter_mut().enumerate() {
        sample.z = z;
        sample.plies_to_end = (n - 1 - i) as u32;
    }
    result
}
