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
    /// SCRAMBLE THE SEED. A bare xorshift64's first output is strongly correlated with its
    /// state, and callers seed this from small structured values -- the search uses
    /// `(gen << 24) ^ candidate ^ 0xBEEF`. Taking `first_next() % 8` off such a seed is not a
    /// uniform draw, and it showed: with the operator chosen uniformly BEFORE placement, 119
    /// real proposals came out Delete 40, InsertMax 39, SwapSiblings 2, where ~15 each is
    /// expected. The operator set was still being drawn from unevenly, one layer below the
    /// selection bias already fixed.
    ///
    /// splitmix64's finalizer decorrelates the seed before the stream starts. It is the
    /// standard remedy and `board::zobrist` already uses the same constants for the same reason.
    pub fn new(seed: u64) -> Self {
        let mut z = seed.wrapping_add(0x9E3779B97F4A7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        Rng((z ^ (z >> 31)) | 1)
    }
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
/// The operator table itself, extracted so `mutate_at` and `try_at` cannot drift apart.
/// `r` is a pre-drawn random word: the operators that need randomness (a constant delta,
/// a relation, a field) consume it without needing the Rng, which keeps this a pure
/// function of (node, op, r) and therefore reproducible from a seed.
fn apply_op(n: &Node, op: Op, r: u64) -> Option<Node> {
    match op {
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
    }
}

/// Apply `op` at a RANDOM position. Kept for callers that want one shot.
pub fn mutate(p: &Program, op: Op, rng: &mut Rng) -> Option<Program> {
    let fi = rng.below(p.funcs.len());
    let total = count_nodes(&p.funcs[fi].body);
    let k = rng.below(total);
    mutate_at(p, op, rng, fi, k)
}

/// Apply `op` at a SPECIFIC node index of a specific function.
///
/// Exposed so a caller can try an operator at every position before giving up on it. With a
/// single random position, an operator that matches only a few node types abandons most of the
/// time, and abandoning is what makes the effective operator set differ from the declared one.
pub fn mutate_at(p: &Program, op: Op, rng: &mut Rng, fi: usize, k0: usize) -> Option<Program> {
    let mut out = p.clone();
    let mut k = k0;
    let mut applied = false;
    let r = rng.next();

    let body = map_nth(&out.funcs[fi].body, &mut k, &mut applied, &mut |n| apply_op(n, op, r));

    if !applied { return None; }   // reached the site but the operator did not apply there
    out.funcs[fi].body = body;
    // GRAMMAR 3: reject ill-typed candidates HERE, not at the gate.
    typecheck::check_program(&out).ok()?;
    Some(out)
}

/// Why a placement failed. `mutate_at` collapses both into None, which conflates two facts that
/// mean very different things about the operator SET:
///   NoMatch  -- no node of the right shape at that position. Says nothing about the operator.
///   IllTyped -- the operator applied and produced a program that does not type-check. An
///               operator whose every placement is ill-typed is EFFECTIVELY ABSENT from the
///               set GRAMMAR 4 declares, and that is worth reporting rather than silently
///               counting as "did not apply".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement { Applied, NoMatch, IllTyped }

/// Like `mutate_at`, but says WHY it failed.
pub fn try_at(p: &Program, op: Op, rng: &mut Rng, fi: usize, k0: usize)
    -> (Option<Program>, Placement)
{
    let mut out = p.clone();
    let mut k = k0;
    let mut applied = false;
    let r = rng.next();
    let body = map_nth(&out.funcs[fi].body, &mut k, &mut applied, &mut |n| apply_op(n, op, r));
    if !applied { return (None, Placement::NoMatch); }
    out.funcs[fi].body = body;
    match typecheck::check_program(&out) {
        Ok(()) => (Some(out), Placement::Applied),
        Err(_) => (None, Placement::IllTyped),
    }
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
        // PICK THE OPERATOR FIRST, then retry that SAME operator on different nodes.
        //
        // The original drew a fresh random operator on every retry and kept whichever applied
        // first, which is a race that broadly-applicable operators always win. `mutate` picks
        // ONE random node and returns None if the operator does not match it, so an operator's
        // chance of winning is proportional to how many node types it accepts. MEASURED from
        // the search ledger over 67 proposals:
        //     InsertMax 32   Delete 16   WrapIf 9   WrapLoop 4   ReplaceConst 4   SwapSiblings 2
        //     Tweak 0
        // InsertMax wraps any Score node; Tweak matches only Const/Cmp/Field, about 6 of the
        // seed's 71 nodes, so it never won a race and was effectively absent from the operator
        // set. GRAMMAR 4 declares that set as a Given-column entry, so the set the search
        // ACTUALLY draws from has to be the declared one, not a subset weighted by how easy
        // each operator is to place.
        let op = ALL_OPS[rng.below(ALL_OPS.len())];
        // Try the chosen operator at EVERY position, in a shuffled order, before giving up on
        // it. One random position per attempt made an operator matching few node types abandon
        // most of the time (89/200 proposals produced no change), and abandoning is itself a
        // bias -- it silently thins exactly the operators the skew already under-represented.
        let mut ok = None;
        for fi in 0..cur.funcs.len() {
            let total = count_nodes(&cur.funcs[fi].body);
            let mut order: Vec<usize> = (0..total).collect();
            for i in (1..order.len()).rev() { let j = rng.below(i + 1); order.swap(i, j); }
            for k in order {
                if let Some(next) = mutate_at(&cur, op, rng, fi, k) { ok = Some(next); break; }
            }
            if ok.is_some() { break; }
        }
        // If this operator cannot be placed anywhere after 40 tries, the candidate is abandoned
        // rather than silently substituting a different operator -- that substitution is what
        // produced the skew.
        cur = ok?;
        applied.push(op);
    }
    Some((cur, applied))
}
