//! REACHABILITY: can the mutation operators actually build the ladder they are supposed to climb?
//!
//! GRAMMAR 9 asserts "a path of single mutations from the seed exists where every step is fitter".
//! GRAMMAR 4 supplies the operators that would have to walk it. Nothing checked the two against
//! each other, and they do not compose: the operators can tune constants, rearrange existing
//! structure, and wrap statements in an If or a Loop, but they cannot introduce a PRIMITIVE the
//! program does not already contain.
//!
//! Consequences, measured before this test existed: `evolve` ran ~690 candidates across two runs
//! and accepted zero. Two of the three declared rungs need a primitive the seed lacks and no
//! operator builds -- capture extension needs `Pred`, hash reuse needs `Probe`/`Key`/`Field`/
//! `Store` -- so neither is reachable at any edit count, depth or budget.
//!
//! The third, table reduction, is NOT settled by this test and the first run of it said so. It
//! needs `TRead(3, [d, i])`, and the KIND `TRead` is already in the seed as `TRead(0)`/`TRead(1)`
//! with empty argument lists, so a kind-granularity check reports "nothing missing". No operator
//! constructs a TRead or appends an argument to one, so it is probably unreachable as well --
//! but this instrument cannot prove that, and it says so rather than rounding up.
//!
//! This test derives the constructible set EMPIRICALLY, by applying every operator at every
//! position of every reference program and recording which node kinds appear that were not in the
//! input. Hardcoding my reading of mutate.rs would only re-assert the belief being tested.
use grammar::ast::*;
use grammar::mutate::{self, Op};
use grammar::reference;
use std::collections::BTreeSet;

fn kind(n: &Node) -> &'static str {
    use Node::*;
    match n {
        Budget => "Budget", Const(_) => "Const", Var(_) => "Var", OutcomeLit(_) => "OutcomeLit",
        Nop => "Nop", Moves(_) => "Moves", Terminal(_) => "Terminal", Key(_) => "Key",
        Eval(_) => "Eval", Ret(_) => "Ret", Probe(_) => "Probe", Field(..) => "Field",
        Set(..) => "Set", Apply(..) => "Apply", Max(..) => "Max", Min(..) => "Min",
        Avg(..) => "Avg", ScoreOf(..) => "ScoreOf", Cmp(..) => "Cmp", Pred(..) => "Pred",
        Loop(..) => "Loop", Store(..) => "Store", Mix(..) => "Mix", Foreach(..) => "Foreach",
        Argmax(..) => "Argmax", Sort(..) => "Sort", Sample(..) => "Sample", Let(..) => "Let",
        If(..) => "If", Call(..) => "Call", Arith(..) => "Arith", TRead(..) => "TRead",
    }
}

fn kinds_in(n: &Node, out: &mut BTreeSet<&'static str>) {
    out.insert(kind(n));
    for c in child_refs(n) { kinds_in(c, out); }
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

fn prog_kinds(p: &Program) -> BTreeSet<&'static str> {
    let mut s = BTreeSet::new();
    for f in &p.funcs { kinds_in(&f.body, &mut s); }
    s
}

/// Every node kind any operator INTRODUCES, found by applying all of them everywhere.
fn constructible() -> BTreeSet<&'static str> {
    // ALL_OPS, not a copy of it. The first version listed the eight operators by hand, so when
    // WrapIfPred was added the test kept measuring the OLD set and reported "still unreachable"
    // for a rung that had just become reachable -- a check that silently stops checking the thing
    // it exists for. Same hardcoded-list trap found three times elsewhere today.
    let ops = mutate::ALL_OPS;
    let mut new_kinds = BTreeSet::new();
    for (_, prog) in reference::all() {
        let before = prog_kinds(&prog);
        for op in ops {
            for fi in 0..prog.funcs.len() {
                for k in 0..400 {
                    let mut rng = mutate::Rng::new((k as u64) << 8 ^ fi as u64 ^ 0xABCD);
                    if let Some(m) = mutate::mutate_at(&prog, op, &mut rng, fi, k) {
                        for kd in prog_kinds(&m) {
                            if !before.contains(kd) { new_kinds.insert(kd); }
                        }
                    }
                }
            }
        }
    }
    new_kinds
}

