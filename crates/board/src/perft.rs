//! Perft: count leaf nodes at a fixed depth. The correctness harness for the movegen, and the
//! first thing P0 must pass (MASTER_PLAN P0).

use crate::chess::Position;
use crate::types::Move;

/// Leaf count at `depth`. Depth 0 counts the position itself.
pub fn perft(pos: &mut Position, depth: u32) -> u64 {
    if depth == 0 {
        return 1;
    }
    let list = pos.legal_moves();
    if depth == 1 {
        return list.len() as u64;
    }
    let mut n = 0;
    for &m in list.as_slice() {
        let u = pos.make_move(m);
        n += perft(pos, depth - 1);
        pos.unmake_move(m, u);
    }
    n
}

/// Per-root-move breakdown, which is how a perft mismatch is actually localised: compare
/// against a reference, descend into the first move whose count differs.
pub fn divide(pos: &mut Position, depth: u32) -> Vec<(Move, u64)> {
    let mut out = Vec::new();
    for &m in pos.legal_moves().as_slice() {
        let u = pos.make_move(m);
        let n = if depth <= 1 { 1 } else { perft(pos, depth - 1) };
        pos.unmake_move(m, u);
        out.push((m, n));
    }
    out
}
