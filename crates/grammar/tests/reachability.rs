//! REACHABILITY: can the mutation operators actually build the ladder they are supposed to climb?
//!
//! GRAMMAR 9 asserts "a path of single mutations from the seed exists where every step is fitter".
//! GRAMMAR 4 supplies the operators that would have to walk it. Nothing checked the two against
//! each other, and they do not compose: the operators can tune constants, rearrange existing
//! structure, and wrap statements in an If or a Loop.
//!
//! ⚠ THE NEXT SENTENCE USED TO READ "but they cannot introduce a PRIMITIVE the program does not
//! already contain". THAT IS STALE and is refuted by this file's own output: `constructible()`
//! prints `{Budget, Const, Field, Key, Loop, Max, Pred, Probe, Store}`. `Op::ProbeRead` and
//! `Op::StoreHere` were added after this header was written, and they introduce every TT primitive
//! (ProbeRead emits `Field(Probe(Key(Var "p")), f)` in a single edit). Hash reuse is REACHABLE by
//! mutation; what stops it is the conjunctive VALLEY -- probe alone 0.991x, store alone 0.997x,
//! the pair 1.024x -- not expressibility. Corrected 2026-09-10 after the stale header misled a
//! whole analysis into calling the barrier an expressiveness gap.
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
// RENAMED 2026-09-10. This was `the_operators_cannot_introduce_a_primitive_the_seed_lacks`, which
// asserts the OPPOSITE of what the body now checks -- `assert!(unreachable.is_empty())`, i.e. every
// declared rung IS constructible. The body has been correct since ProbeRead/StoreHere landed on
// 2026-09-08 and says so at length; only the NAME still carried the old world.
//
// A green test whose name states a negative claim is that claim in its most authoritative form: it
// shows up in `cargo test` output as `..._cannot_introduce_a_primitive_... ok`, which reads as the
// repo CONFIRMING unreachability on every run. It cost a full analysis on 2026-09-10 -- the file's
// header said the same thing, I believed it, and built a conclusion about an "expressiveness gap"
// that this test's own output refutes.
fn every_declared_rung_is_constructible() {
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
    //
    // ⚠ SETTLED, THEN FIXED, BOTH ON 2026-09-10. `tests/shape_reachability.rs` -- the follow-up
    // named on the line above -- measured at shape (kind + arity) granularity that rung 7's missing
    // element was exactly `("TRead", 2)`: seed TRead arities `[0]`, newly constructible `[]`. So
    // "very likely unreachable" became MEASURED unreachable.
    //
    // `Op::TReadIndex` then closed the SHAPE gap: TRead/1 is constructible in one edit and TRead/2
    // in two, measured by composing the operator with itself. But it is PARKED, not drawn
    // (`mutate::PARKED_OPS`), because `interp/tests/tread_index_is_inert.rs` measures that
    // appending an index changes NOTHING -- `tables_nd` is never populated, so the indices are
    // never consulted. Every candidate it can make is its parent, larger; under FITNESS 3 that is
    // strictly worse. So rung 7 is EXPRESSIBLE and still not climbable, and the operator is code
    // waiting on an architectural change rather than a live part of the search.
    //
    // Still open, and the shape file asserts it: 0 of 858 applied mutations change
    // `Program::funcs.len()`, so GRAMMAR 4's `add-fn` remains absent and every program the search
    // can reach has exactly one function, against a type checker that admits 1..4.
    //
    // This test is left asserting only what IT can prove at kind granularity; the shape file
    // carries the rest.
    let names: Vec<&str> = unreachable.iter().map(|(n, _)| *n).collect();

    // STATE AS OF 2026-09-08, after Op::ProbeRead and Op::StoreHere landed. Every declared rung is
    // now CONSTRUCTIBLE: the operators can introduce Pred (capture extension) and
    // Probe/Key/Field/Store (hash reuse). This test has now fired twice as designed -- once for
    // WrapIfPred, once for the memory operators -- and each time the failure WAS the signal.
    //
    // READ THIS BEFORE CONCLUDING THE LADDER IS CLIMBABLE. Reachability is NECESSARY and NOT
    // SUFFICIENT, and the difference is measured, not hypothetical. ladder_valley_RESULT.md scores
    // the two halves of hash reuse with the search track's own fitness: probe-only 0.991x,
    // store-only 0.997x, both together 1.024x. Each half alone is WORSE than the seed, so the
    // payoff is conjunctive and a search accepting only `rate > best_rate` can never take the
    // first step. Constructible does not imply a monotone path exists.
    //
    // Both blockers had to be lifted together, which is why the operators and the population /
    // plateau-tolerant acceptance landed in the same change. Lifting either alone would have
    // produced a negative result that could not be attributed to a cause.
    assert!(
        unreachable.is_empty(),
        "a rung became UNREACHABLE again -- an operator that could build one of its primitives was \
         removed or narrowed. Every declared rung has been constructible since the memory \
         operators landed, so this is a regression in the search space itself: {unreachable:?}"
    );
    assert_eq!(
        buildable,
        ["Budget", "Const", "Field", "Key", "Loop", "Max", "Pred", "Probe", "Store"]
            .into_iter()
            .collect::<BTreeSet<_>>(),
        "The set of node kinds the operators can introduce has CHANGED. That set is the entire \
         limit on what the search track can discover, so it should change deliberately and be \
         recorded here. It was {{Budget, Const, Loop, Max}} originally, {{.., Pred}} after \
         WrapIfPred, and gained {{Probe, Key, Field, Store}} with ProbeRead and StoreHere."
    );
}


