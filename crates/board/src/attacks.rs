//! Attack tables, built at compile time so there is no runtime initialisation and no lazy
//! statics on the hot path.
//!
//! Sliding attacks use the classical ray method: precomputed rays per (square, direction),
//! masked by the first blocker found with a bit scan. Magic bitboards are faster and are the
//! obvious later optimisation, but they are a big table plus a verification burden, and P0's
//! job is a movegen that is provably correct against perft. Speed work comes after
//! `xtask bench-interp`, which is the measurement that decides whether the interpreter design
//! survives at all (CRATE.md 11c).

use crate::bitboard::{Bb, bit, lsb, msb};

/// Ray directions, indexed 0..8. Positive-shift directions are 0..4 so a bit scan can pick
/// `lsb` for them and `msb` for the rest.
pub const NORTH: usize = 0;
pub const NORTH_EAST: usize = 1;
pub const EAST: usize = 2;
pub const SOUTH_EAST: usize = 3;
pub const SOUTH: usize = 4;
pub const SOUTH_WEST: usize = 5;
pub const WEST: usize = 6;
pub const NORTH_WEST: usize = 7;

const DIRS: [(i8, i8); 8] = [
    (0, 1),   // N
    (1, 1),   // NE
    (1, 0),   // E
    (1, -1),  // SE
    (0, -1),  // S
    (-1, -1), // SW
    (-1, 0),  // W
    (-1, 1),  // NW
];

/// RAYS[dir][sq]: every square from `sq` outward in `dir`, exclusive of `sq`.
pub static RAYS: [[Bb; 64]; 8] = build_rays();

const fn build_rays() -> [[Bb; 64]; 8] {
    let mut rays = [[0u64; 64]; 8];
    let mut d = 0;
    while d < 8 {
        let (df, dr) = DIRS[d];
        let mut sq = 0usize;
        while sq < 64 {
            let mut f = (sq % 8) as i8;
            let mut r = (sq / 8) as i8;
            let mut b = 0u64;
            loop {
                f += df;
                r += dr;
                if f < 0 || f > 7 || r < 0 || r > 7 {
                    break;
                }
                b |= 1u64 << (r * 8 + f);
            }
            rays[d][sq] = b;
            sq += 1;
        }
        d += 1;
    }
    rays
}

pub static KNIGHT: [Bb; 64] = build_step(&[(1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2)]);
pub static KING: [Bb; 64] = build_step(&[(0, 1), (1, 1), (1, 0), (1, -1), (0, -1), (-1, -1), (-1, 0), (-1, 1)]);

const fn build_step(offsets: &[(i8, i8)]) -> [Bb; 64] {
    let mut t = [0u64; 64];
    let mut sq = 0usize;
    while sq < 64 {
        let f = (sq % 8) as i8;
        let r = (sq / 8) as i8;
        let mut i = 0;
        let mut b = 0u64;
        while i < offsets.len() {
            let (df, dr) = offsets[i];
            let nf = f + df;
            let nr = r + dr;
            if nf >= 0 && nf <= 7 && nr >= 0 && nr <= 7 {
                b |= 1u64 << (nr * 8 + nf);
            }
            i += 1;
        }
        t[sq] = b;
        sq += 1;
    }
    t
}

/// PAWN[color][sq]: the two diagonal capture targets. White = 0 moves toward rank 8.
pub static PAWN: [[Bb; 64]; 2] = build_pawn();

const fn build_pawn() -> [[Bb; 64]; 2] {
    let mut t = [[0u64; 64]; 2];
    let mut sq = 0usize;
    while sq < 64 {
        let f = (sq % 8) as i8;
        let r = (sq / 8) as i8;
        let mut c = 0usize;
        while c < 2 {
            let dr: i8 = if c == 0 { 1 } else { -1 };
            let mut b = 0u64;
            let nr = r + dr;
            if nr >= 0 && nr <= 7 {
                if f - 1 >= 0 {
                    b |= 1u64 << (nr * 8 + f - 1);
                }
                if f + 1 <= 7 {
                    b |= 1u64 << (nr * 8 + f + 1);
                }
            }
            t[c][sq] = b;
            c += 1;
        }
        sq += 1;
    }
    t
}

