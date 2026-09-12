//! Closes a caveat I raised on my OWN measurement.
//!
//! `mutation_function_bias.rs` measured where `mutate_program_n` places edits, using the
//! hand-written multi-function REFERENCE programs. Its caveat: an AddFn lift produces a `funcs[1]`
//! that is a subtree CUT FROM `funcs[0]`, so the size ratio -- which that test showed is what drives
//! the split -- is different. The direction was robust; the rate for a lifted program was not
//! measured. This measures it.
//!
//! It matters for the `Op::AddFn` unpark condition (`mutate.rs:148`): "once something can diverge
//! the lifted body from its origin". A lifted function that is never edited cannot diverge.
//!
//! ASSERTS NOTHING about the rate. Prints it. A threshold written from a prediction would have
//! failed on correct behaviour -- which is exactly what happened to my prediction last time.

use grammar::mutate::{self, Op, Rng};
use grammar::{reference, Program};

#[test]
fn how_often_is_an_addfn_lifted_function_edited() {
    let seeds: Vec<(&str, Program)> = reference::all()
        .into_iter()
        .filter(|(_, p)| p.funcs.len() == 1)
        .take(3)
        .collect();
    assert!(!seeds.is_empty(), "no 1-function reference program -- broken fixture, not a finding");

    for (name, seed) in &seeds {
        // Apply AddFn DIRECTLY. It is parked out of ALL_OPS, but `mutate_at` takes any Op, so the
        // operator is still exercisable without touching what the search draws.
        let n0 = seed.funcs[0].body.size();
        let mut lifted: Option<Program> = None;
        'outer: for k in 0..n0 {
            for s in 0..8u64 {
                let mut rng = Rng::new(1000 + s);
                if let Some(p) = mutate::mutate_at(seed, Op::AddFn, &mut rng, 0, k) {
                    if p.funcs.len() == 2 { lifted = Some(p); break 'outer; }
                }
            }
        }
        let Some(lp) = lifted else {
            println!("\n=== {name}: AddFn could not lift anywhere ({n0} nodes) ===");
            println!("  -> `contains_set` refuses every Set-bearing subtree, so a seed whose body is");
            println!("     mostly Set-rooted offers no legal lift site. Recorded, not asserted.");
            continue;
        };

        let s0 = lp.funcs[0].body.size();
        let s1 = lp.funcs[1].body.size();
        let mut hits = [0usize; 2];
        let mut drawn = 0usize;
        for seed_i in 0..4000u64 {
            let mut rng = Rng::new(seed_i);
            if let Some((out, _)) = mutate::mutate_program_n(&lp, &mut rng, 1) {
                if out.funcs.len() != 2 { continue; }
                drawn += 1;
                for i in 0..2 {
                    if format!("{:?}", out.funcs[i].body) != format!("{:?}", lp.funcs[i].body) {
                        hits[i] += 1;
                    }
                }
            }
        }
        let tot = (s0 + s1) as f64;
        println!("\n=== {name}: AddFn-LIFTED, funcs sizes [{s0}, {s1}] (seed body was {n0}) ===");
        println!("  draws applied: {drawn}");
        println!("  funcs[0] edited {:>5} = {:>6.2}%   (by size {:>6.2}%)",
                 hits[0], 100.0*hits[0] as f64/drawn.max(1) as f64, 100.0*s0 as f64/tot);
        println!("  funcs[1] edited {:>5} = {:>6.2}%   (by size {:>6.2}%)  <- the LIFTED body",
                 hits[1], 100.0*hits[1] as f64/drawn.max(1) as f64, 100.0*s1 as f64/tot);
        println!("  -> a lifted body CAN diverge on {:.2}% of draws", 100.0*hits[1] as f64/drawn.max(1) as f64);
    }
}
