//! Isolate the budget's POSITION channel: same labeller, different trajectory, matched row counts.
//!
//! ## Why this is the missing half
//!
//! `candidate_a_channel_FINDING.md` named three channels a node budget moves at once. Two are now
//! measured and neither explains the harm:
//!
//! ```text
//!   LABEL       budget_label_channel_RESULT.md   0.4960 NULL, with 47.8% of labels changed
//!   PER-MOVE    budget_allocation_RESULT.md      flat-vs-depth3 agreement sign FLIPS across seeds
//! ```
//!
//! and `budget_allocation_RESULT.md` closes reallocation as a repair: depth is logarithmic in nodes,
//! so buying depth where it is expensive by selling it where it is cheap loses on every seed.
//!
//! What is left is the trajectory. The budget picks a different move in 58% of the widest positions
//! (`budget_undersearches_wide_positions_RESULT.md`), and different moves lead to different
//! positions, which compound over 2,000 generations. That is invisible to every diagnostic in this
//! chain, because all of them score a FIXED position set.
//!
//! ## The one-variable construction
//!
//! Mirror of `budget_label_ab.rs` with the roles swapped. There, positions were held fixed and the
//! label column was swapped; here the LABELLER is held fixed and the trajectory is swapped:
//!
//! ```text
//!   ARM depth  : moves chosen by fixed depth 3,  labels from fixed depth 3
//!   ARM budget : moves chosen by the node budget, labels from fixed depth 3
//! ```
//!
//! Both arms start each game from the SAME random opening (same rng draws), so the only difference
//! is which move gets played. `blend = 1.0`, so the outcome term is switched off — that matters here
//! more than it did for labels, because budget games are 14.5 points more decisive
//! (`budget_makes_games_decisive_RESULT.md`) and letting `z` through would smuggle the decisiveness
//! channel back in.
//!
//! ## The volume confound, controlled
//!
//! The same file measured the budget yielding **+26.7% more usable rows** out of the same game count.
//! A win or loss bought with more data is not a statement about composition, so both corpora are
//! TRUNCATED to the smaller row count before training. The truncation is reported; if it is large,
//! say so rather than hiding it.
//!
//! Usage: budget_position_ab [net] [games] [budget] [seed] [epochs] [width] [lr]
//! Writes `bpa_depth.net` and `bpa_budget.net` — DIAGNOSTIC only, never ship or gate them.

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
    println!("  LABELLER held fixed at depth 3 in BOTH arms; only the played move differs\n");

    // Each arm plays its own trajectory but from the same openings, so `openings` is drawn once.
    let mut orng = Rng(seed);
    let mut openings: Vec<Position> = Vec::new();
    for _ in 0..games {
        let mut pos = Position::startpos();
        for _ in 0..6 {
            let l = pos.legal_moves();
            if l.is_empty() { break; }
            let m = l.as_slice()[orng.below(l.len())];
            pos.make_move(m);
        }
        openings.push(pos);
    }

    // arm: 0 = depth-driven, 1 = budget-driven. The LABEL is depth 3 in both.
    let play = |arm: usize| -> (Vec<Sample>, usize, f64) {
        let mut data: Vec<Sample> = Vec::new();
        let mut decisive = 0usize;
        let mut plies_total = 0usize;
        for (g, start) in openings.iter().enumerate() {
            let mut pos = start.clone();
            let mut s = Searcher::with_seed(0x5EED ^ (g as u64));
            let mut ds = Searcher::with_seed(0xD00D ^ (g as u64));
            let begin = data.len();
            let mut result = board::Outcome::Draw;
            for ply in 0..160 {
                let l = pos.legal_moves();
                if l.is_empty() { result = pos.outcome(); decisive += 1; break; }
                if pos.halfmove >= 100 { break; }
                // THE LABEL: always depth 3, in both arms. This is what makes the comparison
                // one-variable -- budget_label_channel_RESULT already showed the label channel is
                // null, so letting it move here would reintroduce a channel known to be inert and
                // muddy a result about a different one.
                let (mv_d, sc_d) = ds.best_move(&mut pos, 3, &net);
                // THE MOVE: depth 3 for arm 0, the budget for arm 1.
                let mv = if arm == 0 {
                    mv_d
                } else {
                    s.best_move_budget(&mut pos, &net, budget, 10,
                                       (ply as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1).0
                };
                if mv == board::types::MOVE_NONE { break; }
                data.push(Sample { fen: pos.to_fen(), z: 0.0, root: sc_d, plies_to_end: 0 });
                plies_total += 1;
                pos.make_move(mv);
            }
            let n = data.len() - begin;
            let z = match result {
                board::Outcome::Loss => if pos.stm == board::Color::White { -1.0 } else { 1.0 },
                _ => 0.0,
            };
            for i in 0..n {
                data[begin + i].z = z;
                data[begin + i].plies_to_end = (n - 1 - i) as u32;
            }
        }
        let avg = plies_total as f64 / games as f64;
        (data, decisive, avg)
    };

    let (mut d_data, d_dec, d_len) = play(0);
    let (mut b_data, b_dec, b_len) = play(1);
    println!("  ARM depth : {} rows, {} decisive/{} games, {:.1} plies/game",
             d_data.len(), d_dec, games, d_len);
    println!("  ARM budget: {} rows, {} decisive/{} games, {:.1} plies/game",
             b_data.len(), b_dec, games, b_len);

    // The trajectories MUST differ, or this measures nothing twice.
    let same = d_data.iter().zip(b_data.iter()).filter(|(a, b)| a.fen == b.fen).count();
    let cmp = d_data.len().min(b_data.len());
    println!("  positions identical in both arms: {} of {} compared ({:.1}%)",
             same, cmp, 100.0 * same as f64 / cmp.max(1) as f64);
    if same == cmp {
        eprintln!("  ABORT: the trajectories are identical -- the budget flag was inert");
        std::process::exit(1);
    }

    // ---- VOLUME CONTROL: truncate both to the smaller row count ------------------------------
    let keep = d_data.len().min(b_data.len());
    let dropped_d = d_data.len() - keep;
    let dropped_b = b_data.len() - keep;
    d_data.truncate(keep);
    b_data.truncate(keep);
    println!("  truncated to {keep} rows each (dropped {dropped_d} depth / {dropped_b} budget) \
              -- volume is NOT the variable\n");
    if keep < 500 { eprintln!("  only {keep} rows -- too few to train on, aborting"); std::process::exit(1); }

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
    let (d_tr, d_ho) = split(&d_data);
    let (b_tr, b_ho) = split(&b_data);
    println!("  split {} train / {} holdout (depth arm), {} / {} (budget arm)",
             d_tr.len(), d_ho.len(), b_tr.len(), b_ho.len());

    let tr = Trainer::new(lr, 1.0);  // blend 1.0: the outcome term is OFF, so decisiveness cannot leak in
    let mut nets = Vec::new();
    for (name, data, out) in [("depth", &d_tr, "bpa_depth.net"), ("budget", &b_tr, "bpa_budget.net")] {
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
    println!("\n  HELD-OUT MSE, lower is better");
    println!("  {:<12} {:>18} {:>18}", "net", "on depth holdout", "on budget holdout");
    for (name, nt) in &nets {
        println!("  {:<12} {:>18.5} {:>18.5}", name, tr.loss(nt, &dref), tr.loss(nt, &bref));
    }
    println!("\n  READING: the holdouts are DIFFERENT position sets here (that is the treatment), so");
    println!("  the cross-comparison is not the validity check it was in budget_label_ab -- what");
    println!("  matters is that both arms beat `untrained` by a wide margin. If they do not, both are");
    println!("  undertrained and any game comparison between them is VOID rather than negative.");
    println!("  Measure the two nets with netmatch; never ship or gate them.");
}
