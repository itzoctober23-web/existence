//! A match that played ZERO games has resolved nothing, and must not be able to say otherwise.
//!
//! `gate.rs` already carries this lesson for one degenerate case: when every pair lands in the same
//! bucket the sample variance is 0, and it used to report "0.500 +/- 0.000" -- "a perfectly measured
//! dead heat, from a match that measured nothing at all". The rule-of-three fallback fixed that.
//!
//! The EMPTY case slipped through the same hole. `Score::default()` is an honest zero-game record,
//! written by `main.rs` on the no-match short-circuit (`--gate-every` skipping a generation). For it:
//!   pent_rate() -> rate() -> points()/games().max(1) = 0/1 = 0.0
//!   ci95()      -> n < 2, binomial fallback, 1.96*sqrt(0*(1-0)/1) = 0.0
//! so `resolved_down = rate + ci95 < 0.5` is 0.0 + 0.0 < 0.5 = TRUE. A match that never happened is
//! classified as RESOLVED, and resolved in the WORSE direction.
//!
//! WHY IT MATTERS BEYOND TIDINESS. `main.rs` writes that flag into the ledger as
//! `GateEvidence.resolved`, and the ledger is the queryable record -- the file's own comment says
//! "every quantitative claim made about this loop today came from the ledger". Counting resolved
//! accepts in `ledger.jsonl` returns 22; the true answer is 0, because all 22 played zero games.
//! The flag does not merely omit information, it manufactures it.

use pipeline::gate::Score;

#[test]
fn a_zero_game_score_resolves_nothing() {
    let empty = Score::default();
    assert_eq!(empty.games(), 0, "positive control: Score::default() must be an empty record");

    // The interval from no data cannot be zero. Zero would mean perfect knowledge.
    assert!(
        empty.ci95() >= 0.5,
        "ci95() on a ZERO-GAME score is {:.4}; an interval from no data must span the whole \
         range, not claim certainty. This is what let a match that never happened read as resolved.",
        empty.ci95()
    );

    // The two clauses `main.rs` uses to decide whether the gate resolved anything.
    let resolved_up = empty.pent_rate() - empty.ci95() > 0.5;
    let resolved_down = empty.pent_rate() + empty.ci95() < 0.5;
    assert!(!resolved_up, "a zero-game score reported resolved_up");
    assert!(
        !resolved_down,
        "a zero-game score reported resolved_DOWN -- it was being read as 'the gate resolved this \
         candidate as WORSE', from a match that was never played"
    );

    // NEGATIVE CONTROL: a real, decisive record must still resolve, or this test would pass by
    // making the gate useless rather than honest.
    let mut decisive = Score::default();
    decisive.pent = [0, 0, 0, 2, 30];      // 32 pairs, nearly all 2-0 sweeps
    decisive.wins = 62; decisive.draws = 2; decisive.losses = 0;
    assert!(
        decisive.pent_rate() - decisive.ci95() > 0.5,
        "a decisive 32-pair record must still resolve UP (rate {:.3}, ci95 {:.3}); if it does not, \
         the fix has broken the gate instead of the flag",
        decisive.pent_rate(), decisive.ci95()
    );

    // And a genuinely ambiguous record must resolve NEITHER way.
    let mut tied = Score::default();
    tied.pent = [1, 6, 18, 6, 1];
    tied.wins = 16; tied.draws = 32; tied.losses = 16;
    assert!(tied.pent_rate() - tied.ci95() <= 0.5 && tied.pent_rate() + tied.ci95() >= 0.5,
            "a balanced record must remain unresolved");
}
