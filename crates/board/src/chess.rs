//! Standard chess: position, make/unmake, and a fully-legal move generator.
//!
//! The generator produces LEGAL moves directly rather than pseudo-legal-then-filter, because
//! GRAMMAR.md defines `moves: Pos -> List` as the legal list and every search program in the
//! grammar assumes it. Legality is enforced with a check mask and pin rays; the only move that
//! still needs an explicit simulation is en passant, where removing the captured pawn can
//! discover a rank attack on the king (the classic perft trap).

use crate::attacks;
use crate::bitboard::{self as bb, Bb};
use crate::types::*;

/// Castling rights, one bit each: WK, WQ, BK, BQ.
pub const CASTLE_WK: u8 = 1;
pub const CASTLE_WQ: u8 = 2;
pub const CASTLE_BK: u8 = 4;
pub const CASTLE_BQ: u8 = 8;

#[derive(Copy, Clone, Debug)]
pub struct Undo {
    pub captured: Option<PieceKind>,
    pub castling: u8,
    pub ep: Option<Square>,
    pub halfmove: u16,
}

#[derive(Clone, Debug)]
pub struct Position {
    /// pieces[color][kind]
    pub pieces: [[Bb; N_PIECE_KINDS]; 2],
    pub occ: [Bb; 2],
    pub all: Bb,
    pub stm: Color,
    pub castling: u8,
    pub ep: Option<Square>,
    pub halfmove: u16,
    pub fullmove: u16,
}

impl Default for Position {
    fn default() -> Self {
        Self::startpos()
    }
}

impl Position {
    pub fn empty() -> Self {
        Position {
            pieces: [[0; N_PIECE_KINDS]; 2],
            occ: [0; 2],
            all: 0,
            stm: Color::White,
            castling: 0,
            ep: None,
            halfmove: 0,
            fullmove: 1,
        }
    }

    pub fn startpos() -> Self {
        Self::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap()
    }

    #[inline]
    fn refresh(&mut self) {
        for c in 0..2 {
            self.occ[c] = self.pieces[c].iter().fold(0, |a, b| a | b);
        }
        self.all = self.occ[0] | self.occ[1];
    }

    #[inline]
    pub fn king_sq(&self, c: Color) -> u8 {
        bb::lsb(self.pieces[c.idx()][PieceKind::King.idx()])
    }

    #[inline]
    pub fn piece_at(&self, sq: u8) -> Option<(Color, PieceKind)> {
        let b = bb::bit(sq);
        if self.all & b == 0 {
            return None;
        }
        let c = if self.occ[0] & b != 0 { Color::White } else { Color::Black };
        for k in PieceKind::ALL {
            if self.pieces[c.idx()][k.idx()] & b != 0 {
                return Some((c, k));
            }
        }
        None
    }

    /// Every square attacked by `c`, given an explicit occupancy. The occupancy is a parameter
    /// so the king can be removed from it when testing its own escape squares — otherwise the
    /// king appears to block the very slider it is fleeing.
    pub fn attacks_by(&self, c: Color, occ: Bb) -> Bb {
        let p = &self.pieces[c.idx()];
        let mut a = 0;
        let pawns = p[PieceKind::Pawn.idx()];
        a |= if c == Color::White {
            bb::north_east(pawns) | bb::north_west(pawns)
        } else {
            bb::south_east(pawns) | bb::south_west(pawns)
        };
        for s in bb::squares(p[PieceKind::Knight.idx()]) {
            a |= attacks::knight(s);
        }
        for s in bb::squares(p[PieceKind::Bishop.idx()] | p[PieceKind::Queen.idx()]) {
            a |= attacks::bishop(s, occ);
        }
        for s in bb::squares(p[PieceKind::Rook.idx()] | p[PieceKind::Queen.idx()]) {
            a |= attacks::rook(s, occ);
        }
        a |= attacks::king(bb::lsb(p[PieceKind::King.idx()]));
        a
    }

    /// Pieces of `by` that attack `sq`, given `occ`.
    pub fn attackers_to(&self, sq: u8, by: Color, occ: Bb) -> Bb {
        let p = &self.pieces[by.idx()];
        // A pawn attacks sq iff sq is attacked from the OTHER colour's perspective at sq.
        let pawn_from = attacks::pawn(by.flip().idx(), sq) & p[PieceKind::Pawn.idx()];
        pawn_from
            | (attacks::knight(sq) & p[PieceKind::Knight.idx()])
            | (attacks::king(sq) & p[PieceKind::King.idx()])
            | (attacks::bishop(sq, occ) & (p[PieceKind::Bishop.idx()] | p[PieceKind::Queen.idx()]))
            | (attacks::rook(sq, occ) & (p[PieceKind::Rook.idx()] | p[PieceKind::Queen.idx()]))
    }

