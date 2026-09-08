//! GRAMMAR 9's ladder check, and FITNESS 3's mates-per-cost metric it depends on.
//!
//! Mine MATE-1 positions by retrograde walk from real self-play terminals (rules-derived: a
//! position is MATE-1 iff some legal move ends the game as a win). Then run each rung of the
//! ladder over the same set and report mates found per unit of COST.
//!
//! Cost, not evaluations: proof-number search calls `eval` nowhere, so a per-eval denominator
//! would divide by zero and let an eval-free prover dominate the metric (FITNESS 3).
use board::{Outcome, Position};
use grammar::reference;
use interp::Interp;
use nnue::Net;

fn main() {
    let net = Net::random(32, 20260907);
    let mut rng: u64 = 0x1234_5678;
    let mut mates: Vec<Position> = Vec::new();

    // Random legal walks; keep any position from which a single move wins.
    while mates.len() < 120 {
        let mut p = Position::startpos();
        for _ in 0..60 {
            let l = p.legal_moves();
            if l.is_empty() { break; }
            // is this position mate-in-1 for the side to move?
            let mut winning = false;
            for &m in l.as_slice() {
                let u = p.make_move(m);
                if p.legal_moves().is_empty() && p.outcome() == Outcome::Loss { winning = true; }
                p.unmake_move(m, u);
                if winning { break; }
            }
            if winning { mates.push(p.clone()); break; }
            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
            let m = l.as_slice()[(rng % l.len() as u64) as usize];
            p.make_move(m);
        }
    }
    println!("  MATE-1 set: {} positions (retrograde from random legal walks)", mates.len());

    // DEPTH SWEEP, not a single depth. At D=2 the metric cannot evaluate steps 4-7 at all:
    // iteration 1 of iterative deepening stores entries at depth 1, iteration 2 probes needing
    // depth >= 2 and rejects every one, so a transposition table has literally no reuse to find
    // and contributes only probe/store cost. Measuring hash reuse there and calling it a LOSS
    // says nothing about hash reuse -- it says the test was too shallow to contain the effect.
    // Two hypotheses died to learn that (TT alone should gain; ID should give the TT traffic),
    // which is the signal that the harness is the thing to doubt.
    let depths: Vec<i64> = std::env::args().skip(1).filter_map(|a| a.parse().ok()).collect();
    let depths = if depths.is_empty() { vec![2, 3, 4] } else { depths };
    for depth in depths {
    println!("\n  === D = {depth} ===");
    println!("  {:<32} {:>7} {:>14} {:>13} {:>10}",
             "program", "mates", "cost", "mates/Mcost", "vs seed");
    // The seed's cost at this depth is the denominator that matters: "1.22x the seed" is
    // legible where "0.2 vs 0.1 mates/Mcost" rounds the whole effect away.
    let mut seed_cost = 0u64;
    for (name, prog) in reference::all() {
        let mut it = Interp::new(&net, vec![depth, 32_000, 8]);
        let (mut found, mut cost) = (0u32, 0u64);
        for p in &mates {
            let mv = it.run(&prog, p, 16);
            cost += it.cost;
            if mv != board::types::MOVE_NONE {
                let mut q = p.clone();
                if let Some(m) = q.legal_moves().as_slice().iter().copied().find(|x| *x == mv) {
                    q.make_move(m);
                    if q.legal_moves().is_empty() && q.outcome() == Outcome::Loss { found += 1; }
                }
            }
        }
        if name.starts_with("bare alpha-beta") { seed_cost = cost; }
        let vs = if seed_cost > 0 && cost > 0 {
            format!("{:.2}x", cost as f64 / seed_cost as f64)
        } else { "-".into() };
        println!("  {:<32} {:>7} {:>14} {:>13.3} {:>10}", name, found, cost,
            found as f64 * 1e6 / cost.max(1) as f64, vs);
    }
    }
    println!("\n  a rung must BEAT the previous one on this metric (GRAMMAR 9)");
}