/// CROSSOVER must be able to move the primitives a HYBRID would need between lineages.
///
/// The two seeds are disjoint in exactly the way that matters: bare alpha-beta has no `Avg`, no
/// `Sample` and no `Field(count/sum)` -- those are UCT's averaging backup and its stochastic
/// selection -- while UCT has no window arithmetic. So a hybrid cannot be reached by mutating
/// either seed alone at any edit count; the primitives are not merely unreachable, they are in the
/// OTHER program. Crossover is the only route, and this asserts it actually carries them rather
/// than assuming a generic subtree graft happens to.
///
/// It is deliberately a REACHABILITY test, not a quality one. Moving `Avg` into alpha-beta almost
/// certainly produces a worse program; the claim here is only that the search SPACE contains the
/// combination, which is the precondition for discovery being possible at all.
#[test]
fn crossover_can_move_primitives_between_lineages() {
    let ab = reference::bare_alpha_beta();
    let uct = reference::uct_mcts();
    let ab_kinds = prog_kinds(&ab);
    let uct_kinds = prog_kinds(&uct);

    // What UCT has that alpha-beta does not. If this is empty the test is vacuous, so assert it.
    let only_uct: BTreeSet<&str> = uct_kinds.difference(&ab_kinds).copied().collect();
    println!("kinds only in UCT: {only_uct:?}");
    assert!(
        !only_uct.is_empty(),
        "the two lineage seeds share every node kind, so crossover has nothing to carry and the \
         second lineage adds no reachability -- check the seeds before trusting any hybrid claim"
    );

    // Graft UCT subtrees into alpha-beta many times and collect every kind that arrives.
    let mut moved = BTreeSet::new();
    for k in 0..600u64 {
        let mut rng = mutate::Rng::new(k ^ 0xC0FFEE);
        if let Some(child) = mutate::crossover(&ab, &uct, &mut rng) {
            for kd in prog_kinds(&child) {
                if !ab_kinds.contains(kd) { moved.insert(kd); }
            }
        }
    }
    println!("crossover moved UCT -> alpha-beta: {moved:?}");
    assert!(
        !moved.is_empty(),
        "crossover produced no candidate carrying a kind alpha-beta lacks. Either it never \
         type-checks a graft, or it only ever grafts kinds both seeds already share -- both make \
         the hybrid unreachable and neither is visible without this test."
    );

    // And the reverse direction, since a hybrid may be built on either seed.
    let mut moved_back = BTreeSet::new();
    for k in 0..600u64 {
        let mut rng = mutate::Rng::new(k ^ 0xBEEF11);
        if let Some(child) = mutate::crossover(&uct, &ab, &mut rng) {
            for kd in prog_kinds(&child) {
                if !uct_kinds.contains(kd) { moved_back.insert(kd); }
            }
        }
    }
    println!("crossover moved alpha-beta -> UCT: {moved_back:?}");

    // THE NAMED PRIMITIVES. A hybrid needs UCT's averaging backup (Avg, and Field for count/sum)
    // and, for hash reuse, Probe/Store. Assert each individually so a partial regression names the
    // primitive that stopped moving instead of failing on a set comparison.
    for kd in ["Probe", "Store", "Avg", "Field"] {
        assert!(
            moved.contains(kd),
            "crossover no longer carries `{kd}` from UCT into alpha-beta. That primitive is in the \
             hybrid's critical path and this is the only operator that can move it."
        );
    }

    // TWO DEVIATIONS FROM THE BRIEF, RECORDED RATHER THAN PAPERED OVER.
    //
    // `Sample` was named as a primitive crossover should move, and it CANNOT BE: the reference UCT
    // does not contain one. GRAMMAR 6's uct_mcts selects with `Argmax` over the UCT formula, which
    // is what faithful UCT does -- `Sample` is the grammar's stochastic-selection primitive and no
    // reference program uses it. So there is nowhere to move it FROM. Asserting it would be
    // asserting a property of a program that does not exist.
    assert!(
        !uct_kinds.contains("Sample"),
        "the reference UCT now contains Sample -- the note above is stale and the assertion list \
         should be extended to cover it."
    );
    // `Loop` IS unique to UCT and is never successfully grafted. Recorded as the current state so
    // a future improvement to crossover shows up here as a failure rather than passing silently.
    assert!(
        !moved.contains("Loop"),
        "crossover now moves `Loop` between lineages, which it previously could not. Good news: \
         update this assertion and note what changed."
    );
    assert!(
        !moved_back.is_empty(),
        "crossover is one-directional: it carries kinds into alpha-beta but not into UCT. A \
         hybrid built on the MCTS seed would be unreachable and the asymmetry would be silent."
    );
}