    #[inline]
    pub fn in_check(&self, c: Color) -> bool {
        self.attackers_to(self.king_sq(c), c.flip(), self.all) != 0
    }

    // ---------------------------------------------------------------- FEN

    pub fn from_fen(fen: &str) -> Result<Self, String> {
        let mut pos = Position::empty();
        let mut it = fen.split_whitespace();
        let board = it.next().ok_or("fen: missing board")?;

        let mut rank: i32 = 7;
        let mut file: i32 = 0;
        for ch in board.chars() {
            match ch {
                '/' => {
                    rank -= 1;
                    file = 0;
                    if rank < 0 {
                        return Err("fen: too many ranks".into());
                    }
                }
                '1'..='8' => file += ch as i32 - '0' as i32,
                _ => {
                    let color = if ch.is_ascii_uppercase() { Color::White } else { Color::Black };
                    let kind = match ch.to_ascii_lowercase() {
                        'p' => PieceKind::Pawn,
                        'n' => PieceKind::Knight,
                        'b' => PieceKind::Bishop,
                        'r' => PieceKind::Rook,
                        'q' => PieceKind::Queen,
                        'k' => PieceKind::King,
                        _ => return Err(format!("fen: bad piece '{ch}'")),
                    };
                    if !(0..8).contains(&file) || !(0..8).contains(&rank) {
                        return Err("fen: square out of range".into());
                    }
                    pos.pieces[color.idx()][kind.idx()] |= bb::bit((rank * 8 + file) as u8);
                    file += 1;
                }
            }
        }

        pos.stm = match it.next() {
            Some("w") | None => Color::White,
            Some("b") => Color::Black,
            Some(x) => return Err(format!("fen: bad side to move '{x}'")),
        };

        if let Some(c) = it.next() {
            for ch in c.chars() {
                match ch {
                    'K' => pos.castling |= CASTLE_WK,
                    'Q' => pos.castling |= CASTLE_WQ,
                    'k' => pos.castling |= CASTLE_BK,
                    'q' => pos.castling |= CASTLE_BQ,
                    '-' => {}
                    _ => return Err(format!("fen: bad castling '{ch}'")),
                }
            }
        }

        pos.ep = match it.next() {
            Some("-") | None => None,
            Some(s) => Some(Square::from_str(s).ok_or_else(|| format!("fen: bad ep '{s}'"))?),
        };
        pos.halfmove = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        pos.fullmove = it.next().and_then(|s| s.parse().ok()).unwrap_or(1);

        pos.refresh();
        if pos.pieces[0][PieceKind::King.idx()].count_ones() != 1
            || pos.pieces[1][PieceKind::King.idx()].count_ones() != 1
        {
            return Err("fen: each side needs exactly one king".into());
        }
        Ok(pos)
    }

    pub fn to_fen(&self) -> String {
        let mut s = String::new();
        for rank in (0..8).rev() {
            let mut empty = 0;
            for file in 0..8 {
                match self.piece_at(rank * 8 + file) {
                    None => empty += 1,
                    Some((c, k)) => {
                        if empty > 0 {
                            s.push_str(&empty.to_string());
                            empty = 0;
                        }
                        let ch = k.ch();
                        s.push(if c == Color::White { ch.to_ascii_uppercase() } else { ch });
                    }
                }
            }
            if empty > 0 {
                s.push_str(&empty.to_string());
            }
            if rank > 0 {
                s.push('/');
            }
        }
        s.push(' ');
        s.push(if self.stm == Color::White { 'w' } else { 'b' });
        s.push(' ');
        if self.castling == 0 {
            s.push('-');
        } else {
            for (m, ch) in [(CASTLE_WK, 'K'), (CASTLE_WQ, 'Q'), (CASTLE_BK, 'k'), (CASTLE_BQ, 'q')] {
                if self.castling & m != 0 {
                    s.push(ch);
                }
            }
        }
        s.push(' ');
        match self.ep {
            Some(sq) => s.push_str(&sq.to_string()),
            None => s.push('-'),
        }
        s.push_str(&format!(" {} {}", self.halfmove, self.fullmove));
        s
    }

    // ------------------------------------------------------- make / unmake

    #[inline]
    fn put(&mut self, c: Color, k: PieceKind, sq: u8) {
        let b = bb::bit(sq);
        self.pieces[c.idx()][k.idx()] |= b;
        self.occ[c.idx()] |= b;
        self.all |= b;
    }

