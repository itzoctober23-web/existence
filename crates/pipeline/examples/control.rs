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
    // --opponent NET plays two saved nets against EACH OTHER instead of against the origin.
    //
    // This exists to test NON-TRANSITIVITY, which is the open question after eight runs today: the
    // per-generation gate says candidates beat their own champion (mean rate >= 0.5 in ALL EIGHT
    // runs, 0.5011 to 0.5275), yet not one final net is above the 0.864 champion they all started
    // from -- three are resolved WORSE. "Beats its parent" and "is stronger" are pulling apart, and
    // with only champion-vs-origin scoring there was no way to see it happen directly.
    //
    // The decisive shape: ep_1 scores 0.834 against the origin where champion_long scores 0.864.
    // If ep_1 nonetheless BEATS champion_long head to head, non-transitivity is demonstrated rather
    // than inferred, and the loop's objective is measurably not the thing it is trying to maximise.
    let opponent_path = get("--opponent");
    let (origin, opp_desc) = match &opponent_path {
        Some(path) => match Net::load(path) {
            Ok(n) => {
                // WIDTH MISMATCH IS ALLOWED, BUT ONLY THE EQUAL-TIME MEASUREMENT IS REPORTED.
                //
                // This used to exit(2). The reasoning was right and the remedy was too broad: a
                // FIXED-DEPTH match across widths hands the wider net more computation per node
                // and charges it nothing, so it flatters capacity by construction. But that is an
                // argument against measurement 1, not against the comparison -- and measurement 2
                // already exists precisely for this, budgeting both sides to EQUAL TIME via
                // arch::equal_time_caps.
                //
                // Blocking it outright blocked the one question the ceiling analysis says matters:
                // 44 readings show every run flat in a 0.79-0.86 band at ARCH rung 0 (width 16,
                // 12,528 weights), and "does more capacity raise the ceiling" cannot be asked by
                // scoring each width against its OWN random origin -- those are different
                // opponents and the rates are not commensurable. It has to be head to head, at
                // equal cost.
                if n.n_hidden != champ.n_hidden {
                    eprintln!("NOTE: width mismatch ({} vs {}). The fixed-depth measurement is \
                               SKIPPED -- at equal depth the wider net gets more computation for \
                               free, which flatters it by construction. Only the equal-TIME capped \
                               result below is a fair comparison.",
                              champ.n_hidden, n.n_hidden);
                }
                (n, path.clone())
            }
            Err(e) => { eprintln!("could not load opponent {path}: {e}"); std::process::exit(2); }
        },
        // The origin is reproducible: main.rs builds it as Net::random(WIDTH_MENU[rung], seed)
        // with the default seed. Same construction, so this is the frozen iteration-zero net.
        None => (Net::random(champ.n_hidden, 20260907),
                 format!("origin Net::random({}, 20260907)", champ.n_hidden)),
    };
    println!("champion {champ_path} (hidden {})  vs  {opp_desc}", champ.n_hidden);
    println!("{pairs} pairs per gate, both sides of every opening\n");

    // 1. The per-generation gate: fixed depth, uncapped. SKIPPED across widths -- see the note
    //    where the opponent is loaded: equal depth is not equal cost, and reporting it anyway
    //    would put an unfair number next to a fair one and invite the wrong one being quoted.
    let same_width = champ.n_hidden == origin.n_hidden;
    for depth in [2u32, 3] {
        if !same_width { break; }
        let sc = gate::match_nets(&champ, &origin, depth, pairs, seed);
        println!("fixed depth {depth}, uncapped   {}W-{}D-{}L   rate {:.3} +/- {:.3}   {}",
                 sc.wins, sc.draws, sc.losses, sc.pent_rate(), sc.ci95(),
                 verdict(sc.pent_rate(), sc.ci95()));
    }

    // 2. The control gate AS THE LOOP RUNS IT -- which means tracking the loop's defaults, not
    //    the ones it was fixed away from.
    //
    //    This hardcoded `6` and a fixed 4000-node budget: exactly the pair main.rs measured as
    //    broken and replaced. Measured here on the champion: the budget bought 4152 nodes
    //    against the 992,296 a full depth-6 search costs from startpos -- 0.4% of the tree.
    //    Neither side finishes its first root move, so both play near-randomly, 113 of 128 games
    //    drew, and it reported 0.504 +/- 0.008: a TIGHT interval around no-difference, which
    //    reads as a confident null and is no evidence at all. The diagnostic written to catch
    //    that failure in the loop had the failure itself.
    //
    //    Depth cap and budget now follow main.rs: cap 4, and the budget DERIVED from the
    //    measured full-tree size at that cap so it means the same thing for every net rather
    //    than silently meaning something different for each.
    let cap_depth: u32 = get("--gate-depth-cap").and_then(|v| v.parse().ok()).unwrap_or(4);
    let derived_nodes = {
        let mut probe = pipeline::search::Searcher::with_seed(1);
        let mut p0 = board::Position::startpos();
        probe.best_move_capped(&mut p0, cap_depth, &origin, u64::MAX, 1);
        probe.nodes.max(1)
    };
    let budget_ns = arch::ns_per_node(&origin, 3, 5) * derived_nodes as f64;
    let (ca, cb) = arch::equal_time_caps(&champ, &origin, budget_ns, 3);
    let sc = gate::match_nets_capped(&champ, &origin, cap_depth, ca, cb, pairs, seed, 4);
    println!("depth cap {cap_depth}, {ca}/{cb} nodes (full tree at that cap = {derived_nodes})   \
{}W-{}D-{}L   rate {:.3} +/- {:.3}   {}",
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
