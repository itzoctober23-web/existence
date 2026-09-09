//! Is the A/A distribution DEGENERATE? Look at the pentanomial directly instead of inferring it.
//!
//! WHY. `ci95_curve.rs` returned 0.250 at 6 pairs and 0.125 at 12 -- an exact halving for a doubling,
//! i.e. `1/n`. A confidence interval scales `1/sqrt(n)`, which predicts 0.177. Something is wrong with
//! the DISTRIBUTION, not the arithmetic, and `gate_power_RESULT.md` now carries a caveat saying so.
//!
//! The hypothesis: two IDENTICAL deterministic programs playing a colour-swapped pair split it
//! exactly, so nearly every pair lands in the middle bucket (index 2) and the variance comes from a
//! small, roughly fixed number of pairs. If the count of non-middle pairs does not grow with `n`, the
//! sample variance falls as `1/n` and ci95 as `1/n` -- exactly the shape observed.
//!
//! This tests that DIRECTLY rather than by curve-fitting: print the raw pentanomial for an A/A and
//! for a genuinely DIFFERENT pair at the same size. If the A/A piles into bucket 2 while the A/B
//! spreads, the A/A is degenerate and cannot model the near-parity variance of two distinct programs
//! -- which is the claim the caveat needs settled.
use grammar::reference;
use nnue::Net;
use pipeline::gate;

fn row(label: &str, sc: &gate::Score) {
    let n: u32 = sc.pent.iter().sum();
    let mid = sc.pent[2];
    println!("  {label:<34} pent {:?}  n={n}  middle={mid} ({:.1}%)  rate {:.3} +/- {:.3}",
             sc.pent, 100.0 * mid as f64 / n.max(1) as f64, sc.pent_rate(), sc.ci95());
}

fn main() {
    let pairs: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(24);
    let depth: i64 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(3);
    let net = Net::random(32, 20260907);
    let seed_p = reference::bare_alpha_beta();
    let other = reference::capture_extension();
    let t = vec![depth, 32_000, interp::uct_exploration()];

    println!("pentanomial shape — {pairs} pairs, depth {depth}, budget 16");
    println!("  buckets: 0=LL 1=LD/DL 2=LW/DD/WL 3=DW/WD 4=WW\n");

    let aa = gate::match_progs(&seed_p, &seed_p, &net, t.clone(), 16, pairs, 0xA1A0, 4, u64::MAX);
    row("A/A  seed vs ITSELF", &aa);
    let ab = gate::match_progs(&seed_p, &other, &net, t.clone(), 16, pairs, 0xA1A0, 4, u64::MAX);
    row("A/B  seed vs capture_extension", &ab);

    let aan = aa.pent.iter().sum::<u32>() - aa.pent[2];
    let abn = ab.pent.iter().sum::<u32>() - ab.pent[2];
    println!("\n  non-middle pairs: A/A {aan}, A/B {abn}");
    println!("  If A/A is overwhelmingly middle and A/B is not, the A/A distribution is DEGENERATE");
    println!("  and cannot stand in for the near-parity variance of two DIFFERENT programs.");
}
