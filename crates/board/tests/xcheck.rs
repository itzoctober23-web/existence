//! CRATE.md 2, half (b): random full games walked to terminal, comparing the LEGAL MOVE SET
//! against an external engine at every ply.
//!
//! This was only an `example` until now, which meant `cargo test` never ran it and it gated
//! nothing. Half (a) — the frozen perft fixture in `tests/perft.rs` — is NOT a substitute:
//!   - a fixture only probes the positions in it. On the sibling 4PC project a 40-position
//!     perft fixture agreed 40/40 while 94 genuine rules divergences existed, because the
//!     fixture never reached the states where they occur;
//!   - perft compares COUNTS, so two compensating errors cancel. This compares the SETS.
//!
//! ON SKIPPING: if the reference engine is missing this test passes, because the repo must be
//! testable on a machine without Stockfish. That would be a silent hole, so it prints a machine
//! -readable marker either way and `check.sh` greps for the RAN marker — the test is portable,
//! the GATE is strict. A check that can quietly not-run is not a check.

use board::xcheck;

#[test]
fn movegen_agrees_with_an_external_engine() {
    let path = xcheck::engine_path();
    let r = match xcheck::cross_check(&path, 25, 120, 0x2026_09_07_C0FFEE, |_| {}) {
        Ok(r) => r,
        Err(e) => {
            println!("XCHECK-SKIPPED: no reference engine at '{path}' ({e}); set XCHECK_ENGINE");
            return;
        }
    };

    for d in &r.detail {
        println!("DIVERGENCE {d}");
    }
    println!(
        "XCHECK-RAN: {} games, {} plies compared, {} divergences \
         (ply-cap {}, checkmate {}, stalemate {})",
        r.games, r.plies, r.divergences, r.terminals[0], r.terminals[1], r.terminals[2]
    );

    assert_eq!(
        r.divergences, 0,
        "movegen disagreed with {path} on {} of {} plies",
        r.divergences, r.plies
    );
    // A run that compared almost nothing would pass the divergence check trivially. Games start
    // from the initial position and walk randomly, so 25 games must reach a few hundred plies;
    // anything less means the walk is terminating immediately and the test is vacuous.
    assert!(
        r.plies > 300,
        "only {} plies compared across 25 games — the walk is not reaching real positions, \
         so a zero-divergence result proves nothing",
        r.plies
    );
}
