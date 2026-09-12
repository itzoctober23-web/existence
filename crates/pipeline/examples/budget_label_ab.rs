//! Isolate the BUDGET's LABEL channel the only way a self-play loop permits: on a FIXED corpus.
//!
//! ## Why this exists, and why the live "cell C" arm could not do it
//!
//! `structural_next_PREREG.md` Candidate A frames a node budget as a LABEL-QUALITY intervention.
//! `candidate_a_channel_FINDING.md` shows the budget also moves the position distribution and the
//! decisive-game rate, so a live budget arm confounds three channels at once.
//!
//! The obvious fix -- an arm whose MOVES come from fixed depth and whose LABELS come from the budget
//! -- was built (`--datagen-budget-labels-only`) and **does not work across generations**. In a
//! self-play loop the data generator IS the net being trained: the arm tracks the control for
//! generation 1 and diverges the moment the differing labels produce a differing net. Measured on
//! the live run: positions identical on **4 of 1180** generations, diverging at generation 2.
//!
//! `label_source_ab.rs` is the design that works, and this is that design with the budget in place
//! of Stockfish: build ONE corpus, score every position TWICE, and train two nets on the identical
//! row set with only the `root` column swapped. No self-play during training, so nothing can drift.
//!
//! ## The one-variable construction
//!
//! ```text
//!   ARM depth  : root = fixed-depth-3 search score      (what production trains on)
//!   ARM budget : root = best_move_budget(5269) score    (the same position, budget-allocated)
//! ```
//!
//! The trajectory is driven by the DEPTH search only, so both arms see byte-identical FENs, `z` and
//! `plies_to_end`. `blend = 1.0`, so the outcome term is switched off and nothing but the label
//! moves -- the same reason `label_source_ab` uses it.
//!
//! A SECOND `Searcher` computes the budget label so it cannot advance the driving searcher's
//! shuffle rng, and its seed comes from the ply rather than the shared stream. Both matter:
//! `candidate_a_channel_FINDING.md` measured that sharing a searcher flips ~4% of moves on ties.
//!
//! Usage: budget_label_ab [net] [games] [budget] [seed] [epochs] [width] [lr]
//! Writes `bla_depth.net` and `bla_budget.net` — DIAGNOSTIC artefacts. Measure them with netmatch
//! or sf_ruler.py; never ship or gate them.

use board::Position;
use nnue::Net;
use pipeline::datagen::{Rng, Sample};
use pipeline::search::Searcher;
use pipeline::trainer::Trainer;

fn arg<T: std::str::FromStr>(i: usize, d: T) -> T {
    std::env::args().nth(i).and_then(|v| v.parse().ok()).unwrap_or(d)
}

