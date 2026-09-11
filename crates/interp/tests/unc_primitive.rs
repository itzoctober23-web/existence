//! `unc(p) -> Int` — the Given row, checked end to end.
//!
//! The point of exposing it is that a program can READ the net's uncertainty. Four things have to
//! hold for that to be true rather than declared, and the 4PC side supplied a cautionary example
//! today: `hybridWidenBase` is documented at length in a header, registered nowhere, and the engine
//! answers "unknown search param" for it. A primitive that type-checks but is mispriced, or that
//! never reaches the interpreter, is the same failure in a different language.

use board::chess::Position;
use grammar::ast::*;
use grammar::typecheck;
use interp::Interp;
use nnue::Net;

fn tables() -> Vec<i64> { vec![3, 32_000, interp::uct_exploration()] }

/// `choose(p, B) = argmax(moves(p), m -> score_of(DRAW, unc(p)))` — the unc read is the only thing
/// that can distinguish two programs built by this helper.
fn prog_reading_unc() -> Program {
    Program {
        funcs: vec![Func {
            name: "choose".into(),
            params: vec![("p".into(), Ty::Pos), ("B".into(), Ty::Int)],
            ret: Ty::Move,
            body: Node::Argmax(
                Box::new(Node::Moves(Box::new(Node::Var("p".into())))),
                "m".into(),
                Box::new(Node::ScoreOf(
                    Box::new(Node::OutcomeLit(OutcomeLit::Draw)),
                    Box::new(Node::Unc(Box::new(Node::Var("p".into())))),
                )),
            ),
        }],
        lineage: Lineage::Main,
    }
}

#[test]
fn unc_is_an_int_and_type_checks_in_a_real_program() {
    let p = prog_reading_unc();
    assert!(typecheck::check_program(&p).is_ok(),
            "a program reading unc must type-check: {:?}",
            typecheck::check_program(&p).err().map(|e| e.what));

    // Int, NOT Score. A spread is not a position evaluation and must not be substitutable for one.
    let f = &p.funcs[0];
    let t = typecheck::type_of_in(&Node::Unc(Box::new(Node::Var("p".into()))), f, &p);
    assert_eq!(t, Some(Ty::Int), "unc must be Int so it cannot stand in for an eval");
}

#[test]
fn unc_is_wrong_typed_on_a_non_position() {
    // The argument is Pos. Handing it an Int must be rejected at generation time, which is the
    // cheap path -- a mutation that builds this is discarded before it ever reaches a gate.
    let mut p = prog_reading_unc();
    p.funcs[0].body = Node::Argmax(
        Box::new(Node::Moves(Box::new(Node::Var("p".into())))),
        "m".into(),
        Box::new(Node::ScoreOf(
            Box::new(Node::OutcomeLit(OutcomeLit::Draw)),
            Box::new(Node::Unc(Box::new(Node::Budget))),   // Int, not Pos
        )),
    );
    assert!(typecheck::check_program(&p).is_err(), "unc(Int) must not type-check");
}

#[test]
fn unc_reads_zero_on_an_untrained_head_and_non_zero_on_a_trained_one() {
    let pos = Position::startpos();
    let prog = prog_reading_unc();

    // A net with no trained head -- every net written before 2026-09-11 -- reads a constant 0.
    let base = Net::random(64, 20260911);      // >= INCREMENTAL_MIN_WIDTH, so the acc path is live
    let mut sc = Vec::new();
    assert_eq!(base.spread(&pos, &mut sc), 0, "an untrained head must read 0");
    let m0 = Interp::new(&base, tables()).run(&prog, &pos, 4);

    // Populate the head; now the same program is reading something real.
    let mut trained = base.clone();
    trained.bu = 4.0;
    for (i, w) in trained.wu.iter_mut().enumerate() { *w = 0.05 * (i as f32 + 1.0); }
    assert!(trained.spread(&pos, &mut sc) > 0, "positive control: a populated head must read > 0");
    let m1 = Interp::new(&trained, tables()).run(&prog, &pos, 4);

    // Both must still produce a legal move -- the point is that the read works, not that it helps.
    assert_ne!(m0, board::types::MOVE_NONE, "program forfeited on the untrained net");
    assert_ne!(m1, board::types::MOVE_NONE, "program forfeited on the trained net");
}

