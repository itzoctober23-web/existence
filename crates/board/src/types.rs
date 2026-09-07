//! Squares, pieces, colours, moves. Rules vocabulary only — nothing here ranks or values
//! anything, per CRATE.md 2: if a function would differ for a game with different RULES it
//! belongs in this crate; if it would differ for a game with different STRATEGY it does not
//! exist here at all.

/// Colour. For 8x8 there are two; `Rules::PLAYERS` generalises the count.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Color {
    White = 0,
    Black = 1,
}

impl Color {
    #[inline]
    pub const fn flip(self) -> Self {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
    #[inline]
    pub const fn idx(self) -> usize {
        self as usize
    }
}

/// Piece kind. Order is fixed because it indexes the piece bitboards and the input planes
/// (SCHEMAS.md 1: one binary plane per piece type per side).
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum PieceKind {
    Pawn = 0,
    Knight = 1,
    Bishop = 2,
    Rook = 3,
    Queen = 4,
    King = 5,
}

pub const N_PIECE_KINDS: usize = 6;

impl PieceKind {
    #[inline]
    pub const fn idx(self) -> usize {
        self as usize
    }
    pub const ALL: [PieceKind; N_PIECE_KINDS] = [
        PieceKind::Pawn,
        PieceKind::Knight,
        PieceKind::Bishop,
        PieceKind::Rook,
        PieceKind::Queen,
        PieceKind::King,
    ];
    /// Letter used by FEN/algebraic rendering. Rendering only; carries no judgement.
    pub const fn ch(self) -> char {
        match self {
            PieceKind::Pawn => 'p',
            PieceKind::Knight => 'n',
            PieceKind::Bishop => 'b',
            PieceKind::Rook => 'r',
            PieceKind::Queen => 'q',
            PieceKind::King => 'k',
        }
    }
}

/// A square index, 0..64 for 8x8. `Square(0)` is a1, `Square(63)` is h8 — rank-major so that
/// `sq >> 3` is the rank and `sq & 7` the file.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct Square(pub u8);

impl Square {
    #[inline]
    pub const fn new(file: u8, rank: u8) -> Self {
        Square(rank * 8 + file)
    }
    #[inline]
    pub const fn file(self) -> u8 {
        self.0 & 7
    }
    #[inline]
    pub const fn rank(self) -> u8 {
        self.0 >> 3
    }
    #[inline]
    pub const fn idx(self) -> usize {
        self.0 as usize
    }
    #[inline]
    pub const fn bb(self) -> u64 {
        1u64 << self.0
    }
    /// Parse "e4". Returns None on anything malformed.
    pub fn from_str(s: &str) -> Option<Self> {
        let b = s.as_bytes();
        if b.len() != 2 {
            return None;
        }
        let f = b[0].wrapping_sub(b'a');
        let r = b[1].wrapping_sub(b'1');
        if f < 8 && r < 8 { Some(Square::new(f, r)) } else { None }
    }
}

impl core::fmt::Display for Square {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}{}",
            (b'a' + self.file()) as char,
            (b'1' + self.rank()) as char
        )
    }
}

/// Move flags. Castling is encoded as king-moves-two, matching how the rules describe it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum MoveFlag {
    Quiet = 0,
    DoublePush = 1,
    Capture = 2,
    EnPassant = 3,
    CastleKing = 4,
    CastleQueen = 5,
    Promo = 6,
    PromoCapture = 7,
}

/// A move, packed into 16 bits: from (6) | to (6) | flag (3) | promo-kind (2 of 3 used).
/// Packed because the search stores millions of these; the layout carries no chess opinion.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Move(pub u32);

pub const MOVE_NONE: Move = Move(0);

impl Move {
    #[inline]
    pub const fn new(from: Square, to: Square, flag: MoveFlag) -> Self {
        Move((from.0 as u32) | ((to.0 as u32) << 6) | ((flag as u32) << 12))
    }
    #[inline]
    pub const fn new_promo(from: Square, to: Square, flag: MoveFlag, kind: PieceKind) -> Self {
        Move(
            (from.0 as u32)
                | ((to.0 as u32) << 6)
                | ((flag as u32) << 12)
                | (((kind as u32) - 1) << 15),
        )
    }
    #[inline]
    pub const fn from(self) -> Square {
        Square((self.0 & 63) as u8)
    }
    #[inline]
    pub const fn to(self) -> Square {
        Square(((self.0 >> 6) & 63) as u8)
    }
    #[inline]
    pub const fn flag(self) -> MoveFlag {
        // SAFETY-free: the 3-bit field is always written from a MoveFlag.
        match (self.0 >> 12) & 7 {
            0 => MoveFlag::Quiet,
            1 => MoveFlag::DoublePush,
            2 => MoveFlag::Capture,
            3 => MoveFlag::EnPassant,
            4 => MoveFlag::CastleKing,
            5 => MoveFlag::CastleQueen,
            6 => MoveFlag::Promo,
            _ => MoveFlag::PromoCapture,
        }
    }
    /// Promotion target, valid only when the flag is Promo/PromoCapture.
    #[inline]
    pub const fn promo(self) -> PieceKind {
        match (self.0 >> 15) & 3 {
            0 => PieceKind::Knight,
            1 => PieceKind::Bishop,
            2 => PieceKind::Rook,
            _ => PieceKind::Queen,
        }
    }
    #[inline]
    pub const fn is_promo(self) -> bool {
        matches!(self.flag(), MoveFlag::Promo | MoveFlag::PromoCapture)
    }
    #[inline]
    pub const fn is_capture(self) -> bool {
        matches!(
            self.flag(),
            MoveFlag::Capture | MoveFlag::EnPassant | MoveFlag::PromoCapture
        )
    }
}

impl core::fmt::Display for Move {
    /// Long algebraic ("e2e4", "e7e8q") — the format UCI speaks.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}{}", self.from(), self.to())?;
        if self.is_promo() {
            write!(f, "{}", self.promo().ch())?;
        }
        Ok(())
    }
}

/// Terminal state. Symbolic, NOT a number — GRAMMAR.md 1 requires that mapping an outcome to
/// a score go through the learned `score_of` table, so that "draw = 0" and any preference for
/// shorter mates stay out of the Given column.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Outcome {
    Ongoing,
    /// The side to move has been checkmated.
    Loss,
    Draw,
}
