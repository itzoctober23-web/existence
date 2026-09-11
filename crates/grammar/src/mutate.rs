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
pub enum Op {
    Tweak, WrapIf, WrapLoop, Delete, Dup, SwapSiblings, InsertMax, ReplaceConst,
    /// GRAMMAR 4 `add-fn`: split a subtree into a new function and call it, threading the
    /// subtree's free variables as parameters. See `try_add_fn`. NOT in `ALL_OPS` until the
    /// measurement in `tests/add_fn.rs` says it earns a draw -- the `TReadIndex` discipline.
    AddFn,
    /// Replace an Int leaf with a hash-slot READ: `field(probe(key(p)), <f>)`.
    /// Introduces Probe, Key and Field, none of which any other operator can build.
    ProbeRead,
    /// Replace a statement with a hash-slot WRITE: `store(key(p), <f>, <int>)`.
    /// Introduces Store. Paired with ProbeRead this makes hash reuse REACHABLE -- not assembled.
    StoreHere,
    /// Wrap a statement in `if pred(m, p, <predicate>)`.
    ///
    /// THE ONLY OPERATOR THAT INTRODUCES A PRIMITIVE, and it exists because nothing else could.
    /// MEASURED 2026-09-08: the operator set could construct exactly {Budget, Const, Loop, Max} --
    /// it could tune constants and rearrange existing structure, but could not add a node kind the
    /// program did not already contain. Every declared ladder rung needs one: capture extension
    /// needs `Pred`, hash reuse needs `Probe`/`Key`/`Field`/`Store`. So no rung was reachable at
    /// any edit count, depth or budget, and `evolve` ran ~690 candidates and accepted zero.
    ///
    /// GRAMMAR 9 writes rung 6 as "wrap-if(pred(m,p,is_capture)) around depth check", which is
    /// this operator applied at one position. It is the smallest change that makes any rung
    /// reachable.
    ///
    /// SCOPE IS HANDLED BY THE TYPE CHECKER -- TRUE SINCE 2026-09-10, AND FALSE WHEN FIRST WRITTEN.
    ///
    /// This comment used to argue the candidate "fails `want(Unit, Move)` and is discarded", and
    /// closed with "checked before adding this, because an unbound variable that merely evaluates to
    /// Unit at runtime would have been a silent corruption." The reasoning was wrong at the exact
    /// step it needed to be right: `want` reads `got == expect || got == Ty::Unit`, so it ACCEPTS
    /// Unit anywhere. Nothing was discarded.
    ///
    /// The silent corruption it names is therefore precisely what happened. MEASURED 2026-09-10 by
    /// `tests/scope_escape.rs`: **26 of 50 applications (52%)** emitted `Var("m")` with no `m` in
    /// scope, all 26 passed `check_program`, and all 26 reached the GATE to be paid for in games.
    /// `typecheck::scope_check` now rejects them and the measured rate is 0; the operator still
    /// applies at its 24 legal sites. The claim in this heading is finally accurate, by a different
    /// mechanism than the one originally asserted.
    WrapIfPred,
    /// Append ONE in-scope Int expression as a `tread` INDEX: `TRead(t, [..]) -> TRead(t, [.., x])`.
    ///
    /// WHY. `tests/shape_reachability.rs` proves rung 7 is otherwise outside the search space.
    /// `table_reduction` needs `TRead(3, [d, i])`; the seed has `TRead(0, [])` and `TRead(1, [])`,
    /// and MEASURED 2026-09-10 no operator lengthens ANY argument list (0 of 823 applied
    /// mutations). At kind granularity the gap is invisible — `TRead` is already in the seed —
    /// which is why this went unnoticed while two other rungs were being fixed.
    ///
    /// NO NEW PRIMITIVE. GRAMMAR 2.7 #26 declares `tread : Tab x Int... -> Int`, already variadic.
    /// This operator supplies nothing the Given column did not already grant.
    ///
    /// NOT `add-arg`. GRAMMAR 4's `add-arg` adds a FUNCTION PARAMETER and threads it through call
    /// sites. Rung 7 needs neither: `d` is `ab`'s existing depth parameter and `i` an existing
    /// local. The two are different operators and only this one makes rung 7 reachable.
    ///
    /// IT MUST NOT SUPPLY THE ALGORITHM. MASTER_PLAN:53 requires the technique be DISCOVERED, and
    /// mutate.rs:205 already refused an operator that emitted probe-and-store together as making
    /// the discovery vacuous. So this appends exactly ONE index and never picks the table id:
    /// reaching `TRead(3, [d, i])` still needs two applications plus finding table 3, which is
    /// search, not a handout.
    ///
    /// SCOPE AND TYPE ARE THE CHECKER'S JOB, as for `WrapIfPred`'s `Var("m")`. Emitting `Var("d")`
    /// where no `d` is bound types as `Ty::Unit`, and `tread index` now requires `Ty::Int`
    /// (typecheck.rs, tightened the same day and for this reason), so the candidate is discarded
    /// at generation time. Before that tightening this operator could have produced a program that
    /// was KEPT and silently mis-evaluated.
    TReadIndex,
}

