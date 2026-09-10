//! SHAPE-LEVEL REACHABILITY — the follow-up `reachability.rs:153` names by hand:
//! *"A shape-level reachability check is the follow-up."*
//!
//! Its sibling derives which node KINDS the operators can introduce, and states its own limit
//! honestly: table reduction needs `TRead(3, [d, i])` — table 3 read with two ARGUMENTS — while
//! the seed already contains `TRead(0, [])` and `TRead(1, [])`. At kind granularity that reads as
//! "nothing missing", so the sibling records the rung as NOT PROVEN EITHER WAY and declines to
//! round up. This file supplies the missing granularity.
//!
//! It measures two things the kind-level sweep structurally cannot see, and both correspond to a
//! GRAMMAR 4 operator that is DECLARED but not implemented:
//!
//!   | GRAMMAR 4 row | declared effect | what it would change |
//!   |---|---|---|
//!   | `add-arg` | "add an Int/Score parameter to a function and thread a value at each call site" | a node's ARITY |
//!   | `add-fn`  | "split a subtree into a new function and call it" | `Program::funcs.len()` |
//!
//! Neither exists in `mutate::ALL_OPS` under any spelling (checked 2026-09-10). This test does not
//! assert that from reading the source — hardcoding a reading of `mutate.rs` would only re-assert
//! the belief under test, which is the mistake its sibling's header records. It applies EVERY
//! operator at EVERY position of EVERY reference program and reports what actually came out.
//!
//! WHY THE FUNCTION COUNT MATTERS BEYOND `add-fn`. `typecheck.rs:19` admits programs with 1..4
//! functions. If no operator and no crossover can raise the count, then a search seeded with a
//! 1-function program can only ever produce 1-function programs, and three quarters of the
//! declared program space is unreachable — not unlikely, unreachable. `Call` is in the constructible
//! kind set, so the kind-level test cannot distinguish "can call a function" from "can create one".

use grammar::ast::*;
use grammar::mutate::{self};
use grammar::reference;
use std::collections::BTreeSet;

/// Kind PLUS arity. The whole point of this file: `TRead/0` and `TRead/2` are different shapes and
/// the same kind.
fn shape(n: &Node) -> (&'static str, usize) {
    use Node::*;
    let k = match n {
        Budget => "Budget", Const(_) => "Const", Var(_) => "Var", OutcomeLit(_) => "OutcomeLit",
        Nop => "Nop", Moves(_) => "Moves", Terminal(_) => "Terminal", Key(_) => "Key",
        Eval(_) => "Eval", Ret(_) => "Ret", Probe(_) => "Probe", Field(..) => "Field",
        Set(..) => "Set", Apply(..) => "Apply", Max(..) => "Max", Min(..) => "Min",
        Avg(..) => "Avg", ScoreOf(..) => "ScoreOf", Cmp(..) => "Cmp", Pred(..) => "Pred",
        Loop(..) => "Loop", Store(..) => "Store", Mix(..) => "Mix", Foreach(..) => "Foreach",
        Argmax(..) => "Argmax", Sort(..) => "Sort", Sample(..) => "Sample", Let(..) => "Let",
        If(..) => "If", Call(..) => "Call", Arith(..) => "Arith", TRead(..) => "TRead",
    };
    // Arity is the VARIADIC length where one exists, since that is the dimension `add-arg` moves.
    // For fixed-arity nodes it is the child count, which never varies and so never produces a
    // spurious "new shape".
    let a = match n {
        Call(_, args) | Arith(_, args) | TRead(_, args) => args.len(),
        _ => child_refs(n).len(),
    };
    (k, a)
}

fn child_refs(n: &Node) -> Vec<&Node> {
    use Node::*;
    match n {
        Budget | Const(_) | Var(_) | OutcomeLit(_) | Nop => vec![],
        Moves(a) | Terminal(a) | Key(a) | Eval(a) | Ret(a) | Probe(a) | Field(a, _) | Set(_, a) => vec![a],
        Apply(a, b) | Max(a, b) | Min(a, b) | Avg(a, b) | ScoreOf(a, b) | Cmp(a, b, _)
        | Pred(a, b, _) | Loop(a, b) => vec![a, b],
        Store(a, _, b) => vec![a, b],
        Mix(a, b, c) => vec![a, b, c],
        Foreach(a, _, b) | Argmax(a, _, b) | Sort(a, _, b) | Sample(a, _, b) | Let(_, a, b) => vec![a, b],
        If(c, t, e) => match e { Some(e) => vec![c, t, e], None => vec![c, t] },
        Call(_, args) | Arith(_, args) | TRead(_, args) => args.iter().collect(),
    }
}

