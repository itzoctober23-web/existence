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
        REF_EVALS.with(|c| c.set(c.get() + 1));
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

use std::cell::Cell;
thread_local!(static REF_EVALS: Cell<u64> = const { Cell::new(0) });
fn net_evals_ref() -> u64 { REF_EVALS.with(|c| c.get()) }

fn main() {
    let depth: i64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(3);
    let net = Net::random(256, 0xE1_57_E0_1C);
    let mut pos = Position::startpos();

    // Reference. It must mirror the SEED's root exactly: `choose` is
    //   argmax(moves(p), m -> neg(ab(apply(p,m), D, neg(INF), INF)))
    // i.e. every root move is searched with a FULL window, with no alpha narrowing between
    // root moves. Narrowing here (the natural thing to write) is a different algorithm and
    // makes the arms search different trees -- caught by the eval-count check as 1361 vs 2099.
    // REPEAT AND TAKE THE MEDIAN. A single depth-3 pass is ~3,700 nodes and ~14 ms, which is far
    // too short to produce a stable ratio on a loaded box. Measured 2026-09-10, six consecutive runs
    // of the ONE-SHOT version, identical binary and data, equivalence check passing every time:
    //
    //     0.779  1.298  1.151  0.965  0.951  0.890      -> a 1.67x spread
    //
    // The gate (>= 0.50) is coarse enough to survive that, but the HEADLINE FIGURE was not: "the
    // interpreter gate PASSED 0.98x" quotes three digits the instrument cannot resolve, and any later
    // comparison against 0.98 -- after the register bytecode, say -- would have been comparing noise.
    // So the ratio is now a median over reps with the spread printed beside it, and a run whose
    // spread is wide says so instead of returning one lucky number.
    let reps: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(9);
    let mut ratios: Vec<f64> = Vec::with_capacity(reps);
    let (mut nodes, mut t_ref, mut t_int, mut cost, mut ev_i, mut ev_r) = (0u64, 0.0, 0.0, 0u64, 0u64, 0u64);
    let mut mv = board::types::MOVE_NONE;
    for _ in 0..reps {
        let mut s = Vec::new();
        let ev_r0 = net_evals_ref();
        nodes = 0;
        let t0 = Instant::now();
        {
            let list = pos.legal_moves();
            let mut best = -32_000;
            for &m in list.as_slice() {
                let u = pos.make_move(m);
                let v = -ab(&mut pos, depth as i32 - 1, -32_000, 32_000, &net, &mut s, &mut nodes);
                pos.unmake_move(m, u);
                if v > best { best = v; }
            }
        }
        t_ref = t0.elapsed().as_secs_f64();
        ev_r = net_evals_ref() - ev_r0;

        let prog = reference::bare_alpha_beta();
        let mut it = Interp::new(&net, vec![(depth - 1).max(0), 32_000]);
        let t1 = Instant::now();
        mv = it.run(&prog, &pos, 0);
        t_int = t1.elapsed().as_secs_f64();
        cost = it.cost;
        ev_i = it.evals;
        ratios.push(t_ref / t_int);
    }
    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let med = ratios[ratios.len() / 2];
    let (lo, hi) = (ratios[0], ratios[ratios.len() - 1]);

    println!("  depth {depth}   reps {reps}");
    println!("  hand-written    {:>12} nodes  {:>8.4}s  {:>12.0} nps", nodes, t_ref, nodes as f64 / t_ref);
    println!("  interpreted     {:>12} cost   {:>8.4}s  {:>12.0} cost/s   best={}", cost, t_int, cost as f64 / t_int, mv);
    println!();
    println!("  EQUIVALENCE CHECK  evals: hand {} vs interp {}  -> {}", ev_r, ev_i,
        if ev_r == ev_i { "same tree" } else { "DIFFERENT TREES - ratio is meaningless" });
    println!("  wall-clock ratio (interp/hand): MEDIAN {:.3}x  [min {:.3}, max {:.3}]  acceptance >= 0.50", med, lo, hi);
    if hi / lo > 1.25 {
        println!("  ⚠ SPREAD {:.2}x ACROSS REPS -- the box is contended or the run is too short.", hi / lo);
        println!("    The PASS/FAIL is still sound (the bound is 0.50); the third digit is not. Do not");
        println!("    quote this median as a precise figure or compare it to another one digit by digit.");
    }
    println!("  NOTE: stage-1 tree-walker; bytecode is the target, so this is a LOWER BOUND.");
}
