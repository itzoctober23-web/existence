//! WHICH FUNCTION does the SEARCH actually mutate?
//!
//! `shape_reachability.rs` sweeps EVERY operator at EVERY position of EVERY function via
//! `mutate_at`, and answers REACHABILITY: can a shape be produced at all. The search does not do
//! that. It calls `mutate_program_n`, which picks one operator and then walks functions IN INDEX
//! ORDER, breaking at the first successful placement:
//!
//! ```ignore
//! for fi in 0..cur.funcs.len() {
//!     ... shuffled positions within fi ...
//!     if ok.is_some() { break; }      // funcs[1..] reached only if funcs[0] matched NOWHERE
//! }
//! ```
//!
//! So this test asks the DISTRIBUTION question instead: over many draws, how often does an edit
//! land outside `funcs[0]`? That is the same declared-vs-actually-drawn defect `mutate.rs` already
//! records and fixed for OPERATORS ("a race that broadly-applicable operators always win"), applied
//! to the FUNCTION dimension, where it was never fixed.
//!
//! It matters because `Op::AddFn`'s unpark condition (`mutate.rs:148`) is "once something can
//! diverge the lifted body from its origin". A lifted function that is never edited can never
//! diverge, so it would sit inert paying its `Call` and earning nothing.
//!
//! This test ASSERTS NOTHING about what the number should be. It prints it. A threshold invented
//! before the first measurement is a guess wearing a test's clothes.

use grammar::{mutate, reference};

#[test]
fn which_function_do_search_draws_land_in() {
    let multi: Vec<(&str, grammar::ast::Program)> = reference::all()
        .into_iter()
        .filter(|(_, p)| p.funcs.len() > 1)
        .collect();

    assert!(
        !multi.is_empty(),
        "no multi-function reference program: this test cannot measure what it exists to measure. \
         An empty fixture set is a broken probe, never a finding."
    );

    for (name, prog) in &multi {
        let nfun = prog.funcs.len();
        let sizes: Vec<usize> = (0..nfun).map(|i| prog.funcs[i].body.size()).collect();
        let total: usize = sizes.iter().sum();

        let mut hits = vec![0usize; nfun];
        let mut drawn = 0usize;
        let mut abandoned = 0usize;

        for seed in 0..4000u64 {
            let mut rng = mutate::Rng::new(seed);
            match mutate::mutate_program_n(prog, &mut rng, 1) {
                Some((out, _ops)) => {
                    drawn += 1;
                    // Which function's body changed? Compare structurally, per function.
                    if out.funcs.len() != nfun {
                        continue; // funcs.len() changed -- a different phenomenon, counted elsewhere
                    }
                    for i in 0..nfun {
                        if format!("{:?}", out.funcs[i].body) != format!("{:?}", prog.funcs[i].body) {
                            hits[i] += 1;
                        }
                    }
                }
                None => abandoned += 1,
            }
        }

        println!("\n=== {name}: {nfun} functions, node counts {sizes:?} (total {total}) ===");
        println!("  draws that applied: {drawn}, abandoned: {abandoned}");
        for i in 0..nfun {
            let share = sizes[i] as f64 / total as f64;
            let got = hits[i] as f64 / drawn.max(1) as f64;
            println!(
                "  funcs[{i}]: hit {:>5} times = {:>6.2}%   (uniform-by-size would be {:>6.2}%)",
                hits[i], 100.0 * got, 100.0 * share
            );
        }
        println!(
            "  -> funcs[0] share {:.2}% vs {:.2}% expected if position were chosen uniformly over ALL nodes",
            100.0 * hits[0] as f64 / drawn.max(1) as f64,
            100.0 * sizes[0] as f64 / total as f64
        );
    }
}
