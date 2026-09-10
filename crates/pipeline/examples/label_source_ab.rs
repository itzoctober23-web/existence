//! CONTROL 2: same positions, same trainer, same architecture — only the LABEL SOURCE differs.
//!
//! ## What it decides
//!
//! `untrained_baseline_RESULT.md` establishes that the learner WORKS: an untrained w16 net reads
//! −366 on the ruler and the champion reads −104, so training is worth ~235 Elo. It also shows the
//! gain is a STEP — gen200 already reads −114, statistically identical to the champion 2000
//! generations later. The learner learns, then stops.
//!
//! That leaves one question with two answers and different fixes:
//!
//! * **the labels stop carrying signal** → better labels restart the climb, and the fix is DATAGEN
//!   (deeper search, longer games)
//! * **something else is saturated** → better labels change nothing, and the fix is elsewhere
//!
//! Substituting a known-good label source answers it directly.
//!
//! ## The one-variable design, and why both arms come from one file
//!
//! Both arms train on **the same positions**, produced by `datagen::play_games` with the champion at
//! the loop's own depth. `dump_positions` recorded each position's FEN *and the loop's own root
//! score*, so the self-play arm needs no re-generation — the two arms are the same rows with one
//! column swapped:
//!
//! ```text
//!   ARM self : target = tanh(loop_root_cp / scale)     <- what the loop mostly trains on today
//!   ARM sf   : target = tanh(sf_cp        / scale)     <- Stockfish's eval on the SAME position
//! ```
//!
//! `blend = 1.0` in both, so the outcome term `z` is switched off and the comparison is purely
//! label-source. Blending would mix the two questions.
//!
//! ## Spec position
//!
//! Stockfish is used here as METHODOLOGY, the same standing as the ruler and the perft oracle. The
//! net this produces is a diagnostic artefact: it must never be shipped, gated, or fed back into the
//! loop. MASTER_PLAN's tabula-rasa constraint governs what the ENGINE learns from, not what the
//! experimenter is allowed to measure with.

use nnue::Net;
use pipeline::datagen::Sample;
use pipeline::trainer::Trainer;
use std::collections::HashMap;

fn arg<T: std::str::FromStr>(name: &str, d: T) -> T {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(d)
}
fn sarg(name: &str, d: &str) -> String {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).cloned().unwrap_or_else(|| d.to_string())
}

