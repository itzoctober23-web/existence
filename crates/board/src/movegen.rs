//! Legal move generation.
//!
//! Structure: find the checkers and the pinned pieces first, then generate only moves that are
//! already legal. A piece that is pinned may move only along the pin line; when in single check
//! every non-king move must either capture the checker or block it; in double check only the
//! king may move.
//!
//! En passant is the one case this scheme cannot decide statically: the capture removes a pawn
//! that is not on the destination square, so it can expose the king along a RANK even when
//! neither the moving pawn nor the captured pawn is pinned. That one is verified by simulating
//! the occupancy change. It is the standard perft trap and the reason position 3 of the
//! canonical suite exists.

use crate::attacks;
use crate::bitboard::{self as bb, Bb};
use crate::chess::*;
use crate::types::*;

/// A small fixed-capacity move buffer. 218 is the highest legal move count known for a
/// reachable chess position; 256 keeps it a round number and leaves headroom.
pub const MAX_MOVES: usize = 256;

pub struct MoveList {
    pub moves: [Move; MAX_MOVES],
    pub len: usize,
}

impl MoveList {
    #[inline]
    pub fn new() -> Self {
        MoveList { moves: [MOVE_NONE; MAX_MOVES], len: 0 }
    }
    #[inline]
    pub fn push(&mut self, m: Move) {
        debug_assert!(self.len < MAX_MOVES, "move list overflow");
        self.moves[self.len] = m;
        self.len += 1;
    }
    #[inline]
    pub fn as_slice(&self) -> &[Move] {
        &self.moves[..self.len]
    }
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn contains(&self, m: Move) -> bool {
        self.as_slice().contains(&m)
    }
}

impl Default for MoveList {
    fn default() -> Self {
        Self::new()
    }
}

impl Position {
    /// Squares of our pieces that are pinned against our king, and the enemy sliders pinning
    /// them. A pinned piece may only move on `line(king, pinner)`.
    fn pinned(&self, us: Color) -> Bb {
        let them = us.flip();
        let ksq = self.king_sq(us);
        let p = &self.pieces[them.idx()];
        // Sliders that would attack the king if every blocker vanished.
        let snipers = (attacks::rook(ksq, 0) & (p[PieceKind::Rook.idx()] | p[PieceKind::Queen.idx()]))
            | (attacks::bishop(ksq, 0) & (p[PieceKind::Bishop.idx()] | p[PieceKind::Queen.idx()]));
        let mut pinned = 0;
        for s in bb::squares(snipers) {
            let blockers = attacks::between(ksq, s) & self.all;
            // Exactly one blocker, and it is ours -> pinned.
            if blockers.count_ones() == 1 && blockers & self.occ[us.idx()] != 0 {
                pinned |= blockers;
            }
        }
        pinned
    }

    /// All legal moves for the side to move.
    pub fn legal_moves(&self) -> MoveList {
        let mut list = MoveList::new();
        let us = self.stm;
        let them = us.flip();
        let ksq = self.king_sq(us);
        let checkers = self.attackers_to(ksq, them, self.all);
        let n_checkers = checkers.count_ones();

        // King moves are always available; generate them against an occupancy with the king
        // removed so a slider's ray extends through the square it currently occupies.
        let occ_no_king = self.all ^ bb::bit(ksq);
        let danger = self.attacks_by(them, occ_no_king);
        let king_targets = attacks::king(ksq) & !self.occ[us.idx()] & !danger;
        for to in bb::squares(king_targets) {
            let flag = if self.occ[them.idx()] & bb::bit(to) != 0 { MoveFlag::Capture } else { MoveFlag::Quiet };
            list.push(Move::new(Square(ksq), Square(to), flag));
        }

        if n_checkers >= 2 {
            return list; // double check: only the king may move
        }

        // Where a non-king move must land.
        let (capture_mask, push_mask) = if n_checkers == 1 {
            let csq = bb::lsb(checkers);
            (checkers, attacks::between(ksq, csq))
        } else {
            (self.occ[them.idx()], !self.all)
        };
        let target = capture_mask | push_mask;

        let pinned = self.pinned(us);
        let p = &self.pieces[us.idx()];

        // ---- knights: a pinned knight can never move ----
        for from in bb::squares(p[PieceKind::Knight.idx()] & !pinned) {
            for to in bb::squares(attacks::knight(from) & target) {
                let flag = if self.occ[them.idx()] & bb::bit(to) != 0 { MoveFlag::Capture } else { MoveFlag::Quiet };
                list.push(Move::new(Square(from), Square(to), flag));
            }
        }

        // ---- sliders ----
        for (kind, f) in [
            (PieceKind::Bishop, attacks::bishop as fn(u8, Bb) -> Bb),
            (PieceKind::Rook, attacks::rook as fn(u8, Bb) -> Bb),
            (PieceKind::Queen, attacks::queen as fn(u8, Bb) -> Bb),
        ] {
            for from in bb::squares(p[kind.idx()]) {
                let mut t = f(from, self.all) & target;
                if pinned & bb::bit(from) != 0 {
                    t &= attacks::line(ksq, from);
                }
                for to in bb::squares(t) {
                    let flag = if self.occ[them.idx()] & bb::bit(to) != 0 { MoveFlag::Capture } else { MoveFlag::Quiet };
                    list.push(Move::new(Square(from), Square(to), flag));
                }
            }
        }

        self.gen_pawns(&mut list, us, target, capture_mask, push_mask, pinned, ksq);

        if n_checkers == 0 {
            self.gen_castles(&mut list, us, danger);
        }
        list
    }

