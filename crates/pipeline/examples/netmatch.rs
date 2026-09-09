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
    let pairs: usize = a.next().and_then(|s| s.parse().ok()).unwrap_or(448);
    // DEFAULT DEPTH 4, NOT 2, and the default matters because I got this wrong all day.
    // This project judges strength at depth 4: gate_depth_cap defaults to 4 and the gate sizes its
    // budget as "7061 nodes = 100% coverage of a full depth-4 search". Depth 2 is what the DATAGEN
    // uses, which is a different thing, and defaulting to it meant every comparison I ran -- the
    // blend reversal, the b2_5 gain, the capacity null, draws, epochs -- answered "which net is
    // better at depth 2" while I read them as "which net is stronger".
    // A careless invocation should measure the thing that decides, so the careless case is now d4.
    let depth: u32 = a.next().and_then(|s| s.parse().ok()).unwrap_or(4);
    let seed: u64 = a.next().and_then(|s| s.parse().ok()).unwrap_or(20260907);

    // "random:<width>:<seed>" constructs a net instead of loading one, so the ORIGIN can be an
    // opponent here. Without it the origin is reachable only from inside `learn`, and the question
    // "does an origin-increment recover what a direct match says" cannot be asked at a matched
    // depth -- which is the only way to ask it, since both previous disagreements were confounded
    // by the control playing depth 4 while the direct match played depth 2.
    let load = |spec: &str| -> Net {
        if let Some(rest) = spec.strip_prefix("random:") {
            let mut it = rest.split(':');
            let w: usize = it.next().and_then(|x| x.parse().ok()).expect("random:<width>:<seed>");
            let sd: u64 = it.next().and_then(|x| x.parse().ok()).expect("random:<width>:<seed>");
            Net::random(w, sd)
        } else {
            Net::load(spec).unwrap_or_else(|e| panic!("{spec}: {e}"))
        }
    };
    let na = load(&pa);
    let nb = load(&pb);
    // A width mismatch is not an error to paper over -- the nets face each other directly, so
    // differing widths is a legitimate comparison, but it must be VISIBLE in the output or a
    // capacity difference gets read as a training difference.
    println!("netmatch: {pa} (w{}) vs {pb} (w{})", na.n_hidden, nb.n_hidden);

    // ARM SIZES, PRINTED ALWAYS. Three separate results today were confounded by arms that did
    // unequal amounts of training, and every one was caught (or missed) here, at the comparison:
    //   batch_ab       11 generations vs 5
    //   depth_parity   96 vs ~1
    //   the epochs sweep  28 vs 26  -- which is what made epochs look like a winner for two days
    // The last one survived because I used ep_*.net without ever reading its settings line. A
    // number that does not carry its arm size can be read as a setting effect when it is a training
    // effect, so the size now travels with every result this tool prints.
    let gens = |spec: &str| -> Option<usize> {
        let log = spec.strip_suffix(".net")?.to_string() + ".log";
        let txt = std::fs::read_to_string(log).ok()?;
        Some(txt.lines().filter(|l| l.starts_with("gen ")).count())
    };
    if let (Some(ga), Some(gb)) = (gens(&pa), gens(&pb)) {
        print!("  arms: {ga} vs {gb} generations");
        if ga != gb {
            // ~0.0114 per generation is the measured per-generation edge in this tree.
            let bias = (ga as f64 - gb as f64).abs() * 0.0114;
            println!("  <-- UNEQUAL by {}, worth ~{bias:.3} of advantage on its own",
                     (ga as i64 - gb as i64).abs());
            println!("  Any effect smaller than that is training amount, not the setting under test.");
        } else {
            println!(" (matched)");
        }
    }
    let std_note = if depth == 4 { " (project standard for strength)" }
                   else if depth == 2 { " (DATAGEN depth -- NOT the strength standard, which is 4)" }
                   else { "" };
    println!("  {pairs} pairs, depth {depth}{std_note}, seed {seed}");

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

    // BETWEEN-SEED POWER. Everything above is WITHIN-RUN precision: it says how well this match
    // pinned down the difference between THESE TWO NETS. It says nothing about whether the same
    // experiment on a different TRAINING SEED would land in the same place, and that is the error
    // this project keeps making -- an interval clear of 0.5 gets read as a settled result.
    //
    // Withdrawn on exactly this in one day: the "+0.025 datagen depth lever"; blend 1.00's
    // "advantage is depth-2 only, z = 4.4" (it reverses on the second seed); and a blend 0.85
    // reading of mine that looked clear of 0.5 at +0.040.
    //
    // THE CONSTANT IS MEASURED, not assumed, and measured on the RIGHT quantity: the movement of a
    // PAIRED difference between training seeds, from two independent cross-seed comparisons of the
    // same net pair (0.75-vs-1.00 at depth 4: 0.511 -> 0.456, movement 0.055; at depth 2: 0.450 ->
    // 0.502, movement 0.052). E[range] = 1.128*sd at n = 2, so sd ~ 0.047.
    //
    // It must NOT be compared against frozen-origin increments -- that is a different instrument,
    // which instrument_saturation_RESULT.md shows disagreeing by up to 5x and reversing sign twice.
    //
    // Two estimates make this crude; it should be re-derived as more paired cross-seed comparisons
    // accumulate, which now happens for free whenever an arm is replicated.
    const SEED_SD: f64 = 0.047;
    let effect = (r - 0.5).abs();
    if effect > 0.0 {
        let seeds = (2.8 * SEED_SD / effect).powi(2);
        println!("\n  BETWEEN-SEED POWER (one training seed measured here):");
        println!("    effect {effect:.3} against a between-seed sd of {SEED_SD:.3} -> ~{seeds:.0} seeds \
for ~80% power");
        if seeds > 2.0 {
            println!("    ** ONE SEED CANNOT SETTLE THIS ** -- the interval above is within-run only.");
        } else {
            println!("    Effect is large relative to seed noise; one seed is defensible.");
        }
    }
}
