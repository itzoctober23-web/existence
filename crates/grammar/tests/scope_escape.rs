//! DOES A MUTATION EVER MOVE A VARIABLE OUT OF ITS BINDER'S SCOPE?
//!
//! GRAMMAR 3 states the economics this whole stage exists for: *"Mutations must type-check;
//! ill-typed candidates are discarded at generation time (cheap) rather than at gate time
//! (expensive)."* A gate costs thousands of games, a type check costs a tree walk.
//!
//! There is a class of broken candidate the checker cannot currently discard, because **two
//! independent fallbacks conspire to make it look well-typed**:
//!
//! ```text
//!   typecheck.rs   Var(name) => *env.get(name).unwrap_or(&Ty::Unit)   unbound var  -> Unit
//!   typecheck.rs   want(got, expect) accepts  got == Ty::Unit         Unit         -> accepted anywhere
//!   interp/lib.rs  lookup(env, name) ... .unwrap_or(Value::Unit)      unbound var  -> Unit at RUNTIME
//! ```
//!
//! So a program that reads a variable nothing bound type-checks, runs, and silently substitutes
//! Unit for the value. It is not rejected, and it is not obviously broken — it is a program that
//! quietly computes with a hole in it.
//!
//! `Dup` is the operator that can produce one: it copies a subtree to another same-typed site, and
//! nothing stops the source subtree from referencing a `Foreach`/`Let` binder that does not enclose
//! the destination. `Delete` can do it from the other direction, by replacing a binding node with
//! one of its children and orphaning every use underneath.
//!
//! WHY THIS IS ON THE CRITICAL PATH. `gate_power_RESULT.md` resolved P2's headline question with the
//! sequential gate: 16/16 verdicts, pooled 0.4527 [0.4371, 0.4683], candidates worse by ~-33 Elo —
//! *"the generator, not the gate, is what has no gradient."* A scope escape is one concrete way the
//! generator emits a guaranteed-worse candidate, and the expensive part is that it is discovered by
//! spending GAMES rather than by a tree walk.
//!
//! This test does not assert the defect exists. It applies every operator at every position of every
//! reference program and REPORTS the count, the same discipline `shape_reachability.rs` uses — the
//! recorded failure mode in this repo is asserting a mechanism instead of measuring it.
//!
//! ## The check is deliberately CONSERVATIVE
//!
//! `Set(name, _)` binds dynamically: `Interp::assign` updates an existing slot or pushes a new one,
//! so whether a `Var` is live can depend on execution order, which no static walk can settle here.
//! So a variable is counted unbound ONLY if its name appears nowhere in the enclosing function as a
//! parameter, a `Let`, a loop binder, or a `Set` target. That under-reports and cannot over-report:
//! **any nonzero count below is a real escape**, which is the direction a claim like this needs.

use grammar::ast::*;
use grammar::mutate;
use grammar::reference;
use grammar::typecheck;
use std::collections::BTreeSet;

