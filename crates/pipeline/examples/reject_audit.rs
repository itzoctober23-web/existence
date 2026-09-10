//! DOES THE DEPTH-1 GATE'S "NO" MEAN ANYTHING AT THE DEPTH STRENGTH IS JUDGED AT?
//!
//! The loop gates candidates at the datagen depth (1 by default) while this project judges strength
//! at depth 4. Measured on two rungs, same files and seed with only the depth changing: the later
//! net wins by +0.043 at depth 1 and +0.139 at depth 4. So the gate is deciding on roughly a THIRD
//! of the signal that exists in the game that counts.
//!
//! The natural worry is that it therefore throws away real improvements. That is an INFERENCE. This
//! is the discriminator: `--save-rejects` banks every Nth REJECTED candidate together with the
//! champion it lost to at that moment, and this replays those exact pairs at depth 4.
//!
//! ## The null is NOT 0.5, and getting that wrong would invert the reading
//!
//! Rejects are a SELECTED sample -- they are precisely the candidates the gate scored below its
//! accept threshold. So:
//!
//! * pooled **well below 0.5** -> the gate's "no" TRANSFERS. The candidates it rejected really are
//!   weaker at depth 4. The cheap gate is vindicated and there is nothing to fix.
//! * pooled **at 0.5** -> the gate's decision carries NO information about the judged game. It is
//!   rejecting candidates that are, on average, exactly as good as the champion at depth 4 -- i.e.
//!   the selection is noise with respect to what we actually care about.
//! * pooled **above 0.5** -> actively anti-correlated: the gate is systematically discarding nets
//!   that are BETTER at depth 4. That is the expensive case, and it would price the 200x saving.
//!
//! Reporting the pooled number against 0.5 without saying which of these is expected would let any
//! outcome be told as a success story. The three readings are written down before the run.
//!
//! ## Why pooled rather than per-candidate
//!
//! A per-candidate verdict at any affordable game count is noise: 64 pairs carries ci95 ~0.06,
//! which cannot resolve the effect sizes involved. The quantity with teeth is the MEAN over many
//! rejects, and pentanomial pair counts pool exactly (each pair is an independent unit), so N
//! rejects at 64 pairs give the precision of a single 64N-pair match.
//!
//! Per-candidate scores are still printed, because the spread is itself informative: a gate that
//! rejects a mix of clear wins and clear losses is failing differently from one that rejects a
//! tight cluster of neutrals.

use nnue::Net;
use pipeline::gate;