#[test]
fn unc_costs_an_eval_not_a_leaf() {
    // cost_of ends in `_ => 2`, so a new node that reads the net is charged 2 unless it is listed.
    // FITNESS 3 is mates per COST: an 80x underpriced primitive is a standing invitation for the
    // search to spend everything on it for free. Measured through the interpreter rather than by
    // reading the table, so the assertion covers the path that actually runs.
    let net = Net::random(64, 4);
    let pos = Position::startpos();
    let leaf = Program {
        funcs: vec![Func { name: "choose".into(),
            params: vec![("p".into(), Ty::Pos), ("B".into(), Ty::Int)], ret: Ty::Move,
            body: Node::Argmax(Box::new(Node::Moves(Box::new(Node::Var("p".into())))), "m".into(),
                Box::new(Node::ScoreOf(Box::new(Node::OutcomeLit(OutcomeLit::Draw)),
                                       Box::new(Node::Const(1))))) }],
        lineage: Lineage::Main };

    let mut a = Interp::new(&net, tables()); a.run(&leaf, &pos, 4);
    let mut b = Interp::new(&net, tables()); b.run(&prog_reading_unc(), &pos, 4);
    assert!(b.cost > a.cost,
            "reading unc must cost more than reading a constant: {} vs {}", b.cost, a.cost);
}

#[test]
fn unc_survives_an_sexp_round_trip() {
    // Programs are persisted and re-read as s-expressions; a node the writer emits and the reader
    // cannot parse would silently truncate a saved program.
    let p = prog_reading_unc();
    let text = grammar::sexp::to_string(&p);
    assert!(text.contains("unc"), "the writer must emit unc: {text}");
    let back = grammar::sexp::from_str(&text).expect("reader must parse unc");
    assert_eq!(back, p, "unc did not survive a round trip");
}

#[test]
fn the_carried_accumulator_gives_the_same_spread_as_a_fresh_recompute() {
    // THE INVARIANT THAT MAKES `unc(p)` TRUSTWORTHY INSIDE SEARCH.
    //
    // `spread_with` reads the CARRIED hidden layer when one is live and only falls back to
    // `net.spread(&pos)` when it is empty. Those two paths must agree, for the same reason
    // `incremental.rs` asserts refresh() == update(): if the carried accumulator can drift from the
    // position it is supposed to describe, `unc(p)` returns a plausible number for the WRONG
    // position. Nothing downstream could detect that -- a spread has no legality or sign check to
    // violate, so a stale read would look exactly like a real one.
    //
    // THIS TEST MUST NOT BE RUN ON A ZERO HEAD. With an untrained head both sides return 0 and the
    // assertion passes while proving nothing, which is the same vacuous-pass trap that let a broken
    // column guard report "n/a" instead of failing. So the head is populated first and the test
    // asserts the values actually VARY before it trusts the equality.
    use board::types::MOVE_NONE;
    use interp::{Delta, PosAcc};

    let mut net = Net::random(64, 20260912);
    net.bu = 3.0;
    for (i, w) in net.wu.iter_mut().enumerate() {
        *w = 0.03 * ((i % 7) as f32 + 1.0);
    }

    let mut scratch = Vec::new();
    let mut delta = Delta::new();
    let mut seen = Vec::new();
    let mut checked = 0usize;

    // Walk a few plies of real children so the accumulator is UPDATED rather than rebuilt.
    let root = PosAcc::fresh(&net, Position::startpos());
    let mut frontier = vec![root];
    for _ply in 0..3 {
        let mut next = Vec::new();
        for pa in &frontier {
            let ml = pa.pos.legal_moves();
            for &mv in ml.as_slice().iter().take(6) {
                if mv == MOVE_NONE {
                    continue;
                }
                let child = pa.child(&net, mv, &mut delta);

                let carried = child.spread_with(&net, &mut scratch);
                let fresh = net.spread(&child.pos, &mut scratch);
                assert_eq!(
                    carried, fresh,
                    "carried accumulator disagrees with a fresh recompute: \
                     unc(p) would report a real number for the WRONG position"
                );
                seen.push(carried);
                checked += 1;
                if next.len() < 8 {
                    next.push(child);
                }
            }
        }
        frontier = next;
    }

    assert!(checked > 50, "only {checked} positions checked -- too few to mean anything");
    let lo = *seen.iter().min().unwrap();
    let hi = *seen.iter().max().unwrap();
    assert!(
        hi > lo,
        "every spread was identical ({lo}) -- the head is degenerate here, so the equality above \
         proved nothing. Populate the head before trusting this test."
    );
}
