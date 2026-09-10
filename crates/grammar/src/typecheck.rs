//! Type checker. GRAMMAR 3: "Programs are statically typed. Mutations must type-check;
//! ill-typed candidates are discarded at generation time (cheap) rather than at gate time
//! (expensive)."
//!
//! That economics is the whole point. A gate costs thousands of games; a type check costs a
//! tree walk. Every mutation the evolver proposes passes through here first.

use crate::ast::*;
use std::collections::HashMap;

#[derive(Debug)]
pub struct TypeError {
    pub what: String,
}

type Env = HashMap<String, Ty>;

pub fn check_program(p: &Program) -> Result<(), TypeError> {
    if p.funcs.is_empty() || p.funcs.len() > 4 {
        return Err(TypeError { what: format!("program has {} functions; GRAMMAR 3 allows 1..4", p.funcs.len()) });
    }
    let entry = &p.funcs[0];
    if entry.ret != Ty::Move {
        return Err(TypeError { what: format!("entry returns {}, must be Move", entry.ret) });
    }
    if entry.params.len() != 2 || entry.params[0].1 != Ty::Pos || entry.params[1].1 != Ty::Int {
        return Err(TypeError { what: "entry must be choose(Pos, Int) -> Move".into() });
    }
    for f in &p.funcs {
        let mut env: Env = f.params.iter().cloned().collect();
        check(&f.body, p, &mut env)?;
    }
    scope_check(p)?;
    Ok(())
}

/// Reject programs that READ A VARIABLE NOTHING BINDS.
///
/// `check()` cannot catch this, and not by oversight — three fallbacks conspire:
///
/// ```text
///   typecheck  Var(name) => *env.get(name).unwrap_or(&Ty::Unit)   unbound -> Unit
///   typecheck  want(got, expect) accepts got == Ty::Unit          Unit    -> fits anywhere
///   typecheck  Foreach/Argmax/Sort/Sample insert their binder and NEVER remove it,
///              so the loop variable stays live for the rest of the function
///   interp     lookup(..).unwrap_or(Value::Unit)                  unbound -> Unit at RUNTIME
/// ```
///
/// The result is a program with a hole in it that type-checks, runs, and silently computes with
/// Unit where a real value belongs. It is not rejected and it is not visibly broken.
///
/// MEASURED, `tests/scope_escape.rs`: `Op::WrapIfPred` produced one in **26 of 50 applications
/// (52%)**, and all 26 passed `check_program`. It hardcodes `Var("m")` and `Var("p")` and never
/// asks whether `m` is in scope at the wrap site, so wrapping any statement outside a
/// `Foreach(_, "m", _)` reads an unbound move. The `If` condition then evaluates on Unit and is
/// effectively constant, making the candidate either inert-but-larger or silently statement-
/// disabling. Under FITNESS 3 (mates per COST) both are guaranteed-worse candidates.
///
/// GRAMMAR 3 states the economics exactly: *"ill-typed candidates are discarded at generation time
/// (cheap) rather than at gate time (expensive)."* Those 26 were reaching the gate and being paid
/// for in GAMES. This is the tree walk that stops them.
///
/// CONSERVATIVE ON `Set`, STRICT ON LEXICAL BINDERS. `Interp::assign` updates an existing slot or
/// PUSHES a new one, so whether a `Set`-bound name is live depends on execution order and no static
/// walk can settle it. Every `Set` target in a function is therefore treated as bound throughout
/// that function. `Let`/`Foreach`/`Argmax`/`Sort`/`Sample` binders get true lexical scope — they
/// cover their body and nothing else. That direction can only miss an escape, never invent one,
/// and it is what makes this safe to enforce: all 10 reference programs pass it unchanged.
pub fn scope_check(p: &Program) -> Result<(), TypeError> {
    for (fi, f) in p.funcs.iter().enumerate() {
        let mut scope: Vec<String> = f.params.iter().map(|(n, _)| n.clone()).collect();
        collect_set_targets(&f.body, &mut scope);
        if let Some(name) = find_unbound(&f.body, &mut scope) {
            return Err(TypeError {
                what: format!("function {fi} (`{}`) reads unbound variable `{name}`", f.name),
            });
        }
    }
    Ok(())
}

/// Names a subtree READS but does not itself bind — what a graft of this subtree would need the
/// destination to supply. `crossover` uses it to pick donors that can legally land.
pub fn free_vars(n: &Node) -> Vec<String> {
    let mut scope: Vec<String> = Vec::new();
    // A `Set` inside the subtree creates its own name, so it is not a requirement on the site.
    collect_set_targets(n, &mut scope);
    let mut out = Vec::new();
    gather_free(n, &mut scope, &mut out);
    out
}

