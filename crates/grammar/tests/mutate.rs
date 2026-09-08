//! Every mutation must produce a TYPE-CHECKING program. GRAMMAR 3's whole economics is that
//! an ill-typed candidate is rejected by a tree walk instead of by thousands of gate games,
//! so a mutation operator that can emit an ill-typed tree defeats the design.
use grammar::mutate::{self, Rng, ALL_OPS};
use grammar::{reference, typecheck};

#[test]
fn every_mutation_typechecks() {
    let mut rng = Rng::new(0x5EED_5EED);
    let mut produced = 0usize;
    for (name, base) in reference::all() {
        for i in 0..400 {
            let mut r = Rng::new(1000 + i as u64);
            if let Some(m) = mutate::mutate_program(&base, &mut r) {
                typecheck::check_program(&m)
                    .unwrap_or_else(|e| panic!("mutation of {name} is ill-typed: {}", e.what));
                produced += 1;
            }
        }
    }
    let _ = rng.next();
    assert!(produced > 500, "only {produced} mutations produced; operators may not be applying");
    println!("{produced} mutations, all type-checked");
}

#[test]
fn mutations_are_deterministic_from_a_seed() {
    // SCHEMAS 2 stores the mutation list; a candidate that passes a gate must be exactly
    // reproducible from (parent, seed).
    let base = reference::bare_alpha_beta();
    for seed in [1u64, 7, 99] {
        let a = mutate::mutate_program(&base, &mut Rng::new(seed));
        let b = mutate::mutate_program(&base, &mut Rng::new(seed));
        assert_eq!(format!("{:?}", a), format!("{:?}", b), "seed {seed} not reproducible");
    }
}

#[test]
fn mutations_actually_change_the_program() {
    let base = reference::bare_alpha_beta();
    let mut changed = 0;
    for seed in 0..200u64 {
        if let Some(m) = mutate::mutate_program(&base, &mut Rng::new(seed + 1)) {
            if format!("{:?}", m) != format!("{:?}", base) { changed += 1; }
        }
    }
    assert!(changed > 100, "only {changed}/200 mutations changed anything");
}

#[test]
fn every_operator_applies_somewhere() {
    let base = reference::bare_alpha_beta();
    for op in ALL_OPS {
        let mut hit = false;
        for seed in 0..600u64 {
            if mutate::mutate(&base, op, &mut Rng::new(seed + 1)).is_some() { hit = true; break; }
        }
        assert!(hit, "operator {op:?} never applied to the seed in 600 tries");
    }
}

/// Report, per operator, how its placements actually resolve on the seed.
///
/// GRAMMAR 4 declares the operator set as a Given-column entry, so the composition of that set
/// is a number the project owes rather than assumes. `mutate_at` collapses "no node of the
/// right shape here" and "applied but produced an ill-typed program" into the same None, and
/// those mean different things: the first says nothing about the operator, the second says the
/// operator is effectively absent from the set.
#[test]
fn every_operator_is_reported_by_how_it_fails() {
    use grammar::mutate::{try_at, Placement};
    let base = reference::bare_alpha_beta();
    println!("{:<14} {:>9} {:>9} {:>9}", "operator", "applied", "nomatch", "illtyped");
    for op in ALL_OPS {
        let (mut ok, mut nomatch, mut ill) = (0, 0, 0);
        let mut rng = Rng::new(0xA11 ^ format!("{op:?}").len() as u64);
        for fi in 0..base.funcs.len() {
            for k in 0..80 {
                match try_at(&base, op, &mut rng, fi, k).1 {
                    Placement::Applied => ok += 1,
                    Placement::NoMatch => nomatch += 1,
                    Placement::IllTyped => ill += 1,
                }
            }
        }
        println!("{:<14} {ok:>9} {nomatch:>9} {ill:>9}", format!("{op:?}"));
        // An operator that can NEVER be applied anywhere on the seed is not in the effective
        // set, whatever the document says. That is a finding, not a crash — but it must be
        // visible, so assert it and let the failure carry the number.
        assert!(ok > 0, "{op:?} never applied successfully anywhere on the seed \
                         ({nomatch} no-match, {ill} ill-typed)");
    }
}

/// The operator draw must be UNIFORM across sequentially-seeded generators.
///
/// Callers seed a fresh Rng per candidate from small structured values like
/// `(gen << 24) ^ candidate`. A bare xorshift64's first output is correlated with its seed, so
/// `first_next() % 8` was not a uniform choice — 119 real proposals came out Delete 40,
/// InsertMax 39, SwapSiblings 2 against an expected ~15 each. That is a 20x skew in WHICH
/// PART OF THE DECLARED OPERATOR SET the search actually explores, and nothing else in the
/// system would have reported it.
#[test]
fn operator_choice_is_uniform_across_fresh_seeds() {
    let mut counts = [0usize; ALL_OPS.len()];
    let n = 8000;
    for g in 0..20u64 {
        for i in 0..(n / 20) as u64 {
            // exactly the shape the search uses
            let mut rng = Rng::new((g << 24) ^ i ^ 0xBEEF);
            counts[rng.below(ALL_OPS.len())] += 1;
        }
    }
    let expect = n as f64 / ALL_OPS.len() as f64;
    for (i, &c) in counts.iter().enumerate() {
        let dev = (c as f64 - expect).abs() / expect;
        assert!(dev < 0.15,
            "{:?} drawn {c} times against {expect:.0} expected ({:.0}% off) — the operator \
             choice is not uniform, so the search explores a skewed subset of the declared set",
            ALL_OPS[i], dev * 100.0);
    }
}
