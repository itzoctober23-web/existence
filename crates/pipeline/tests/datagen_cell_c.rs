//! Cell C must walk the CONTROL's positions exactly, and differ from it only in the label.
//!
//! That is the entire reason cell C exists. `candidate_a_channel_FINDING.md`: arm B changes the
//! labels and the position distribution together, so its result cannot be attributed. Cell C
//! isolates the label channel — but only if its trajectory really is the control's. If the games
//! drift apart, cell C is just a second confounded arm and the decomposition is worthless.
//!
//! Two mechanisms are supposed to guarantee it, and both are asserted here rather than assumed:
//!   * the budget label is computed on a SEPARATE `Searcher`, so it cannot advance the driving
//!     searcher's shuffle rng (which would flip the ~4% of moves that are ties)
//!   * its seed comes from the ply, not `rng.next()`, so the shared stream is not advanced
//!
//! ONE test fn in its OWN test binary: these are process-global atomics and cargo runs tests in one
//! binary on parallel threads, so a second test here would race the globals.

use nnue::Net;
use pipeline::datagen::{self, Rng, Sample};
use std::sync::atomic::Ordering;

fn run(budget: u64, labels_only: u64, seed: u64, games: usize, net: &Net) -> Vec<Sample> {
    datagen::BUDGET.store(budget, Ordering::Relaxed);
    datagen::BUDGET_LABELS_ONLY.store(labels_only, Ordering::Relaxed);
    datagen::BUDGET_MAX_DEPTH.store(8, Ordering::Relaxed);
    let mut out: Vec<Sample> = Vec::new();
    let mut rng = Rng(seed);
    for _ in 0..games {
        datagen::play_game(net, 3, &mut rng, 6, 160, &mut out);
    }
    datagen::BUDGET.store(0, Ordering::Relaxed);
    datagen::BUDGET_LABELS_ONLY.store(0, Ordering::Relaxed);
    out
}

#[test]
fn cell_c_walks_the_controls_positions_and_changes_only_the_label() {
    let net = Net::random(32, 20260912);
    let seed = 4242;
    let games = 6;

    let a = run(0, 0, seed, games, &net); // ARM A: fixed depth 3, the control
    let c = run(5_269, 1, seed, games, &net); // CELL C: control's moves, budget labels
    let b = run(5_269, 0, seed, games, &net); // ARM B: budget drives everything

    assert!(!a.is_empty(), "control produced no samples");

    // 1. IDENTICAL TRAJECTORY. Same count, same FEN at every index.
    assert_eq!(
        a.len(),
        c.len(),
        "cell C produced {} samples against the control's {} -- the games diverged",
        c.len(),
        a.len()
    );
    let first_diff = a.iter().zip(c.iter()).position(|(x, y)| x.fen != y.fen);
    assert!(
        first_diff.is_none(),
        "cell C left the control's trajectory at sample {:?} -- the position confound is back, \
         which is the one thing cell C exists to remove",
        first_diff
    );

    // 2. THE LABEL ACTUALLY MOVED. If it did not, cell C is a duplicate of the control and the
    //    label contribution it is meant to measure would be zero by construction.
    let moved = a.iter().zip(c.iter()).filter(|(x, y)| x.root != y.root).count();
    assert!(
        moved > 0,
        "cell C's labels are identical to the control's on all {} samples -- the budget label \
         never reached Sample.root",
        a.len()
    );

    // 3. And it is genuinely a THIRD cell: arm B must NOT be the same as cell C, or the
    //    decomposition has no second dimension.
    let same_as_b = b.len() == c.len() && b.iter().zip(c.iter()).all(|(x, y)| x.fen == y.fen);
    assert!(
        !same_as_b,
        "arm B and cell C produced the same trajectory -- B is supposed to move off the control's \
         positions, so there is nothing to decompose"
    );

    eprintln!(
        "  control {} samples, cell C identical FENs, {} labels changed ({:.1}%), arm B diverges ({} samples)",
        a.len(),
        moved,
        100.0 * moved as f64 / a.len() as f64,
        b.len()
    );
}