/// The operators the search DRAWS FROM. `Op::TReadIndex` is implemented and tested but is
/// deliberately NOT here — see `PARKED_OPS`.
pub const ALL_OPS: [Op; 11] = [
    Op::Tweak, Op::WrapIf, Op::WrapLoop, Op::Delete,
    Op::Dup, Op::SwapSiblings, Op::InsertMax, Op::ReplaceConst,
    Op::WrapIfPred, Op::ProbeRead, Op::StoreHere,
];

/// Implemented, tested, and NOT DRAWN — because enabling it today would cost the search and buy it
/// nothing. Kept as code rather than deleted so that flipping it on is one line once its blocker
/// clears.
///
/// `Op::TReadIndex` closes a real gap: `tests/shape_reachability.rs` measured that no operator could
/// lengthen an argument list, which put ladder rung 7 (`TRead(3, [d, i])`) outside the search space
/// at any edit count. With it, TRead/1 is one edit away and TRead/2 two (both measured).
///
/// THAT IS A FACT ABOUT SHAPES, AND THE BEHAVIOURAL FACT POINTS THE OTHER WAY.
/// `interp/tests/tread_index_is_inert.rs` MEASURES that appending an index changes nothing:
/// `Interp` resolves a TRead through `tables_nd` first and falls back to the scalar `tables`, and
/// `tables_nd` is declared, initialised to `Vec::new()`, read — and never populated, while every
/// `Interp::new` call site passes at most three scalars. The indices are never consulted.
///
/// So every candidate this operator can produce today is behaviourally identical to its parent and
/// strictly LARGER. FITNESS 3 is mates per COST, so each one is strictly worse and cannot be
/// accepted — while still consuming a `1/|ALL_OPS|` share of every draw and diluting the eleven
/// operators that can do something. Adding it to the drawn set would be a measurable regression to
/// the search dressed up as new capability.
///
/// UNPARK IT WHEN, and only when, `tables_nd` is populated AND tables are part of the genome.
/// GRAMMAR 9 records that second half as architectural: `Program { funcs, lineage }` has no table
/// field, tables arrive as an `Interp::new` constructor argument, so no mutation can change a
/// table's contents. Hand-filling the reduction table would make the rung work and the discovery
/// claim vacuous, which MASTER_PLAN:53 forbids.
/// `Op::AddFn` is parked for a DIFFERENT reason than TReadIndex, and the distinction matters.
/// TReadIndex is inert (it cannot change behaviour at all). AddFn is behaviour-PRESERVING by
/// construction -- it is a refactor -- so every candidate it produces is its parent plus a `Call`
/// node, which `interp::cost_of` charges 2.
///
/// ⚠ "BEHAVIOUR-PRESERVING BY CONSTRUCTION" WAS FALSE UNTIL 2026-09-11, and the sentence is kept
/// above because the correction is the point. AddFn lifted `Set("best", Max(Var("best"),
/// Var("vv")))` -- alpha-beta's score update -- out of the seed and dropped the write, because
/// `Node::Call` discards the callee's frame. The program played a different move and cost 91x LESS,
/// which under mates-per-cost makes a gutted candidate look FITTER, not worse. `contains_set` now
/// refuses it, `interp/tests/add_fn_is_behaviour_preserving.rs` pins it, and the cost claim is
/// measured at <= 1.004x rather than asserted. See `add_fn_drops_writes_RESULT.md`. Under FITNESS 3 (mates per COST) that is strictly
/// worse, so it cannot be accepted ON ITS OWN. Its value is as an ENABLER: it is the only operator
/// that can raise `funcs.len()`, which `shape_reachability.rs` measured at 0 of 858, and
/// `add-arg` cannot apply to anything until it runs (the entry is pinned to `choose(Pos, Int)`).
/// UNPARK WHEN there is a reason for a second function to EARN its call -- i.e. once something can
/// diverge the lifted body from its origin, or the cost model stops charging a bare call.
pub const PARKED_OPS: [Op; 2] = [Op::TReadIndex, Op::AddFn];

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
        // Handled by `try_at` before this point: it adds a FUNCTION, which this
        // (one node) -> (one node) signature cannot express. Listed so the match stays exhaustive
        // and a future operator cannot be added without deciding where it belongs.
        Op::AddFn => None,
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
        // Append ONE index to a tread. Capped at 2 indices: `TRead(3, [d, i])` is the widest read
        // any reference program performs, and an uncapped operator would grow argument lists
        // without bound, spending candidates on arity rather than on structure.
        Op::TReadIndex => match n {
            Node::TRead(t, args) if args.len() < 2 => {
                let mut a = args.clone();
                // Three index sources, and the mix is deliberate. Budget and Const are ALWAYS
                // well-typed, so the operator is never a guaranteed waste; `Var("d")` is the one
                // that can actually reach rung 7, and it is discarded by the type checker wherever
                // no Int `d` is in scope. That is the same trade WrapIfPred makes with Var("m").
                a.push(match r % 3 {
                    0 => Node::Budget,
                    1 => Node::Const((r % 9) as i8 - 4),
                    _ => Node::Var("d".into()),
                });
                Some(Node::TRead(*t, a))
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
        // ---- THE TWO MEMORY OPERATORS. Added 2026-09-08.
        //
        // WHY THEY EXIST. `tests/reachability.rs` proves the other nine operators cannot introduce
        // Probe, Key, Field or Store at any edit count, so hash reuse -- the ONLY rung ever
        // measured as fitter than the seed -- was outside the search space entirely. Plateau
        // tolerance alone would not have changed that: it fixes the VALLEY
        // (ladder_valley_RESULT.md), and reachability is a separate, prior blocker. Both had to be
        // lifted or a negative result would have been unattributable.
        //
        // WHAT THEY DELIBERATELY DO NOT DO. Neither inserts a transposition table. `ab_hash` is
        // +104 nodes of validity marker, depth comparison and three bound-type branches; these add
        // ONE read and ONE write. That is the same standard WrapIfPred was held to -- it
        // introduced `Pred` without introducing move ordering -- and it is what MASTER_PLAN line
        // 53 requires: the technique must be DISCOVERED, so the grammar may supply the primitive
        // and must not supply the algorithm. An operator that emitted probe-and-store together
        // would make the discovery vacuous, and is refused.
        //
        // `p` is the position parameter in every reference program's recursive function and in
        // `choose`. Where it is not in scope the type checker discards the candidate, which is the
        // cheap path (generation time, not gate time).
        Op::ProbeRead => match n {
            // An Int-typed LEAF becomes a slot read. Leaves only: replacing an interior expression
            // would delete a subtree, which is `Delete`'s job, not this one.
            Node::Const(_) | Node::Budget => {
                const FS: [FieldId; 5] =
                    [FieldId::Score, FieldId::Depth, FieldId::Flag, FieldId::Count, FieldId::Sum];
                Some(Node::Field(
                    Box::new(Node::Probe(Box::new(Node::Key(Box::new(Node::Var("p".into())))))),
                    FS[(r % 5) as usize],
                ))
            }
            _ => None,
        },
        Op::StoreHere => match n {
            // A statement-position node becomes a slot write. Same positions WrapIfPred accepts,
            // so the two compose: a store can later be made conditional on a predicate.
            //
            // The stored VALUE is Budget or a small Const, never a named local. mutate_at has no
            // scope information, so emitting `Var("r")` would be guessing at a binding that
            // usually does not exist and would be discarded as ill-typed. Budget is Unit -> Int
            // and always well-typed. Storing a not-yet-useful value is the point: the search has
            // to find the useful one, and Tweak/Replace can reach it from here.
            Node::Store(..) | Node::Set(..) | Node::Nop => {
                const FS: [FieldId; 5] =
                    [FieldId::Score, FieldId::Depth, FieldId::Flag, FieldId::Count, FieldId::Sum];
                let val = if r % 2 == 0 {
                    Node::Budget
                } else {
                    Node::Const((r % 9) as i8 - 4)
                };
                // APPEND, DO NOT REPLACE. This originally replaced the statement, which meant
                // landing on a `Set` destroyed an accumulator update -- `set best`, `set alpha` --
                // and wrecked the search outright.
                //
                // MEASURED CONSEQUENCE: generation 1 reported `mate-ok 0`, all 12 offspring
                // failing the correctness guard, so the plateau tolerance could never be exercised
                // because nothing survived to reach it.
                //
                // Appending makes this the one BEHAVIOUR-PRESERVING mutation in the set: a store
                // nothing reads cannot change what the search returns, only what it costs. That is
                // precisely the harmless half of the transposition-table valley (store-only,
                // measured at 0.997x), so it keeps all 25 answers and lands slightly cheaper-
                // than-nothing -- exactly the step the population is there to carry until a probe
                // arrives to read it back.
                Some(Node::seq(vec![
                    n.clone(),
                    Node::Store(
                        Box::new(Node::Key(Box::new(Node::Var("p".into())))),
                        FS[(r % 5) as usize],
                        Box::new(val),
                    ),
                ]))
            }
            _ => None,
        },
        Op::WrapIfPred => match n {
            // Same shape as WrapIf, but the condition is a PREDICATE on the move rather than a
            // budget comparison. Predicate choice is part of the search: all seven are reachable,
            // so the loop discovers WHICH property matters rather than being told it is captures.
            Node::Store(..) | Node::Set(..) | Node::Nop => {
                const PREDS: [PredId; 7] = [
                    PredId::IsCapture, PredId::GivesCheck, PredId::IsPromotion,
                    PredId::CapturedType, PredId::MovingType, PredId::FromSquare, PredId::ToSquare,
                ];
                Some(Node::If(
                    Box::new(Node::Pred(
                        Box::new(Node::Var("m".into())),
                        Box::new(Node::Var("p".into())),
                        PREDS[(r % 7) as usize],
                    )),
                    Box::new(n.clone()),
                    None,
                ))
            }
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
    // Program-level operators are dispatched here TOO, not only in `try_at`. These are two
    // independent entry points -- `try_at` is not a wrapper around this one -- and adding the
    // AddFn branch to only one of them made the operator silently never apply (0 of 200 attempts
    // per program) while compiling and testing clean.
    if matches!(op, Op::AddFn) { return try_add_fn(p, fi, k0).0; }
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
/// `Ret` unwinds a FRAME, and `Node::Call` converts a callee's `Flow::Ret` into a plain value
/// (`interp/lib.rs`: `Flow::Ret(v) | Flow::Normal(v) => v`). So lifting a subtree that contains a
/// `Ret` changes WHICH function returns — the original unwound the enclosing function, the lifted
/// copy unwinds only the new one. That is a silent semantic change, not a refactor, so `AddFn`
/// refuses those subtrees.
fn contains_ret(n: &Node) -> bool {
    matches!(n, Node::Ret(_)) || children(n).iter().any(|c| contains_ret(c))
}

/// A lift cannot carry a WRITE across a call boundary, so a subtree containing one must be refused.
///
/// MEASURED 2026-09-11, `interp/tests/add_fn_diagnose.rs`. On the bare alpha-beta seed AddFn lifted
/// `Set("best", Max(Var("best"), Var("vv")))` -- the score update at the heart of the algorithm --
/// into `lifted2(vv: Score) -> Unit`. `Node::Call` builds a FRESH env from the parameters, runs the
/// body against it and DROPS it; only the return value escapes. So the callee assigned its own local
/// `best` and the caller's was never updated. The seed played Move(6030); the "refactored" program
/// played Move(1025) and cost 1,463,398 against 133,906,979 -- 91x CHEAPER, because the search had
/// been gutted.
///
/// Nothing upstream catches this. `typecheck::free_vars` states the reason in its own comment --
/// "a `Set` inside the subtree creates its own name, so it is not a requirement on the site" --
/// which is right for deciding what a site REQUIRES and wrong for deciding what a lift must THREAD
/// OUT. `best` was therefore never made a parameter, and `scope_check` sees the `Set` as a binder,
/// so it reports no unbound read. A well-typed, well-scoped, semantically destroyed program.
///
/// THIS ALSO REVERSES THE SAFETY ARGUMENT FOR PARKING. `PARKED_OPS` says AddFn "cannot be accepted
/// ON ITS OWN" because a candidate is its parent plus a `Call` and so strictly costlier. A lift that
/// drops a write is not costlier, it is 91x cheaper, and FITNESS 3 is mates per COST. The operator
/// was not harmlessly-worse; it could be spuriously-BETTER. (Whether such a candidate still finds
/// enough mates to be selected is NOT measured here and is not claimed.)
///
/// Conservative on purpose: it refuses every `Set`, including one whose target the subtree itself
/// binds and which would lift correctly. Deciding that needs exactly the binder analysis shown above
/// to be wrong for this question, and the operator is parked, so a lost valid lift costs nothing
/// while a lost write corrupts a candidate silently. Same shape as the `contains_ret` refusal.
fn contains_set(n: &Node) -> bool {
    matches!(n, Node::Set(..)) || children(n).iter().any(|c| contains_set(c))
}

/// GRAMMAR 4's `add-fn`: "split a subtree into a new function and call it."
///
/// Measured absent for as long as anything has looked — `shape_reachability.rs` reports **0 of 858
/// applied mutations changed `funcs.len()`**, so a search seeded with a 1-function program could
/// only ever produce 1-function programs while `typecheck.rs:19` admits 1..4. Three quarters of the
/// declared program space was not unlikely, it was unreachable.
///
/// IT THREADS FREE VARIABLES AS PARAMETERS, which is what makes it correct rather than a hole
/// generator. A lifted subtree usually reads names its new function does not bind; those become the
/// function's parameters and the call site passes them. Without that this operator would be the
/// single richest source of exactly the defect `typecheck::scope_check` was added to stop.
///
/// It also subsumes what `add-arg` was needed for HERE. `add-arg` as declared cannot apply to a
/// 1-function program at all: `check_program` pins the entry to `choose(Pos, Int) -> Move`, so
/// adding a parameter to `funcs[0]` is rejected by construction, and there is no other function to
/// add one to until this operator creates it. The two declared-but-missing operators had a
/// dependency nobody had written down, and this is the one that comes first.
fn try_add_fn(p: &Program, fi: usize, k0: usize) -> (Option<Program>, Placement) {
    // GRAMMAR 3 admits 1..4 functions.
    if p.funcs.len() >= 4 { return (None, Placement::NoMatch); }

    let mut k = k0;
    let sub = match get_nth(&p.funcs[fi].body, &mut k) { Some(x) => x, None => return (None, Placement::NoMatch) };

    if contains_ret(&sub) { return (None, Placement::NoMatch); }
    // A write inside the lifted body would be made to the callee's discarded frame. See
    // `contains_set` for the measurement that found this on the alpha-beta seed.
    if contains_set(&sub) { return (None, Placement::NoMatch); }
    // Lifting a leaf buys a call node and nothing else; lifting the whole body just renames it.
    if matches!(sub, Node::Const(_) | Node::Var(_) | Node::Nop | Node::Budget) {
        return (None, Placement::NoMatch);
    }
    if sub == p.funcs[fi].body { return (None, Placement::NoMatch); }

    let f = &p.funcs[fi];
    let ret = match crate::typecheck::type_of_in(&sub, f, p) {
        Some(t) => t,
        None => return (None, Placement::IllTyped),
    };
    let env = crate::typecheck::env_of(f, p);
    let mut params: Vec<(String, Ty)> = Vec::new();
    for name in crate::typecheck::free_vars(&sub) {
        match env.get(&name) {
            Some(t) => params.push((name, *t)),
            // A name with no known type cannot become a typed parameter. Refusing is the whole
            // point -- guessing one would reintroduce the unbound-read defect by another route.
            None => return (None, Placement::NoMatch),
        }
    }

    let new_idx = p.funcs.len();
    let call = Node::Call(new_idx, params.iter().map(|(n, _)| Node::Var(n.clone())).collect());

    // SELF-CHECKING SECOND PASS. `get_nth` located the subtree and `map_nth` replaces it, and this
    // operator is the only caller that needs the two walks to agree on the SAME tree (crossover
    // reads a donor and writes a recipient, so it never does). Rather than assume they agree, the
    // replacement closure verifies it landed on the node that was lifted and declines otherwise --
    // a disagreement becomes a no-op instead of a program that calls a function built from some
    // other subtree.
    let mut k2 = k0;
    let mut applied = false;
    let mut matched = true;
    let body = map_nth(&p.funcs[fi].body, &mut k2, &mut applied, &mut |n: &Node| {
        if *n == sub { Some(call.clone()) } else { matched = false; None }
    });
    if !applied || !matched { return (None, Placement::NoMatch); }

    let mut out = p.clone();
    out.funcs.push(Func { name: format!("lifted{new_idx}"), params, ret, body: sub });
    out.funcs[fi].body = body;
    match crate::typecheck::check_program(&out) {
        Ok(()) => (Some(out), Placement::Applied),
        Err(_) => (None, Placement::IllTyped),
    }
}

pub fn try_at(p: &Program, op: Op, rng: &mut Rng, fi: usize, k0: usize)
    -> (Option<Program>, Placement)
{
    // Program-level: it adds a FUNCTION, which `apply_op`'s (one node) -> (one node) signature
    // cannot express -- the same reason `crossover` is a free function rather than an `Op` arm.
    if matches!(op, Op::AddFn) { return try_add_fn(p, fi, k0); }
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

/// Extract the k-th node in pre-order, for crossover's donor side.
fn get_nth(n: &Node, k: &mut usize) -> Option<Node> {
    if *k == 0 { *k = usize::MAX; return Some(n.clone()); }
    *k = k.saturating_sub(1);
    for c in children(n) {
        if let Some(found) = get_nth(c, k) { return Some(found); }
    }
    None
}

/// CROSSOVER: graft a typed subtree from a DONOR program into a type-compatible site in the
/// RECIPIENT. Generic and chess-blind -- it moves whatever subtree it lands on.
///
/// WHY IT IS A FUNCTION AND NOT AN `Op` VARIANT. Every other operator has the signature
/// (one program) -> (one program) and is dispatched through `mutate_at`. Crossover needs a SECOND
/// program, so bolting it into that enum would mean threading a donor through every call site of a
/// nine-variant match that does not want one. Stated plainly rather than forced into the shape the
/// brief assumed, because the brief's `Op::Cross(a, b)` cannot be typed against the existing API.
///
/// TYPE COMPATIBILITY IS DECIDED BY THE EXISTING CHECKER, NOT BY A SECOND ONE. The obvious
/// implementation infers the donor subtree's type and looks for a site of the same type, which
/// means writing type inference a second time and letting it drift from typecheck.rs -- and a
/// crossover that silently disagrees with the checker would produce candidates rejected for
/// reasons no log explains. Instead this SPLICES and then asks `check_program`, keeping the first
/// graft that type-checks. GRAMMAR 3's economics say exactly this: an ill-typed candidate costs a
/// tree walk, and a bad one that reaches the gate costs thousands of games.
///
/// This is the ONLY route by which a hybrid may appear. No operator inserts a multi-primitive
/// gadget; a program that combines UCT's averaging backup with alpha-beta's window has to be built
/// by moving one subtree at a time, which is what makes "the hybrid was discovered" a real claim.
pub fn crossover(recipient: &Program, donor: &Program, rng: &mut Rng) -> Option<Program> {
    let rn: usize = recipient.funcs.iter().map(|f| count_nodes(&f.body)).sum();
    let dn: usize = donor.funcs.iter().map(|f| count_nodes(&f.body)).sum();
    if rn == 0 || dn == 0 { return None; }
    for _ in 0..96 {
        let dfi = rng.below(donor.funcs.len());
        let dsize = count_nodes(&donor.funcs[dfi].body);
        let mut dk = rng.below(dsize);
        let sub = match get_nth(&donor.funcs[dfi].body, &mut dk) { Some(x) => x, None => continue };
        let rfi = rng.below(recipient.funcs.len());
        // SELECT A DONOR THAT CAN LEGALLY LAND, rather than splicing blind and hoping.
        //
        // Most alpha-beta subtrees read its own internals (`a`, `b`, `vv`). Grafted into the MCTS
        // seed, which has no such names, they used to produce a program that type-checked and RAN
        // while silently reading Unit -- `typecheck::scope_check` documents the three fallbacks
        // that made that invisible. With scope enforced those grafts are correctly rejected, and
        // blind sampling then found nothing in 96 tries: `reachability.rs` measured alpha-beta ->
        // UCT collapse from {"Max", "Set"} to {} the moment holed programs stopped being accepted.
        //
        // So the fix is not more retries, it is choosing donors whose free variables the
        // destination can actually supply. `always_in_scope` is the site-independent guarantee
        // (params + Set targets); a subtree needing only those is graftable at ANY position of the
        // recipient function. This narrows what crossover attempts and cannot admit a hole --
        // `check_program` below remains the final authority.
        let need = crate::typecheck::free_vars(&sub);
        if !need.is_empty() {
            let have = crate::typecheck::always_in_scope(&recipient.funcs[rfi]);
            if !need.iter().all(|n| have.iter().any(|h| h == n)) { continue; }
        }
        let rsize = count_nodes(&recipient.funcs[rfi].body);
        let mut rk = rng.below(rsize);
        let mut applied = false;
        let grafted = map_nth(&recipient.funcs[rfi].body, &mut rk, &mut applied,
                              &mut |_n: &Node| Some(sub.clone()));
        if !applied { continue; }
        let mut out = recipient.clone();
        out.funcs[rfi].body = grafted;
        // Reject a no-op graft: splicing a subtree onto an identical one burns a candidate slot
        // and reports success, the same defect `applied` was added to catch for mutation.
        if format!("{:?}", out.funcs[rfi].body) == format!("{:?}", recipient.funcs[rfi].body) { continue; }
        if crate::typecheck::check_program(&out).is_ok() { return Some(out); }
    }
    None
}

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
