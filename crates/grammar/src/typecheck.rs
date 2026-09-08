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
    Ok(())
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
        TRead(_, args) => { for a in args { check(a, p, env)?; } Ty::Int }

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
