//! Does a WIDTH-AWARE node budget pick better moves than a FLAT one, at EXACTLY the same total nodes?
//!
//! ## The question this exists to answer
//!
//! `budget_undersearches_wide_positions_RESULT.md` measured that a flat node budget searches the
//! WIDEST positions 0.85 plies shallower than fixed depth 3 and the narrowest 0.50 plies deeper,
//! monotone in width on all three seeds, and that move agreement collapses to 41.9% (against a 97.2%
//! control) in positions with more than 36 legal moves. `budget_realised_depth_RESULT.md` explains
//! why: a wide position costs more per ply, so it exhausts a flat budget sooner.
//!
//! The obvious repair is a minimum DEPTH floor, and it cannot be tested cleanly: a floor at the
//! control's own depth 3 makes the arm spend strictly MORE than the control on wide positions while
//! keeping its surplus on narrow ones, which destroys the compute-matching
//! `budget_realised_depth_RESULT.md` records as the property the A/B needs.
//!
//! **Proportional allocation has no such problem.** Give position i a budget of `B * w_i / mean(w)`.
//! The total is `B * sum(w_i) / mean(w) = B * N` — byte-for-byte the flat arm's total, by
//! construction, with no tuning constant and no clamp. Budget adherence is EXACT (every move spends
//! exactly its cap, min = p10 = median = p90 = max), so the match is provable rather than
//! approximate. This is the direct test of "the budget allocates backwards".
//!
//! ## Construction
//!
//! Two passes, because the allocation needs the mean width before it can allocate:
//!
//! ```text
//!   pass 1  play `games` games at fixed depth 3, STORING every position and its legal-move count
//!   ---     mean width W over all stored positions
//!   pass 2  score each stored position four ways:
//!             ref    depth `ref_depth` (default 4) -- one ply deeper than the control
//!             ctrl   depth `ref_depth` AGAIN, different searcher seed -- the tie-noise baseline
//!             flat   best_move_budget(B)
//!             prop   best_move_budget(B * w_i / W)
//! ```
//!
//! Positions are IDENTICAL for all four, because the trajectory is driven by depth 3 alone. Agreement
//! is measured against `ref`, and **`ctrl` is not optional**: `shuffle_children` advances the
//! searcher rng, so two identical searches disagree on ties at a rate that varies with width
//! (measured 89-98%). Reading `flat` or `prop` without subtracting that baseline overstates both.
//!
//! Every arm gets a FRESH searcher seeded from the ply index, so no arm perturbs another's rng and
//! all four are treated identically.
//!
//! Usage: budget_allocation_ab [net] [games] [budget] [seed] [ref_depth]
//! Reports nothing that ships: this is a search diagnostic, not a strength measurement.

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
fn bucket(w: usize) -> usize {
    EDGES.iter().position(|&e| w <= e).unwrap_or(NB - 1)
}