fn gather_free(n: &Node, scope: &mut Vec<String>, out: &mut Vec<String>) {
    use Node::*;
    match n {
        Var(name) => {
            if !scope.iter().any(|s| s == name) && !out.iter().any(|s| s == name) {
                out.push(name.clone());
            }
        }
        Let(name, val, body) => {
            gather_free(val, scope, out);
            scope.push(name.clone());
            gather_free(body, scope, out);
            scope.pop();
        }
        Foreach(list, v, body) | Argmax(list, v, body) | Sort(list, v, body) | Sample(list, v, body) => {
            gather_free(list, scope, out);
            scope.push(v.clone());
            gather_free(body, scope, out);
            scope.pop();
        }
        _ => for c in kids(n) { gather_free(c, scope, out) },
    }
}

/// Names guaranteed live at EVERY position of a function: its parameters, plus every `Set` target
/// (which `Interp::assign` will push if absent). Deliberately excludes lexical binders — a loop
/// variable is live only inside its own body, so a graft site chosen anywhere in the function
/// cannot rely on it.
pub fn always_in_scope(f: &Func) -> Vec<String> {
    let mut s: Vec<String> = f.params.iter().map(|(n, _)| n.clone()).collect();
    collect_set_targets(&f.body, &mut s);
    s
}