#[test]
fn the_operators_cannot_introduce_a_primitive_the_seed_lacks() {
    let seed = prog_kinds(&reference::bare_alpha_beta());
    let buildable = constructible();

    // Every rung, and the primitives it needs that the seed does not already have.
    let rungs: Vec<(&str, Program)> = vec![
        ("capture extension (rung 6)", reference::capture_extension()),
        ("table reduction (rung 7)", reference::table_reduction()),
        ("alpha-beta + hash reuse", reference::ab_hash()),
    ];

    let mut unreachable = Vec::new();
    for (name, prog) in &rungs {
        let needed: Vec<&str> = prog_kinds(prog)
            .into_iter()
            .filter(|k| !seed.contains(k) && !buildable.contains(k))
            .collect();
        println!("{name:<32} missing primitives: {needed:?}");
        if !needed.is_empty() { unreachable.push((*name, needed)); }
    }
    println!("\noperators can introduce: {buildable:?}");

    // THIS TEST DOCUMENTS A KNOWN DEFECT AND MUST NOT BE "FIXED" BY DELETING IT. It asserts the
    // CURRENT, BROKEN state on purpose; when an insert-primitive operator lands, this FAILS, and
    // that failure is the signal the ladder became climbable.
    //
    // IT ALSO REFUTED THE CLAIM THAT PROMPTED IT, on its first run, which is why it asserts
    // per-rung rather than "all rungs". I had written that NO rung is reachable. This test says
    // two of three are provably unreachable and the third cannot be judged at this granularity:
    //
    //   capture extension  missing ["Pred"]                          UNREACHABLE, proven
    //   hash reuse         missing ["Field","Key","Probe","Store"]    UNREACHABLE, proven
    //   table reduction    missing []                                 NOT PROVEN EITHER WAY
    //
    // Table reduction needs `TRead(3, [d, i])` -- table 3, read with two ARGUMENTS. The KIND
    // `TRead` is already in the seed (TRead(0) and TRead(1), both with empty argument lists), so a
    // kind-granularity check cannot see the gap. No operator constructs a TRead or appends an
    // argument to one, so it is very likely unreachable too -- but "very likely" is not what a
    // test should assert, and the honest limit of this instrument is node KINDS, not node shapes.
    //
    // Assert only the two that are proven. A shape-level reachability check is the follow-up.
    let names: Vec<&str> = unreachable.iter().map(|(n, _)| *n).collect();

    // STATE AS OF 2026-09-08, after Op::WrapIfPred landed. This test previously asserted that
    // capture extension was UNREACHABLE and failed the moment that stopped being true, which is
    // exactly what it was written to do -- the failure message said "GOOD NEWS: record which
    // operator did it". WrapIfPred did it, and the assertions are updated to the new state rather
    // than deleted, so the next change is caught the same way.
    assert!(
        !names.contains(&"capture extension (rung 6)"),
        "capture extension became UNREACHABLE again -- an operator that could build a Pred was \
         removed or narrowed. That is a regression: rung 6 is the smallest step from the seed \
         (+9 nodes) and the only one the search track can currently attempt."
    );
    assert!(
        names.contains(&"alpha-beta + hash reuse"),
        "hash reuse became reachable -- an operator can now build Probe/Key/Field/Store. That is \
         the ONLY rung measured as FITTER than the seed (0.98x its cost at D=3). NOTE, measured \
         2026-09-08 (ladder_valley_RESULT.md): making it reachable is NOT sufficient and this \
         assertion firing is NOT good news on its own. Each half of the rung is measured WORSE \
         than the seed -- probe-only 0.991x, store-only 0.997x, both together 1.024x -- so a \
         search accepting only `rate > best_rate` can never take either step and can never \
         assemble the pair. Reachability is necessary, a monotone path is not implied, and here \
         it provably does not exist. If an operator makes this constructible, the search ALSO \
         needs to tolerate the valley before re-running the track."
    );
    assert_eq!(
        buildable,
        ["Budget", "Const", "Loop", "Max", "Pred"].into_iter().collect::<BTreeSet<_>>(),
        "The set of node kinds the operators can introduce has CHANGED. That set is the entire \
         limit on what the search track can discover, so it should change deliberately and be \
         recorded here. It was {{Budget, Const, Loop, Max}} until WrapIfPred added Pred."
    );
}
