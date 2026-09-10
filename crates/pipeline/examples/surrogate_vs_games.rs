//! THE CLEAN TEST OF THE GRAMMAR FITNESS: does its ranking match the GAMES' ranking?
//!
//! `surrogate_validity.py` measured the surrogate against the game result on 134 real gated
//! candidates and found r = +0.0617, 95% CI [-0.109, +0.229] -- no detectable relationship. That
//! measurement names its own limit: the gates are 12-game matches whose own ci95 is +/-0.10-0.25,
//! so noise in the OUTCOME attenuates the correlation and more candidates at 12 games will not
//! tighten it. **Wider matches would.**
//!
//! This is that experiment, run on the one set of programs where it can be done cleanly. The
//! reference ladder is ten hand-written programs of KNOWN, DELIBERATELY DIFFERENT search paradigms
//! -- depth-one, alpha-beta, +hash, +ID, MCTS, capture extension, proof-number. Their strength
//! differences are large and structural rather than a one-token mutation, and every one can be
//! played for as many pairs as the budget allows.
//!
//! So: score each program with FITNESS 3's mates-per-cost (the number the search actually
//! optimises), then play a full round robin with `match_progs` -- same net, same budget, both
//! sides, pentanomial pairs -- and compare the two rankings.
//!
//! WHY THIS IS THE DECISIVE FORM. On mutants, a null is ambiguous: the candidates may simply be
//! near-identical to their parent, so neither number can separate them and r=0 says nothing about
//! the metric. These ten are NOT near-identical -- reference_audit records MCTS at 20/23 forced
//! mates and PN at 23/23 at budget 256, and depth-one is a different algorithm entirely. If the
//! surrogate cannot rank programs THIS different in the order games rank them, it cannot rank
//! anything.
//!
//! DECLARED BEFORE RUNNING:
//!   * Spearman clearly positive -> the surrogate orders real programs correctly, and its failure
//!     on mutants is a resolution limit, not a validity one. Keep it; fix the gate width instead.
//!   * Spearman ~0 or negative -> it does not order even structurally different programs. Then
//!     MASTER_PLAN P2's "fitness is wrong" is established on the cleanest evidence available, and
//!     no amount of gate width rescues it.
//!
//! COST NOTE: a round robin is n(n-1)/2 matches. With the default subset that is 10 matches; --all
//! runs all 45. Both sides get the same budget so a slow paradigm is not also handicapped.

use board::{Outcome, Position};
use grammar::reference;
use interp::Interp;
use nnue::Net;
use pipeline::gate;

fn arg<T: std::str::FromStr>(name: &str, d: T) -> T {
    let a: Vec<String> = std::env::args().collect();
    a.iter().position(|x| x == name).and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(d)
}

/// Mine MATE-1 positions exactly as ladder.rs does -- a position from which some legal move ends
/// the game as a win. Same construction deliberately: a lookalike set would make the surrogate
/// here a different number from the one the search optimises.
fn mate_set(n: usize, seed: u64) -> Vec<Position> {
    let mut rng = seed | 1;
    let mut out = Vec::new();
    while out.len() < n {
        let mut p = Position::startpos();
        for _ in 0..60 {
            let l = p.legal_moves();
            if l.is_empty() { break; }
            let mut winning = false;
            for &m in l.as_slice() {
                let u = p.make_move(m);
                if p.legal_moves().is_empty() && p.outcome() == Outcome::Loss { winning = true; }
                p.unmake_move(m, u);
                if winning { break; }
            }
            if winning { out.push(p.clone()); break; }
            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
            let m = l.as_slice()[(rng % l.len() as u64) as usize];
            p.make_move(m);
        }
    }
    out
}

/// MATE-IN-TWO set: positions with NO mate in one, where some move FORCES mate next turn.
///
/// Copied from `interp/examples/mate_surrogate_probe.rs` (mate_in_two_set / score_forcing) rather
/// than reinvented -- that code is already the project's verified definition, and a lookalike set
/// would silently be measuring something else.
///
/// WHY IT MATTERS HERE. On the mate-1 set, 4 of 5 programs score 60/60: the surrogate's numerator
/// SATURATES, so mates/Mcost degenerates into a pure cost race and ranks the strongest program
/// last (surrogate_inverts_RESULT.md). Rejecting positions that have a mate in one is exactly what
/// stops depth-1 from solving everything, which is what restores variance to the numerator.
/// ladder.rs measured the difference directly: mate-in-1 depth1 80/80, but mate-in-2 depth1 17/40
/// against depth2 40/40.
fn mate_in_two_set(n: usize, cap: usize, seed: u64) -> Vec<(Position, board::Move)> {
    let mut rng: u64 = seed | 1;
    let mut out = Vec::new();
    let mut tries = 0;
    while out.len() < n && tries < cap {
        tries += 1;
        let mut p = Position::startpos();
        for _ in 0..(10 + rng % 40) {
            let l = p.legal_moves();
            if l.is_empty() { break; }
            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
            p.make_move(l.as_slice()[(rng % l.len() as u64) as usize]);
        }
        if p.legal_moves().is_empty() { continue; }
        let mut has_m1 = false;
        for &m in p.legal_moves().as_slice() {
            let u = p.make_move(m);
            if p.legal_moves().is_empty() && p.outcome() == Outcome::Loss { has_m1 = true; }
            p.unmake_move(m, u);
            if has_m1 { break; }
        }
        if has_m1 { continue; }
        let moves: Vec<_> = p.legal_moves().as_slice().to_vec();
        for m in moves {
            let u = p.make_move(m);
            let replies: Vec<_> = p.legal_moves().as_slice().to_vec();
            let mut all_lose = !replies.is_empty();
            for r in replies {
                let u2 = p.make_move(r);
                let mut mates = false;
                for &m2 in p.legal_moves().as_slice() {
                    let u3 = p.make_move(m2);
                    if p.legal_moves().is_empty() && p.outcome() == Outcome::Loss { mates = true; }
                    p.unmake_move(m2, u3);
                    if mates { break; }
                }
                p.unmake_move(r, u2);
                if !mates { all_lose = false; break; }
            }
            p.unmake_move(m, u);
            if all_lose { out.push((p.clone(), m)); break; }
        }
    }
    out
}

