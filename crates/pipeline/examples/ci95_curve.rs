//! How much does the game gate actually resolve? An A/A measurement of ci95 vs pair count.
//!
//! WHY THIS EXISTS. `gate_power_RESULT.md` establishes, from 202 logged decisions, that the gate has
//! never once accepted a candidate and that its acceptance bar (`0.5 + ci95`) has never been inside
//! the range of rates the instrument produces (max ever seen: 0.542). The cause is sample size:
//! `evolve.rs:1297` defaults `gate_pairs` to **6**. The fix is to raise it -- but SIZING the fix needs
//! the ci95-vs-pairs curve, and that curve is not measured. The two anchors on hand disagree by 1.6x
//! (6 pairs -> 0.177 over 202 decisions; 96 pairs -> 0.027 over exactly ONE observation), which is
//! precisely the situation where extrapolating an exponent produces a confident wrong number.
//!
//! WHY A/A. The gate's decisions cluster near 0.5, and a confidence interval's width depends on the
//! underlying rate. Matching a program against ITSELF pins the true rate at 0.500 by symmetry, so
//! every bit of observed spread is instrument noise -- which is exactly the quantity that sets the
//! acceptance bar. It also gives a free validity check: an A/A that does not centre on 0.500 means
//! the harness is biased and NOTHING measured with it can be trusted. This project has been burned by
//! unvalidated harnesses often enough that the control is worth its cost.
//!
//! EQUAL TOTAL WORK PER SIZE. Each size runs enough replicates to spend ~the same number of pairs, so
//! the mean ci95 at small sizes is estimated from many samples rather than one. A single 6-pair ci95
//! is itself extremely noisy -- averaging replicates is the whole point, since the 6-pair figure is
//! suspected of being inflated by sample-sd estimation on six numbers against a 1.96 multiplier.
//!
//! READING IT. `mean_ci95` is the instrument's half-width at that size; `bar` is the strict rule's
//! threshold `0.5 + ci95`. The smallest size whose `bar` drops below 0.542 (the largest rate ever
//! observed in a real gate decision) is the smallest gate that could ever promote anything.
use grammar::reference;
use nnue::Net;
use pipeline::gate;

fn main() {
    let depth: i64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(3);
    let net = Net::random(32, 20260907);
    let p = reference::bare_alpha_beta();
    println!("ci95 vs pairs — A/A: bare_alpha_beta against ITSELF, depth {depth}, budget 16");
    println!("  true rate is 0.500 by symmetry, so all spread is instrument noise");
    println!("  a mean_rate far from 0.500 invalidates the harness and everything measured with it\n");
    println!("  {:>6} {:>5} {:>10} {:>10} {:>7}  {}", "pairs", "reps", "mean_rate", "mean_ci95", "bar", "vs max-ever 0.542");

    for (pairs, reps) in [(6usize, 8usize), (12, 4), (24, 2), (48, 1), (96, 1)] {
        let (mut rs, mut cs) = (0.0f64, 0.0f64);
        for r in 0..reps {
            let sc = gate::match_progs(&p, &p, &net, vec![depth, 32_000, interp::uct_exploration()],
                                       16, pairs, 0xA1A0_0000 ^ (pairs as u64) << 8 ^ r as u64, 4,
                                       u64::MAX);
            rs += sc.pent_rate();
            cs += sc.ci95();
        }
        let (rate, ci) = (rs / reps as f64, cs / reps as f64);
        println!("  {pairs:>6} {reps:>5} {rate:>10.3} {ci:>10.3} {:>7.3}  {}", 0.5 + ci,
                 if 0.5 + ci < 0.542 { "REACHABLE" } else { "out of reach" });
    }

    println!("\n  The smallest REACHABLE row is the smallest gate that could ever promote a candidate.");
    println!("  If no row is reachable, the strict rule needs more pairs than tested here.");
}
