//! The gate's interval has to be honest about the case where it learned nothing.

use pipeline::gate::Score;

fn from_pent(pent: [u32; 5]) -> Score {
    let mut s = Score::default();
    s.pent = pent;
    // games are irrelevant to the pentanomial interval, but keep them consistent
    let n: u32 = pent.iter().sum();
    s.draws = n * 2;
    s
}

#[test]
fn all_draws_is_no_information_not_certainty() {
    // 12 pairs, every one DD. The sample variance is exactly zero, and the first version
    // reported "0.500 +/- 0.000" -- a perfectly measured dead heat from a match that measured
    // nothing. Any caller asking "ci95 < 0.05, can this gate resolve?" was told yes.
    let s = from_pent([0, 0, 12, 0, 0]);
    assert!((s.pent_rate() - 0.5).abs() < 1e-12);
    assert!(s.ci95() > 0.0, "zero observed variance must not report zero uncertainty");
    assert!(s.ci95() > 0.05, "12 all-drawn pairs must read as UNRESOLVED, got +/-{:.4}", s.ci95());
    // rule of three: 1.5/n
    assert!((s.ci95() - 1.5 / 12.0).abs() < 1e-9);
}

#[test]
fn a_sweep_is_also_zero_variance_and_must_not_read_as_infinite_confidence() {
    // Every pair a 2-0 sweep. Real, but 4 pairs of it is still thin evidence.
    let s = from_pent([0, 0, 0, 0, 4]);
    assert!((s.pent_rate() - 1.0).abs() < 1e-12);
    assert!(s.ci95() > 0.0);
    assert!((s.ci95() - 1.5 / 4.0).abs() < 1e-9);
}

#[test]
fn a_spread_result_uses_the_pair_variance() {
    // Mixed outcomes: the interval comes from the spread of pairs, not the rule of three.
    // Pair values 0, 0.5, 1, 1.5, 2 with counts 2,3,10,3,2 -> mean 1.0, var 5.5/19.
    let s = from_pent([2, 3, 10, 3, 2]);
    let n: f64 = 20.0;
    let var: f64 = 5.5 / 19.0;
    let expect = 1.96 * (var / n).sqrt() / 2.0;
    assert!((s.ci95() - expect).abs() < 1e-12, "got {:.6}, expected {expect:.6}", s.ci95());
    // And it is WIDER than the zero-variance bound, which is the correct ordering: observing
    // actual spread is less certain than observing none. (The first draft of this test asserted
    // the opposite and was wrong -- the rule-of-three value is a floor on honesty, not a
    // ceiling on precision.)
    assert!(s.ci95() > 1.5 / n);
}

#[test]
fn more_pairs_of_the_same_all_draw_result_narrows_the_bound() {
    // The bound must still improve with evidence; it is uninformative, not frozen.
    let few = from_pent([0, 0, 10, 0, 0]);
    let many = from_pent([0, 0, 200, 0, 0]);
    assert!(many.ci95() < few.ci95());
}