fn main() {
    let pairs: usize = arg("--pairs", 24);
    let budget: i64 = arg("--budget", 256);
    let nmates: usize = arg("--mates", 60);
    let seed: u64 = arg("--seed", 20260907);
    // COST CEILING PER MOVE. u64::MAX = "no ceiling", which is what evolve.rs uses
    // (COST_PER_MOVE at evolve.rs:2790).
    //
    // I passed 0 first, and it silently destroyed the whole experiment. Interp::cost_cap defaults
    // to 2e9; setting it to 0 makes `cost >= cost_cap` true on the FIRST node, so every program
    // returns MOVE_NONE on every move. A forfeit is scored Some(!is_a) -- decisive, not a draw --
    // so playing both colours gave one win and one loss per pair, pair score 2, and EXACTLY 0.500
    // with zero variance in all ten matches. Depth-one "drew" with alpha-beta; the ranking was
    // pure artefact.
    let cost_cap: u64 = arg("--cost-cap", u64::MAX);
    let all = std::env::args().any(|x| x == "--all");

    let net = Net::random(32, seed);
    let tables: Vec<i64> = vec![0, 32_000, 3, 16, 2, 1, 8, 4, 0, 0, 0, 0, 0, 0, 0, 0];

    // A subset by default: one program per paradigm, which is where the ranking question lives.
    let keep: Vec<&str> = vec![
        "depth-one (purity seed)",
        "bare alpha-beta (main seed)",
        "alpha-beta + hash reuse",
        "alpha-beta + hash + ID",
        "capture extension (rung 6)",
    ];
    let progs: Vec<(&'static str, grammar::ast::Program)> = reference::all()
        .into_iter()
        .filter(|(n, _)| all || keep.contains(n))
        .collect();

    println!("surrogate_vs_games: {} programs, {pairs} pairs/match, budget {budget}, {nmates} mate-1 positions",
             progs.len());
    println!("  the SAME net and budget on both sides of every match, so this ranks PROGRAMS\n");

    // ---- 1. the surrogate: FITNESS 3 mates-per-cost -------------------------------------------
    let hard = std::env::args().any(|x| x == "--hard");
    let (set, hard_set) = if hard {
        let h = mate_in_two_set(nmates, 400_000, seed);
        println!("  MATE-2 set: {} positions (no mate-in-1 exists, so depth 1 cannot solve them)\n", h.len());
        (Vec::new(), h)
    } else {
        let m = mate_set(nmates, seed);
        println!("  mate-1 set: {} positions\n", m.len());
        (m, Vec::new())
    };
    // Scoring loop copied from ladder.rs rather than reinvented: same Interp construction, same
    // `it.run(prog, pos, budget)`, same mate test. A lookalike would make the number here a
    // different metric from the one the search optimises, which is the whole point of comparing it.
    let mut sur: Vec<(usize, f64, u32, u64)> = Vec::new();
    for (i, (name, prog)) in progs.iter().enumerate() {
        let mut it = Interp::new(&net, tables.clone());
        let (mut found, mut cost) = (0u32, 0u64);
        let total = if hard { hard_set.len() } else { set.len() };
        if hard {
            // Credit the FORCING move, not a mate on this ply -- mate_surrogate_probe.rs's rule.
            for (p, best) in &hard_set {
                let mv = it.run(prog, p, budget);
                cost += it.cost;
                if mv == *best { found += 1; }
            }
        } else {
            for p in &set {
                let mv = it.run(prog, p, budget);
                cost += it.cost;
                if mv != board::types::MOVE_NONE {
                    let mut q = p.clone();
                    if let Some(m) = q.legal_moves().as_slice().iter().copied().find(|x| *x == mv) {
                        q.make_move(m);
                        if q.legal_moves().is_empty() && q.outcome() == Outcome::Loss { found += 1; }
                    }
                }
            }
        }
        let rate = found as f64 / (cost as f64 / 1e6).max(1e-9);
        println!("  {:<46} {:>3}/{:<3} solved cost {:>13}  {:.6} per Mcost", name, found, total, cost, rate);
        sur.push((i, rate, found, cost));
    }

    // ---- 2. the truth: round-robin games -------------------------------------------------------
    println!("\n  round robin ({} matches):", progs.len() * (progs.len() - 1) / 2);
    // The harness counts forfeits precisely so a caller can refuse to believe a score built on
    // them -- gate.rs:355 says "the caller has to be able to see how many of these happened before
    // believing the score". Not reading it is how the 0.500 artefact above survived a whole run.
    gate::FORFEITS.store(0, std::sync::atomic::Ordering::Relaxed);
    let mut pts = vec![0.0f64; progs.len()];
    let mut played = vec![0.0f64; progs.len()];
    for i in 0..progs.len() {
        for j in (i + 1)..progs.len() {
            let s = gate::match_progs(&progs[i].1, &progs[j].1, &net, tables.clone(), budget,
                                      pairs, seed ^ ((i * 31 + j) as u64), 4, cost_cap);
            let r = s.pent_rate();
            println!("    {:<30} vs {:<30} {:.3}", short(progs[i].0), short(progs[j].0), r);
            pts[i] += r; played[i] += 1.0;
            pts[j] += 1.0 - r; played[j] += 1.0;
        }
    }

    let forfeits = gate::FORFEITS.load(std::sync::atomic::Ordering::Relaxed);
    let games = (progs.len() * (progs.len() - 1) / 2 * pairs * 2) as u64;
    println!("\n  forfeits: {forfeits} of {games} games ({:.1}%)", 100.0 * forfeits as f64 / games as f64);
    if forfeits * 5 > games {
        println!("\n  ABORT: more than 20% of games ended in a FORFEIT, so this measures the cost");
        println!("  ceiling rather than the programs. Raise --cost-cap. No ranking is reported --");
        println!("  a symmetric forfeit produces exactly 0.500 with zero variance, which reads as a");
        println!("  confident tie and is nothing of the kind.");
        return;
    }

    // ---- 3. compare the two orderings ----------------------------------------------------------
    let game: Vec<f64> = pts.iter().zip(&played).map(|(p, n)| p / n.max(1.0)).collect();
    println!("\n  {:<46} {:>12} {:>12}", "program", "surrogate", "game score");
    let mut rows: Vec<(usize, f64, f64)> = (0..progs.len()).map(|i| (i, sur[i].1, game[i])).collect();
    rows.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
    for (i, s, g) in &rows {
        println!("  {:<46} {:>12.6} {:>12.3}", progs[*i].0, s, g);
    }

    let rho = spearman(&rows.iter().map(|r| r.1).collect::<Vec<_>>(),
                       &rows.iter().map(|r| r.2).collect::<Vec<_>>());
    println!("\n  === SPEARMAN rank correlation, surrogate vs games: {:+.3} (n={}) ===", rho, rows.len());
    println!();
    if rho > 0.6 {
        println!("  VERDICT: the surrogate ORDERS real programs the way games do. Its failure on");
        println!("  mutants is then a RESOLUTION limit, not a validity one -- keep the metric and");
        println!("  widen the gate instead.");
    } else if rho < 0.2 {
        println!("  VERDICT: the surrogate does NOT order even structurally different programs the");
        println!("  way games do. MASTER_PLAN P2's 'fitness is wrong' is established on the cleanest");
        println!("  evidence this project can produce, and no gate width fixes it.");
    } else {
        println!("  PARTIAL: the orderings agree weakly. With n={} programs this cannot separate a", rows.len());
        println!("  real weak relationship from noise -- run --all for the full ladder before deciding.");
    }
}

fn short(s: &str) -> String { s.chars().take(30).collect() }

fn spearman(a: &[f64], b: &[f64]) -> f64 {
    let rank = |v: &[f64]| -> Vec<f64> {
        let mut idx: Vec<usize> = (0..v.len()).collect();
        idx.sort_by(|&x, &y| v[x].partial_cmp(&v[y]).unwrap());
        let mut r = vec![0.0; v.len()];
        for (pos, &i) in idx.iter().enumerate() { r[i] = pos as f64; }
        r
    };
    let (ra, rb) = (rank(a), rank(b));
    let n = a.len() as f64;
    let (ma, mb) = (ra.iter().sum::<f64>() / n, rb.iter().sum::<f64>() / n);
    let num: f64 = ra.iter().zip(&rb).map(|(x, y)| (x - ma) * (y - mb)).sum();
    let da: f64 = ra.iter().map(|x| (x - ma).powi(2)).sum::<f64>().sqrt();
    let db: f64 = rb.iter().map(|y| (y - mb).powi(2)).sum::<f64>().sqrt();
    if da == 0.0 || db == 0.0 { return 0.0; }
    num / (da * db)
}
