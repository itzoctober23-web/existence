//! Does appending an index to a `tread` change what the program DOES?
//!
//! `Op::TReadIndex` (added 2026-09-10) makes `TRead/1` reachable in one edit and `TRead/2` in two,
//! which closes the shape-reachability gap that put ladder rung 7 outside the search space. That is
//! a fact about SHAPES. This file asks the behavioural question, which is separate and was not
//! answered by adding the operator:
//!
//!   `Interp` resolves a TRead by checking `tables_nd` (genuinely indexed tables) FIRST and falling
//!   back to `tables` (scalars). Repo-wide, `tables_nd` is declared, initialised to `Vec::new()`,
//!   and read — **nothing populates it** — and every `Interp::new` call site passes at most three
//!   scalars. So a TRead resolves through the scalar path regardless of how many indices it carries,
//!   and the indices are never consulted.
//!
//! If that is right, appending an index is a NO-OP that costs nodes: same moves, same scores,
//! larger program. An operator that generates those is not neutral — it takes a 1/|ALL_OPS| share
//! of every draw and returns candidates that cannot be fitter, because FITNESS 3 is mates per
//! COST and the cost strictly rose.
//!
//! This test PINS that state so it cannot change silently. When `tables_nd` is populated it must
//! start failing, and that failure is the signal that rung 7 became live.

use board::chess::Position;
use grammar::ast::*;
use interp::Interp;
use nnue::Net;

fn tables() -> Vec<i64> { vec![3, 32_000, interp::uct_exploration()] }

/// `choose(p, B) = argmax(moves(p), m -> score_of(DRAW, <tread>))`, so the tread's value is the
/// only thing that can distinguish two programs built by this helper.
fn prog_with_tread(t: usize, idx: Vec<Node>) -> Program {
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
                    Box::new(Node::TRead(t, idx)),
                )),
            ),
        }],
        lineage: Lineage::Main,
    }
}

#[test]
fn appending_a_tread_index_changes_nothing_because_tables_nd_is_empty() {
    let net = Net::random(16, 12345);

    // PREMISE, asserted rather than assumed: the indexed-table store is empty. Every claim below
    // is downstream of this, so if it ever stops holding the test must stop rather than mislead.
    let probe = Interp::new(&net, tables());
    assert!(
        probe.tables_nd.is_empty(),
        "tables_nd is populated now, so tread indices may actually be consulted. \
         Rung 7 may be LIVE -- re-measure it and update this test's premise."
    );

    let bare = prog_with_tread(0, vec![]);
    let one = prog_with_tread(0, vec![Node::Budget]);
    let two = prog_with_tread(0, vec![Node::Budget, Node::Const(2)]);

    // The programs must genuinely differ as SYNTAX, or "identical behaviour" is vacuous.
    assert_ne!(bare, one, "positive control: the two programs must not be the same program");
    assert!(one.funcs[0].body.size() > bare.funcs[0].body.size(),
            "positive control: appending an index must make the program LARGER");

    let positions = [
        Position::startpos(),
        Position::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1").unwrap(),
        Position::from_fen("r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3").unwrap(),
    ];

    for (i, p) in positions.iter().enumerate() {
        let a = Interp::new(&net, tables()).run(&bare, p, 2);
        let b = Interp::new(&net, tables()).run(&one, p, 2);
        let c = Interp::new(&net, tables()).run(&two, p, 2);
        assert_eq!(
            a, b,
            "position {i}: appending ONE tread index changed the result. If tables_nd is still \
             empty this is a real behavioural difference and the fall-through has changed."
        );
        assert_eq!(a, c, "position {i}: appending TWO tread indices changed the result");
    }
}
