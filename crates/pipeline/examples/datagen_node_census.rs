//! How many nodes does ONE depth-3 datagen move actually cost, and how much does that vary?
//!
//! WHY THIS EXISTS. Two pre-registrations are both blocked on this one unmeasured number:
//!
//!   * `structural_next_PREREG.md:100` — "node budget set to the depth-3 arm's *measured mean*
//!     nodes/move".
//!   * `p1_compounding_PREREG.md:82`  — "set to the **measured median nodes/move** of depth-3
//!     datagen".
//!
//! Neither number exists. The only figure on disk is the cost table in
//! `crates/pipeline/src/main.rs:190` — `d3 10,309` — and its own comment says it is "midgame (the
//! EXPENSIVE case, so this never overspends)". A conservative upper bound is not a mean and is not
//! a median, and using it as one would set the budget too high by construction.
//!
//! WHAT IT ALSO DECIDES. `structural_next_PREREG.md:92-96` pre-registers a gating check: the node
//! budget can only beat fixed depth if effort per position is currently UNEQUAL. If the cost of a
//! depth-3 move is near-constant, there is nothing for a budget to reallocate and the whole
//! candidate is measuring nothing. That check is stated to "run FIRST and gate the rest". The
//! spread reported here is that check's input, measured on the fixed-depth arm where the question
//! actually lives.
//!
//! HOW IT MEASURES. The move loop below is a faithful copy of `datagen::play_game_ext` — same
//! random `open_plies`, same `ply < 6 && rng % 4 == 0` temperature, same 160-ply cap, same
//! 100-halfmove rule, same `Searcher::with_seed(rng.next())` per game. That matters: node cost is a
//! property of the POSITION DISTRIBUTION, so sampling from anything other than datagen's own
//! distribution would answer a different question. The only addition is zeroing `s.nodes` before
//! each search and recording it after.
//!
//! `best_move` does NOT reset the counter (only `best_move_capped` does, `search.rs:303`), so the
//! reset here is load-bearing; without it every number would be a running total.
//!
//! Usage: datagen_node_census [net] [games] [depth]

use board::{Outcome, Position};
use nnue::Net;
use pipeline::datagen::Rng;
use pipeline::search::Searcher;

