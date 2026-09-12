//! `best_move_budget` — the iterate-to-budget primitive `structural_next_PREREG.md` Candidate A
//! needs, and the properties that make it usable as a datagen labeller.
//!
//! These assert rather than print. The one that matters most is `realised_depth_varies`: it is the
//! pre-registered gating check for the whole node-budget candidate. If realised depth came out
//! constant, a budget would have nothing to reallocate and the experiment would be measuring
//! nothing (`structural_next_PREREG.md:92-96`).

use board::Position;
use nnue::Net;
use pipeline::datagen::Rng;
use pipeline::search::{Searcher, INF};

/// Walk a few random plies to get a position that is not the start position.
fn pos_after(plies: usize, seed: u64) -> Position {
    let mut p = Position::startpos();
    let mut r = Rng(seed);
    for _ in 0..plies {
        let l = p.legal_moves();
        if l.is_empty() {
            break;
        }
        let m = l.as_slice()[r.below(l.len())];
        p.make_move(m);
    }
    p
}

/// A budget must ALWAYS return a completed answer. `-INF` written into a label becomes
/// `tanh(-32000/600) = -1.0`, a confidently-lost label on a position nobody evaluated --
/// the exact failure `datagen.rs:189` guards against in the capped path.
#[test]
fn never_returns_an_incomplete_score() {
    let net = Net::random(32, 20260912);
    let mut s = Searcher::with_seed(1);
    // Deliberately brutal budgets, including ones far too small to finish depth 1.
    for budget in [1u64, 2, 7, 50, 500, 5_000] {
        for ply in [0usize, 8, 20] {
            let mut p = pos_after(ply, 99 + ply as u64);
            if p.legal_moves().is_empty() {
                continue;
            }
            let (mv, sc, d) = s.best_move_budget(&mut p, &net, budget, 8, 7);
            assert!(sc > -INF, "budget {budget} ply {ply}: returned -INF (incomplete score)");
            assert!(mv != board::types::MOVE_NONE, "budget {budget} ply {ply}: no move");
            assert!(d >= 1, "budget {budget} ply {ply}: realised depth {d} < 1");
        }
    }
}

/// THE PRE-REGISTERED GATING CHECK. At one fixed budget, realised depth must differ across
/// positions -- that is the whole mechanism. `datagen_node_census_RESULT.md` measures the cost of a
/// depth-3 move varying 18-25x (p90/p10, CV ~0.80), so at a fixed budget the cheap positions must
/// reach deeper than the expensive ones.
#[test]
fn realised_depth_varies_across_positions() {
    let net = Net::random(32, 20260912);
    let mut s = Searcher::with_seed(5);
    let budget = 5_269; // the measured MEAN nodes/move of depth-3 datagen
    let mut depths = Vec::new();
    for i in 0..40u64 {
        let mut p = pos_after((i % 25) as usize + 2, 1000 + i);
        if p.legal_moves().is_empty() {
            continue;
        }
        let (_, _, d) = s.best_move_budget(&mut p, &net, budget, 10, i + 1);
        depths.push(d);
    }
    assert!(depths.len() >= 20, "too few positions sampled: {}", depths.len());
    let lo = *depths.iter().min().unwrap();
    let hi = *depths.iter().max().unwrap();
    assert!(
        hi > lo,
        "realised depth was CONSTANT at {lo} across {} positions -- the node-budget mechanism is \
         absent and structural_next Candidate A is measuring nothing",
        depths.len()
    );
}

/// More budget must never buy less depth.
#[test]
fn depth_is_monotone_in_budget() {
    let net = Net::random(32, 20260912);
    let mut s = Searcher::with_seed(9);
    for ply in [2usize, 10, 18] {
        let mut last = 0u32;
        for budget in [200u64, 1_000, 5_000, 40_000] {
            let mut p = pos_after(ply, 5150 + ply as u64);
            if p.legal_moves().is_empty() {
                continue;
            }
            let (_, _, d) = s.best_move_budget(&mut p, &net, budget, 10, 3);
            assert!(d >= last, "ply {ply}: budget {budget} gave depth {d} < previous {last}");
            last = d;
        }
    }
}

/// The budget is a BUDGET. Overshoot is bounded by one completed iteration, never unbounded:
/// a depth that would exceed the budget is aborted and discarded, so the only spend past the
/// line is the aborted iteration's partial work.
#[test]
fn spend_is_bounded() {
    let net = Net::random(32, 20260912);
    let mut s = Searcher::with_seed(11);
    for budget in [500u64, 2_000, 10_000] {
        let mut p = pos_after(12, 777);
        let _ = s.best_move_budget(&mut p, &net, budget, 10, 4);
        // The abort check fires before counting each node, so the hard ceiling is the budget
        // itself; allow a small slack for the root frames that unwind after the flag is set.
        assert!(
            s.nodes <= budget + 64,
            "budget {budget} but spent {} nodes",
            s.nodes
        );
    }
}

/// REGRESSION GUARD. `best_move_capped` is used by `gate.rs:453` (the equal-cost architecture
/// gate) and must keep its fixed-depth abort semantics: a capped search is NOT iterative and must
/// not silently start deepening.
#[test]
fn capped_search_still_does_not_iterate() {
    let net = Net::random(32, 20260912);

    // FRESH SEARCHER PER CALL. `Searcher` carries `rng` and shuffles child move order on every
    // visit (`shuffle_children` defaults true, search.rs:110), so two identical calls on the SAME
    // searcher explore different orderings and legitimately differ in node count. The first
    // version of this test reused one searcher, measured 218 then 189, and blamed iteration --
    // it was measuring searcher state carryover. Node counts are only comparable from equal state.
    let mut a = Searcher::with_seed(3);
    let mut p = pos_after(6, 4242);
    let _ = a.best_move_capped(&mut p, 2, &net, u64::MAX, 1);
    let d2 = a.nodes;

    // Same search from identical state, with a cap far above what it needs. If it had become
    // iterative it would keep deepening and spend more.
    let mut b = Searcher::with_seed(3);
    let mut q = pos_after(6, 4242);
    let _ = b.best_move_capped(&mut q, 2, &net, d2 * 100, 1);
    assert_eq!(b.nodes, d2, "best_move_capped started iterating; gate.rs depends on it not doing so");
}
