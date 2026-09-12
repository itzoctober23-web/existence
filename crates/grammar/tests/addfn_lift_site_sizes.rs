//! Closes the last limit I stated on the AddFn chain.
//!
//! `lifted_function_edit_rate.rs` measured ONE lift (depth-one seed, funcs [9,2]) and found the
//! lifted body edited 0.00% of draws. Its stated limit: "a larger lift is untested. If AddFn could
//! lift a big subtree, funcs[1] would be large and the reference-program rates suggest it would be
//! edited often. Whether such a lift site exists under `contains_set` is not measured here."
//!
//! This enumerates EVERY lift site on EVERY 1-function reference program and reports the SIZE
//! DISTRIBUTION of the resulting `funcs[1]`. If every legal lift is tiny, the park is robust by
//! construction rather than by the accident of which seed was tried.
//!
//! ASSERTS NOTHING about the sizes. Prints them.

use grammar::mutate::{self, Op, Rng};
use grammar::{reference, Program};

#[test]
fn how_big_can_an_addfn_lift_be() {
    let ones: Vec<(&str, Program)> = reference::all()
        .into_iter()
        .filter(|(_, p)| p.funcs.len() == 1)
        .collect();
    assert!(!ones.is_empty(), "no 1-function reference program -- broken fixture");

    println!("\n{:<34} {:>5} {:>6} {:>7} {:>9} {:>9}",
             "program", "nodes", "sites", "lifts", "max f1", "max f1 %");
    let mut grand_max = 0usize;
    let mut grand_max_pct = 0.0f64;

    for (name, p) in &ones {
        let n = p.funcs[0].body.size();
        let mut lifts = 0usize;
        let mut sizes: Vec<usize> = Vec::new();
        for k in 0..n {
            // several rngs per site: AddFn's internal choices can vary
            for s in 0..6u64 {
                let mut rng = Rng::new(7000 + s);
                if let Some(out) = mutate::mutate_at(p, Op::AddFn, &mut rng, 0, k) {
                    if out.funcs.len() == 2 {
                        lifts += 1;
                        sizes.push(out.funcs[1].body.size());
                        break;
                    }
                }
            }
        }
        let mx = sizes.iter().copied().max().unwrap_or(0);
        let pct = if n > 0 { 100.0 * mx as f64 / n as f64 } else { 0.0 };
        if mx > grand_max { grand_max = mx; }
        if pct > grand_max_pct { grand_max_pct = pct; }
        println!("{:<34} {:>5} {:>6} {:>7} {:>9} {:>8.1}%", name, n, n, lifts, mx, pct);
    }

    println!("\n  LARGEST lifted body across every site of every 1-function program: {grand_max} nodes");
    println!("  largest as a share of its parent body: {grand_max_pct:.1}%");
    println!("  (reference programs whose funcs[1] IS edited had 58-148 nodes)");
}
