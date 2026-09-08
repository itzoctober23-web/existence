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
/// Movegen cross-check against an external engine. A LIBRARY, not only an example, so that
/// `tests/xcheck.rs` and `examples/xcheck.rs` run identical code. Two copies of "the same"
/// logic are free to drift — which is exactly the bug that had the pipeline's search and the
/// shipped engine's search silently disagreeing while a comment asserted they matched.
///
/// It spawns an external process, so it is verification tooling rather than rules. It lives
/// here because it verifies THIS crate and nothing above it: no evaluation, no strategy.
pub mod xcheck;
pub mod zobrist;

pub use chess::Position;
pub use movegen::{MAX_MOVES, MoveList};
pub use perft::{divide, perft};
pub use types::{Color, Move, MoveFlag, Outcome, PieceKind, Square};
