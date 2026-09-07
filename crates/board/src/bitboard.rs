//! 64-bit board sets for 8x8. `Rules::Bitboard` generalises this; the 14x14 board will need a
//! 256-bit type with the corners masked (CRATE.md 2), which is why the rest of the crate talks
//! to bitboards through these helpers rather than open-coding u64 tricks everywhere.

pub type Bb = u64;

pub const EMPTY: Bb = 0;
pub const FULL: Bb = !0;

pub const FILE_A: Bb = 0x0101_0101_0101_0101;
pub const FILE_H: Bb = FILE_A << 7;
pub const RANK_1: Bb = 0xFF;
pub const RANK_2: Bb = RANK_1 << 8;
pub const RANK_4: Bb = RANK_1 << 24;
pub const RANK_5: Bb = RANK_1 << 32;
pub const RANK_7: Bb = RANK_1 << 48;
pub const RANK_8: Bb = RANK_1 << 56;

#[inline]
pub const fn bit(sq: u8) -> Bb {
    1u64 << sq
}

/// Index of the least significant set bit. Caller guarantees `b != 0`.
#[inline]
pub const fn lsb(b: Bb) -> u8 {
    b.trailing_zeros() as u8
}

/// Index of the most significant set bit. Caller guarantees `b != 0`.
#[inline]
pub const fn msb(b: Bb) -> u8 {
    63 - b.leading_zeros() as u8
}

/// Pop the least significant bit, returning its index.
#[inline]
pub fn pop_lsb(b: &mut Bb) -> u8 {
    let s = lsb(*b);
    *b &= *b - 1;
    s
}

#[inline]
pub const fn count(b: Bb) -> u32 {
    b.count_ones()
}

/// One-square shifts with wrap masking. Directions are from White's point of view; the mover's
/// perspective is applied by the caller, never baked in here.
#[inline]
pub const fn north(b: Bb) -> Bb {
    b << 8
}
#[inline]
pub const fn south(b: Bb) -> Bb {
    b >> 8
}
#[inline]
pub const fn east(b: Bb) -> Bb {
    (b & !FILE_H) << 1
}
#[inline]
pub const fn west(b: Bb) -> Bb {
    (b & !FILE_A) >> 1
}
#[inline]
pub const fn north_east(b: Bb) -> Bb {
    (b & !FILE_H) << 9
}
#[inline]
pub const fn north_west(b: Bb) -> Bb {
    (b & !FILE_A) << 7
}
#[inline]
pub const fn south_east(b: Bb) -> Bb {
    (b & !FILE_H) >> 7
}
#[inline]
pub const fn south_west(b: Bb) -> Bb {
    (b & !FILE_A) >> 9
}

/// Iterate set squares, least significant first.
pub struct Squares(pub Bb);

impl Iterator for Squares {
    type Item = u8;
    #[inline]
    fn next(&mut self) -> Option<u8> {
        if self.0 == 0 {
            None
        } else {
            Some(pop_lsb(&mut self.0))
        }
    }
}

#[inline]
pub fn squares(b: Bb) -> Squares {
    Squares(b)
}

/// Render a bitboard as 8 lines, a8 top-left. Debugging only.
pub fn render(b: Bb) -> String {
    let mut s = String::with_capacity(72);
    for rank in (0..8).rev() {
        for file in 0..8 {
            s.push(if b & bit(rank * 8 + file) != 0 { '1' } else { '.' });
        }
        s.push('\n');
    }
    s
}
