//! `Op::AddFn` — the only operator that can raise `funcs.len()`.
//!
//! `shape_reachability.rs` measured **0 of 858** applied mutations changing the function count while
//! `typecheck.rs:19` admits 1..4, so three quarters of the declared program space was unreachable.
//! This is the operator that closes it. It is PARKED (see `mutate::PARKED_OPS`): a refactor is
//! behaviour-preserving, so every candidate is its parent plus a `Call` node costing 2, which is
//! strictly worse under mates-per-cost. Its value is as an enabler, not as a candidate.

use grammar::{ast::*, mutate, reference, typecheck};

#[test]
fn add_fn_raises_the_function_count_and_stays_well_scoped() {
    let mut raised = 0usize;
    let mut applied = 0usize;
    for (name, prog) in reference::all() {
        for fi in 0..prog.funcs.len() {
            for k in 0..200 {
                let mut rng = mutate::Rng::new(k as u64 * 31 + 7);
                let Some(m) = mutate::mutate_at(&prog, mutate::Op::AddFn, &mut rng, fi, k) else { continue };
                applied += 1;
                assert!(m.funcs.len() > prog.funcs.len(),
                        "{name}: AddFn applied without adding a function");
                // The whole hazard of lifting a subtree is that its free variables lose their
                // binder. They must have become PARAMETERS.
                assert!(typecheck::scope_check(&m).is_ok(),
                        "{name}: AddFn produced an unbound read: {:?}",
                        typecheck::scope_check(&m).err().map(|e| e.what));
                assert!(typecheck::check_program(&m).is_ok(), "{name}: AddFn produced an ill-typed program");
                if m.funcs.len() > prog.funcs.len() { raised += 1; }
            }
        }
    }
    println!("\n  AddFn applied {applied} times, raised funcs.len() {raised} times \
              (was 0 of 858 before this operator existed)\n");
    assert!(applied > 0, "AddFn never applied anywhere -- the gap it closes is still open");
}

/// `Ret` unwinds a FRAME and `Node::Call` swallows a callee's `Flow::Ret`, so lifting a subtree
/// containing one silently changes which function returns. That is a semantic change wearing a
/// refactor's clothes, and the operator must refuse it.
#[test]
fn add_fn_never_lifts_a_ret() {
    fn has_ret(n: &Node) -> bool {
        if matches!(n, Node::Ret(_)) { return true }
        match n {
            Node::Moves(a) | Node::Terminal(a) | Node::Key(a) | Node::Eval(a) | Node::Ret(a)
            | Node::Probe(a) | Node::Field(a, _) | Node::Set(_, a) => has_ret(a),
            Node::Apply(a, b) | Node::Max(a, b) | Node::Min(a, b) | Node::Avg(a, b)
            | Node::ScoreOf(a, b) | Node::Cmp(a, b, _) | Node::Pred(a, b, _) | Node::Loop(a, b)
            | Node::Store(a, _, b) | Node::Foreach(a, _, b) | Node::Argmax(a, _, b)
            | Node::Sort(a, _, b) | Node::Sample(a, _, b) | Node::Let(_, a, b) => has_ret(a) || has_ret(b),
            Node::Mix(a, b, c) => has_ret(a) || has_ret(b) || has_ret(c),
            Node::If(c, t, e) => has_ret(c) || has_ret(t) || e.as_ref().is_some_and(|x| has_ret(x)),
            Node::Call(_, xs) | Node::Arith(_, xs) | Node::TRead(_, xs) => xs.iter().any(has_ret),
            _ => false,
        }
    }
    for (name, prog) in reference::all() {
        for fi in 0..prog.funcs.len() {
            for k in 0..200 {
                let mut rng = mutate::Rng::new(k as u64 * 31 + 7);
                let Some(m) = mutate::mutate_at(&prog, mutate::Op::AddFn, &mut rng, fi, k) else { continue };
                let lifted = m.funcs.last().expect("a function was added");
                assert!(!has_ret(&lifted.body),
                        "{name}: AddFn lifted a subtree containing Ret, which changes which frame returns");
            }
        }
    }
}
