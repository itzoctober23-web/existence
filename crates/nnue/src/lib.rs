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
    /// [N_INPUTS][n_hidden], INPUT-major. Stored this way on purpose: the feature vector is
    /// one-hot-ish (about 38 of 782 inputs are ever non-zero -- 32 pieces at most, plus side to
    /// move, castling and ep), so evaluation is a gather-add over the rows of the ACTIVE inputs
    /// rather than a dense 256x782 sweep. Same arithmetic, ~20x fewer operations, and each row
    /// is contiguous so it vectorises.
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
            w1: (0..N_INPUTS * n_hidden).map(|_| r.normal() * s1).collect(),
            b1: vec![0.0; n_hidden],
            w2: (0..n_hidden).map(|_| r.normal() * s2).collect(),
            b2: 0.0,
            scale: 600.0,
        }
    }

    /// Indices of the ACTIVE inputs. Absolute frame: no flipping, no mirroring, no
    /// king-relativity (MASTER_PLAN Given). Whether a mover-relative view helps is something
    /// architecture search may DISCOVER; it is not given.
    pub fn active(pos: &Position, out: &mut Vec<u16>) {
        out.clear();
        for c in [Color::White, Color::Black] {
            for k in PieceKind::ALL {
                let base = ((c.idx() * N_PIECE_KINDS + k.idx()) * 64) as u16;
                for sq in bb::squares(pos.pieces[c.idx()][k.idx()]) {
                    out.push(base + sq as u16);
                }
            }
        }
        if pos.stm == Color::White {
            out.push(IDX_STM as u16);
        }
        for i in 0..4 {
            if pos.castling & (1 << i) != 0 {
                out.push((IDX_CASTLE + i) as u16);
            }
        }
        if let Some(ep) = pos.ep {
            out.push(IDX_EP as u16 + ep.file() as u16);
        }
    }

    /// Dense reference version, kept ONLY so tests can prove the sparse path agrees with it.
    pub fn features_dense(pos: &Position, out: &mut [f32]) {
        out.fill(0.0);
        let mut idx = Vec::new();
        Self::active(pos, &mut idx);
        for i in idx {
            out[i as usize] = 1.0;
        }
    }

    /// Evaluate from WHITE's point of view, then flip for the mover. The flip is a convention
    /// of the search interface (negamax wants mover-relative), not a claim about the position.
    pub fn eval(&self, pos: &Position, scratch: &mut Vec<f32>) -> Score {
        let mut idx: Vec<u16> = Vec::with_capacity(40);
        Self::active(pos, &mut idx);
        scratch.clear();
        scratch.extend_from_slice(&self.b1);
        for i in idx {
            let row = &self.w1[i as usize * self.n_hidden..(i as usize + 1) * self.n_hidden];
            for (a, w) in scratch.iter_mut().zip(row.iter()) {
                *a += *w;
            }
        }
        let mut acc = 0.0f32;
        for h in 0..self.n_hidden {
            let s = scratch[h];
            if s > 0.0 {
                acc += s * self.w2[h]; // ReLU
            }
        }
        let white_pov = (acc + self.b2) * self.scale;
        let v = if pos.stm == Color::White { white_pov } else { -white_pov };
        v.clamp(-30_000.0, 30_000.0) as Score
    }
}

/// Incremental accumulator. `eval` rebuilds the hidden sums from ~38 active feature rows at
/// every node; a move changes at most 4 features (from, to, a capture, a castled rook), so the
/// same value is reachable with 2-4 row updates instead of ~38. That is the difference between
/// search depth and datagen volume being a trade-off and not being one.
///
/// Correctness is not assumed: `tests/incremental.rs` asserts refresh() == update() over every
/// legal move of thousands of positions from random games.
#[derive(Clone)]
pub struct Acc {
    pub vals: Vec<f32>,
}

impl Acc {
    pub fn new(net: &Net) -> Self {
        Acc { vals: net.b1.clone() }
    }
    /// Full rebuild from a position.
    pub fn refresh(&mut self, net: &Net, pos: &Position) {
        self.vals.clear();
        self.vals.extend_from_slice(&net.b1);
        let mut idx = Vec::with_capacity(40);
        Net::active(pos, &mut idx);
        for i in idx {
            self.add(net, i, 1.0);
        }
    }
    #[inline]
    fn add(&mut self, net: &Net, feature: u16, sign: f32) {
        let h = net.n_hidden;
        let row = &net.w1[feature as usize * h..(feature as usize + 1) * h];
        if sign > 0.0 {
            for (a, w) in self.vals.iter_mut().zip(row) { *a += *w; }
        } else {
            for (a, w) in self.vals.iter_mut().zip(row) { *a -= *w; }
        }
    }
    /// Apply a feature-set delta: features that turned on and off.
    pub fn update(&mut self, net: &Net, on: &[u16], off: &[u16]) {
        for &f in off { self.add(net, f, -1.0); }
        for &f in on { self.add(net, f, 1.0); }
    }
    /// Output head, given the accumulated hidden sums. White-POV, like Net::eval's internals.
    pub fn output(&self, net: &Net) -> f32 {
        let mut acc = 0.0f32;
        for h in 0..net.n_hidden {
            let s = self.vals[h];
            if s > 0.0 { acc += s * net.w2[h]; }
        }
        acc + net.b2
    }
    /// Mover-relative Score, matching Net::eval exactly.
    pub fn score(&self, net: &Net, pos: &Position) -> Score {
        let white = self.output(net) * net.scale;
        let v = if pos.stm == Color::White { white } else { -white };
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
