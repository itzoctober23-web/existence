//! The sequential test decides how much evidence to gather. FITNESS 7.2 splits the gate into
//! three parts with three different answers, and this is the ENGINE-DECIDED one: "a candidate
//! near a bound gets thousands of pairs, an obvious dud a few hundred; nobody picks the count,
//! the evidence does."
//!
//! Tested on constructed pair distributions rather than played games, so the STATISTIC is
//! verified independently of the thing it will be used to measure.

use pipeline::gate::{elo_to_score, Score, LLR_BOUND};

fn pent(p: [u32; 5]) -> Score {
    let mut s = Score::default();
    s.pent = p;
    s
}

#[test]
fn a_clearly_winning_candidate_crosses_the_accept_bound() {
    // 200 pairs skewed hard toward WW.
    let s = pent([2, 8, 40, 80, 70]);
    let llr = s.llr(0.0, 5.0);
    assert!(llr > LLR_BOUND, "a dominant result should accept; llr {llr:.2}");
}

#[test]
fn a_clearly_losing_candidate_crosses_the_reject_bound() {
    let s = pent([70, 80, 40, 8, 2]);
    let llr = s.llr(0.0, 5.0);
    assert!(llr < -LLR_BOUND, "a dominated result should reject; llr {llr:.2}");
}

#[test]
fn a_dead_heat_stays_inconclusive() {
    // Symmetric spread centred on 0.5: real games, no signal. This must NOT cross either
    // bound — the whole point of a sequential test is that it keeps looking rather than
    // guessing, and a gate that concludes from a tie is the degenerate one.
    let s = pent([20, 40, 80, 40, 20]);
    let llr = s.llr(0.0, 5.0);
    assert!(llr.abs() < LLR_BOUND, "a dead heat must not conclude; llr {llr:.2}");
}

#[test]
fn zero_variance_reports_no_evidence_rather_than_infinite_evidence() {
    // Every pair identical. The variance is 0 and the naive formula divides by it. Reporting
    // an infinite LLR from the least informative result the gate can produce is the same
    // failure the ci95 rule-of-three fix exists for.
    let s = pent([0, 0, 60, 0, 0]);
    let llr = s.llr(0.0, 5.0);
    assert!(llr.is_finite(), "llr must be finite on zero variance, got {llr}");
    assert_eq!(llr, 0.0, "zero observed variance is NO evidence, not certainty");
}

#[test]
fn more_evidence_moves_the_llr_further_from_zero() {
    // The same result at 4x the pairs must be 4x more convincing, or the statistic is not
    // accumulating evidence and a sequential stop would be meaningless.
    let small = pent([1, 4, 20, 40, 35]);
    let big = pent([4, 16, 80, 160, 140]);
    let (a, b) = (small.llr(0.0, 5.0), big.llr(0.0, 5.0));
    assert!(b > a * 3.5, "llr should scale with n: {a:.3} -> {b:.3}");
}

#[test]
fn elo_to_score_is_the_standard_logistic() {
    assert!((elo_to_score(0.0) - 0.5).abs() < 1e-12);
    assert!(elo_to_score(400.0) > 0.9 && elo_to_score(400.0) < 0.91);
    assert!((elo_to_score(-100.0) + elo_to_score(100.0) - 1.0).abs() < 1e-12,
            "must be symmetric about 0.5");
}