    fn gen_pawns(
        &self,
        list: &mut MoveList,
        us: Color,
        _target: Bb,
        capture_mask: Bb,
        push_mask: Bb,
        pinned: Bb,
        ksq: u8,
    ) {
        let them = us.flip();
        let pawns = self.pieces[us.idx()][PieceKind::Pawn.idx()];
        let promo_rank = if us == Color::White { bb::RANK_8 } else { bb::RANK_1 };
        let start_rank = if us == Color::White { bb::RANK_2 } else { bb::RANK_7 };

        for from in bb::squares(pawns) {
            let pin_line = if pinned & bb::bit(from) != 0 { attacks::line(ksq, from) } else { !0 };

            // Single and double pushes.
            let one = if us == Color::White { from as i16 + 8 } else { from as i16 - 8 };
            if (0..64).contains(&one) {
                let one = one as u8;
                if self.all & bb::bit(one) == 0 {
                    if push_mask & bb::bit(one) != 0 && pin_line & bb::bit(one) != 0 {
                        self.push_pawn(list, from, one, promo_rank, MoveFlag::Quiet, MoveFlag::Promo);
                    }
                    if start_rank & bb::bit(from) != 0 {
                        let two = if us == Color::White { one + 8 } else { one - 8 };
                        if self.all & bb::bit(two) == 0
                            && push_mask & bb::bit(two) != 0
                            && pin_line & bb::bit(two) != 0
                        {
                            list.push(Move::new(Square(from), Square(two), MoveFlag::DoublePush));
                        }
                    }
                }
            }

            // Captures.
            let caps = attacks::pawn(us.idx(), from) & self.occ[them.idx()] & capture_mask & pin_line;
            for to in bb::squares(caps) {
                self.push_pawn(list, from, to, promo_rank, MoveFlag::Capture, MoveFlag::PromoCapture);
            }

            // En passant. The captured pawn is not on `to`, so removing it can open a rank onto
            // our king; simulate the exact occupancy change and test.
            if let Some(ep) = self.ep {
                if attacks::pawn(us.idx(), from) & bb::bit(ep.0) != 0 {
                    let cap_sq = if us == Color::White { ep.0 - 8 } else { ep.0 + 8 };
                    let occ = (self.all ^ bb::bit(from) ^ bb::bit(cap_sq)) | bb::bit(ep.0);
                    let p = &self.pieces[them.idx()];
                    let rq = (p[PieceKind::Rook.idx()] | p[PieceKind::Queen.idx()]) & !bb::bit(cap_sq);
                    let bq = (p[PieceKind::Bishop.idx()] | p[PieceKind::Queen.idx()]) & !bb::bit(cap_sq);
                    let exposed =
                        (attacks::rook(ksq, occ) & rq) != 0 || (attacks::bishop(ksq, occ) & bq) != 0;
                    // While in check, an ep capture is only legal if it removes the checker or
                    // blocks; capture_mask holds the checker's square, which is cap_sq here.
                    let resolves = capture_mask & bb::bit(cap_sq) != 0 || push_mask & bb::bit(ep.0) != 0;
                    if !exposed && resolves {
                        list.push(Move::new(Square(from), ep, MoveFlag::EnPassant));
                    }
                }
            }
        }
    }

    #[inline]
    fn push_pawn(
        &self,
        list: &mut MoveList,
        from: u8,
        to: u8,
        promo_rank: Bb,
        plain: MoveFlag,
        promo: MoveFlag,
    ) {
        if promo_rank & bb::bit(to) != 0 {
            for k in [PieceKind::Queen, PieceKind::Rook, PieceKind::Bishop, PieceKind::Knight] {
                list.push(Move::new_promo(Square(from), Square(to), promo, k));
            }
        } else {
            list.push(Move::new(Square(from), Square(to), plain));
        }
    }

    fn gen_castles(&self, list: &mut MoveList, us: Color, danger: Bb) {
        let (ksq, kc, qc, rk, rq) = if us == Color::White {
            (4u8, CASTLE_WK, CASTLE_WQ, 7u8, 0u8)
        } else {
            (60u8, CASTLE_BK, CASTLE_BQ, 63u8, 56u8)
        };
        let rooks = self.pieces[us.idx()][PieceKind::Rook.idx()];

        // King side: f,g empty; e,f,g not attacked.
        if self.castling & kc != 0 && rooks & bb::bit(rk) != 0 {
            let empty = bb::bit(ksq + 1) | bb::bit(ksq + 2);
            let path = bb::bit(ksq) | empty;
            if self.all & empty == 0 && danger & path == 0 {
                list.push(Move::new(Square(ksq), Square(ksq + 2), MoveFlag::CastleKing));
            }
        }
        // Queen side: b,c,d empty; e,d,c not attacked (b may be attacked).
        if self.castling & qc != 0 && rooks & bb::bit(rq) != 0 {
            let empty = bb::bit(ksq - 1) | bb::bit(ksq - 2) | bb::bit(ksq - 3);
            let path = bb::bit(ksq) | bb::bit(ksq - 1) | bb::bit(ksq - 2);
            if self.all & empty == 0 && danger & path == 0 {
                list.push(Move::new(Square(ksq), Square(ksq - 2), MoveFlag::CastleQueen));
            }
        }
    }

    /// Terminal state. Symbolic on purpose (types::Outcome) — turning it into a number is the
    /// learned `score_of` table's job, not the rules'.
    pub fn outcome(&self) -> Outcome {
        if !self.legal_moves().is_empty() {
            return Outcome::Ongoing;
        }
        if self.in_check(self.stm) { Outcome::Loss } else { Outcome::Draw }
    }
}
