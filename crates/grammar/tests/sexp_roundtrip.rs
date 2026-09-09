//! `read(write(p)) == p` for every reference program, plus the edge cases a tree walker gets wrong.
//!
//! The point of the format is not that it produces a file -- `{:#?}` already did that, and it is
//! exactly what made every evolved champion unrecoverable. The point is that the tree survives the
//! trip. So the assertion is structural equality, not a byte comparison of the text.
use grammar::ast::*;
use grammar::{reference, sexp};

#[test]
fn every_reference_program_round_trips() {
    for (name, p) in reference::all() {
        let text = sexp::to_string(&p);
        let back = sexp::from_str(&text)
            .unwrap_or_else(|e| panic!("{name}: parse failed: {e}\n---\n{text}"));
        assert_eq!(p, back, "{name}: round-trip changed the tree");
        // and it must be STABLE: writing the recovered tree gives the same text
        assert_eq!(text, sexp::to_string(&back), "{name}: second write differs");
    }
}

#[test]
fn node_count_survives_the_trip() {
    // A tree can compare equal only if it is the same tree, but node count is the quantity GRAMMAR 6
    // actually reports, so it is asserted directly rather than trusted to follow.
    for (name, p) in reference::all() {
        let back = sexp::from_str(&sexp::to_string(&p)).unwrap();
        let a: usize = p.funcs.iter().map(|f| f.body.size()).sum();
        let b: usize = back.funcs.iter().map(|f| f.body.size()).sum();
        assert_eq!(a, b, "{name}: node count changed {a} -> {b}");
    }
}

#[test]
fn if_without_else_stays_without_else() {
    // The one variable-arity form in the grammar. A parser that reads the closing paren as an
    // else-branch, or writes `nop` for a missing one, silently changes program semantics.
    let p = Program {
        lineage: Lineage::Main,
        funcs: vec![Func {
            name: "f".into(), params: vec![("p".into(), Ty::Pos)], ret: Ty::Unit,
            body: Node::If(Box::new(Node::Const(1)), Box::new(Node::Ret(Box::new(Node::Const(2)))), None),
        }],
    };
    let back = sexp::from_str(&sexp::to_string(&p)).unwrap();
    assert_eq!(p, back);
    match &back.funcs[0].body {
        Node::If(_, _, e) => assert!(e.is_none(), "else-branch was invented"),
        other => panic!("shape changed: {other:?}"),
    }
}

#[test]
fn awkward_identifiers_and_negative_consts_survive() {
    // Var names are written quoted, so a name containing a paren, a quote or a backslash must not
    // break the lexer. Const is i8, so the negative end is a real boundary.
    let p = Program {
        lineage: Lineage::Purity,
        funcs: vec![Func {
            name: "has \"quote\" and \\slash".into(),
            params: vec![("v (paren)".into(), Ty::Int)],
            ret: Ty::Score,
            body: Node::Arith(ArithOp::Add, vec![
                Node::Var("v (paren)".into()),
                Node::Const(-128),
                Node::Const(127),
            ]),
        }],
    };
    let back = sexp::from_str(&sexp::to_string(&p)).unwrap();
    assert_eq!(p, back);
}

#[test]
fn garbage_is_an_error_not_a_panic() {
    // This is also the entry point for a hand-edited file, so it must report rather than unwind.
    for bad in ["", "(", "(program)", "(program Main (func))", "(program Nonsense)",
                "(program Main (func \"f\" () Int (bogus)))"] {
        assert!(sexp::from_str(bad).is_err(), "expected an error for {bad:?}");
    }
}
