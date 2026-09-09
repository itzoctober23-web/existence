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
    let verdict = if r - c > 0.5 {
        "A is stronger, interval clear of 0.5"
    } else if r + c < 0.5 {
        "B is stronger, interval clear of 0.5"
    } else if c < 0.03 {
        "INDISTINGUISHABLE, and precisely so -- a narrow interval containing 0.5"
    } else {
        "UNRESOLVED -- the interval is wide AND contains 0.5; this needs more pairs, not a verdict"
    };
    println!("  => {verdict}");
    println!("\n  A narrow interval around 0.5 is a RESULT (no difference); a wide one is IGNORANCE.");
    println!("  Two random movers score 0.500 with a narrow interval, so tightness alone proves");
    println!("  nothing about whether these nets were ever separable.");
}