fn kids(n: &Node) -> Vec<&Node> {
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

fn collect_set_targets(n: &Node, out: &mut Vec<String>) {
    if let Node::Set(name, _) = n {
        if !out.iter().any(|s| s == name) { out.push(name.clone()); }
    }
    for c in kids(n) { collect_set_targets(c, out); }
}

fn find_unbound(n: &Node, scope: &mut Vec<String>) -> Option<String> {
    use Node::*;
    match n {
        Var(name) => {
            if scope.iter().any(|s| s == name) { None } else { Some(name.clone()) }
        }
        // The bound expression is evaluated OUTSIDE the binding; the body is inside it.
        Let(name, val, body) => {
            if let Some(b) = find_unbound(val, scope) { return Some(b) }
            scope.push(name.clone());
            let r = find_unbound(body, scope);
            scope.pop();
            r
        }
        Foreach(list, v, body) | Argmax(list, v, body) | Sort(list, v, body) | Sample(list, v, body) => {
            if let Some(b) = find_unbound(list, scope) { return Some(b) }
            scope.push(v.clone());
            let r = find_unbound(body, scope);
            scope.pop();
            r
        }
        _ => {
            for c in kids(n) {
                if let Some(b) = find_unbound(c, scope) { return Some(b) }
            }
            None
        }
    }
}

/// Score and Int are both signed integers (GRAMMAR 1); Score is the mover-relative one.
/// Numeric operations must PRESERVE Score rather than collapsing everything to Int -- the
/// alpha-beta seed does `set a = max(a, vv)` where `a` is a Score parameter, and a checker
/// that returns Int from max() rejects the very program the grammar is seeded with.
fn numeric_join(a: Ty, b: Ty) -> Ty {
    if a == Ty::Score || b == Ty::Score { Ty::Score } else { Ty::Int }
}

fn want(got: Ty, expect: Ty, ctx: &str) -> Result<(), TypeError> {
    if got == expect || got == Ty::Unit {
        Ok(())
    } else {
        Err(TypeError { what: format!("{ctx}: expected {expect}, got {got}") })
    }
}

/// Infer a node's type, verifying its children.
pub fn check(n: &Node, p: &Program, env: &mut Env) -> Result<Ty, TypeError> {
    use Node::*;
    Ok(match n {
        Const(_) | Budget => Ty::Int,
        Nop => Ty::Unit,
        OutcomeLit(_) => Ty::Outcome,
        Var(name) => *env.get(name).unwrap_or(&Ty::Unit),

        Moves(a) => { want(check(a, p, env)?, Ty::Pos, "moves")?; Ty::List }
        Apply(a, b) => {
            want(check(a, p, env)?, Ty::Pos, "apply pos")?;
            want(check(b, p, env)?, Ty::Move, "apply move")?;
            Ty::Pos
        }
        Terminal(a) => { want(check(a, p, env)?, Ty::Pos, "terminal")?; Ty::Outcome }
        Key(a) => { want(check(a, p, env)?, Ty::Pos, "key")?; Ty::Key }
        Eval(a) => { want(check(a, p, env)?, Ty::Pos, "eval")?; Ty::Score }
        Pred(a, b, _) => {
            want(check(a, p, env)?, Ty::Move, "pred move")?;
            want(check(b, p, env)?, Ty::Pos, "pred pos")?;
            Ty::Bool
        }

        Arith(_, args) => {
            let mut t = Ty::Int;
            for a in args { t = numeric_join(t, check(a, p, env)?); }
            t
        }
        Cmp(a, b, _) => {
            let (ta, tb) = (check(a, p, env)?, check(b, p, env)?);
            // Outcome compares only with Outcome — there is no coercion to a number, which is
            // what keeps "draw = 0" out of the Given column (GRAMMAR 2.4 #15).
            if (ta == Ty::Outcome) != (tb == Ty::Outcome) {
                return Err(TypeError { what: format!("cmp mixes {ta} with {tb}") });
            }
            Ty::Bool
        }
        Max(a, b) | Min(a, b) | Avg(a, b) => {
            let (ta, tb) = (check(a, p, env)?, check(b, p, env)?);
            numeric_join(ta, tb)
        }
        Mix(a, b, c) => {
            let (ta, tb) = (check(a, p, env)?, check(b, p, env)?);
            check(c, p, env)?;
            numeric_join(ta, tb)
        }
        ScoreOf(a, b) => {
            want(check(a, p, env)?, Ty::Outcome, "score_of outcome")?;
            check(b, p, env)?;
            Ty::Score
        }
        // GRAMMAR 2.7 primitive 26: `tread : Tab x Int... -> Int`. The indices are Int, and this
        // used to `check()` them without CONSTRAINING them -- proving each index was well-formed
        // and never that it was an Int, so a Pos or a Score passed. Harmless while nothing built a
        // tread index (measured 2026-09-10: no operator lengthens any argument list, 0 of 823
        // applied mutations); not harmless the moment one does.
        //
        // It failed in the expensive direction. `mutate.rs:207` sets the standard: an operator may
        // emit `Var("p")` out of scope precisely BECAUSE the checker discards it -- "wasted
        // candidates rather than silently wrong ones". An unconstrained index inverts that: the
        // candidate is KEPT, evaluated, and wrong in a way no gate reports as an error.
        TRead(_, args) => {
            for a in args { want(check(a, p, env)?, Ty::Int, "tread index")?; }
            Ty::Int
        }

        Probe(a) => { want(check(a, p, env)?, Ty::Key, "probe")?; Ty::Slot }
        Store(k, _, v) => {
            want(check(k, p, env)?, Ty::Key, "store key")?;
            check(v, p, env)?;
            Ty::Unit
        }
        Field(a, f) => { want(check(a, p, env)?, Ty::Slot, "field")?; f.ty() }

        Foreach(l, var, body) => {
            want(check(l, p, env)?, Ty::List, "foreach list")?;
            env.insert(var.clone(), Ty::Move);
            check(body, p, env)?;
            Ty::Unit
        }
        Argmax(l, var, key) | Sort(l, var, key) | Sample(l, var, key) => {
            want(check(l, p, env)?, Ty::List, "selection list")?;
            env.insert(var.clone(), Ty::Move);
            check(key, p, env)?;
            if matches!(n, Sort(..)) { Ty::List } else { Ty::Move }
        }
        Loop(c, body) => { check(c, p, env)?; check(body, p, env)?; Ty::Unit }
        If(c, t, e) => {
            check(c, p, env)?;
            let tt = check(t, p, env)?;
            if let Some(e) = e { check(e, p, env)?; }
            tt
        }
        Let(name, init, body) => {
            let t = check(init, p, env)?;
            env.insert(name.clone(), t);
            check(body, p, env)?
        }
        Set(name, v) => {
            let t = check(v, p, env)?;
            if let Some(prev) = env.get(name) {
                // Score/Int assignments are compatible in both directions: both are signed
                // integers and a table read (Int) legitimately initialises a Score accumulator.
                let compatible = numeric_join(*prev, t) == numeric_join(t, *prev)
                    && matches!((*prev, t), (Ty::Score | Ty::Int, Ty::Score | Ty::Int));
                if *prev != t && !compatible && *prev != Ty::Unit && t != Ty::Unit {
                    return Err(TypeError { what: format!("set {name}: {prev} := {t}") });
                }
            }
            Ty::Unit
        }
        Ret(a) => check(a, p, env)?,
        Call(idx, args) => {
            let f = p.funcs.get(*idx).ok_or(TypeError { what: format!("call to function {idx} which does not exist") })?;
            if args.len() != f.params.len() {
                return Err(TypeError { what: format!("call {}: {} args for {} params", f.name, args.len(), f.params.len()) });
            }
            for a in args { check(a, p, env)?; }
            f.ret
        }
    })
}
