//! Every reference program must type-check in its own grammar. If the SEED does not, the
//! grammar cannot express the thing it is seeded with -- exactly the defect the first
//! self-audit found (the seed used `>=` and `!=`, which cmp did not have).
use grammar::{reference, typecheck};

#[test]
fn all_reference_programs_typecheck() {
    for (name, p) in reference::all() {
        typecheck::check_program(&p)
            .unwrap_or_else(|e| panic!("{name} does not type-check: {}", e.what));
    }
}

#[test]
fn entry_signature_is_enforced() {
    let mut p = reference::bare_alpha_beta();
    p.funcs[0].ret = grammar::Ty::Score;
    assert!(typecheck::check_program(&p).is_err(), "wrong entry return type accepted");
}

#[test]
fn outcome_cannot_be_compared_to_a_number() {
    // GRAMMAR 2.4 #15: no Outcome-to-number coercion. That is what keeps "draw = 0" out of the
    // Given column -- it must go through the learned score_of table.
    use grammar::ast::*;
    let p = Program {
        funcs: vec![Func {
            name: "choose".into(),
            params: vec![("p".into(), Ty::Pos), ("B".into(), Ty::Int)],
            ret: Ty::Move,
            body: Node::If(
                Box::new(Node::Cmp(
                    Box::new(Node::Terminal(Box::new(Node::Var("p".into())))),
                    Box::new(Node::Const(0)),
                    Rel::Eq,
                )),
                Box::new(Node::Nop),
                None,
            ),
        }],
        lineage: Lineage::Main,
    };
    assert!(typecheck::check_program(&p).is_err(), "Outcome compared to Int was accepted");
}