fn main() {
    let net_path = std::env::args().nth(1).unwrap_or_else(|| "cand_start.net".into());
    let games: usize = arg(2, 60);
    let budget: u64 = arg(3, 5_269);
    let seed: u64 = arg(4, 20260912);
    let epochs: usize = arg(5, 12);
    let width: usize = arg(6, 16);
    let lr: f32 = arg(7, 0.0002);

    let net = match Net::load(&net_path) {
        Ok(n) => n,
        Err(e) => { eprintln!("cannot load {net_path}: {e}"); std::process::exit(1); }
    };
    println!("  corpus from {net_path}: {games} games, budget {budget}, seed {seed}");
    println!("  trajectory driven by DEPTH 3 only, so both arms see identical positions\n");

    let mut rng = Rng(seed);
    let mut depth_data: Vec<Sample> = Vec::new();
    let mut budget_data: Vec<Sample> = Vec::new();
    let mut moved = 0usize;

    for _ in 0..games {
        let mut pos = Position::startpos();
        let mut s = Searcher::with_seed(rng.next());
        let mut ls = Searcher::with_seed(0xC0FFEE ^ rng.next()); // separate: must not touch s.rng
        for _ in 0..6 {
            let l = pos.legal_moves();
            if l.is_empty() { break; }
            let m = l.as_slice()[rng.below(l.len())];
            pos.make_move(m);
        }
        let start = depth_data.len();
        let mut result = board::Outcome::Draw;
        for ply in 0..160 {
            let l = pos.legal_moves();
            if l.is_empty() { result = pos.outcome(); break; }
            if pos.halfmove >= 100 { break; }
            let (mv, sc_d) = s.best_move(&mut pos, 3, &net);
            let (_bm, sc_b, _d) = ls.best_move_budget(
                &mut pos, &net, budget, 10,
                (ply as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
            if mv == board::types::MOVE_NONE { break; }
            let fen = pos.to_fen();
            if sc_d != sc_b { moved += 1; }
            depth_data.push(Sample { fen: fen.clone(), z: 0.0, root: sc_d, plies_to_end: 0 });
            budget_data.push(Sample { fen, z: 0.0, root: sc_b, plies_to_end: 0 });
            let m = if ply < 6 && rng.next() % 4 == 0 { l.as_slice()[rng.below(l.len())] } else { mv };
            pos.make_move(m);
        }
        // Label the game's rows with its outcome, exactly as datagen does. Both arms share this.
        let z = match result {
            board::Outcome::Loss => if pos.stm == board::Color::White { -1.0 } else { 1.0 },
            _ => 0.0,
        };
        let n = depth_data.len() - start;
        for i in 0..n {
            let pte = (n - 1 - i) as u32;
            depth_data[start + i].z = z;  depth_data[start + i].plies_to_end = pte;
            budget_data[start + i].z = z; budget_data[start + i].plies_to_end = pte;
        }
    }

    let n = depth_data.len();
    if n < 500 { eprintln!("  only {n} positions -- too few to train on, aborting"); std::process::exit(1); }
    // The arms MUST be one column apart. Assert it rather than trust it.
    assert_eq!(depth_data.len(), budget_data.len());
    for (a, b) in depth_data.iter().zip(budget_data.iter()) {
        assert_eq!(a.fen, b.fen, "arms diverged on FEN -- the corpus is not shared");
        assert_eq!(a.z, b.z);
        assert_eq!(a.plies_to_end, b.plies_to_end);
    }
    println!("  corpus {n} positions, identical FEN/z/plies in both arms (asserted)");
    println!("  labels differing: {moved} ({:.1}%)\n", 100.0 * moved as f64 / n as f64);

    // Split by FEN hash, not file position: consecutive rows are plies of the same game, so a
    // prefix/suffix split leaks the tail of every training game into the holdout.
    let hash = |s: &str| -> u64 {
        let mut h = 1469598103934665603u64;
        for b in s.as_bytes() { h ^= *b as u64; h = h.wrapping_mul(1099511628211); }
        h
    };
    let split = |d: &Vec<Sample>| -> (Vec<Sample>, Vec<Sample>) {
        let (mut tr, mut ho) = (Vec::new(), Vec::new());
        for s in d { if hash(&s.fen) % 10 == 0 { ho.push(s.clone()) } else { tr.push(s.clone()) } }
        (tr, ho)
    };
    let (d_tr, d_ho) = split(&depth_data);
    let (b_tr, b_ho) = split(&budget_data);
    println!("  split {} train / {} holdout (by FEN hash)", d_tr.len(), d_ho.len());

    let tr = Trainer::new(lr, 1.0); // blend 1.0: target is the ROOT term alone
    let mut nets = Vec::new();
    for (name, data, out) in [("depth", &d_tr, "bla_depth.net"), ("budget", &b_tr, "bla_budget.net")] {
        let mut nt = Net::random(width, seed);
        let mut last = 0.0;
        for e in 0..epochs { last = tr.epoch(&mut nt, data, seed ^ (e as u64 + 1)); }
        match nt.save(out) {
            Ok(()) => println!("  ARM {name}: {epochs} epochs on {} rows, final train loss {last:.5} -> {out}", data.len()),
            Err(e) => eprintln!("  ARM {name}: save failed: {e}"),
        }
        nets.push((name, nt));
    }
    nets.push(("untrained", Net::random(width, seed ^ 0xFFFF)));

    let dref: Vec<&Sample> = d_ho.iter().collect();
    let bref: Vec<&Sample> = b_ho.iter().collect();
    println!("\n  HELD-OUT MSE ({} positions), lower is better", d_ho.len());
    println!("  {:<12} {:>18} {:>18}", "net", "vs depth label", "vs budget label");
    for (name, nt) in &nets {
        println!("  {:<12} {:>18.5} {:>18.5}", name, tr.loss(nt, &dref), tr.loss(nt, &bref));
    }
    println!("\n  READING: each arm should fit its OWN column best. If neither beats `untrained` by");
    println!("  much, both are undertrained and any game comparison between them is void, not");
    println!("  negative -- the trap label_source_RESULT.md had to rule out before it could report.");
    println!("  Both nets are DIAGNOSTIC. Measure with netmatch; never ship or gate them.");
}