/// Can crossover carry the TT rung's primitives from the MCTS lineage into alpha-beta — and can a
/// SINGLE graft deliver both halves at once?
///
/// WHY THIS IS THE DECIDING QUESTION. This file's header records that no mutation operator can
/// introduce `Probe`/`Key`/`Field`/`Store`, so hash reuse "is not reachable at any edit count". That
/// is true of MUTATION and says nothing about CROSSOVER, which is a separate route
/// (`evolve.rs:1586` runs it on one candidate in four). Measured 2026-09-09, `uct_mcts` tags
/// `P10S5K15F10` — byte-identical to `ab_hash`. The second lineage seed already contains a full TT
/// complement at every call site, and `donors` (`evolve.rs:1561`) flat-maps over EVERY lineage's
/// population, so a MAIN recipient can draw an MCTS donor.
///
/// `crossover_can_move_primitives_between_lineages` above asserts only that the moved set is
/// NON-EMPTY. It never names these four kinds, so it passes just as happily if the only thing
/// crossover ever carries is `Avg`.
///
/// The valley is CONJUNCTIVE — probe-only 0.991x, store-only 0.997x, both 1.024x — so a child
/// carrying `Probe` without `Store` is on the valley floor and cannot be accepted. Whether ONE graft
/// can deliver both is therefore the difference between "the rung needs a lucky pair of retained
/// halves plus a second crossover" and "the rung is one graft away from the seed".
#[test]
fn crossover_can_carry_the_tt_rung_from_mcts() {
    let ab = reference::bare_alpha_beta();
    let uct = reference::uct_mcts();
    let ab_kinds = prog_kinds(&ab);
    let tt_kinds = ["Probe", "Store", "Key", "Field"];

    // Non-vacuity, asserted rather than assumed: the recipient must LACK what the donor HAS.
    for k in tt_kinds {
        assert!(!ab_kinds.contains(k), "alpha-beta already contains {k} — this test proves nothing");
        assert!(prog_kinds(&uct).contains(k), "UCT lacks {k} — it cannot donate what it does not have");
    }

    let (mut any_tt, mut both_halves, mut children) = (0usize, 0usize, 0usize);
    let mut arrived: BTreeSet<&str> = BTreeSet::new();
    for k in 0..4000u64 {
        let mut rng = mutate::Rng::new(k ^ 0x7EA_5EED);
        let Some(child) = mutate::crossover(&ab, &uct, &mut rng) else { continue };
        children += 1;
        let ck = prog_kinds(&child);
        let got: Vec<&str> = tt_kinds.iter().copied().filter(|k| ck.contains(k)).collect();
        if !got.is_empty() { any_tt += 1; for g in &got { arrived.insert(g); } }
        if ck.contains("Probe") && ck.contains("Store") { both_halves += 1; }
    }

    println!("crossover ab<-uct: {children} well-typed children of 4000 attempts");
    println!("  carrying >=1 TT kind : {any_tt} ({:.1}%)", 100.0 * any_tt as f64 / children.max(1) as f64);
    println!("  carrying Probe AND Store (the united rung, in ONE graft): {both_halves} ({:.1}%)",
             100.0 * both_halves as f64 / children.max(1) as f64);
    println!("  TT kinds that ever arrived: {arrived:?}");

    assert!(
        any_tt > 0,
        "crossover never carried a single TT primitive from UCT into alpha-beta in {children} \
         well-typed children. Then the rung is unreachable by BOTH routes -- mutation cannot build \
         these kinds (see this file's header) and crossover cannot move them -- and every claim \
         about EPS retaining 'halves' is moot, because no half can ever appear."
    );
}
