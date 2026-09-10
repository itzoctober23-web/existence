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

    // blend 1.0: target is the ROOT term alone, so `z` plays no part and the comparison is purely
    // label-source. Identical trainer, identical lr, identical init seed for both arms.
    let tr = Trainer::new(lr, 1.0);
    for (name, data, out) in [("self", &self_data, &out_self), ("sf", &sf_data, &out_sf)] {
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
    }
    println!("  Both nets are DIAGNOSTIC artefacts. Measure them on sf_ruler.py; never ship or gate them.");
}