fn pct(sorted: &[u64], p: f64) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let i = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[i]
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let net_path = a.get(1).cloned().unwrap_or_else(|| "p1_champion.net".into());
    let games: usize = a.get(2).and_then(|v| v.parse().ok()).unwrap_or(40);
    let depth: u32 = a.get(3).and_then(|v| v.parse().ok()).unwrap_or(3);
    // Seeded because one sample is a lottery: the mean has to be shown stable across independent
    // game sets before it is written into a pre-registration as "the measured mean".
    let seed: u64 = a.get(4).and_then(|v| v.parse().ok()).unwrap_or(20260912);
    // BUDGET MODE. 0 = the original fixed-depth census. >0 switches every move to
    // `best_move_budget` and reports the REALISED DEPTH distribution, which is the literal
    // quantity `structural_next_PREREG.md:92-96` registers as the gate on Candidate A:
    // "the variance of realised depth across positions must be > 0".
    let budget: u64 = a.get(5).and_then(|v| v.parse().ok()).unwrap_or(0);

    let net = match Net::load(&net_path) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("cannot load {net_path}: {e}");
            std::process::exit(1);
        }
    };
    if budget > 0 {
        println!("  net {net_path}, BUDGET {budget} nodes/move (iterative deepening, max depth 10), {games} games, seed {seed}\n");
    } else {
        println!("  net {net_path}, depth {depth}, {games} games, seed {seed} (datagen's own move loop)\n");
    }

    let open_plies = 6usize;
    let max_plies = 160usize;
    let mut rng = Rng(seed);
    let mut nodes: Vec<u64> = Vec::new();
    let mut by_ply: Vec<(usize, u64)> = Vec::new();
    let mut rdepth: Vec<u32> = Vec::new();
    let mut rmoves: Vec<usize> = Vec::new();
    let mut decisive = 0usize;

    for _ in 0..games {
        let mut pos = Position::startpos();
        let mut s = Searcher::with_seed(rng.next());

        for _ in 0..open_plies {
            let l = pos.legal_moves();
            if l.is_empty() {
                break;
            }
            let m = l.as_slice()[rng.below(l.len())];
            pos.make_move(m);
        }

        let mut result = Outcome::Draw;
        for ply in 0..max_plies {
            let l = pos.legal_moves();
            if l.is_empty() {
                result = pos.outcome();
                break;
            }
            if pos.halfmove >= 100 {
                break;
            }
            s.nodes = 0; // best_move does not zero it; see header
            let mv = if budget > 0 {
                let (m, _sc, d) = s.best_move_budget(&mut pos, &net, budget, 10, rng.next());
                rdepth.push(d);
                rmoves.push(l.len());
                m
            } else {
                s.best_move(&mut pos, depth, &net).0
            };
            nodes.push(s.nodes);
            by_ply.push((ply, s.nodes));
            if mv == board::types::MOVE_NONE {
                break;
            }
            let m = if ply < 6 && rng.next() % 4 == 0 {
                l.as_slice()[rng.below(l.len())]
            } else {
                mv
            };
            pos.make_move(m);
        }
        if result == Outcome::Loss {
            decisive += 1;
        }
    }

    let n = nodes.len();
    if n == 0 {
        println!("  no positions sampled");
        return;
    }
    let mut srt = nodes.clone();
    srt.sort_unstable();
    let sum: u64 = nodes.iter().sum();
    let mean = sum as f64 / n as f64;
    let median = pct(&srt, 0.50);
    let p10 = pct(&srt, 0.10);
    let p90 = pct(&srt, 0.90);

    println!("  positions {n}   decisive games {decisive}/{games}");
    if budget > 0 {
        // In budget mode every move spends the budget EXACTLY -- the abort fires at
        // `nodes >= node_cap`, so the spend distribution is degenerate by construction and says
        // nothing about position cost. Reporting it under the PREREG labels would be a lie.
        println!("  MEAN   {mean:>12.1}   (== budget by construction; NOT the PREREG quantity)");
        println!("  spend is exactly the budget on every move -> adherence exact, overshoot 0");
    } else {
        println!("  MEAN   {mean:>12.1}   <- structural_next_PREREG asks for this");
        println!("  MEDIAN {median:>12}   <- p1_compounding_PREREG asks for this");
    }
    println!(
        "  min {:>10}   p10 {:>10}   p25 {:>10}   p50 {:>10}",
        srt[0], p10, pct(&srt, 0.25), median
    );
    println!(
        "  p75 {:>10}   p90 {:>10}   p99 {:>10}   max {:>10}",
        pct(&srt, 0.75), p90, pct(&srt, 0.99), srt[n - 1]
    );

    // The pre-registered gating quantity: is effort per position actually unequal?
    let spread = if p10 > 0 { p90 as f64 / p10 as f64 } else { f64::INFINITY };
    let var = nodes.iter().map(|&x| (x as f64 - mean).powi(2)).sum::<f64>() / n as f64;
    let cv = var.sqrt() / mean;
    println!("\n  p90/p10 spread {spread:>8.2}x     coefficient of variation {cv:.3}");
    println!(
        "  table's midgame figure for depth 3 is 10,309; measured mean is {:.2}x that",
        mean / 10_309.0
    );
    println!(
        "  fraction of moves costing MORE than the table figure: {:.1}%",
        100.0 * nodes.iter().filter(|&&x| x > 10_309).count() as f64 / n as f64
    );

    if !rdepth.is_empty() {
        println!("\n  REALISED DEPTH under the budget — the pre-registered gate on Candidate A:");
        let lo = *rdepth.iter().min().unwrap();
        let hi = *rdepth.iter().max().unwrap();
        let mean_d = rdepth.iter().map(|&d| d as f64).sum::<f64>() / rdepth.len() as f64;
        let var_d = rdepth.iter().map(|&d| (d as f64 - mean_d).powi(2)).sum::<f64>() / rdepth.len() as f64;
        for d in lo..=hi {
            let c = rdepth.iter().filter(|&&x| x == d).count();
            if c == 0 { continue; }
            let pctg = 100.0 * c as f64 / rdepth.len() as f64;
            let bar: String = std::iter::repeat('#').take((pctg / 2.0).round() as usize).collect();
            println!("    depth {d:>2}  {c:>6}  {pctg:>5.1}%  {bar}");
        }
        println!("    min {lo}  max {hi}  mean {mean_d:.2}  variance {var_d:.3}  sd {:.3}", var_d.sqrt());
        // WHICH WAY does a budget reallocate? The PREREG says "a hard position gets more depth
        // and a simple one less". Under an equal-NODE budget the opposite must hold: a wide
        // position costs more per ply, so it exhausts the budget at LOWER depth. Measured here
        // rather than argued, by reporting branching factor against realised depth.
        println!("    branching factor by realised depth (tests the DIRECTION of the reallocation):");
        for d in lo..=hi {
            let v: Vec<usize> = rdepth.iter().zip(rmoves.iter()).filter(|(x, _)| **x == d).map(|(_, m)| *m).collect();
            if v.is_empty() { continue; }
            let mb = v.iter().sum::<usize>() as f64 / v.len() as f64;
            println!("      depth {d:>2}  n {:>6}  mean legal moves {mb:>6.2}", v.len());
        }
        println!(
            "    => {}",
            if var_d > 0.0 {
                "VARIES -- the mechanism is present; a budget has effort to reallocate"
            } else {
                "CONSTANT -- mechanism ABSENT, Candidate A would be measuring nothing"
            }
        );
    }

    // Cost by game phase -- the table calls midgame "the expensive case", which is checkable.
    println!("\n  mean nodes by ply bucket:");
    for (lo, hi) in [(0usize, 9usize), (10, 29), (30, 59), (60, 99), (100, 159)] {
        let v: Vec<u64> = by_ply.iter().filter(|(p, _)| *p >= lo && *p <= hi).map(|(_, c)| *c).collect();
        if v.is_empty() {
            continue;
        }
        let m = v.iter().sum::<u64>() as f64 / v.len() as f64;
        println!("    ply {lo:>3}-{hi:<3}  n {:>6}  mean {m:>12.1}", v.len());
    }
}
