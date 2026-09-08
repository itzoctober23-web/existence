//! Mutation operators (GRAMMAR 4). Every one is TYPE-PRESERVING by construction, and the
//! result is type-checked before it is returned — GRAMMAR 3's economics is that an ill-typed
//! candidate costs a tree walk to reject, while a bad candidate that reaches the gate costs
//! thousands of games.
//!
//! Determinism matters: a mutation is reproducible from (program, seed), so a candidate that
//! passes a gate can be regenerated exactly (SCHEMAS 2 stores the mutation list).

use crate::ast::*;
use crate::typecheck;

pub struct Rng(pub u64);
impl Rng {
    pub fn new(seed: u64) -> Self { Rng(seed | 1) }
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next() % n as u64) as usize }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Op { Tweak, WrapIf, WrapLoop, Delete, Dup, SwapSiblings, InsertMax, ReplaceConst }

pub const ALL_OPS: [Op; 8] = [
    Op::Tweak, Op::WrapIf, Op::WrapLoop, Op::Delete,
    Op::Dup, Op::SwapSiblings, Op::InsertMax, Op::ReplaceConst,
];

/// Collect mutable positions as a flat index, so an operator can address "the k-th node".
fn count_nodes(n: &Node) -> usize {
    1 + children(n).iter().map(|c| count_nodes(c)).sum::<usize>()
}

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

/// Apply `f` to the k-th node in pre-order, rebuilding the tree around it.
///
/// `applied` is set ONLY when `f` actually returned a replacement. Using the visit counter
/// alone cannot distinguish "site never reached" from "reached but the operator did not apply
/// there", and conflating them made 92% of mutations return the program UNCHANGED while
/// reporting success -- evolution would have proposed identical candidates and spent gate time
/// on them.
fn map_nth(n: &Node, k: &mut usize, applied: &mut bool, f: &mut dyn FnMut(&Node) -> Option<Node>) -> Node {
    if *k == 0 {
        *k = usize::MAX;
        if let Some(rep) = f(n) { *applied = true; return rep; }
        return n.clone();
    }
    *k = k.saturating_sub(1);
    use Node::*;
    let b = |x: &Node, k: &mut usize, ap: &mut bool, f: &mut dyn FnMut(&Node) -> Option<Node>| Box::new(map_nth(x, k, ap, f));
    match n {
        Moves(a) => Moves(b(a, k, applied, f)),
        Terminal(a) => Terminal(b(a, k, applied, f)),
        Key(a) => Key(b(a, k, applied, f)),
        Eval(a) => Eval(b(a, k, applied, f)),
        Ret(a) => Ret(b(a, k, applied, f)),
        Probe(a) => Probe(b(a, k, applied, f)),
        Field(a, x) => Field(b(a, k, applied, f), *x),
        Set(s, a) => Set(s.clone(), b(a, k, applied, f)),
        Apply(x, y) => Apply(b(x, k, applied, f), b(y, k, applied, f)),
        Max(x, y) => Max(b(x, k, applied, f), b(y, k, applied, f)),
        Min(x, y) => Min(b(x, k, applied, f), b(y, k, applied, f)),
        Avg(x, y) => Avg(b(x, k, applied, f), b(y, k, applied, f)),
        ScoreOf(x, y) => ScoreOf(b(x, k, applied, f), b(y, k, applied, f)),
        Cmp(x, y, r) => Cmp(b(x, k, applied, f), b(y, k, applied, f), *r),
        Pred(x, y, pi) => Pred(b(x, k, applied, f), b(y, k, applied, f), *pi),
        Loop(x, y) => Loop(b(x, k, applied, f), b(y, k, applied, f)),
        Store(x, fi, y) => Store(b(x, k, applied, f), *fi, b(y, k, applied, f)),
        Mix(x, y, z) => Mix(b(x, k, applied, f), b(y, k, applied, f), b(z, k, applied, f)),
        Foreach(x, s, y) => Foreach(b(x, k, applied, f), s.clone(), b(y, k, applied, f)),
        Argmax(x, s, y) => Argmax(b(x, k, applied, f), s.clone(), b(y, k, applied, f)),
        Sort(x, s, y) => Sort(b(x, k, applied, f), s.clone(), b(y, k, applied, f)),
        Sample(x, s, y) => Sample(b(x, k, applied, f), s.clone(), b(y, k, applied, f)),
        Let(s, x, y) => Let(s.clone(), b(x, k, applied, f), b(y, k, applied, f)),
        If(c, t, e) => If(b(c, k, applied, f), b(t, k, applied, f), e.as_ref().map(|x| b(x, k, applied, f))),
        Call(i, args) => Call(*i, args.iter().map(|a| map_nth(a, k, applied, f)).collect()),
        Arith(o, args) => Arith(*o, args.iter().map(|a| map_nth(a, k, applied, f)).collect()),
        TRead(i, args) => TRead(*i, args.iter().map(|a| map_nth(a, k, applied, f)).collect()),
        other => other.clone(),
    }
}