    #[inline]
    fn take(&mut self, c: Color, k: PieceKind, sq: u8) {
        let b = !bb::bit(sq);
        self.pieces[c.idx()][k.idx()] &= b;
        self.occ[c.idx()] &= b;
        self.all &= b;
    }

    pub fn make_move(&mut self, m: Move) -> Undo {
        let us = self.stm;
        let them = us.flip();
        let from = m.from().0;
        let to = m.to().0;
        let undo = Undo {
            captured: None,
            castling: self.castling,
            ep: self.ep,
            halfmove: self.halfmove,
        };
        let mut undo = undo;

        let (_, kind) = self.piece_at(from).expect("make_move: empty from-square");

        // Capture (en passant removes a pawn that is NOT on the destination square).
        match m.flag() {
            MoveFlag::EnPassant => {
                let cap_sq = if us == Color::White { to - 8 } else { to + 8 };
                self.take(them, PieceKind::Pawn, cap_sq);
                undo.captured = Some(PieceKind::Pawn);
            }
            MoveFlag::Capture | MoveFlag::PromoCapture => {
                let (_, ck) = self.piece_at(to).expect("make_move: capture on empty square");
                self.take(them, ck, to);
                undo.captured = Some(ck);
            }
            _ => {}
        }

        self.take(us, kind, from);
        if m.is_promo() {
            self.put(us, m.promo(), to);
        } else {
            self.put(us, kind, to);
        }

        // Castling moves the rook too. Rights are cleared below by the from/to masks.
        match m.flag() {
            MoveFlag::CastleKing => {
                let (rf, rt) = if us == Color::White { (7, 5) } else { (63, 61) };
                self.take(us, PieceKind::Rook, rf);
                self.put(us, PieceKind::Rook, rt);
            }
            MoveFlag::CastleQueen => {
                let (rf, rt) = if us == Color::White { (0, 3) } else { (56, 59) };
                self.take(us, PieceKind::Rook, rf);
                self.put(us, PieceKind::Rook, rt);
            }
            _ => {}
        }

        // Castling rights: lost when the king moves, or when a rook leaves or is captured on
        // its home square. Driven off from/to so captures are covered without a special case.
        self.castling &= !castle_mask(from) & !castle_mask(to);

        self.ep = if m.flag() == MoveFlag::DoublePush {
            Some(Square(if us == Color::White { from + 8 } else { from - 8 }))
        } else {
            None
        };

        self.halfmove = if kind == PieceKind::Pawn || m.is_capture() { 0 } else { self.halfmove + 1 };
        if us == Color::Black {
            self.fullmove += 1;
        }
        self.stm = them;
        undo
    }

    pub fn unmake_move(&mut self, m: Move, undo: Undo) {
        let them = self.stm;
        let us = them.flip();
        let from = m.from().0;
        let to = m.to().0;

        self.stm = us;
        if us == Color::Black {
            self.fullmove -= 1;
        }
        self.castling = undo.castling;
        self.ep = undo.ep;
        self.halfmove = undo.halfmove;

        match m.flag() {
            MoveFlag::CastleKing => {
                let (rf, rt) = if us == Color::White { (7, 5) } else { (63, 61) };
                self.take(us, PieceKind::Rook, rt);
                self.put(us, PieceKind::Rook, rf);
            }
            MoveFlag::CastleQueen => {
                let (rf, rt) = if us == Color::White { (0, 3) } else { (56, 59) };
                self.take(us, PieceKind::Rook, rt);
                self.put(us, PieceKind::Rook, rf);
            }
            _ => {}
        }

        if m.is_promo() {
            self.take(us, m.promo(), to);
            self.put(us, PieceKind::Pawn, from);
        } else {
            let (_, kind) = self.piece_at(to).expect("unmake: empty to-square");
            self.take(us, kind, to);
            self.put(us, kind, from);
        }

        if let Some(ck) = undo.captured {
            match m.flag() {
                MoveFlag::EnPassant => {
                    let cap_sq = if us == Color::White { to - 8 } else { to + 8 };
                    self.put(them, PieceKind::Pawn, cap_sq);
                }
                _ => self.put(them, ck, to),
            }
        }
    }
}

/// Rights removed when a piece leaves or lands on this square.
#[inline]
const fn castle_mask(sq: u8) -> u8 {
    match sq {
        0 => CASTLE_WQ,
        4 => CASTLE_WK | CASTLE_WQ,
        7 => CASTLE_WK,
        56 => CASTLE_BQ,
        60 => CASTLE_BK | CASTLE_BQ,
        63 => CASTLE_BK,
        _ => 0,
    }
}
