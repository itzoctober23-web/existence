//! Zobrist keys. Engineering, not a chess opinion — GRAMMAR 2.1 #4 lists `key` as memoisation
//! with no content. It exists so `probe`/`store` are usable at all: the first cut hashed a
//! rendered FEN string, which costs a format and an allocation per node and would have made
//! every hash-using program look slow for reasons that have nothing to do with the program.

use crate::types::{Color, N_PIECE_KINDS, PieceKind};
use crate::{Position, bitboard as bb};

pub struct Keys {
    pub piece: [[[u64; 64]; N_PIECE_KINDS]; 2],
    pub side: u64,
    pub castle: [u64; 16],
    pub ep_file: [u64; 8],
}

const fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E3779B97F4A7C15);
    let mut x = z;
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
    x ^ (x >> 31)
}

pub static KEYS: Keys = build();

const fn build() -> Keys {
    let mut k = Keys {
        piece: [[[0; 64]; N_PIECE_KINDS]; 2],
        side: splitmix(1),
        castle: [0; 16],
        ep_file: [0; 8],
    };
    let mut n: u64 = 2;
    let mut c = 0;
    while c < 2 {
        let mut p = 0;
        while p < N_PIECE_KINDS {
            let mut s = 0;
            while s < 64 {
                k.piece[c][p][s] = splitmix(n);
                n += 1;
                s += 1;
            }
            p += 1;
        }
        c += 1;
    }
    let mut i = 0;
    while i < 16 {
        k.castle[i] = splitmix(n);
        n += 1;
        i += 1;
    }
    let mut i = 0;
    while i < 8 {
        k.ep_file[i] = splitmix(n);
        n += 1;
        i += 1;
    }
    k
}

impl Position {
    /// Full key from scratch: ~32 XORs. Incremental update in make/unmake is the obvious next
    /// step, and the test that would guard it is `incremental == from_scratch` at every node.
    pub fn zobrist(&self) -> u64 {
        let mut h = 0u64;
        for c in [Color::White, Color::Black] {
            for k in PieceKind::ALL {
                for sq in bb::squares(self.pieces[c.idx()][k.idx()]) {
                    h ^= KEYS.piece[c.idx()][k.idx()][sq as usize];
                }
            }
        }
        if self.stm == Color::Black {
            h ^= KEYS.side;
        }
        h ^= KEYS.castle[(self.castling & 15) as usize];
        if let Some(ep) = self.ep {
            h ^= KEYS.ep_file[ep.file() as usize];
        }
        h
    }
}
