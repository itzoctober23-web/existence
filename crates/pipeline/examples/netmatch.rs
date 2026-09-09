//! Play two SAVED nets against each other and report the pentanomial rate. Infrastructure that was
//! missing: every comparison so far has gone through a fixed anchor, which cannot answer "is A
//! actually stronger than B" for two trained nets without two matches and a two-sample test.
//!
//! THE QUESTION IT EXISTS FOR. The from-scratch blend arms reach 0.847 +/- 0.022 against the frozen
//! origin after TWENTY generations. champion_long, carrying far more training, sits at 0.861. Those
//! two numbers are one interval apart, but both are measured against a RANDOM opponent, and beating
//! a random net is a saturating measurement -- two nets can both crush it while differing a lot, or
//! barely differ while looking apart. Playing them directly is the only way to tell, and it also
//! avoids the compression the champion gate shows against a NEAR-EQUAL opponent, since these two
//! are not near-equal by construction.
//!
//! Reads rates as pentanomial PAIRS, the same statistic the gates use, so the numbers are
//! comparable to everything else in the tree.
use nnue::Net;
use pipeline::gate;

fn main() {
    let mut a = std::env::args().skip(1);
    let pa = a.next().unwrap_or_else(|| "champion_long.net".into());
    let pb = a.next().unwrap_or_else(|| "bn_075.net".into());
    let pairs: usize = a.next().and_then(|s| s.parse().ok()).unwrap_or(224);
    let depth: u32 = a.next().and_then(|s| s.parse().ok()).unwrap_or(2);
    let seed: u64 = a.next().and_then(|s| s.parse().ok()).unwrap_or(20260907);

    let na = Net::load(&pa).unwrap_or_else(|e| panic!("{pa}: {e}"));
    let nb = Net::load(&pb).unwrap_or_else(|e| panic!("{pb}: {e}"));
    // A width mismatch is not an error to paper over -- the nets face each other directly, so
    // differing widths is a legitimate comparison, but it must be VISIBLE in the output or a
    // capacity difference gets read as a training difference.
    println!("netmatch: {pa} (w{}) vs {pb} (w{})", na.n_hidden, nb.n_hidden);
    println!("  {pairs} pairs, depth {depth}, seed {seed}");

    let s = gate::match_nets(&na, &nb, depth, pairs, seed);
    let r = s.pent_rate();
    let c = s.ci95();
    println!("  {pa} scores {r:.3} +/- {c:.3}  (interval [{:.3}, {:.3}])", r - c, r + c);
    // MARGINAL BAND. "Clear of 0.5" is true at a lower bound of 0.5001 and it is not a result.
    // wd_r0 vs wd_r2 read 0.522 +/- 0.022 -- lower bound 0.501, a margin of 0.001 -- and went into
    // the top of STATE.md as "w16 is stronger". At 960 pairs it is a precise NULL. A verdict whose
    // margin is small next to its own interval is one more sample from being nothing, so it gets
    // named rather than reported as a win.
    let margin = (r - c - 0.5).max(0.5 - (r + c)); // >0 when the interval clears 0.5
    let marginal = margin > 0.0 && margin < 0.5 * c;
    let verdict = if marginal && r > 0.5 {
        "A leads, but MARGINALLY -- the margin is small next to the interval; needs more pairs"
    } else if marginal {
        "B leads, but MARGINALLY -- the margin is small next to the interval; needs more pairs"
    } else if r - c > 0.5 {
        "A is stronger, interval clear of 0.5"
    } else if r + c < 0.5 {
        "B is stronger, interval clear of 0.5"
    } else if c < 0.015 {
        "INDISTINGUISHABLE, and precisely so -- a narrow interval containing 0.5"
    } else {
        "UNRESOLVED -- contains 0.5 but is NOT tight enough to call a null; more pairs, not a verdict"
    };
    println!("  => {verdict}");
    println!("\n  A narrow interval around 0.5 is a RESULT (no difference); a wide one is IGNORANCE.");
    println!("  Two random movers score 0.500 with a narrow interval, so tightness alone proves");
    println!("  nothing about whether these nets were ever separable.");
    // THE NULL THRESHOLD IS 0.015, NOT 0.03, AND IT COST A WRONG CLAIM TO LEARN THAT.
    // At 224 pairs ci95 is ~0.030, so the old `c < 0.03` test stamped "precisely indistinguishable"
    // on the LEAST precise reading this tool can produce. bh_100 vs champion_long read 0.529 +/-
    // 0.030 at 224 pairs and was reported as a tie; at 896 pairs on a fresh seed it is 0.458 +/-
    // 0.016, champion_long stronger with the interval clear of 0.5. Same direction all along, but
    // the "tie" was underpowered, not a null. 0.015 needs ~896 pairs and excludes the 0.02-0.05
    // effects this project actually cares about.
    if c >= 0.015 && (r - c) <= 0.5 && (r + c) >= 0.5 {
        let need = ((1.96 * 0.2362 / 0.015_f64).powi(2)).ceil() as u64;
        println!("\n  NOT a null: at {pairs} pairs this cannot exclude a 0.02-0.05 effect.");
        println!("  Re-run at ~{need} pairs before calling it a tie.");
    }
}