fn shapes_in(n: &Node, out: &mut BTreeSet<(&'static str, usize)>) {
    out.insert(shape(n));
    for c in child_refs(n) { shapes_in(c, out); }
}

fn prog_shapes(p: &Program) -> BTreeSet<(&'static str, usize)> {
    let mut s = BTreeSet::new();
    for f in &p.funcs { shapes_in(&f.body, &mut s); }
    s
}

/// Exhaustive sweep: every operator, every function, every position, over every reference program.
/// Returns (shapes introduced that the input lacked, greatest function count ever produced,
/// greatest function count any input had).
///
/// The function-count result is reported PER PROGRAM, not as an aggregate max. A global
/// `max(out) == max(in)` would be satisfied vacuously: `reference::all()` contains a 2-function
/// program, so a 1-function program growing a second function would leave the global maximum at 2
/// and the check would pass while the thing it tests had happened. Pooling over programs with
/// different baselines is the same defect as a dedup key that is missing a varied dimension.
fn sweep() -> (BTreeSet<(&'static str, usize)>, Vec<(String, usize, usize)>, usize) {
    // ALL_OPS, never a hand-copied list — the sibling records that a hardcoded operator list made
    // the test silently stop checking the thing it exists for when WrapIfPred landed.
    let ops = mutate::ALL_OPS;
    let mut new_shapes = BTreeSet::new();
    let mut changed: Vec<(String, usize, usize)> = Vec::new();  // per-program count changes
    let mut applied = 0usize;
    for (name, prog) in reference::all() {
        let before = prog_shapes(&prog);
        let n_in = prog.funcs.len();
        for op in ops {
            for fi in 0..prog.funcs.len() {
                for k in 0..400 {
                    let mut rng = mutate::Rng::new((k as u64) << 8 ^ fi as u64 ^ 0xABCD);
                    if let Some(m) = mutate::mutate_at(&prog, op, &mut rng, fi, k) {
                        applied += 1;
                        if m.funcs.len() != n_in {
                            changed.push((name.to_string(), n_in, m.funcs.len()));
                        }
                        for sh in prog_shapes(&m) {
                            if !before.contains(&sh) { new_shapes.insert(sh); }
                        }
                    }
                }
            }
        }
    }
    // An empty result from a sweep that never applied anything is a broken probe, not a finding.
    assert!(applied > 0, "the sweep applied ZERO mutations -- this instrument measured nothing");
    println!("sweep applied {applied} mutations across {} reference programs",
             reference::all().len());
    (new_shapes, changed, applied)
}

#[test]
fn shape_level_reachability_of_the_two_undeclared_operators() {
    let (buildable, func_count_changes, applied) = sweep();
    let seed = prog_shapes(&reference::bare_alpha_beta());

    println!("operators can introduce these SHAPES (kind/arity) that their input lacked:");
    for (k, a) in &buildable { println!("    {k}/{a}"); }

    // ---- add-arg: can any operator produce a TRead with ARGUMENTS? -------------------------
    // `table_reduction` (rung 7) needs TRead(3, [d, i]); the seed has TRead(_, []) only.
    let seed_tread_arities: Vec<usize> =
        seed.iter().filter(|(k, _)| *k == "TRead").map(|(_, a)| *a).collect();
    let buildable_tread_arities: Vec<usize> =
        buildable.iter().filter(|(k, _)| *k == "TRead").map(|(_, a)| *a).collect();
    println!("\nTRead arities: seed {seed_tread_arities:?}  newly constructible {buildable_tread_arities:?}");

    let rung7 = reference::table_reduction();
    let needed_shapes: Vec<(&str, usize)> = prog_shapes(&rung7)
        .into_iter()
        .filter(|s| !seed.contains(s) && !buildable.contains(s))
        .collect();
    println!("table reduction (rung 7) missing SHAPES: {needed_shapes:?}");

    // ---- add-fn: can any operator change the FUNCTION COUNT? ------------------------------
    println!("\nfunction count: {} of {applied} applied mutations changed it  (typecheck admits 1..4)",
             func_count_changes.len());
    for (n, a, b) in &func_count_changes { println!("    {n}: {a} -> {b}"); }

    // ---- SECOND-ORDER: does TRead/2 arrive in TWO edits? -----------------------------------
    // The sweep above applies ONE mutation, and Op::TReadIndex appends ONE index, so TRead/2 is
    // out of its reach BY CONSTRUCTION -- "missing [(TRead, 2)]" above is a statement about edit
    // DEPTH, not about reachability. Compose the operator with itself rather than asserting the
    // two-edit path exists because it sounds obvious.
    let mut second_order: BTreeSet<(&'static str, usize)> = BTreeSet::new();
    for (_, prog) in reference::all() {
        for fi in 0..prog.funcs.len() {
            for k in 0..400 {
                let mut rng = mutate::Rng::new((k as u64) << 8 ^ fi as u64 ^ 0xABCD);
                let Some(once) = mutate::mutate_at(&prog, mutate::Op::TReadIndex, &mut rng, fi, k)
                    else { continue };
                for fj in 0..once.funcs.len() {
                    for k2 in 0..400 {
                        let mut r2 = mutate::Rng::new((k2 as u64) << 12 ^ fj as u64 ^ 0x1234);
                        if let Some(twice) =
                            mutate::mutate_at(&once, mutate::Op::TReadIndex, &mut r2, fj, k2)
                        {
                            for sh in prog_shapes(&twice) { second_order.insert(sh); }
                        }
                    }
                }
            }
        }
    }
    let tread2_in_two_edits = second_order.contains(&("TRead", 2));
    println!("\nTWO edits of TReadIndex reach TRead/2: {tread2_in_two_edits}");

    // ---- ASSERTIONS ------------------------------------------------------------------------
    // These encode the CURRENT state on purpose, as the sibling does. When `add-fn` lands the last
    // one FAILS, and that failure is the signal.
    //
    // HISTORY, kept because the transition is the point. Until Op::TReadIndex landed (2026-09-10)
    // this file asserted that NO operator could build a TRead with arguments, and the sibling
    // recorded rung 7 as NOT PROVEN EITHER WAY at kind granularity. Both are now settled.
    // TReadIndex is PARKED (mutate::PARKED_OPS), so it is not in the ALL_OPS sweep above and
    // `buildable` correctly does NOT contain TRead/1. Test the operator directly: the question
    // "can the operator set express this" is separate from "is the operator switched on".
    let mut direct: BTreeSet<(&'static str, usize)> = BTreeSet::new();
    for (_, prog) in reference::all() {
        for fi in 0..prog.funcs.len() {
            for k in 0..400 {
                let mut rng = mutate::Rng::new((k as u64) << 8 ^ fi as u64 ^ 0xABCD);
                if let Some(m) = mutate::mutate_at(&prog, mutate::Op::TReadIndex, &mut rng, fi, k) {
                    for sh in prog_shapes(&m) { direct.insert(sh); }
                }
            }
        }
    }
    println!("TReadIndex applied directly reaches TRead/1: {}", direct.contains(&("TRead", 1)));
    assert!(
        direct.contains(&("TRead", 1)),
        "Op::TReadIndex no longer builds a TRead index; rung 7 goes back to being unreachable."
    );
    assert!(
        !buildable.contains(&("TRead", 1)),
        "TRead/1 is reachable from ALL_OPS, so TReadIndex has been UNPARKED. That is only correct \
         once tables_nd is populated -- until then every candidate it makes is a no-op that costs \
         nodes (interp/tests/tread_index_is_inert.rs). Re-measure before leaving it enabled."
    );
    assert!(
        tread2_in_two_edits,
        "TRead/2 is not reachable in two applications of TReadIndex, so rung 7 is still outside \
         the search space and the operator does not do what it was added for."
    );

    // The stronger of the two results. Mutation cannot change the function count, and neither can
    // crossover (`crossover()` writes `out.funcs[rfi].body`, never pushing a func), so a search
    // seeded with one function is confined to one function forever.
    assert!(
        func_count_changes.is_empty(),
        "a mutation changed the function count -- add-fn has landed: {func_count_changes:?}. \
         typecheck.rs admits 1..4 functions, so this unlocks previously unreachable program space."
    );
}
