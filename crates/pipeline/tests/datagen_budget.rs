//! `datagen::BUDGET` is wired in and actually changes what datagen does.
//!
//! ONE test function in its OWN test binary on purpose. `BUDGET` is a process-global `AtomicU64`,
//! and Rust runs the tests inside one binary on parallel threads -- a second test in this file
//! could observe the global mid-flight and the suite would be flaky in a way that looks like a
//! real failure. A separate binary with a single test cannot race.
//!
//! WHY THIS EXISTS AT ALL. A flag that parses, stores, and is then never read is the standard way
//! an experiment arm silently becomes its own control: the run looks configured, prints a banner,
//! and generates identical data. This asserts the BEHAVIOUR differs, not that the flag was set.

use nnue::Net;
use pipeline::datagen::{self, Rng, Sample};
use std::sync::atomic::Ordering;

#[test]
fn budget_is_wired_in_and_changes_the_data() {
    let net = Net::random(32, 20260912);

    // Control: budget OFF, fixed depth 3 -- the current production path.
    datagen::BUDGET.store(0, Ordering::Relaxed);
    let mut a: Vec<Sample> = Vec::new();
    let mut ra = Rng(4242);
    datagen::play_game(&net, 3, &mut ra, 6, 160, &mut a);

    // Treatment: same seed, same everything, budget ON at the measured depth-3 mean.
    datagen::BUDGET.store(5_269, Ordering::Relaxed);
    datagen::BUDGET_MAX_DEPTH.store(8, Ordering::Relaxed);
    let mut b: Vec<Sample> = Vec::new();
    let mut rb = Rng(4242);
    datagen::play_game(&net, 3, &mut rb, 6, 160, &mut b);

    datagen::BUDGET.store(0, Ordering::Relaxed); // leave the global as we found it

    assert!(!a.is_empty(), "control produced no samples");
    assert!(!b.is_empty(), "budget arm produced no samples");

    // No label may be a confidently-lost score on a position nobody evaluated. -INF becomes
    // tanh(-32000/600) = -1.0; this is the failure datagen.rs:189 guards in the capped path and
    // the one best_move_budget's uncapped depth-1 guarantees against.
    for s in &b {
        assert!(
            s.root > -31_000 && s.root < 31_000,
            "budget arm produced an out-of-range root score {}",
            s.root
        );
    }

    // The whole point: identical inputs must NOT produce identical data, or the flag is inert.
    let same_len = a.len() == b.len();
    let same_all = same_len && a.iter().zip(b.iter()).all(|(x, y)| x.fen == y.fen && x.root == y.root);
    assert!(
        !same_all,
        "BUDGET changed nothing: {} vs {} samples, identical fens and roots -- the flag is inert \
         and a 'node budget' arm would be its own control",
        a.len(),
        b.len()
    );
}
