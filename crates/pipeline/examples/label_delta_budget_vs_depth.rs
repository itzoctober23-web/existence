//! With POSITIONS HELD FIXED, how different is a budget label from a fixed-depth-3 label?
//!
//! WHY THIS IS THE DISCRIMINATOR. Candidate A (`structural_next_PREREG.md`) is motivated by
//! `datagen_depth_RESULT.md`: +128 Elo, RESOLVED, from changing `--depth`. But that experiment let
//! each arm generate its OWN data — 610 generations at depth 1 against **6** at depth 3 — so
//! labels, position distribution and sample efficiency all moved at once.
//!
//! `label_source_RESULT.md` is the experiment that isolates the label channel: same positions, same
//! trainer, same seed, "literally one column apart". Swapping the loop's depth-1 label for
//! **Stockfish @10k nodes** — an enormous upgrade — bought +49 Elo against intervals of +/-58 and
//! +/-69, i.e. UNRESOLVED, even though the label was demonstrably learned (11x better held-out fit
//! than untrained). Its conclusion: "better labels were absorbed; strength did not follow."
//!
//! Candidate A's stated mechanism is LABEL QUALITY ("label quality is worst exactly where positions
//! are hardest"). That is the channel `label_source` measured as null. So before spending 2x2000
//! generations, the cheap question is: **how much does the label even move?** If a budget label and
//! a depth-3 label are nearly the same number on the same position, the label channel is negligible
//! by construction and any effect Candidate A produces must come from the position distribution
//! instead — which would mean the PREREG's mechanism is wrong for the second time.
//!
//! HOW. One fixed trajectory (so the positions are IDENTICAL for both labellers — the whole point;
//! letting each labeller pick its own moves would reintroduce the confound being removed). At every
//! position, score it twice: fixed depth 3, and `best_move_budget` at the measured depth-3 mean
//! cost. Report the difference in centipawns AND in the units the trainer actually sees,
//! `tanh(cp/600)`, because that is `Sample.root` and a cp gap deep in a won position is squashed to
//! nothing by the tanh.
//!
//! Usage: label_delta_budget_vs_depth [net] [games] [budget] [seed]

use board::{Outcome, Position};
use nnue::Net;
use pipeline::datagen::Rng;
use pipeline::search::Searcher;

fn tanhf(x: f64) -> f64 {
    x.tanh()
}

fn pct(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    sorted[((sorted.len() - 1) as f64 * p).round() as usize]
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let net_path = a.get(1).cloned().unwrap_or_else(|| "p1_champion.net".into());
    let games: usize = a.get(2).and_then(|v| v.parse().ok()).unwrap_or(30);
    let budget: u64 = a.get(3).and_then(|v| v.parse().ok()).unwrap_or(5_269);
    let seed: u64 = a.get(4).and_then(|v| v.parse().ok()).unwrap_or(20260912);

    let net = match Net::load(&net_path) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("cannot load {net_path}: {e}");
            std::process::exit(1);
        }
    };
    println!("  net {net_path}, {games} games, budget {budget}, seed {seed}");
    println!("  POSITIONS HELD FIXED: one trajectory, both labellers score every position\n");

    let mut rng = Rng(seed);
    let mut dcp: Vec<f64> = Vec::new();
    let mut dtanh: Vec<f64> = Vec::new();
    let mut agree_sign = 0usize;
    let mut same_move = 0usize;
    let mut n = 0usize;

    for _ in 0..games {
        let mut pos = Position::startpos();
        let mut s = Searcher::with_seed(rng.next());
        for _ in 0..6 {
            let l = pos.legal_moves();
            if l.is_empty() {
                break;
            }
            let m = l.as_slice()[rng.below(l.len())];
            pos.make_move(m);
        }
        for ply in 0..160 {
            let l = pos.legal_moves();
            if l.is_empty() || pos.halfmove >= 100 {
                break;
            }
            // Label A: the current production path.
            let (mv_d, sc_d) = s.best_move(&mut pos, 3, &net);
            // Label B: the same position under a node budget.
            let (mv_b, sc_b, _rd) = s.best_move_budget(&mut pos, &net, budget, 10, rng.next());

            let a_cp = sc_d as f64;
            let b_cp = sc_b as f64;
            dcp.push((b_cp - a_cp).abs());
            dtanh.push((tanhf(b_cp / 600.0) - tanhf(a_cp / 600.0)).abs());
            if (a_cp >= 0.0) == (b_cp >= 0.0) {
                agree_sign += 1;
            }
            if mv_d == mv_b {
                same_move += 1;
            }
            n += 1;

            // THE TRAJECTORY IS DRIVEN BY ONE LABELLER ONLY (depth 3, the control), so both see
            // the same positions. Driving it with whichever moved last would make the comparison
            // path-dependent and silently reintroduce the position confound.
            let m = if ply < 6 && rng.next() % 4 == 0 {
                l.as_slice()[rng.below(l.len())]
            } else {
                mv_d
            };
            if m == board::types::MOVE_NONE {
                break;
            }
            pos.make_move(m);
        }
        let _ = Outcome::Draw;
    }

    if n == 0 {
        println!("  no positions");
        return;
    }
    let mut scp = dcp.clone();
    scp.sort_by(|x, y| x.partial_cmp(y).unwrap());
    let mut st = dtanh.clone();
    st.sort_by(|x, y| x.partial_cmp(y).unwrap());

    let mcp = dcp.iter().sum::<f64>() / n as f64;
    let mt = dtanh.iter().sum::<f64>() / n as f64;

    println!("  positions {n}");
    println!("  same best move          {:>6.1}%", 100.0 * same_move as f64 / n as f64);
    println!("  same sign of eval       {:>6.1}%", 100.0 * agree_sign as f64 / n as f64);
    println!("\n  |delta| in CENTIPAWNS   mean {mcp:>8.1}   p50 {:>8.1}   p90 {:>8.1}   max {:>8.1}",
             pct(&scp, 0.50), pct(&scp, 0.90), scp[n - 1]);
    println!("  |delta| in TRAINER UNITS (tanh(cp/600), i.e. Sample.root):");
    println!("     mean {mt:>8.4}   p50 {:>8.4}   p90 {:>8.4}   max {:>8.4}",
             pct(&st, 0.50), pct(&st, 0.90), st[n - 1]);
    let unchanged = dtanh.iter().filter(|&&d| d < 0.01).count();
    println!("     labels moving less than 0.01: {:>5.1}%  (target range is [-1,1])",
             100.0 * unchanged as f64 / n as f64);
    println!(
        "\n  => {}",
        if mt < 0.02 {
            "the label barely moves — Candidate A's LABEL channel is small by construction"
        } else {
            "the label moves materially — the label channel is live"
        }
    );
}