fn main() {
    let net_path = std::env::args().nth(1).unwrap_or_else(|| "cand_start.net".into());
    let games: usize = arg(2, 40);
    let budget: u64 = arg(3, 5_269);
    let seed: u64 = arg(4, 20260912);
    let ref_depth: u32 = arg(5, 4);

    let net = match Net::load(&net_path) {
        Ok(n) => n,
        Err(e) => { eprintln!("cannot load {net_path}: {e}"); std::process::exit(1); }
    };
    println!("  net {net_path}, {games} games, flat budget {budget}, seed {seed}, reference depth {ref_depth}");

    // ---- pass 1: fixed trajectory at depth 3, recording positions and widths ------------------
    let mut rng = Rng(seed);
    let mut store: Vec<(Position, usize)> = Vec::new();
    for _ in 0..games {
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
            let (mv, _sc) = s.best_move(&mut pos, 3, &net);
            if mv == board::types::MOVE_NONE { break; }
            store.push((pos.clone(), l.len()));
            let m = if ply < 6 && rng.next() % 4 == 0 { l.as_slice()[rng.below(l.len())] } else { mv };
            pos.make_move(m);
        }
    }
    let n = store.len();
    if n < 200 { eprintln!("  only {n} positions -- too few, aborting"); std::process::exit(1); }
    let meanw: f64 = store.iter().map(|(_, w)| *w as f64).sum::<f64>() / n as f64;
    println!("  {n} positions, mean width {meanw:.2}\n");

    // ---- pass 2: four searches per position ---------------------------------------------------
    let mut b_n = [0usize; NB];
    let mut b_w = [0f64; NB];
    let mut b_ctrl = [0usize; NB];
    let mut b_flat = [0usize; NB];
    let mut b_prop = [0usize; NB];
    // THE ARM THAT WAS MISSING: plain fixed depth 3, the thing a budget REPLACES. Without it the
    // table compares two budgets to each other and never asks whether either beats the control.
    let mut b_d3 = [0usize; NB];
    let mut b_dflat = [0f64; NB];
    let mut b_dprop = [0f64; NB];
    let mut b_alloc = [0f64; NB];
    let (mut tot_flat, mut tot_prop) = (0u64, 0u64);

    for (i, (p, w)) in store.iter().enumerate() {
        let k = i as u64;
        let alloc = ((budget as f64) * (*w as f64) / meanw).round().max(1.0) as u64;
        tot_flat += budget;
        tot_prop += alloc;

        let mut pr = p.clone();
        let (mv_ref, _) = Searcher::with_seed(0xA11CE ^ k).best_move(&mut pr, ref_depth, &net);
        let mut pc = p.clone();
        let (mv_ctrl, _) = Searcher::with_seed(0xB0B ^ k).best_move(&mut pc, ref_depth, &net);
        let mut pf = p.clone();
        let (mv_flat, _, df) = Searcher::with_seed(0xC0FFEE ^ k)
            .best_move_budget(&mut pf, &net, budget, 10, k | 1);
        let mut p3 = p.clone();
        let (mv_d3, _) = Searcher::with_seed(0xD3D3 ^ k).best_move(&mut p3, 3, &net);
        let mut pp = p.clone();
        let (mv_prop, _, dp) = Searcher::with_seed(0xC0FFEE ^ k)
            .best_move_budget(&mut pp, &net, alloc, 10, k | 1);

        let bi = bucket(*w);
        b_n[bi] += 1;
        b_w[bi] += *w as f64;
        b_alloc[bi] += alloc as f64;
        b_dflat[bi] += df as f64;
        b_dprop[bi] += dp as f64;
        if mv_ctrl == mv_ref { b_ctrl[bi] += 1; }
        if mv_flat == mv_ref { b_flat[bi] += 1; }
        if mv_prop == mv_ref { b_prop[bi] += 1; }
        if mv_d3 == mv_ref { b_d3[bi] += 1; }
    }

    // ---- compute-matching is the precondition; assert it rather than hope ---------------------
    let skew = (tot_prop as f64 - tot_flat as f64).abs() / tot_flat as f64;
    println!("  TOTAL NODES   flat {tot_flat}   proportional {tot_prop}   skew {:.4}%", 100.0 * skew);
    if skew > 0.01 {
        println!("  ** ABORT-WORTHY: the arms are NOT compute-matched; every comparison below is void.");
    } else {
        println!("  compute-matched to within 1% -- the comparison is valid\n");
    }

    println!("  AGREEMENT WITH DEPTH-{ref_depth} REFERENCE, by branching factor");
    println!("  {:<8} {:>6} {:>7} {:>9} {:>8} {:>8} {:>8} {:>7} {:>7} {:>8}",
             "width", "n", "mean w", "alloc", "ctrl", "depth3", "flat", "prop", "flat-d3", "prop-d3");
    let (mut tc, mut tf, mut tp, mut tn) = (0usize, 0usize, 0usize, 0usize);
    let mut t3 = 0usize;
    for i in 0..NB {
        if b_n[i] == 0 { continue; }
        let k = b_n[i] as f64;
        println!("  {:<8} {:>6} {:>7.1} {:>9.0} {:>7.1}% {:>7.1}% {:>7.1}% {:>7.1}% {:>+7.1} {:>+7.1}",
                 NAMES[i], b_n[i], b_w[i] / k, b_alloc[i] / k,
                 100.0 * b_ctrl[i] as f64 / k, 100.0 * b_d3[i] as f64 / k,
                 100.0 * b_flat[i] as f64 / k, 100.0 * b_prop[i] as f64 / k,
                 100.0 * (b_flat[i] as f64 - b_d3[i] as f64) / k,
                 100.0 * (b_prop[i] as f64 - b_d3[i] as f64) / k);
        tc += b_ctrl[i]; tf += b_flat[i]; tp += b_prop[i]; tn += b_n[i]; t3 += b_d3[i];
    }
    let f = tn as f64;
    println!("\n  OVERALL   ctrl {:.1}%   depth3 {:.1}%   flat {:.1}%   prop {:.1}%",
             100.0 * tc as f64 / f, 100.0 * t3 as f64 / f, 100.0 * tf as f64 / f, 100.0 * tp as f64 / f);
    println!("            flat-depth3 {:+.1} points    prop-depth3 {:+.1} points    prop-flat {:+.1} points",
             100.0 * (tf as f64 - t3 as f64) / f, 100.0 * (tp as f64 - t3 as f64) / f,
             100.0 * (tp as f64 - tf as f64) / f);
    println!("\n  READING: `ctrl` is two identical depth-{ref_depth} searches and is the ceiling -- no arm can");
    println!("  beat it, and the gap to it is what tie-noise alone costs. The claim under test is that");
    println!("  `prop` beats `flat` at IDENTICAL total nodes. If prop-flat is positive and concentrated");
    println!("  in the wide buckets, the flat budget's allocation is the defect. If it is flat or");
    println!("  negative, allocation is NOT the lever and the wide-bucket damage must come from");
    println!("  somewhere else. No games are played here -- this is a search diagnostic, not strength.");
}
