//! IS THE LEARNING REAL, OR IS THE CONTROL GATE BROKEN?
//!
//! The high-volume run accepted on gates that RESOLVED (20W-44D-0L, 0.656 +/- 0.057 at
//! generation 1) and then reported `control vs origin: 4W-56D-4L, 0.500 +/- 0.047`. Both
//! cannot be right: if each champion genuinely beat the one before it on decisive evidence,
//! the composition has to beat the origin.
//!
//! The two numbers come from DIFFERENT GATES, which is the first thing to rule out:
//!   per-generation : fixed depth 2, no node cap
//!   control        : depth cap 6 with a 4,000-node budget
//! A depth-6 search costs on the order of 30^6 nodes. A 4,000-node budget does not finish the
//! FIRST root move, so `best_move_capped` returns having examined one or two shuffled root
//! moves — i.e. it plays nearly at random, for both sides, which would produce exactly the
//! all-drawn 0.500 seen. That would make the control BLIND, not negative, and the difference
//! matters enormously: blind means "no evidence", negative means "the learning is fake".
//!
//! This runs the same two nets through both gates and also reports how many root moves the
//! capped search actually completes. Same nets, same openings, same seeds.

use nnue::Net;
use pipeline::{arch, gate};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let get = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let champ_path = get("--champion").unwrap_or_else(|| "champion_vol.net".into());
    let pairs: usize = get("--pairs").and_then(|v| v.parse().ok()).unwrap_or(64);
    let seed: u64 = get("--seed").and_then(|v| v.parse().ok()).unwrap_or(0x5EED);

    let champ = match Net::load(&champ_path) {
        Ok(n) => n,
        Err(e) => { eprintln!("could not load {champ_path}: {e}"); std::process::exit(2); }
    };
    // The origin is reproducible: main.rs builds it as Net::random(WIDTH_MENU[rung], seed) with
    // the default seed. Same construction here, so this really is the frozen iteration-zero net.
    let origin = Net::random(champ.n_hidden, 20260907);
    println!("champion {champ_path} (hidden {})  vs  origin Net::random({}, 20260907)",
             champ.n_hidden, champ.n_hidden);
    println!("{pairs} pairs per gate, both sides of every opening\n");

    // 1. The per-generation gate: fixed depth, uncapped.
    for depth in [2u32, 3] {
        let sc = gate::match_nets(&champ, &origin, depth, pairs, seed);
        println!("fixed depth {depth}, uncapped   {}W-{}D-{}L   rate {:.3} +/- {:.3}   {}",
                 sc.wins, sc.draws, sc.losses, sc.pent_rate(), sc.ci95(),
                 verdict(sc.pent_rate(), sc.ci95()));
    }

    // 2. The control gate as the loop runs it.
    let budget_ns = arch::ns_per_node(&origin, 3, 5) * 4000.0;
    let (ca, cb) = arch::equal_time_caps(&champ, &origin, budget_ns, 3);
    let sc = gate::match_nets_capped(&champ, &origin, 6, ca, cb, pairs, seed, 4);
    println!("depth cap 6, {ca}/{cb} nodes   {}W-{}D-{}L   rate {:.3} +/- {:.3}   {}",
             sc.wins, sc.draws, sc.losses, sc.pent_rate(), sc.ci95(),
             verdict(sc.pent_rate(), sc.ci95()));

    // 3. THE DIAGNOSIS: how much of the tree does that budget actually buy? If a depth-6 search
    //    under this budget cannot even finish one root move, the "gate" is two random movers.
    let mut s = pipeline::search::Searcher::with_seed(1);
    let mut p = board::Position::startpos();
    s.best_move_capped(&mut p, 6, &champ, u64::MAX, 1);
    let full_d6 = s.nodes;
    let mut p = board::Position::startpos();
    s.best_move_capped(&mut p, 6, &champ, ca, 1);
    println!("\nfrom startpos: a FULL depth-6 search costs {full_d6} nodes; the budget is {ca}.");
    println!("  budget buys {:.4}% of the tree, aborted={}", 100.0 * ca as f64 / full_d6 as f64, s.aborted);
    // What budget would actually cover the tree? The clock gate is only meaningful if BOTH
    // nets can see the whole root at the cap depth; the handicap should come from the wide net
    // being SLOWER PER NODE, not from neither side finishing a move.
    println!("\n  full-tree cost by depth (startpos, so a lower bound on midgame):");
    for d in [2u32, 3, 4, 5] {
        let mut q = board::Position::startpos();
        s.best_move_capped(&mut q, d, &champ, u64::MAX, 1);
        println!("    depth {d}: {} nodes", s.nodes);
    }
    if (ca as f64) < full_d6 as f64 / 20.0 {
        println!("  => the capped search sees a tiny fraction of the first root moves. It is not\n  \
                     playing the position, it is returning whichever shuffled move it got to.\n  \
                     The control gate is BLIND, and 0.500 from it is NO EVIDENCE, not a negative.");
    }
}

fn verdict(rate: f64, ci: f64) -> &'static str {
    if ci >= 0.05 { "UNRESOLVED (interval too wide to decide)" }
    else if rate - ci > 0.5 { "champion stronger" }
    else if rate + ci < 0.5 { "champion WEAKER" }
    else { "no difference detected" }
}