fn main() {
    let pos_path = sarg("--positions", "positions.tsv");
    let lab_path = sarg("--sf-labels", "sf_labels.tsv");
    let epochs: usize = arg("--epochs", 12);
    let width: usize = arg("--width", 16);
    let lr: f32 = arg("--lr", 0.01);
    let seed: u64 = arg("--seed", 4242);
    let out_self = sarg("--out-self", "net_self.net");
    let out_sf = sarg("--out-sf", "net_sf.net");

    // SF labels: fen -> centipawns, MOVER-relative (matching Sample.root's convention).
    let mut sf: HashMap<String, i32> = HashMap::new();
    for line in std::fs::read_to_string(&lab_path).expect("sf labels").lines() {
        let mut it = line.split('\t');
        if let (Some(f), Some(c)) = (it.next(), it.next()) {
            if let Ok(v) = c.trim().parse::<i32>() { sf.insert(f.to_string(), v); }
        }
    }
    eprintln!("  {} SF-labelled positions", sf.len());

    // Only rows present in BOTH arms are used, so the two nets see an identical position set.
    // Training one arm on more positions than the other would confound label quality with data
    // volume, which is the whole thing this control is trying to separate.
    let mut self_data: Vec<Sample> = Vec::new();
    let mut sf_data: Vec<Sample> = Vec::new();
    for line in std::fs::read_to_string(&pos_path).expect("positions").lines() {
        let c: Vec<&str> = line.split('\t').collect();
        if c.len() < 4 { continue; }
        let (fen, root, z, pte) = (c[0], c[1].parse::<i32>().unwrap_or(0),
                                   c[2].parse::<f32>().unwrap_or(0.0), c[3].parse::<u32>().unwrap_or(0));
        if let Some(&cp) = sf.get(fen) {
            self_data.push(Sample { fen: fen.to_string(), root, z, plies_to_end: pte });
            sf_data.push(Sample { fen: fen.to_string(), root: cp, z, plies_to_end: pte });
        }
    }
    eprintln!("  {} positions in BOTH arms (identical set, one column swapped)", self_data.len());
    if self_data.len() < 1000 {
        eprintln!("  too few joined rows to train on -- aborting rather than reporting a number from noise");
        std::process::exit(2);
    }

    // HELD-OUT SPLIT. Both ruler arms landed below the champion (self ~1030, sf ~1079 against 1216),
    // which means they share a limit the game comparison cannot see -- and "+49 Elo, intervals
    // +/-58 and +/-69" is unresolved either way. So the games alone cannot say whether the SF label
    // failed to help or was never LEARNED in the first place. This split answers that without
    // spending a single game: if the SF arm predicts held-out SF eval much better than the self arm
    // does, the label transferred and the bottleneck is downstream of the label; if it does not, the
    // training run was too short and the whole control is void.
    //
    // Split on a hash of the FEN, not on position in the file: consecutive rows are plies of the
    // SAME GAME, so a prefix/suffix split would leak the tail of every training game into the
    // holdout and report a flattering number.
    let hash = |s: &str| -> u64 {
        let mut h = 1469598103934665603u64;
        for b in s.as_bytes() { h ^= *b as u64; h = h.wrapping_mul(1099511628211); }
        h
    };
    let is_holdout = |fen: &str| hash(fen) % 10 == 0;

    let split = |d: &Vec<Sample>| -> (Vec<Sample>, Vec<Sample>) {
        let mut tr = Vec::new();
        let mut ho = Vec::new();
        for s in d { if is_holdout(&s.fen) { ho.push(s.clone()) } else { tr.push(s.clone()) } }
        (tr, ho)
    };
    let (self_tr, self_ho) = split(&self_data);
    let (sf_tr, sf_ho) = split(&sf_data);
    eprintln!("  split: {} train / {} holdout (by FEN hash, so no game leaks across the split)",
              self_tr.len(), self_ho.len());

    // blend 1.0: target is the ROOT term alone, so `z` plays no part and the comparison is purely
    // label-source. Identical trainer, identical lr, identical init seed for both arms.
    let tr = Trainer::new(lr, 1.0);
    let mut nets = Vec::new();
    for (name, data, out) in [("self", &self_tr, &out_self), ("sf", &sf_tr, &out_sf)] {
        let mut net = Net::random(width, seed);
        let mut last = 0.0;
        for e in 0..epochs {
            last = tr.epoch(&mut net, data, seed ^ (e as u64 + 1));
        }
        match net.save(out) {
            Ok(()) => println!("  ARM {name}: {epochs} epochs on {} positions, final train loss {last:.5} -> {out}",
                               data.len()),
            Err(e) => eprintln!("  ARM {name}: save failed: {e}"),
        }
        nets.push((name, net));
    }

    // An UNTRAINED net is the reference both arms must beat. Without it a held-out MSE is a bare
    // number with nothing to be better than -- the same gap the absolute ruler existed to close.
    nets.push(("untrained", Net::random(width, seed ^ 0xFFFF)));

    let sf_ref: Vec<&Sample> = sf_ho.iter().collect();
    let self_ref: Vec<&Sample> = self_ho.iter().collect();
    println!("\n  HELD-OUT MSE ({} positions), lower is better", sf_ho.len());
    println!("  {:<12} {:>16} {:>16}", "net", "vs SF label", "vs self label");
    for (name, net) in &nets {
        println!("  {:<12} {:>16.5} {:>16.5}", name, tr.loss(net, &sf_ref), tr.loss(net, &self_ref));
    }
    println!("\n  READING: the SF arm should fit the SF column best and the self arm the self column.");
    println!("  If neither beats `untrained` by much, the arms are undertrained and the ruler");
    println!("  comparison above is void rather than negative.");
    println!("  Both nets are DIAGNOSTIC artefacts. Measure them on sf_ruler.py; never ship or gate them.");
}