fn main() {
    let mut a = std::env::args().skip(1);
    let dir = a.next().unwrap_or_else(|| {
        eprintln!("usage: reject_audit <dir> [pairs_per_reject] [depth] [seed]");
        std::process::exit(2);
    });
    let pairs: usize = a.next().and_then(|s| s.parse().ok()).unwrap_or(64);
    let depth: u32 = a.next().and_then(|s| s.parse().ok()).unwrap_or(4);
    let seed: u64 = a.next().and_then(|s| s.parse().ok()).unwrap_or(20260907);
    // "rej" (default) or "acc". The ACCEPT half is the one that decides whether the loop is doing
    // anything: rejects were measured at 0.4982 [0.4847,0.5117], exactly champion strength, which
    // says the gate discards nothing -- and says nothing about whether it KEEPS anything.
    let prefix: String = std::env::args().skip_while(|x| x != "--prefix").nth(1).unwrap_or_else(|| "rej".into());

    // Collect gen numbers from rej_g<N>_cand.net, keeping only those with a matching _champ.net.
    // An unpaired candidate is silently useless -- the champion it lost to is the ONLY valid
    // opponent, since the champion moves on every accept.
    // ACCEPTS BOTH NAMINGS: rej_g<N>_cand.net and the tagged rej_<tag>_g<N>_cand.net.
    //
    // The trainer gained --run-tag so that a relaunch (its generation counter restarts at 1) cannot
    // overwrite a previous run's samples. That silently broke this parser, which keyed on the
    // literal prefix "rej_g": tagged files match nothing, and the tool would have printed "no
    // complete reject pairs" -- which reads as "the trainer saved none", not "the reader is wrong".
    // Exactly the trap this project keeps writing down: a pattern that finds nothing is usually
    // broken, not evidence of absence.
    //
    // The STEM is kept rather than reconstructed, so the champion file is the one that actually
    // pairs with this candidate instead of a name rebuilt from parts that might not round-trip.
    let mut found: Vec<(u64, String)> = Vec::new();
    let rd = match std::fs::read_dir(&dir) {
        Ok(r) => r,
        Err(e) => { eprintln!("cannot read {dir}: {e}"); std::process::exit(2); }
    };
    for ent in rd.flatten() {
        let name = ent.file_name().to_string_lossy().to_string();
        let core = match name.strip_prefix(&format!("{prefix}_")).and_then(|c| c.strip_suffix("_cand.net")) {
            Some(c) => c,
            None => continue,
        };
        // "g140" (untagged) or "r7_g140" (tagged). rsplit so a tag containing "_g" cannot confuse it.
        let numtxt = match core.rsplit_once("_g") {
            Some((_, n)) => n,
            None => match core.strip_prefix('g') { Some(n) => n, None => continue },
        };
        if let Ok(g) = numtxt.parse::<u64>() {
            if std::path::Path::new(&format!("{dir}/{prefix}_{core}_champ.net")).exists() {
                found.push((g, core.to_string()));
            }
        }
    }
    found.sort_unstable();
    let gens: Vec<u64> = found.iter().map(|(g, _)| *g).collect();
    let stems: std::collections::HashMap<u64, String> = found.iter().cloned().map(|(g, c)| (g, c)).collect();

    if gens.is_empty() {
        println!("no complete reject pairs in {dir} -- nothing to audit.");
        println!("(the trainer writes them only with --save-rejects <dir> --save-rejects-every N)");
        return;
    }

    println!("reject_audit[{prefix}]: {} pairs from {dir}", gens.len());
    println!("  {pairs} pairs each, depth {depth}, seed {seed}");
    println!("  each candidate plays THE CHAMPION IT LOST TO, not a later one\n");

    let mut tw = 0u32; let mut td = 0u32; let mut tl = 0u32; let mut tp = [0u32; 5];
    let mut rates: Vec<(u64, f64)> = Vec::new();

    for g in &gens {
        let stem = &stems[g];
        let cand = match Net::load(&format!("{dir}/{prefix}_{stem}_cand.net")) { Ok(n) => n, Err(e) => { eprintln!("  gen {g}: {e}"); continue } };
        let champ = match Net::load(&format!("{dir}/{prefix}_{stem}_champ.net")) { Ok(n) => n, Err(e) => { eprintln!("  gen {g}: {e}"); continue } };
        // Seed varies per candidate so the openings are not identical across rejects -- otherwise
        // every reject is measured on the same handful of positions and the pooled interval is a
        // lie about how much independent evidence there is.
        let s = gate::match_nets(&cand, &champ, depth, pairs, seed ^ g);
        tw += s.wins; td += s.draws; tl += s.losses;
        for i in 0..5 { tp[i] += s.pent[i]; }
        rates.push((*g, s.pent_rate()));
        println!("  gen {:>5}  cand vs its champion: {:>3}W-{:>3}D-{:>3}L  rate {:.3} +/- {:.3}",
                 g, s.wins, s.draws, s.losses, s.pent_rate(), s.ci95());
    }

    if rates.is_empty() { println!("\nno pairs could be loaded."); return; }

    let pooled = gate::Score { wins: tw, draws: td, losses: tl, pent: tp };
    let r = pooled.pent_rate();
    let c = pooled.ci95();
    println!("\n  === POOLED over {} rejects: {}W-{}D-{}L, {} games ===", rates.len(), tw, td, tl, pooled.games());
    println!("  rate {r:.4} +/- {c:.4}   interval [{:.4}, {:.4}]", r - c, r + c);

    // BETWEEN-CANDIDATE SPREAD, which the pooled interval does NOT contain.
    //
    // Pooling treats every pair as exchangeable, so its interval measures how precisely THESE
    // candidates were measured -- not how much candidates differ from each other. The claim being
    // made ("the gate's rejects are weaker") generalises over candidates, so the candidate is the
    // sampling unit and the spread across candidates is the relevant error term.
    //
    // This project already enforces exactly this distinction elsewhere: netmatch refuses to call a
    // training result from one seed because between-seed sd (0.047) dwarfs within-run precision.
    // Same trap, different axis -- a pooled interval over 4 candidates can exclude 0.5 while the
    // candidate-to-candidate variation is far too large to support the conclusion.
    let n = rates.len() as f64;
    let mean = rates.iter().map(|(_, x)| x).sum::<f64>() / n;
    if rates.len() >= 2 {
        let var = rates.iter().map(|(_, x)| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let sd = var.sqrt();
        let sem = sd / n.sqrt();
        let ci = 1.96 * sem;
        println!("  candidate-level: mean {mean:.4}, sd {sd:.4} across {} candidates, \
                  95% CI +/- {ci:.4}  [{:.4}, {:.4}]", rates.len(), mean - ci, mean + ci);
        if mean + ci >= 0.5 && r + c < 0.5 {
            println!("  ** NOTE: pooled excludes 0.5 but the candidate-level interval does NOT.");
            println!("  ** The pooled reading is over-confident: it is measuring these candidates");
            println!("  ** precisely, not measuring candidates in general. Bank more rejects.");
        }
    } else {
        println!("  candidate-level: only {} candidate(s) -- no between-candidate spread available,", rates.len());
        println!("  so the pooled interval below is a within-candidate number and CANNOT support");
        println!("  a claim about rejects in general.");
    }

    // Verdict against the three readings declared in the header, so the outcome cannot be
    // reinterpreted to suit whichever number appeared.
    println!();
    if prefix == "acc" {
        // The readings INVERT for accepts, and they are declared here rather than reused from the
        // reject branch, where "below 0.5" is the good outcome.
        if r - c > 0.5 {
            println!("  VERDICT: accepted candidates really ARE stronger at depth {depth} (interval");
            println!("  entirely above 0.5). The gate keeps real improvements; the flat ancestor");
            println!("  readings must then be explained by something other than a noisy accept rule.");
        } else if c < 0.02 {
            println!("  VERDICT: accepts pool at {r:.4}, indistinguishable from 0.5 at a TIGHT interval.");
            println!("  The gate is promoting candidates that are NOT stronger than the champion they");
            println!("  replaced. Combined with rejects also at 0.5, the accept/reject decision");
            println!("  carries no depth-{depth} signal at all and the champion is on a RANDOM WALK --");
            println!("  which is exactly what the ancestor control's 0.464 / 0.498 over 400-generation");
            println!("  windows looks like. That would explain the plateau completely.");
        } else {
            let need = ((c / 0.02).powi(2) * pooled.games() as f64 / 2.0).ceil() as u64;
            println!("  UNRESOLVED: interval contains 0.5 and is too wide ({c:.4}). Ignorance, not a");
            println!("  null. Need roughly {need} pairs total; bank more accepts first.");
        }
    } else if r + c < 0.5 {
        println!("  VERDICT: the gate's rejects ARE weaker at depth {depth} (interval entirely below 0.5).");
        println!("  The depth-1 decision TRANSFERS. The cheap gate is vindicated -- do not spend");
        println!("  200x on a deeper gate on the strength of the +0.043-vs-+0.139 asymmetry alone.");
    } else if r - c > 0.5 {
        println!("  VERDICT: the gate is discarding candidates that are BETTER at depth {depth}");
        println!("  (interval entirely above 0.5). This is the expensive case: the 200x saving is");
        println!("  being paid for in real strength, and a deeper tiebreak near the accept");
        println!("  threshold is now justified by measurement rather than by argument.");
    } else if c < 0.02 {
        println!("  VERDICT: rejects pool at {r:.4}, indistinguishable from 0.5 at a TIGHT interval.");
        println!("  The gate's 'no' carries no information about depth-{depth} strength -- it is");
        println!("  rejecting candidates that are on average exactly as good as the champion.");
        println!("  Not harmless: it means selection is noise with respect to the judged game.");
    } else {
        let need = ((c / 0.02).powi(2) * pooled.games() as f64 / 2.0).ceil() as u64;
        println!("  UNRESOLVED: interval contains 0.5 and is too wide ({c:.4}) to call anything.");
        println!("  This is IGNORANCE, not a null -- two random movers also score 0.500.");
        println!("  Need roughly {need} pairs total; bank more rejects before reading this.");
    }
}
