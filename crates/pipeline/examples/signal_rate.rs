//! What fraction of REAL gate candidates give the gate any signal at all — and for those that do,
//! how fast does the interval actually shrink?
//!
//! WHY THE A/A WAS THE WRONG INSTRUMENT. `ci95_curve.rs` matched a program against ITSELF, and every
//! pair tied, so `gate.rs:65-74` returned its zero-variance placeholder `1.5/n` at every size. It
//! measured the fallback formula, not the gate. That is settled: 1.5/6 = 0.250 and 1.5/12 = 0.125
//! reproduced both observed rows exactly.
//!
//! WHY THIS PAIR INSTEAD. The gate's real job is champion-versus-MUTANT, so that is what this matches:
//! `bare_alpha_beta` against `mutate_program` children of itself, which is precisely the population
//! `evolve.rs` feeds it.
//!
//! THE PRE-REGISTERED CROSS-CHECK. Counting the 203 logged gate decisions, 95 (46.8%) carry
//! `ci95 == 0.250` exactly — the zero-variance placeholder — and every one of those has `pent_rate`
//! exactly 0.500. If that reading is right, roughly HALF the mutants here should also produce zero
//! variance. A wildly different fraction means the log-based split is wrong and the conclusions drawn
//! from it need withdrawing.
//!
//! READING IT. `no-signal` counts matches where the gate saw nothing (placeholder fired). `measured
//! ci95` averages ONLY the matches that produced real variance — mixing the two is the error this
//! probe exists to correct, and the one already corrected in `gate_power_RESULT.md`.
use grammar::{mutate, reference};
use nnue::Net;
use pipeline::gate;

fn main() {
    let mutants: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(12);
    let depth: i64 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(3);
    let net = Net::random(32, 20260907);
    let parent = reference::bare_alpha_beta();
    let t = vec![depth, 32_000, interp::uct_exploration()];

    // Build the mutant pool once so every pair count faces the SAME children -- otherwise a
    // difference between sizes could just be a difference between candidate populations.
    let mut pool = Vec::new();
    let mut rng = mutate::Rng::new(0x5165_2026);
    while pool.len() < mutants {
        if let Some(c) = mutate::mutate_program(&parent, &mut rng) {
            if c != parent { pool.push(c); }
        }
    }
    println!("signal rate — parent vs {} distinct mutants, depth {depth}, budget 16", pool.len());
    println!("  placeholder = ci95 exactly 1.5/pairs (gate.rs:65-74, zero observed variance)\n");
    println!("  {:>6} {:>10} {:>12} {:>14} {:>10}", "pairs", "no-signal", "share", "measured ci95", "bar");

    for pairs in [6usize, 12, 24, 48] {
        let ph = 1.5 / pairs as f64;
        // Split the no-signal matches by CAUSE. An all-middle pentanomial arises two ways and they
        // need opposite fixes: MIRRORED means the mutant plays exactly like the parent (the operator
        // produced an inert candidate -- more pairs can never help); ALL-DRAWN means the games are
        // not decisive at these settings (the match setup is the problem, not the sample size).
        let (mut dead, mut sum, mut live) = (0usize, 0.0f64, 0usize);
        let (mut mirrored, mut alldrawn, mut mixed) = (0usize, 0usize, 0usize);
        for (i, c) in pool.iter().enumerate() {
            let sc = gate::match_progs(&parent, c, &net, t.clone(), 16, pairs,
                                       0x51_0000 ^ (pairs as u64) << 8 ^ i as u64, 4, u64::MAX);
            let ci = sc.ci95();
            if (ci - ph).abs() < 1e-9 {
                dead += 1;
                if sc.draws == 0 && sc.wins > 0 { mirrored += 1; }
                else if sc.wins == 0 && sc.losses == 0 { alldrawn += 1; }
                else { mixed += 1; }
            } else { sum += ci; live += 1; }
        }
        let m = if live > 0 { sum / live as f64 } else { f64::NAN };
        println!("  {pairs:>6} {dead:>10} {:>11.1}% {m:>14.4} {:>10.3}   [mirrored {mirrored}, all-drawn {alldrawn}, mixed {mixed}]",
                 100.0 * dead as f64 / pool.len() as f64, 0.5 + m);
    }
    println!("\n  CROSS-CHECK: the gate logs put the no-signal share at 46.8% (95 of 203) at 6 pairs.");
    println!("  A similar share here corroborates that split; a very different one refutes it.");
    println!("  'bar' is the strict rule's threshold 0.5+ci95 among matches that MEASURED something;");
    println!("  compare against 0.542, the highest rate ever seen in a real gate decision.");
}