/// One mutation. Returns None if it produced nothing valid — the caller simply tries again,
/// which is cheaper than making every operator universally applicable.
pub fn mutate(p: &Program, op: Op, rng: &mut Rng) -> Option<Program> {
    let mut out = p.clone();
    let fi = rng.below(out.funcs.len());
    let total = count_nodes(&out.funcs[fi].body);
    let mut k = rng.below(total);
    let mut applied = false;
    let r = rng.next();

    let body = map_nth(&out.funcs[fi].body, &mut k, &mut applied, &mut |n| match op {
        Op::Tweak => match n {
            Node::Const(c) => {
                let d = (r % 7) as i8 - 3;
                Some(Node::Const((*c).saturating_add(d).clamp(-8, 8)))
            }
            Node::Cmp(a, b2, _) => {
                let rels = [Rel::Lt, Rel::Le, Rel::Eq, Rel::Ge, Rel::Gt, Rel::Ne];
                Some(Node::Cmp(a.clone(), b2.clone(), rels[(r % 6) as usize]))
            }
            Node::Field(a, _) => {
                let fs = [FieldId::Score, FieldId::Depth, FieldId::Flag, FieldId::Count, FieldId::Sum];
                Some(Node::Field(a.clone(), fs[(r % 5) as usize]))
            }
            _ => None,
        },
        // wrap a Score/Int node in max(_, const): same result type, so always well-typed
        Op::InsertMax => match n {
            Node::Const(_) | Node::Arith(..) | Node::Max(..) | Node::Min(..) => {
                Some(Node::Max(Box::new(n.clone()), Box::new(Node::Const((r % 9) as i8 - 4))))
            }
            _ => None,
        },
        Op::ReplaceConst => match n {
            Node::Const(_) => Some(Node::Const((r % 17) as i8 - 8)),
            _ => None,
        },
        // guard a statement: if(cond, stmt) has the statement's type when there is no else
        Op::WrapIf => match n {
            Node::Store(..) | Node::Set(..) | Node::Nop => Some(Node::If(
                Box::new(Node::Cmp(
                    Box::new(Node::Budget),
                    Box::new(Node::Const((r % 5) as i8)),
                    Rel::Gt,
                )),
                Box::new(n.clone()),
                None,
            )),
            _ => None,
        },
        Op::WrapLoop => match n {
            Node::Store(..) | Node::Set(..) => Some(Node::Loop(
                Box::new(Node::Const(1 + (r % 3) as i8)),
                Box::new(n.clone()),
            )),
            _ => None,
        },
        // replace a node by one of its same-typed children
        Op::Delete => match n {
            Node::Max(a, _) | Node::Min(a, _) | Node::Avg(a, _) => Some((**a).clone()),
            Node::Arith(_, args) if !args.is_empty() => Some(args[0].clone()),
            _ => None,
        },
        Op::Dup => match n {
            Node::Max(a, _) => Some(Node::Max(a.clone(), a.clone())),
            Node::Min(a, _) => Some(Node::Min(a.clone(), a.clone())),
            _ => None,
        },
        Op::SwapSiblings => match n {
            Node::Max(a, b2) => Some(Node::Max(b2.clone(), a.clone())),
            Node::Min(a, b2) => Some(Node::Min(b2.clone(), a.clone())),
            Node::Arith(o, args) if args.len() == 2 => {
                Some(Node::Arith(*o, vec![args[1].clone(), args[0].clone()]))
            }
            _ => None,
        },
    });

    if !applied { return None; }   // reached the site but the operator did not apply there
    out.funcs[fi].body = body;
    // GRAMMAR 3: reject ill-typed candidates HERE, not at the gate.
    typecheck::check_program(&out).ok()?;
    Some(out)
}

/// 1..3 operators per candidate (GRAMMAR 4), retrying sites that do not apply.
pub fn mutate_program(p: &Program, rng: &mut Rng) -> Option<Program> {
    let edits = 1 + rng.below(3);
    mutate_program_n(p, rng, edits).map(|(prog, _)| prog)
}

/// As `mutate_program`, but with an explicit edit count and a record of WHICH operators were
/// applied.
///
/// Both matter for the search track, and neither was observable before:
///
/// - EDIT COUNT. The original always applied 1-3 stacked edits. Three random edits to a program
///   that already computes the exact minimax value will almost always break it, and the first
///   real search-track run duly saw 90 of 106 well-typed candidates rejected by the correctness
///   oracle. Whether a single edit survives more often is a measurable question, and it decides
///   how much of the search budget is reachable at all.
/// - WHICH OPERATOR. With no record, 128 rejected candidates teach nothing about the operator
///   set. With one, the same run reports which operators produce viable programs and which only
///   ever produce wreckage — and GRAMMAR 4's operator list is a Given column entry, so its
///   composition is exactly the sort of thing that should be reported rather than assumed.
pub fn mutate_program_n(p: &Program, rng: &mut Rng, edits: usize) -> Option<(Program, Vec<Op>)> {
    let mut cur = p.clone();
    let mut applied = Vec::with_capacity(edits);
    for _ in 0..edits.max(1) {
        let mut ok = None;
        for _ in 0..40 {
            let op = ALL_OPS[rng.below(ALL_OPS.len())];
            if let Some(next) = mutate(&cur, op, rng) { ok = Some((next, op)); break; }
        }
        let (next, op) = ok?;
        cur = next;
        applied.push(op);
    }
    Some((cur, applied))
}
