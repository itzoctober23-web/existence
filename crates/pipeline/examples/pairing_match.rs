//! Does pairing the OPENINGS cut the variance of a champion-vs-base increment? Equal games both ways.
//!
//! The batch gate and the anchor gate both estimate "is A better than B" by scoring each against a
//! fixed opponent and subtracting. They give the two sides DIFFERENT opening sets, because
//! `match_nets` derives every opening from `Rng(seed | 1)` (gate.rs:100) and the two call sites
//! differ by `^ g`. A comment at the anchor gate claimed the opposite -- "same seed family ... so
//! they meet the anchor on the same openings" -- which is false: sharing a prefix is not sharing a
//! seed.
//!
//! Observed cost of that, in a live run: after the g10 KEEP the base BECAME that champion, and the
//! SAME NET scored 0.862 +/- 0.020 as champion and 0.833 +/- 0.022 as base. A 0.029 spread on one
//! net, from opening luck, while the increments being judged are +0.033 and +0.021.
//!
//! examples/paired.rs already argues this principle for held-out POSITIONS -- "comparing two
//! correlations throws the pairing away ... the difference has far less variance" -- and the game
//! gate already pairs WITHIN a pair ("play the SAME opening from both sides"). Neither is applied
//! BETWEEN the two sides of an increment. This measures what that costs.
//!
//! EQUAL GAMES BY CONSTRUCTION: both schemes play two matches of `pairs` pairs per replicate. The
//! only difference is whether the second match reuses the first's seed. So a variance reduction
//! here is free -- it is not bought with extra games, which is the trap in every speed and power
//! comparison this project has had to redo.
use nnue::Net;
use pipeline::gate;

fn main() {
    let mut a = std::env::args().skip(1);
    let pairs: usize = a.next().and_then(|s| s.parse().ok()).unwrap_or(112);
    let reps: usize = a.next().and_then(|s| s.parse().ok()).unwrap_or(10);
    let depth: u32 = a.next().and_then(|s| s.parse().ok()).unwrap_or(2);

    // Two SIMILAR nets: the regime that matters. Comparing a champion against a random net would
    // show a huge gap that any scheme resolves, and would say nothing about resolving +0.02.
    let na = Net::load("champion_long.net").expect("champion_long.net");
    let nb = Net::load("bg_5.net").expect("bg_5.net");
    assert_eq!(na.n_hidden, nb.n_hidden, "nets must share a width to face the same origin");
    let origin = Net::random(na.n_hidden, 20260907);

    println!("pairing_match: {pairs} pairs/match, {reps} replicates, depth {depth}");
    println!("  A = champion_long.net   B = bg_5.net   opponent = frozen origin");
    println!("  both schemes play 2 matches per replicate -> EQUAL GAMES\n");

    let (mut du, mut dp) = (Vec::new(), Vec::new());
    for r in 0..reps {
        let s = 0xC0FFEE ^ (r as u64).wrapping_mul(0x9E3779B97F4A7C15);
        // UNPAIRED: the two sides walk different openings, as the shipped gates do.
        let ua = gate::match_nets(&na, &origin, depth, pairs, s);
        let ub = gate::match_nets(&nb, &origin, depth, pairs, s ^ 0xA9C0);
        du.push(ua.pent_rate() - ub.pent_rate());
        // PAIRED: identical seed, so identical openings; opening difficulty is common and cancels.
        let pa = gate::match_nets(&na, &origin, depth, pairs, s);
        let pb = gate::match_nets(&nb, &origin, depth, pairs, s);
        dp.push(pa.pent_rate() - pb.pent_rate());
        // Print every replicate: a run killed partway must still yield its answer. The previous
        // instrument in this project lost 75 candidates of work by printing only after the loop.
        println!("  rep {:2}  unpaired {:+.4}   paired {:+.4}", r + 1, du[r], dp[r]);
    }

    let stat = |v: &Vec<f64>| {
        let n = v.len() as f64;
        let m = v.iter().sum::<f64>() / n;
        let sd = (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (n - 1.0)).sqrt();
        (m, sd)
    };
    let (mu, su) = stat(&du);
    let (mp, sp) = stat(&dp);
    println!("\n  unpaired  mean {mu:+.4}  sd {su:.4}");
    println!("  paired    mean {mp:+.4}  sd {sp:.4}");
    if sp > 0.0 {
        println!("  variance ratio unpaired/paired = {:.2}x", (su / sp).powi(2));
        println!("  -> pairing needs a ratio > 1 to be worth taking; it costs the base cache,");
        println!("     so the batch gate must ALSO beat the cached scheme at equal total games.");
    }
    println!("\n  Both means estimate the SAME quantity. If they differ by much more than their");
    println!("  own spread, the harness is biased, not merely noisy -- check that before reading");
    println!("  the ratio, because two wrong hypotheses in a row means the harness is wrong.");
}
