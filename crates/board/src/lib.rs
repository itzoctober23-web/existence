//! Rules only.
//!
//! CRATE.md 2: if a function here would differ for a game with different RULES it belongs in
//! this crate; if it would differ for a game with different STRATEGY it does not exist here at
//! all. There is no evaluation, no move ordering, and no heuristic of any kind below this line.

pub mod attacks;
pub mod bitboard;
pub mod chess;
pub mod movegen;
pub mod perft;
pub mod types;

pub use chess::Position;
pub use movegen::{MAX_MOVES, MoveList};
pub use perft::{divide, perft};
pub use types::{Color, Move, MoveFlag, Outcome, PieceKind, Square};
