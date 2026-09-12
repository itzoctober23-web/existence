//! Is a node budget's move choice LESS STABLE than fixed depth's, on the same position?
//!
//! `trajectory_drift_RESULT.md` contradicted drift and weakened decisiveness feedback, leaving
//! variance amplification as the only surviving mechanism for a harm that appears solely inside the
//! loop (`budget_harm_is_emergent_RESULT.md`). That hypothesis has a PREMISE which can be tested
//! without any training run: **the budget's move choice must actually be noisier than the control's.**
//!
//! If it is not, all three named mechanisms have failed and the search for the mechanism needs a new
//! hypothesis rather than a bigger experiment.
//!
//! ## Construction
//!
//! One fixed trajectory driven by depth 3, so every arm scores identical positions. At each position,
//! run each arm TWICE with different searcher seeds and different rng streams, and ask whether it
//! picks the same move both times:
//!
//! ```text
//!   depth3  self-agreement : best_move(depth 3) vs best_move(depth 3),  different seeds
//!   budget  self-agreement : best_move_budget(B) vs best_move_budget(B), different seeds
//! ```
//!
//! Both arms get the SAME treatment — two fresh searchers, two different seeds — so the comparison is
//! symmetric. Any difference in self-agreement is the arm's own instability.
//!
//! This is the quantity `budget_allocation_ab` measured only for the depth-4 reference (94.7-96.7%
//! self-agreement, which is why every arm's ~40% agreement with it was a property of depth rather
//! than an indictment). Here it is measured for the two arms that actually differ.
//!
//! Stratified by branching factor, because `budget_undersearches_wide_positions_RESULT.md` found the
//! budget's disagreement concentrated at high width: if instability is the mechanism, it must be
//! worst there too.
//!
//! Usage: search_stability [net] [games] [budget] [seed]

use board::Position;
use nnue::Net;
use pipeline::datagen::Rng;
use pipeline::search::Searcher;

fn arg<T: std::str::FromStr>(i: usize, d: T) -> T {
    std::env::args().nth(i).and_then(|v| v.parse().ok()).unwrap_or(d)
}

const NB: usize = 5;
const EDGES: [usize; 4] = [12, 20, 28, 36];
const NAMES: [&str; NB] = ["<=12", "13-20", "21-28", "29-36", ">36"];
fn bucket(w: usize) -> usize { EDGES.iter().position(|&e| w <= e).unwrap_or(NB - 1) }

