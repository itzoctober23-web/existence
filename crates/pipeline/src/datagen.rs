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

/// Play many games across threads.
///
/// WHY THIS EXISTS: the loop was running 80 games per generation and then agonising over the
/// 1.1% of positions that survive the decided-game and horizon filters — 449,380 positions
/// generated across a 40-generation run, 4,942 actually trained on, and one generation that
/// trained on THREE samples. That is not a filter problem, it is a sample-size problem. At
/// iteration zero the mover is close to random and a game is nearly free, so the answer is
/// more games by two orders of magnitude, not a looser filter whose tightness was justified by
/// an actual measurement.
///
/// DETERMINISM SURVIVES THE THREADING, which is the only hard part. Each worker draws from its
/// own stream seeded from (seed, worker index), and the per-worker results are concatenated in
/// WORKER ORDER, never in completion order. So the output is byte-identical regardless of how
/// the OS scheduled the threads, and a run still replays exactly from its seed — which FITNESS
/// 10's determinism check requires and which a `Mutex<Vec<Sample>>` collecting in completion
/// order would have quietly destroyed.
pub fn play_games(
    net: &Net,
    depth: u32,
    seed: u64,
    games: usize,
    open_plies: usize,
    max_plies: usize,
    threads: usize,
) -> (Vec<Sample>, usize) {
    let threads = threads.max(1).min(games.max(1));
    let per = games / threads;
    let extra = games % threads;

    let chunks: Vec<(usize, u64)> = (0..threads)
        .map(|t| (per + if t < extra { 1 } else { 0 }, seed ^ ((t as u64 + 1).wrapping_mul(0x9E3779B97F4A7C15))))
        .collect();

    let results: Vec<(Vec<Sample>, usize)> = std::thread::scope(|s| {
        let handles: Vec<_> = chunks
            .iter()
            .map(|&(n, sd)| {
                s.spawn(move || {
                    let mut rng = Rng(sd | 1);
                    let mut out = Vec::new();
                    let mut decisive = 0usize;
                    for _ in 0..n {
                        if play_game(net, depth, &mut rng, open_plies, max_plies, &mut out)
                            == Outcome::Loss
                        {
                            decisive += 1;
                        }
                    }
                    (out, decisive)
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().expect("datagen worker panicked")).collect()
    });

    let mut all = Vec::new();
    let mut decisive = 0;
    for (v, d) in results {
        all.extend(v);
        decisive += d;
    }
    (all, decisive)
}

/// HOW a game ended. "Draw" is three different failures wearing one label, and only one of
/// them is a real draw: a stalemate is chess, a 50-move expiry is two players shuffling, and
/// hitting the ply cap is the harness giving up. Generation 37 of the ARCH run produced 0
/// decisive games out of 80 and the loop had no way to say which of the three it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEnd {
    Mate,
    Stalemate,
    FiftyMove,
    PlyCap,
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
    play_game_ext(net, depth, rng, open_plies, max_plies, out).0
}

/// As `play_game`, but also reports HOW the game ended.
pub fn play_game_ext(
    net: &Net,
    depth: u32,
    rng: &mut Rng,
    open_plies: usize,
    max_plies: usize,
    out: &mut Vec<Sample>,
) -> (Outcome, GameEnd) {
    let mut pos = Position::startpos();
    // Seed the shuffle from the caller's stream, so every game explores a different child
    // order instead of every game replaying the same one, and the whole run still replays
    // exactly from its top-level seed.
    let mut s = Searcher::with_seed(rng.next());
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
    let mut how = GameEnd::PlyCap;
    for ply in 0..max_plies {
        let l = pos.legal_moves();
        if l.is_empty() {
            result = pos.outcome();
            how = if result == Outcome::Loss { GameEnd::Mate } else { GameEnd::Stalemate };
            break;
        }
        if pos.halfmove >= 100 {
            result = Outcome::Draw;
            how = GameEnd::FiftyMove;
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
    (result, how)
}
