//! THE TEST THAT WOULD HAVE CAUGHT IT.
//!
//! `crates/interp/tests/` did not exist. The reference programs — the things that calibrate the
//! entire declared prior in GRAMMAR 6, and therefore every claim of the form "the engine chose
//! alpha-beta over MCTS" — had no tests at all. That is how `ab_hash` sat in the table labelled
//! `faithful` while being a program that returned the constant 0 at every leaf and never called
//! `eval` once, for however long it had been there.
//!
//! The check that would have caught it in one line is `assert!(evals > 0)`.
//!
//! Node counts are NOT asserted here. They are supposed to change when a program is made more
//! faithful — pinning them would turn "I improved the encoding" into a test failure and create
//! pressure to keep an encoding wrong. What is asserted is the BEHAVIOUR that makes a node
//! count meaningful in the first place: that the program does the thing its name claims.

use board::{Outcome, Position};
use grammar::reference;
use interp::Interp;
use nnue::Net;

/// Random-walk positions with at least one legal move.
fn positions(n: usize, seed: u64) -> Vec<Position> {
    let mut rng = seed | 1;
    let mut rnd = || { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; rng };
    let mut out = Vec::new();
    while out.len() < n {
        let mut p = Position::startpos();
        let plies = 4 + (rnd() % 16) as usize;
        let mut ok = true;
        for _ in 0..plies {
            let l = p.legal_moves();
            if l.is_empty() { ok = false; break; }
            p.make_move(l.as_slice()[(rnd() % l.len() as u64) as usize]);
        }
        if ok && !p.legal_moves().is_empty() { out.push(p); }
    }
    out
}

#[test]
fn every_reference_program_returns_a_legal_move() {
    let net = Net::random(32, 4242);
    for p0 in positions(20, 0xA1) {
        for (name, prog) in reference::all() {
            let mut it = Interp::new(&net, vec![3, 32_000]);
            let m = it.run(&prog, &p0, 24);
            assert!(
                p0.legal_moves().as_slice().contains(&m),
                "{name} returned a move that is not legal in the position it was given"
            );
        }
    }
}

#[test]
fn eval_based_programs_actually_evaluate() {
    // THE REGRESSION GUARD. A search that never calls `eval` is not searching, whatever its
    // cost accounting says. `ab_hash` failed exactly this and nothing noticed: it returned an
    // empty hash slot's score (the constant 0) at every leaf, measured 0 evals at depths 2, 3
    // and 4, and looked 14x CHEAPER than alpha-beta for it.
    let net = Net::random(32, 99);
    for (name, prog) in reference::all() {
        // Proof-number search is excluded by DESIGN, not by exception: it proves or disproves
        // from terminal values and calls `eval` nowhere. FITNESS 3 relies on that fact — it is
        // why mates-per-cost is denominated in COST rather than evaluations, since a per-eval
        // denominator would divide by zero for exactly this program.
        if name.contains("proof") { continue; }
        let mut total = 0u64;
        for p0 in positions(6, 0xB2) {
            let mut it = Interp::new(&net, vec![3, 32_000]);
            it.run(&prog, &p0, 24);
            total += it.evals;
        }
        assert!(
            total > 0,
            "{name} performed ZERO evaluations across 6 positions — it is not searching, \
             it is returning something constant (this is the ab_hash bug)"
        );
    }
}

#[test]
fn proof_number_search_evaluates_nothing() {
    // The other half of the same guard: PN's eval-free property is DEPENDED ON by FITNESS 3, so
    // it has to break loudly if someone "helpfully" adds an eval call to it.
    let net = Net::random(32, 7);
    let pn = reference::all().into_iter().find(|(n, _)| n.contains("proof")).expect("PN missing");
    let mut total = 0u64;
    for p0 in positions(6, 0xC3) {
        let mut it = Interp::new(&net, vec![3, 32_000]);
        it.run(&pn.1, &p0, 24);
        total += it.evals;
    }
    assert_eq!(total, 0, "proof-number search called eval; FITNESS 3's cost denominator assumes it does not");
}

#[test]
fn the_transposition_table_does_not_change_alpha_betas_answer() {
    // Alpha-beta's soundness theorem: a correct TT changes the COST of the search, never its
    // value. Disagreement means the table is returning a bound outside the window it was
    // searched with, or another position's data entirely.
    //
    // Deliberately at several depths: the broken version agreed 0/60, 2/60 and 1/60, so a
    // single shallow depth could have been mistaken for bad luck.
    let net = Net::random(64, 20260907);
    let bare = reference::bare_alpha_beta();
    let hash = reference::ab_hash();
    for depth in [2i64, 3] {
        let mut agree = 0;
        let ps = positions(15, 0xD4);
        for p0 in &ps {
            let mut a = Interp::new(&net, vec![depth, 32_000]);
            let mut b = Interp::new(&net, vec![depth, 32_000]);
            if a.run(&hash, p0, depth) == b.run(&bare, p0, depth) { agree += 1; }
        }
        assert_eq!(
            agree, ps.len(),
            "at depth {depth} the hash program disagreed with bare alpha-beta on {} of {} \
             positions; a sound TT cannot change the value",
            ps.len() - agree, ps.len()
        );
    }
}

#[test]
fn the_transposition_table_earns_its_keep() {
    // Not just sound — USEFUL. The repaired version must search strictly fewer leaves than bare
    // alpha-beta, because it can cut on stored bounds. If this ever reaches parity the store is
    // dead code again, which is how the original bug half-lived: `Node::seq` desugars to nested
    // `Let`, so a `ret` in the body unwound past the trailing store and the table stayed empty.
    let net = Net::random(64, 20260907);
    let bare = reference::bare_alpha_beta();
    let hash = reference::ab_hash();
    let (mut e_hash, mut e_bare) = (0u64, 0u64);
    for p0 in positions(15, 0xE5) {
        let mut a = Interp::new(&net, vec![3, 32_000]);
        a.run(&hash, &p0, 3);
        e_hash += a.evals;
        let mut b = Interp::new(&net, vec![3, 32_000]);
        b.run(&bare, &p0, 3);
        e_bare += b.evals;
    }
    assert!(e_bare > 0 && e_hash > 0, "both arms must actually evaluate");
    assert!(
        e_hash < e_bare,
        "the TT saved nothing: {e_hash} evals vs bare {e_bare}. Either the store is dead code \
         or nothing ever transposes"
    );
}

#[test]
fn terminal_positions_are_handled_without_searching() {
    // A mated position has no legal moves. Every program must survive being handed one rather
    // than indexing an empty list — the seed's own terminal guard is the first thing in its
    // body and an evolved edit could move it.
    let mated = Position::from_fen("7k/5KQ1/8/8/8/8/8/8 b - - 0 1").expect("valid fen");
    assert!(mated.legal_moves().is_empty() && mated.outcome() == Outcome::Loss,
            "fixture must actually be checkmate");
    let net = Net::random(32, 1);
    for (name, prog) in reference::all() {
        let mut it = Interp::new(&net, vec![3, 32_000]);
        let _ = it.run(&prog, &mated, 8); // must not panic
        let _ = name;
    }
}