fn children(n: &Node) -> Vec<&Node> {
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

/// Every `Set` target anywhere in the function.
///
/// `Set` is the one construct whose scope a static walk cannot pin down: `Interp::assign` updates an
/// existing slot or PUSHES a new one, so a name's liveness depends on execution order. These are
/// therefore seeded into scope for the whole function. That is the conservative direction — it can
/// only hide an escape, never invent one.
fn set_targets(n: &Node, out: &mut BTreeSet<String>) {
    if let Node::Set(name, _) = n { out.insert(name.clone()); }
    for c in children(n) { set_targets(c, out); }
}

/// TRUE LEXICAL SCOPING for the binders that have it: a `Let`/`Foreach`/`Argmax`/`Sort`/`Sample`
/// binder covers its BODY and nothing else.
///
/// The first version of this file used a whole-function approximation — a name counted as bound if
/// it appeared anywhere in the function as a binder — and reported 0 escapes out of 823. That zero
/// was not a measurement: the reference programs reuse loop variables, so a subtree lifted OUT of
/// the `Foreach` that binds `m` still looked bound whenever any other `Foreach` in the same function
/// also bound `m`. Which is exactly the case the file exists to detect. `escape_is_detectable`
/// below is the positive control that keeps this honest.
fn walk(n: &Node, scope: &mut Vec<String>, bad: &mut BTreeSet<String>) {
    use Node::*;
    match n {
        Var(name) => {
            if !scope.iter().any(|s| s == name) { bad.insert(name.clone()); }
        }
        // Binder covers the BODY only; the bound expression is evaluated outside it.
        Let(name, val, body) => {
            walk(val, scope, bad);
            scope.push(name.clone());
            walk(body, scope, bad);
            scope.pop();
        }
        Foreach(list, v, body) | Argmax(list, v, body) | Sort(list, v, body) | Sample(list, v, body) => {
            walk(list, scope, bad);
            scope.push(v.clone());
            walk(body, scope, bad);
            scope.pop();
        }
        _ => for c in children(n) { walk(c, scope, bad) },
    }
}

/// Names read outside the scope of anything that binds them.
fn unbound(p: &Program) -> BTreeSet<String> {
    let mut bad = BTreeSet::new();
    for f in &p.funcs {
        let mut scope: Vec<String> = f.params.iter().map(|(n, _)| n.clone()).collect();
        let mut sets = BTreeSet::new();
        set_targets(&f.body, &mut sets);
        scope.extend(sets);
        walk(&f.body, &mut scope, &mut bad);
    }
    bad
}

#[test]
fn report_scope_escapes_from_every_operator() {
    // ALL_OPS, never a hand-copied list: `shape_reachability.rs` records that a hardcoded operator
    // list made a sibling test silently stop checking the thing it existed for when a new op landed.
    let ops = mutate::ALL_OPS;

    // BASELINE FIRST. If a reference program already reads an unbound name, then making the scope
    // check ENFORCING would reject the seeds themselves -- so this number decides whether the fix
    // can live in `check_program` or has to sit in the mutation path only.
    println!("\n  reference-program baselines (must be empty for an enforcing check to be safe):");
    let mut dirty_seeds = 0usize;
    for (name, prog) in reference::all() {
        let b = unbound(&prog);
        if !b.is_empty() { dirty_seeds += 1; println!("    {name}: {b:?}"); }
    }
    println!("    {} of {} reference programs read an unbound name",
             dirty_seeds, reference::all().len());

    let mut applied = 0usize;
    let mut escaped = 0usize;
    let mut escaped_but_typechecks = 0usize;
    let mut by_op: Vec<(String, usize, usize)> = Vec::new();

    for op in ops {
        let mut op_applied = 0usize;
        let mut op_escaped = 0usize;
        for (_name, prog) in reference::all() {
            // A reference program that ALREADY reads an unbound name would make every mutation of
            // it look like an escape. Measure the baseline and subtract it per program.
            let base = unbound(&prog);
            for fi in 0..prog.funcs.len() {
                for k in 0..400 {
                    let mut rng = mutate::Rng::new((k as u64) << 8 ^ fi as u64 ^ 0xABCD);
                    let Some(m) = mutate::mutate_at(&prog, op, &mut rng, fi, k) else { continue };
                    applied += 1;
                    op_applied += 1;
                    let after = unbound(&m);
                    if after.difference(&base).next().is_some() {
                        escaped += 1;
                        op_escaped += 1;
                        // The point of the file: does the cheap stage reject it, or does it survive
                        // to cost games?
                        if typecheck::check_program(&m).is_ok() {
                            escaped_but_typechecks += 1;
                        }
                    }
                }
            }
        }
        by_op.push((format!("{op:?}"), op_applied, op_escaped));
    }

    println!("\n  SCOPE ESCAPES over every operator x every position x every reference program");
    println!("  {:<16} {:>9} {:>9} {:>8}", "operator", "applied", "escaped", "rate");
    for (name, a, e) in &by_op {
        if *a == 0 { continue }
        println!("  {:<16} {:>9} {:>9} {:>7.1}%", name, a, e, 100.0 * *e as f64 / *a as f64);
    }
    println!("  {:<16} {:>9} {:>9} {:>7.1}%", "TOTAL", applied, escaped,
             100.0 * escaped as f64 / applied.max(1) as f64);
    println!("  of the escapes, {} PASS check_program and would reach the gate", escaped_but_typechecks);
    println!("  (conservative: a name is unbound only if NOTHING in its function could bind it)\n");

    // No assertion on the count. The number is the result; asserting a threshold would freeze
    // today's measurement into a law, which this repo has recorded as its own failure mode.
    assert!(applied > 0, "no mutation applied anywhere -- the sweep itself is broken, \
                          which would report 0 escapes for the wrong reason");
}

/// POSITIVE CONTROL — validate the predicate before believing any zero it reports.
///
/// Builds the precise shape the sweep hunts: a subtree that reads a `Foreach` binder, copied to a
/// site OUTSIDE that `Foreach`, in a function where the SAME loop variable name is bound by another
/// loop. The whole-function approximation this file started with scored that as "bound" and is the
/// reason its first run reported 0 of 823.
#[test]
fn escape_is_detectable() {
    let inner = |body: Node| Node::Foreach(
        Box::new(Node::Moves(Box::new(Node::Var("p".into())))),
        "m".into(),
        Box::new(body),
    );
    // Two sibling loops both binding `m`, and a read of `m` sitting OUTSIDE both of them.
    let escaped_read = Node::Set("acc".into(), Box::new(Node::ScoreOf(
        Box::new(Node::Var("p".into())), Box::new(Node::Var("m".into())))));
    let body = Node::Let("_a".into(), Box::new(Node::Const(0)), Box::new(Node::Mix(
        Box::new(inner(Node::Nop)),
        Box::new(inner(Node::Nop)),
        Box::new(escaped_read),
    )));

    let p = Program {
        funcs: vec![Func {
            name: "choose".into(),
            params: vec![("p".into(), Ty::Pos), ("d".into(), Ty::Int)],
            ret: Ty::Move,
            body,
        }],
        lineage: Lineage::Main,
    };

    let bad = unbound(&p);
    assert!(bad.contains("m"),
            "POSITIVE CONTROL FAILED: a read of `m` outside every Foreach that binds it was not \
             flagged ({bad:?}). Any 0 this file reports would be a broken probe, not an absence.");
    println!("\n  positive control OK: lexical checker flags {bad:?}\n");
}

/// The mechanism, isolated: an unbound variable type-checks in a position that demands a real type.
/// If this ever fails, the two fallbacks have been fixed and the sweep above should read 0.
#[test]
fn unbound_var_currently_typechecks() {
    let (_, prog) = reference::all().into_iter().next().expect("at least one reference program");
    let mut p = prog.clone();
    // Read a name nothing binds, in the entry function.
    let hole = Node::Var("__no_such_binding__".into());
    p.funcs[0].body = Node::Let("__probe".into(), Box::new(hole), Box::new(p.funcs[0].body.clone()));

    let escapes = unbound(&p);
    assert!(escapes.contains("__no_such_binding__"),
            "the scope checker missed an obviously unbound name: {escapes:?}");

    match typecheck::check_program(&p) {
        Ok(()) => println!("\n  CONFIRMED: an unbound variable passes check_program \
                            (Var -> Ty::Unit, and want() accepts Unit anywhere)\n"),
        Err(e) => println!("\n  check_program now REJECTS an unbound variable: {}\n", e.what),
    }
}
