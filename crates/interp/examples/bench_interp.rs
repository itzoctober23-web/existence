//! THE acceptance measurement of GRAMMAR 8 / CRATE 4: the compiled seed must reach >= 50% of
//! the NPS of a hand-written Rust bare alpha-beta with the SAME net. Below that, the
//! evaluation model is revisited before anything is built on it.
//!
//! Stage 1 measures a TREE-WALKER, so this ratio is a LOWER BOUND on the design.

use board::Position;
use grammar::reference;
use interp::Interp;
use nnue::Net;
use std::time::Instant;

// Hand-written reference: identical semantics to the seed program.
fn ab(pos: &mut Position, depth: i32, mut alpha: i32, beta: i32, net: &Net, s: &mut Vec<f32>, nodes: &mut u64) -> i32 {
    *nodes += 1;
    let list = pos.legal_moves();
    if list.is_empty() {
        return match pos.outcome() {
            board::Outcome::Loss => -30_000 + (64 - depth),
            _ => 0,
        };
    }
    if depth == 0 {
        return net.eval(pos, s);
    }
    let mut best = -32_000;
    for &m in list.as_slice() {
        let u = pos.make_move(m);
        let v = -ab(pos, depth - 1, -beta, -alpha, net, s, nodes);
        pos.unmake_move(m, u);
        if v > best { best = v; }
        if best > alpha { alpha = best; }
        if alpha >= beta { break; }
    }
    best
}

fn main() {
    let depth: i64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(3);
    let net = Net::random(256, 0xE1_57_E0_1C);
    let mut pos = Position::startpos();

    // Reference
    let mut s = Vec::new();
    let mut nodes = 0u64;
    let t0 = Instant::now();
    let _ = ab(&mut pos, depth as i32, -32_000, 32_000, &net, &mut s, &mut nodes);
    let t_ref = t0.elapsed().as_secs_f64();
    let nps_ref = nodes as f64 / t_ref;

    // Interpreted seed. Tables: 0 = D (depth), 1 = INF.
    let prog = reference::bare_alpha_beta();
    let mut it = Interp::new(&net, vec![depth, 32_000]);
    let t1 = Instant::now();
    let mv = it.run(&prog, &pos, 0);
    let t_int = t1.elapsed().as_secs_f64();
    let nps_int = it.cost as f64 / t_int;

    println!("  depth {depth}");
    println!("  hand-written    {:>12} nodes  {:>8.3}s  {:>12.0} nps", nodes, t_ref, nps_ref);
    println!("  interpreted     {:>12} cost   {:>8.3}s  {:>12.0} cost/s   best={}", it.cost, t_int, nps_int, mv);
    println!();
    println!("  wall-clock ratio (interp/hand): {:.3}x   acceptance >= 0.50", t_ref / t_int);
    println!("  NOTE: stage-1 tree-walker; bytecode is the target, so this is a LOWER BOUND.");
}