fn main() {
    let net_path = std::env::args().nth(1).unwrap_or_else(|| "cand_start.net".into());
    let games: usize = arg(2, 40);
    let budget: u64 = arg(3, 5_269);
    let seed: u64 = arg(4, 20260912);

    let net = match Net::load(&net_path) {
        Ok(n) => n,
        Err(e) => { eprintln!("cannot load {net_path}: {e}"); std::process::exit(1); }
    };
    println!("  net {net_path}, {games} games, budget {budget}, seed {seed}");
    println!("  one depth-3 trajectory; each arm re-run TWICE with different seeds\n");

    let mut rng = Rng(seed);
    let mut b_n = [0usize; NB];
    let mut b_dstable = [0usize; NB];
    let mut b_bstable = [0usize; NB];
    let mut b_depth_a = [0f64; NB];
    let mut b_depth_b = [0f64; NB];
    // BOUNDARY STRADDLING, measured directly. If a position sits on a depth boundary, the two runs
    // reach DIFFERENT realised depths -- a small rng perturbation decides whether another ply
    // completes. This is the mechanical test of the explanation offered for the inverted-U shape:
    // instability should track this column, not the depth itself.
    let mut b_depthdiff = [0usize; NB];

    for g in 0..games {
        let mut pos = Position::startpos();
        let mut s = Searcher::with_seed(rng.next());
        for _ in 0..6 {
            let l = pos.legal_moves();
            if l.is_empty() { break; }
            let m = l.as_slice()[rng.below(l.len())];
            pos.make_move(m);
        }
        for ply in 0..160 {
            let l = pos.legal_moves();
            if l.is_empty() || pos.halfmove >= 100 { break; }
            let k = (g as u64) << 20 | ply as u64;

            // depth 3, twice, different searcher seeds
            let mut p1 = pos.clone();
            let (d1, _) = Searcher::with_seed(0x1111 ^ k).best_move(&mut p1, 3, &net);
            let mut p2 = pos.clone();
            let (d2, _) = Searcher::with_seed(0x2222 ^ k).best_move(&mut p2, 3, &net);

            // budget, twice, the SAME two searcher seeds and two distinct budget rng streams
            let mut p3 = pos.clone();
            let (b1, _, dep1) = Searcher::with_seed(0x1111 ^ k)
                .best_move_budget(&mut p3, &net, budget, 10, (k << 1) | 1);
            let mut p4 = pos.clone();
            let (b2, _, dep2) = Searcher::with_seed(0x2222 ^ k)
                .best_move_budget(&mut p4, &net, budget, 10, (k << 1) | 3);

            let bi = bucket(l.len());
            b_n[bi] += 1;
            if d1 == d2 { b_dstable[bi] += 1; }
            if b1 == b2 { b_bstable[bi] += 1; }
            b_depth_a[bi] += dep1 as f64;
            b_depth_b[bi] += dep2 as f64;
            if dep1 != dep2 { b_depthdiff[bi] += 1; }

            // trajectory driven by ONE labeller only, so positions are identical for both arms
            let (mv, _) = s.best_move(&mut pos, 3, &net);
            if mv == board::types::MOVE_NONE { break; }
            let m = if ply < 6 && rng.next() % 4 == 0 { l.as_slice()[rng.below(l.len())] } else { mv };
            pos.make_move(m);
        }
    }

    println!("  SELF-AGREEMENT (same position, same arm, two different seeds)");
    println!("  {:<8} {:>7} {:>12} {:>12} {:>12} {:>14}",
             "width", "n", "depth3", "budget", "budget-d3", "realised d / straddle%");
    let (mut td, mut tb, mut tn) = (0usize, 0usize, 0usize);
    for i in 0..NB {
        if b_n[i] == 0 { continue; }
        let k = b_n[i] as f64;
        println!("  {:<8} {:>7} {:>11.1}% {:>11.1}% {:>+11.1} {:>9.2} {:>6.1}%",
                 NAMES[i], b_n[i],
                 100.0 * b_dstable[i] as f64 / k, 100.0 * b_bstable[i] as f64 / k,
                 100.0 * (b_bstable[i] as f64 - b_dstable[i] as f64) / k,
                 0.5 * (b_depth_a[i] + b_depth_b[i]) / k,
                 100.0 * b_depthdiff[i] as f64 / k);
        td += b_dstable[i]; tb += b_bstable[i]; tn += b_n[i];
    }
    let f = tn as f64;
    let straddle: usize = b_depthdiff.iter().sum();
    println!("\n  STRADDLE: {:.1}% of positions reached a DIFFERENT realised depth on the two runs",
             100.0 * straddle as f64 / f);
    println!("  OVERALL  depth3 {:.1}%   budget {:.1}%   budget-depth3 {:+.1} points",
             100.0 * td as f64 / f, 100.0 * tb as f64 / f, 100.0 * (tb as f64 - td as f64) / f);
    println!("\n  READING: variance amplification requires the budget to be LESS self-consistent than");
    println!("  the control -- a NEGATIVE budget-depth3, concentrated in the wide buckets where");
    println!("  budget_undersearches_wide_positions found its damage. A zero or positive difference");
    println!("  falsifies the premise, and with drift contradicted and decisiveness feedback weakened");
    println!("  that would leave NO surviving mechanism from the three named. No games are played.");
}
