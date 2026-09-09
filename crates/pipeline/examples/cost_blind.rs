//! Is the game gate COST-BLIND? Tests the claim instead of arguing it.
//!
//! THE CLAIM. `evolve.rs:1502` sets `const COST_PER_MOVE: u64 = u64::MAX` and passes it to
//! `gate::match_progs`. So both programs get the same BUDGET parameter (playouts / depth) and may
//! spend unlimited cost reaching it. If that is right, a candidate that reaches the same budget more
//! cheaply plays the same and scores exactly 0.500 — its efficiency invisible by construction. That
//! would explain why the acceptance rule never promotes one, and it would mean the fix is the gate's
//! budget basis, not the acceptance rule.
//!
//! WHY IT MATTERS THAT THIS IS TESTED. It competes with the other candidate fix (relax the rule), and
//! this project's standing rule is that the apparatus changes only on measurement. An argument about
//! `u64::MAX` is not a measurement.
//!
//! THE TEST. `capture_extension` (rung 6) searches captures past the horizon: strictly more work per
//! move than `bare_alpha_beta`, and stronger for it. Match the pair twice, changing ONLY the cost
//! ceiling:
//!   * `cost_per_move = u64::MAX`  -- the gate's current setting. Cost is free, so the expensive
//!     program keeps its advantage.
//!   * `cost_per_move = <finite>`  -- the expensive program gets truncated mid-search and must give
//!     some of that advantage back.
//!
//! READING IT. If the score MOVES as the ceiling tightens, cost_per_move materially decides gate
//! outcomes, the current `u64::MAX` really is cost-blind, and making it finite is a live fix that
//! would let the EXISTING strict rule promote a cheaper program. If the score does not move, the
//! ceiling is not the mechanism and the acceptance-rule change is the remaining candidate.
//!
//! Everything here is read-only: no engine setting is modified, and the gate keeps its own value.
use grammar::reference;
use nnue::Net;
use pipeline::gate;

fn main() {
    let pairs: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(24);
    let depth: i64 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(3);
    let net = Net::random(32, 20260907);
    let a = reference::capture_extension();   // more work per move, stronger
    let b = reference::bare_alpha_beta();     // the seed
    println!("cost-blindness probe — capture_extension (a) vs bare_alpha_beta (b)");
    println!("  {pairs} pairs, depth {depth}, budget 16, identical in every respect but the cost ceiling\n");
    println!("  {:>18}  {:>7}  {:>7}", "cost_per_move", "a's score", "ci95");

    for (label, cpm) in [
        ("u64::MAX (gate's)", u64::MAX),
        ("100_000_000", 100_000_000u64),
        ("10_000_000", 10_000_000u64),
        ("1_000_000", 1_000_000u64),
    ] {
        let sc = gate::match_progs(&a, &b, &net, vec![depth, 32_000, interp::uct_exploration()],
                                   16, pairs, 0xC0FFEE, 4, cpm);
        println!("  {label:>18}  {:>7.3}  {:>7.3}", sc.pent_rate(), sc.ci95());
    }

    println!("\n  A score that MOVES as the ceiling tightens means cost_per_move decides gate");
    println!("  outcomes, so the gate's u64::MAX is genuinely cost-blind and making it finite would");
    println!("  let the EXISTING strict rule promote a cheaper program.");
    println!("  A score that does NOT move means the ceiling is not the mechanism, and relaxing the");
    println!("  acceptance rule is the remaining candidate fix.");
}
