//! IS THE ORIGIN CONTROL'S NODE BUDGET A CONSTANT, OR DOES IT MOVE WITH MACHINE LOAD?
//!
//! The periodic control in `main.rs` does NOT play at a fixed node budget. It calls
//! `arch::equal_time_caps(&champion, &origin, budget_ns, ...)` on every invocation, and that
//! function derives its caps from `ns_per_node()` -- a WALL-CLOCK timing probe run at that
//! moment, on this box, under whatever else is running.
//!
//! WHY THAT MATTERS RIGHT NOW. The control reported the champion RESOLVED WORSE across gens
//! 100-400: 0.873 -> 0.871 -> 0.847 -> 0.819, a decline of -0.054 +/- 0.033. Over the same
//! window `netmatch` says gen400 BEATS gen200 head-to-head, 0.539 +/- 0.027. Both intervals
//! exclude their nulls, so at most one of them is measuring what I read it as.
//!
//! The mechanism this file tests: under load, `ns_per_node` rises for BOTH nets, so both caps
//! FALL, and the match is played at a lower node count than the previous reading used. Equal-time
//! keeps the two sides fair to each other -- that part is sound -- but it does NOT keep successive
//! readings comparable, because the operating point moves. And a champion's edge over a weak
//! opponent generally SHRINKS as both are starved of nodes: both play worse, more games draw, the
//! rate drifts toward 0.5. That is the observed direction.
//!
//! It is not hypothetical that the load changed: datagen went from 4 lanes to 8 during this run
//! (commit 5255de0), roughly doubling the competing work on the same cores.
//!
//! WHAT THIS PROBE DOES, and what it deliberately does NOT do. It calls `equal_time_caps` on ONE
//! FIXED PAIR of nets, repeatedly, back to back. The nets never change, so ANY spread in the caps
//! is instrument noise by construction -- there is no strength signal available for it to be.
//! It reports the spread as a percentage of the mean.
//!
//! FALSIFIABLE, stated before running (this project's rule is that a number which cannot vary is
//! a guard, not evidence):
//!   * If the caps are stable to within a percent or two, this hypothesis is REFUTED and the
//!     control's decline has to be explained some other way. I will say so and drop it.
//!   * If the caps swing by tens of percent, then successive control readings were taken at
//!     different operating points and the -0.054 "decline" is not yet attributable to the net.
//!
//! Note what this probe canNOT settle on its own: a moving budget explains how a spurious trend
//! COULD arise, not that it did. Establishing that needs the ladder rungs replayed against the
//! origin at ONE FIXED cap -- which is `ctrl_fixed.rs`, and which is only worth its runtime if
//! the spread here is large.

use nnue::Net;
use pipeline::arch;

fn main() {
    let mut a = std::env::args().skip(1);
    let net_path = a.next().unwrap_or_else(|| {
        eprintln!("usage: cap_stability <net> [reps] [budget_ns] [probe_depth]");
        std::process::exit(2);
    });
    let reps: usize = a.next().and_then(|s| s.parse().ok()).unwrap_or(12);
    // The default matches main.rs's own default budget so the caps printed here are the caps the
    // control actually plays at, not a rescaled lookalike.
    let budget_ns: f64 = a.next().and_then(|s| s.parse().ok()).unwrap_or(4_000_000.0);
    let probe_depth: u32 = a.next().and_then(|s| s.parse().ok()).unwrap_or(3);

    let champ = Net::load(&net_path).unwrap_or_else(|e| {
        eprintln!("could not load {net_path}: {e}");
        std::process::exit(2);
    });
    // The origin is rebuilt exactly as control.rs:77 does, so this is the same opponent the
    // control uses rather than a lookalike.
    let origin = Net::random(champ.n_hidden, 20260907);

    println!("cap_stability: {net_path} (w{}) vs origin, {reps} reps, budget {budget_ns:.0} ns, probe depth {probe_depth}",
             champ.n_hidden);
    println!("  the nets NEVER change across reps, so all spread below is instrument noise\n");

    let mut cas: Vec<f64> = Vec::with_capacity(reps);
    let mut cbs: Vec<f64> = Vec::with_capacity(reps);
    for i in 0..reps {
        let (ca, cb) = arch::equal_time_caps(&champ, &origin, budget_ns, probe_depth);
        cas.push(ca as f64);
        cbs.push(cb as f64);
        println!("  rep {:>2}: champion {:>8} nodes   origin {:>8} nodes   ratio {:.3}",
                 i + 1, ca, cb, ca as f64 / cb as f64);
    }

    let stat = |v: &[f64]| -> (f64, f64, f64, f64) {
        let n = v.len() as f64;
        let mean = v.iter().sum::<f64>() / n;
        let sd = (v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n).sqrt();
        let mn = v.iter().cloned().fold(f64::INFINITY, f64::min);
        let mx = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        (mean, sd, mn, mx)
    };

    let (ma, sa, mna, mxa) = stat(&cas);
    let (mb, sb, mnb, mxb) = stat(&cbs);
    println!("\n  champion caps: mean {ma:.0}  sd {sa:.0} ({:.1}%)  min {mna:.0}  max {mxa:.0}  spread {:.1}%",
             100.0 * sa / ma, 100.0 * (mxa - mna) / ma);
    println!("  origin caps:   mean {mb:.0}  sd {sb:.0} ({:.1}%)  min {mnb:.0}  max {mxb:.0}  spread {:.1}%",
             100.0 * sb / mb, 100.0 * (mxb - mnb) / mb);

    // The verdict is stated against the threshold declared in the header, so it cannot be
    // reinterpreted after the fact to suit whichever number came out.
    let worst = (100.0 * (mxa - mna) / ma).max(100.0 * (mxb - mnb) / mb);
    println!();
    if worst < 5.0 {
        println!("  VERDICT: caps are STABLE ({worst:.1}% spread). The load hypothesis is REFUTED --");
        println!("  successive control readings were taken at effectively the same operating point,");
        println!("  so the -0.054 decline needs a different explanation. Do NOT run ctrl_fixed on");
        println!("  the strength of this.");
    } else {
        println!("  VERDICT: caps MOVE by {worst:.1}% with the nets held fixed. Successive control");
        println!("  readings are therefore NOT taken at a common operating point, and the -0.054");
        println!("  'decline' is confounded with whatever else was running. This does NOT show the");
        println!("  decline is spurious -- it shows the instrument cannot currently distinguish.");
        println!("  Next step is ctrl_fixed: replay the ladder rungs vs origin at ONE FIXED cap.");
    }
}