/// Does this direction move toward HIGHER square indices? Derived from the shift each
/// direction implies, not from its position in the list: N=+8, NE=+9, E=+1, SE=-7, S=-8,
/// SW=-9, W=-1, NW=+7. Getting this wrong lets sliders pass through the first blocker in
/// exactly two directions (SE and NW) — perft d2 caught it as 414 instead of 400.
const POSITIVE: [bool; 8] = [true, true, true, false, false, false, false, true];

/// Ray attacks in one direction, stopping at (and including) the first blocker.
#[inline]
fn ray(dir: usize, sq: u8, occ: Bb) -> Bb {
    let full = RAYS[dir][sq as usize];
    let blockers = full & occ;
    if blockers == 0 {
        return full;
    }
    // Nearest blocker is the lsb going up-board, the msb going down-board.
    let first = if POSITIVE[dir] { lsb(blockers) } else { msb(blockers) };
    full & !RAYS[dir][first as usize]
}

#[inline]
pub fn bishop(sq: u8, occ: Bb) -> Bb {
    ray(NORTH_EAST, sq, occ) | ray(SOUTH_EAST, sq, occ) | ray(SOUTH_WEST, sq, occ) | ray(NORTH_WEST, sq, occ)
}

#[inline]
pub fn rook(sq: u8, occ: Bb) -> Bb {
    ray(NORTH, sq, occ) | ray(EAST, sq, occ) | ray(SOUTH, sq, occ) | ray(WEST, sq, occ)
}

#[inline]
pub fn queen(sq: u8, occ: Bb) -> Bb {
    bishop(sq, occ) | rook(sq, occ)
}

#[inline]
pub fn knight(sq: u8) -> Bb {
    KNIGHT[sq as usize]
}

#[inline]
pub fn king(sq: u8) -> Bb {
    KING[sq as usize]
}

#[inline]
pub fn pawn(color: usize, sq: u8) -> Bb {
    PAWN[color][sq as usize]
}

/// Squares strictly between two squares along a shared line; 0 if they do not share one.
/// Used by the legality filter to find blocking squares when in check.
pub static BETWEEN: [[Bb; 64]; 64] = build_between();

const fn build_between() -> [[Bb; 64]; 64] {
    let mut t = [[0u64; 64]; 64];
    let rays = build_rays();
    let mut a = 0usize;
    while a < 64 {
        let mut d = 0usize;
        while d < 8 {
            let r = rays[d][a];
            let mut bsq = 0usize;
            while bsq < 64 {
                if r & (1u64 << bsq) != 0 {
                    // Everything on the ray from a, minus everything from b onward.
                    t[a][bsq] = r & !rays[d][bsq] & !(1u64 << bsq);
                }
                bsq += 1;
            }
            d += 1;
        }
        a += 1;
    }
    t
}

/// The full line through two squares (both endpoints included), 0 if not aligned.
/// Used to test whether a pinned piece is staying on its pin ray.
pub static LINE: [[Bb; 64]; 64] = build_line();

const fn build_line() -> [[Bb; 64]; 64] {
    let mut t = [[0u64; 64]; 64];
    let rays = build_rays();
    let mut a = 0usize;
    while a < 64 {
        let mut d = 0usize;
        while d < 8 {
            let opp = (d + 4) % 8;
            let r = rays[d][a];
            let mut bsq = 0usize;
            while bsq < 64 {
                if r & (1u64 << bsq) != 0 {
                    t[a][bsq] = rays[d][a] | rays[opp][a] | (1u64 << a);
                }
                bsq += 1;
            }
            d += 1;
        }
        a += 1;
    }
    t
}

#[inline]
pub fn between(a: u8, b: u8) -> Bb {
    BETWEEN[a as usize][b as usize]
}

#[inline]
pub fn line(a: u8, b: u8) -> Bb {
    LINE[a as usize][b as usize]
}

#[inline]
pub fn aligned(a: u8, b: u8, c: u8) -> bool {
    line(a, b) & bit(c) != 0
}
