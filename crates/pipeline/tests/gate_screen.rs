//! The cheap pre-gate screen may only reject a candidate it actually MEASURED.
//!
//! The screen plays a few games before the full sequential gate and drops what is confidently
//! worse, so 400 pairs of SPRT are not spent on obvious duds. Its whole safety argument is that it
//! is one-sided — it never accepts, and it rejects only when the interval lies ENTIRELY below 0.5,
//! which is the mirror image of the gate's own `rate - ci95 >= 0.5`.
//!
//! That argument is only as good as `ci95()`, and `ci95()`'s own comments record it failing exactly
//! here: `rate + ci95 < 0.5` once marked a match NOBODY PLAYED as "resolved in the worse
//! direction", and 22 fabricated accepts reached the ledger. This file pins the cases where the
//! interval is degenerate, because those are the ones where a screen silently deletes good
//! candidates and nothing downstream can tell.

use pipeline::gate::{screen_rejects, Score};

/// n pairs all drawn, with the game counts a finished match would carry.
fn all_drawn(pairs: u32) -> Score {
    let mut s = Score::default();
    s.pent = [0, 0, pairs, 0, 0];
    s.draws = pairs * 2;
    s
}

/// n pairs all lost.
fn all_lost(pairs: u32) -> Score {
    let mut s = Score::default();
    s.pent = [pairs, 0, 0, 0, 0];
    s.losses = pairs * 2;
    s
}

#[test]
fn an_all_drawn_screen_must_not_reject() {
    // THE CASE THE SCREEN WILL MOSTLY SEE. gate_arithmetic_RESULT.md measures 85.3% of these games
    // as draws, so if an all-drawn match screened out, the screen would delete most of the
    // population on a reading that contains no information at all.
    let s = all_drawn(4);
    // rule of three: 1.5/4 = 0.375, so 0.500 + 0.375 = 0.875.
    assert!((s.pent_rate() - 0.5).abs() < 1e-12);
    assert!((s.ci95() - 0.375).abs() < 1e-9, "expected the 1.5/n placeholder, got {}", s.ci95());
    assert!(!screen_rejects(&s, 4), "an all-drawn match carries no evidence and must survive");
}

#[test]
fn an_all_lost_finished_match_is_rejected() {
    // The case the screen EXISTS for: eight games, every one lost. If this did not reject, the
    // screen would be inert and the compute it is meant to save would not be saved.
    let s = all_lost(4);
    assert!(screen_rejects(&s, 4), "0.000 over a finished 4-pair match must screen out");
}

#[test]
fn an_unplayed_match_must_not_reject() {
    // `Score::default()` — the shape written on a skipped match. `ci95()` returns 1.0 for it, so
    // rate + ci95 = 1.0. This is the exact regression that put 22 accepts in the ledger.
    let s = Score::default();
    assert_eq!(s.games(), 0);
    assert!(!screen_rejects(&s, 4), "a match that was never played must not resolve in any direction");
}

#[test]
fn a_forfeited_match_below_two_pairs_must_not_reject() {
    // THE BRANCH THE COMPLETION GUARD EXISTS FOR, and it is not hypothetical: with fewer than two
    // recorded pairs `ci95()` falls back to the binomial, and at p = 0 that is 1.96*sqrt(0) = ZERO
    // WIDTH. Without the guard this reads 0.000 +/- 0.000 and screens out a candidate that
    // forfeited rather than lost.
    let mut s = Score::default();
    s.pent = [1, 0, 0, 0, 0];
    s.losses = 2;
    assert!(s.ci95() <= 1e-12, "precondition: the binomial really is zero-width at p=0, got {}", s.ci95());
    assert!(s.pent_rate() + s.ci95() < 0.5, "precondition: the bare rule WOULD reject this");
    assert!(!screen_rejects(&s, 4), "an unfinished match must not be rejected on a zero-width interval");
}

#[test]
fn the_screen_can_never_reject_what_the_gate_would_accept() {
    // The one-sidedness argument, checked across the whole pentanomial space at 4 pairs rather
    // than asserted in a comment. Gate accepts at `rate - ci95 >= 0.5`; the screen rejects at
    // `rate + ci95 < 0.5`. Those are disjoint for any interval of non-negative width, and ci95()
    // is non-negative everywhere, so no distribution may satisfy both.
    let mut checked = 0;
    for a in 0..=4u32 { for b in 0..=4u32 { for c in 0..=4u32 { for d in 0..=4u32 {
        let e = 4u32.saturating_sub(a + b + c + d);
        if a + b + c + d + e != 4 { continue; }
        let mut s = Score::default();
        s.pent = [a, b, c, d, e];
        // Give it a finished match's game count so the completion term never masks the check.
        s.wins = (d + e) * 2; s.draws = c * 2; s.losses = (a + b) * 2;
        let accepts = s.pent_rate() - s.ci95() >= 0.5;
        let rejects = screen_rejects(&s, 4);
        assert!(!(accepts && rejects),
                "pent {:?} both accepted and screened out: rate {:.3} ci {:.3}",
                s.pent, s.pent_rate(), s.ci95());
        checked += 1;
    }}}}
    assert!(checked > 50, "the sweep degenerated to {checked} cases");
    println!("\n  screen/gate disjointness verified over {checked} pentanomial distributions at 4 pairs\n");
}
