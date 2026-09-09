//! The UCT reference program must SOLVE, not merely run.
//!
//! WHY THIS TEST EXISTS. `uct_mcts` sat in the reference set labelled "faithful" for weeks on the
//! strength of having been COUNTED and READ — the exact state `alpha-beta + hash reuse` was in
//! when it was returning the constant 0. Both were caught by running them. A count is not a check,
//! so the solving bar belongs in `cargo test` where every commit has to clear it.
//!
//! WHAT IT PINS. `Mix(a, b, w)` is `(a*w + b*(16-w))/16`, so the weight that scales the exploration
//! term inside the sqrt ALSO sets that term's blend coefficient — and the coefficient is zero at
//! w = 16 and negative above it. Measured at budget 256 over 23 mate-in-one positions: 20/23 at
//! w=1, 14/23 at w=16 (pure greed), 9/23 at w=64, and 0/23 at w=360000, where the program is
//! actively penalised for exploring. `uct_mcts_sum` selects `argmax(q + u)` and reaches 23/23 from
//! K = 600 upward — 600 being the net's declared eval scale.
//!
//! The test uses a small set and asserts the DIRECTION rather than the exact tally: the full sweep
//! lives in `examples/reference_audit.rs`, and a unit test that takes minutes gets deleted.
use board::{Outcome, Position};
use grammar::reference;
use interp::Interp;
use nnue::Net;

/// A position is mate-in-one if some legal move leaves the opponent checkmated.
fn mate_in_one(p: &Position) -> Option<board::Move> {
    for m in p.legal_moves().as_slice() {
        let mut q = p.clone();
        q.make_move(*m);
        if q.outcome() == Outcome::Loss {
            return Some(*m);
        }
    }
    None
}

fn mate_set(n: usize) -> Vec<(Position, board::Move)> {
    let mut rng = 0xBEEFu64;
    let mut rnd = move || {
        rng ^= rng << 13;
        rng ^= rng >> 7;
        rng ^= rng << 17;
        rng
    };
    let mut out = Vec::new();
    while out.len() < n {
        let mut p = Position::startpos();
        for _ in 0..(6 + rnd() % 26) {
            let l = p.legal_moves();
            if l.is_empty() {
                break;
            }
            p.make_move(l.as_slice()[(rnd() % l.len() as u64) as usize]);
        }
        if let Some(m) = mate_in_one(&p) {
            out.push((p, m));
        }
    }
    out
}

fn solved(prog: &grammar::Program, set: &[(Position, board::Move)], k: i64) -> usize {
    let net = Net::random(16, 7);
    set.iter()
        .filter(|(p, best)| {
            let mut i = Interp::new(&net, vec![2, 32_000, k]);
            i.run(prog, p, 256) == *best
        })
        .count()
}

#[test]
fn sum_selection_solves_forced_mates() {
    let set = mate_set(8);
    let prog = reference::uct_mcts_sum();
    let n = solved(&prog, &set, 600);
    assert_eq!(
        n,
        set.len(),
        "sum-selection UCT must find every mate in one at K=600 (the net's eval scale); got {n}/{}",
        set.len()
    );
}

#[test]
fn mix_selection_is_penalised_above_sixteen() {
    // The regression that matters: `Mix`'s coefficient on the exploration term is `16 - w`, so a
    // large weight INVERTS it. If someone "fixes" MCTS by scaling the weight up, this catches it.
    let set = mate_set(8);
    let prog = reference::uct_mcts();
    let low = solved(&prog, &set, 1);
    let inverted = solved(&prog, &set, 360_000);
    assert!(
        inverted < low,
        "Mix at w=360000 gives u a coefficient of -359984 and must do WORSE than w=1, \
         which weights u at 15/16; got {inverted} vs {low}"
    );
}

#[test]
fn sum_selection_beats_mix_at_the_same_weight() {
    // Same K, one structural difference. This is the substitution that refutes the recorded
    // explanation ("exploration swamps exploitation"): if magnitude were the problem, both would
    // fail here.
    let set = mate_set(8);
    let mix = solved(&reference::uct_mcts(), &set, 360_000);
    let sum = solved(&reference::uct_mcts_sum(), &set, 360_000);
    assert!(
        sum > mix,
        "at identical K the sum encoding must beat the convex blend; got sum {sum} vs mix {mix}"
    );
}
