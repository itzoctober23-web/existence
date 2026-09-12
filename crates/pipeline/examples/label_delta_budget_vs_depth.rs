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

    // STRATIFY BY BRANCHING FACTOR. budget_realised_depth_RESULT.md measured that an equal-node
    // budget gives MORE depth to narrow positions and LESS to wide ones -- branching factor falls
    // monotonically with realised depth, by >10x end to end -- so 31-38% of positions are searched
    // at depth 2, SHALLOWER than the control's fixed depth 3, and those are the widest ones.
    // If that is where the harm lives, the label delta and the move disagreement must both
    // CONCENTRATE at high branching factor. A flat profile refutes it.
    const NB: usize = 5;
    let edges = [12usize, 20, 28, 36];          // <=12, 13-20, 21-28, 29-36, >36
    let bucket = |w: usize| -> usize { edges.iter().position(|&e| w <= e).unwrap_or(NB - 1) };
    let mut b_n = [0usize; NB];
    let mut b_dcp = [0f64; NB];
    let mut b_dtanh = [0f64; NB];
    let mut b_same = [0usize; NB];
    let mut b_depth = [0f64; NB];
    let mut b_width = [0f64; NB];

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
            // Label B: the same position under a node budget -- or, when budget == 0, the SAME
            // depth-3 search again. That is the CONTROL, and it is not optional: both labellers
            // share one Searcher, and `shuffle_children` advances `self.rng` on every visit
            // (search.rs:110), so two searches from the same searcher explore different child
            // orderings. Any disagreement that control produces is shuffle noise, not depth, and
            // must be subtracted from the treatment before the treatment means anything.
            let (mv_b, sc_b, dep_b) = if budget == 0 {
                let r = s.best_move(&mut pos, 3, &net);
                (r.0, r.1, 3u32)
            } else {
                let r = s.best_move_budget(&mut pos, &net, budget, 10, rng.next());
                (r.0, r.1, r.2)
            };

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

            let bi = bucket(l.len());
            b_n[bi] += 1;
            b_dcp[bi] += (b_cp - a_cp).abs();
            b_dtanh[bi] += (tanhf(b_cp / 600.0) - tanhf(a_cp / 600.0)).abs();
            if mv_d == mv_b { b_same[bi] += 1; }
            b_depth[bi] += dep_b as f64;
            b_width[bi] += l.len() as f64;

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

    // ---- the stratified table: the whole point of this run -----------------------------------
    println!("\n  BY BRANCHING FACTOR (legal moves at the position)");
    println!("  {:<10} {:>7} {:>8} {:>10} {:>11} {:>12} {:>11}",
             "width", "n", "mean w", "realised d", "same move", "mean |dcp|", "mean |dth|");
    let names = ["<=12", "13-20", "21-28", "29-36", ">36"];
    for i in 0..NB {
        if b_n[i] == 0 { continue; }
        let k = b_n[i] as f64;
        println!("  {:<10} {:>7} {:>8.1} {:>10.2} {:>10.1}% {:>12.1} {:>11.4}",
                 names[i], b_n[i], b_width[i] / k, b_depth[i] / k,
                 100.0 * b_same[i] as f64 / k, b_dcp[i] / k, b_dtanh[i] / k);
    }
    println!("\n  READING: budget_realised_depth_RESULT.md predicts realised depth FALLS as width");
    println!("  rises, and that the widest bucket is searched SHALLOWER than the control's depth 3.");
    println!("  If the harm lives there, `same move` must fall and |dcp| rise with width. A FLAT");
    println!("  profile refutes the mechanism. Run with budget=0 FIRST -- that control measures");
    println!("  shuffle noise alone, and every number above must be read net of it.");
}
