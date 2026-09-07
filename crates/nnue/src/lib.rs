//! Evaluation network skeleton.
//!
//! The ARCHITECTURE IS DATA (CRATE.md 5): layer sizes and the input spec come from the net
//! file header, and this crate builds itself from them. Nothing here encodes a chess opinion —
//! there is no piece value, no mobility term, no king-safety term. At iteration zero the
//! weights are random, so the eval is noise, exactly as MASTER_PLAN "Iteration zero" states.
//!
//! Effective piece values are a THING TO READ OUT LATER by ablation, never a thing to write in.

use board::types::{Color, N_PIECE_KINDS, PieceKind};
use board::{Position, bitboard as bb};

/// Input layout, derived from the rules and nothing else:
///   12 x 64  one binary plane per (piece kind, side) per square
///        1   side to move
///        4   castling rights
///        8   en-passant file (one-hot, all zero when there is none)
pub const N_PIECE_PLANES: usize = 2 * N_PIECE_KINDS * 64;
pub const IDX_STM: usize = N_PIECE_PLANES;
pub const IDX_CASTLE: usize = IDX_STM + 1;
pub const IDX_EP: usize = IDX_CASTLE + 4;
pub const N_INPUTS: usize = IDX_EP + 8;

/// A score in centipawn-like units. The UNIT is arbitrary and learned; nothing here fixes a
/// scale, and mapping a terminal Outcome to a score is `score_of`'s job, not this crate's.
pub type Score = i32;

#[derive(Clone)]
pub struct Net {
    pub n_hidden: usize,
    /// [n_hidden][N_INPUTS], row-major.
    pub w1: Vec<f32>,
    pub b1: Vec<f32>,
    pub w2: Vec<f32>,
    pub b2: f32,
    /// Multiplier from the network's natural units into `Score`. Declared, not learned-yet.
    pub scale: f32,
}

impl Net {
    /// Deterministic pseudo-random init. Iteration zero has no knowledge in it; the seed only
    /// makes runs reproducible (MASTER_PLAN Safeguards: everything reproducible from a seed).
    pub fn random(n_hidden: usize, seed: u64) -> Self {
        let mut r = Rng(seed | 1);
        // He-style scaling keeps the pre-activations in a sane range at init; this is generic
        // initialisation practice, not a chess choice.
        let s1 = (2.0f32 / N_INPUTS as f32).sqrt();
        let s2 = (2.0f32 / n_hidden as f32).sqrt();
        Net {
            n_hidden,
            w1: (0..n_hidden * N_INPUTS).map(|_| r.normal() * s1).collect(),
            b1: vec![0.0; n_hidden],
            w2: (0..n_hidden).map(|_| r.normal() * s2).collect(),
            b2: 0.0,
            scale: 600.0,
        }
    }

    /// Write the position's input vector. Absolute frame: no flipping, no mirroring, no
    /// king-relativity (MASTER_PLAN Given). Whether a mover-relative view helps is something
    /// architecture search may DISCOVER; it is not given.
    pub fn features(pos: &Position, out: &mut [f32]) {
        out.fill(0.0);
        for c in [Color::White, Color::Black] {
            for k in PieceKind::ALL {
                let base = (c.idx() * N_PIECE_KINDS + k.idx()) * 64;
                for sq in bb::squares(pos.pieces[c.idx()][k.idx()]) {
                    out[base + sq as usize] = 1.0;
                }
            }
        }
        out[IDX_STM] = if pos.stm == Color::White { 1.0 } else { 0.0 };
        for i in 0..4 {
            if pos.castling & (1 << i) != 0 {
                out[IDX_CASTLE + i] = 1.0;
            }
        }
        if let Some(ep) = pos.ep {
            out[IDX_EP + ep.file() as usize] = 1.0;
        }
    }

    /// Evaluate from WHITE's point of view, then flip for the mover. The flip is a convention
    /// of the search interface (negamax wants mover-relative), not a claim about the position.
    pub fn eval(&self, pos: &Position, scratch: &mut Vec<f32>) -> Score {
        scratch.resize(N_INPUTS, 0.0);
        Self::features(pos, scratch);
        let mut acc = 0.0f32;
        for h in 0..self.n_hidden {
            let row = &self.w1[h * N_INPUTS..(h + 1) * N_INPUTS];
            let mut s = self.b1[h];
            for (w, x) in row.iter().zip(scratch.iter()) {
                if *x != 0.0 {
                    s += w * x;
                }
            }
            if s > 0.0 {
                acc += s * self.w2[h]; // ReLU
            }
        }
        let white_pov = (acc + self.b2) * self.scale;
        let v = if pos.stm == Color::White { white_pov } else { -white_pov };
        v.clamp(-30_000.0, 30_000.0) as Score
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    /// Irwin–Hall approximation of a standard normal; adequate for initialisation.
    fn normal(&mut self) -> f32 {
        let mut s = 0.0f32;
        for _ in 0..6 {
            s += (self.next() >> 11) as f32 / (1u64 << 53) as f32;
        }
        (s - 3.0) * 0.7071
    }
}
