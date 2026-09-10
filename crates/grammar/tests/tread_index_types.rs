//! `tread` index types must be enforced, because an operator is about to start generating them.
//!
//! GRAMMAR §2.7 primitive 26 declares `tread : Tab x Int... -> Int` — the indices are Int. The
//! checker walked them without constraining them:
//!
//!     TRead(_, args) => { for a in args { check(a, p, env)?; } Ty::Int }
//!
//! `check()` proves each index is well-FORMED; nothing proved it was an Int. That gap is harmless
//! while nothing constructs a tread index (measured 2026-09-10: no operator lengthens any argument
//! list, 0 of 823 applied mutations). It stops being harmless the moment one does, which is the
//! change this test is a precondition for.
//!
//! It matters in the direction that is expensive. `mutate.rs:207` sets the standard: an operator
//! may emit `Var("p")` out of scope because the type checker discards it — "wasted candidates
//! rather than silently wrong ones". An unconstrained index inverts that: the candidate is KEPT,
//! evaluated, and its eval is wrong in a way no gate reports as an error. A search that cannot
//! express a bad program is better than one that scores it.

use grammar::ast::*;
use grammar::typecheck;

/// The minimal well-typed program: `choose(p: Pos, B: Int) -> Move`, which is what
/// `typecheck.rs:26` requires of the entry. `idx` is spliced in as a tread index.
///
/// Shape derived from the checker, not invented: `Argmax(list, var, key) -> Move` (:120),
/// `Moves(Pos) -> List` (:61), `Eval(Pos) -> Score` (:69). The first version of this file guessed
/// a one-function `ab(...) -> Score` and the POSITIVE CONTROL rejected it with "entry returns
/// Score, must be Move" -- which is the control doing its job before any claim was made.
fn prog_with_index(idx: Node) -> Program {
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
                    Box::new(Node::TRead(3, vec![idx])),
                )),
            ),
        }],
        lineage: Lineage::Main,
    }
}

#[test]
fn a_tread_index_must_be_an_int() {
    // POSITIVE CONTROL FIRST. If a well-typed program does not pass, the rejections below prove
    // nothing -- a checker that refuses everything looks identical to one that works.
    let ok = prog_with_index(Node::Var("B".into()));            // B: Int, the entry's Int param
    assert!(
        typecheck::check_program(&ok).is_ok(),
        "POSITIVE CONTROL failed: a TRead indexed by the Int param B must type-check. \
         Everything below is meaningless until this passes. Error: {:?}",
        typecheck::check_program(&ok).err()
    );
    let ok_const = prog_with_index(Node::Const(1));
    assert!(typecheck::check_program(&ok_const).is_ok(), "TRead indexed by a Const must type-check");

    // THE CLAIM. GRAMMAR 2.7 primitive 26 declares `tread : Tab x Int... -> Int`.
    let bad_pos = prog_with_index(Node::Var("p".into()));       // p: Pos
    assert!(
        typecheck::check_program(&bad_pos).is_err(),
        "TRead accepted a Pos as an index; `tread : Tab x Int... -> Int` says it is ill-typed, so \
         it must be DISCARDED rather than evaluated."
    );

    let bad_score = prog_with_index(Node::Eval(Box::new(Node::Var("p".into()))));  // Score
    assert!(
        typecheck::check_program(&bad_score).is_err(),
        "TRead accepted a Score as an index. Score and Int are distinct types here, and conflating \
         them is how a value on the mate scale ends up used as a table index."
    );

    let bad_move = prog_with_index(Node::Var("m".into()));      // m: Move, bound by the Argmax
    assert!(
        typecheck::check_program(&bad_move).is_err(),
        "TRead accepted a Move as an index."
    );

    // An empty index list must stay legal: the seed uses TRead(0, []) and TRead(1, []) for D and INF.
    let empty = Program {
        funcs: vec![Func {
            name: "choose".into(),
            params: vec![("p".into(), Ty::Pos), ("B".into(), Ty::Int)],
            ret: Ty::Move,
            body: Node::Argmax(
                Box::new(Node::Moves(Box::new(Node::Var("p".into())))),
                "m".into(),
                Box::new(Node::ScoreOf(
                    Box::new(Node::OutcomeLit(OutcomeLit::Draw)),
                    Box::new(Node::TRead(0, vec![])),
                )),
            ),
        }],
        lineage: Lineage::Main,
    };
    assert!(
        typecheck::check_program(&empty).is_ok(),
        "TRead with no indices must remain legal -- the seed depends on it."
    );
}
