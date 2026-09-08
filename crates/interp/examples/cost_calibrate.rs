//! CALIBRATE THE COST MODEL. GRAMMAR 8 specifies "a declared per-primitive cost (cycles
//! estimate) table. The interpreter accumulates it at runtime; that running sum IS the budget
//! unit." CRATE 4 names the file: `configs/cost.toml`.
//!
//! It was never built. `crates/interp` charges `self.cost += 1` per node, so a full NNUE
//! forward pass costs exactly what `const 3` costs.
//!
//! THAT IS NOT A NEUTRAL SIMPLIFICATION. It is a thumb on the scale against any program that
//! trades cheap work for expensive work, which is precisely what a transposition table does:
//! it spends probes (cheap) to skip evals (expensive). Under a flat model the probe and the
//! eval are priced the same, so the trade can only ever look like a loss. Every mates-per-cost
//! number in GRAMMAR 9 -- including today's "hash reuse is a 25% loss, break-even near depth
//! 5-6" -- was computed under that model and has to be re-derived once this lands.
//!
//! Measured, not guessed: each primitive's underlying Rust operation is timed on real
//! positions, and costs are reported RELATIVE to the cheapest (an integer add = 1). Relative
//! is the right form -- the budget only needs the ratios, and ratios survive a change of CPU
//! far better than absolute nanoseconds.

use board::Position;
use nnue::Net;
use std::time::Instant;

fn bench<F: FnMut()>(iters: usize, mut f: F) -> f64 {
    // Warm up, then take the MINIMUM of several passes: timing noise is one-sided (something
    // else stole the core), so the min is the closest estimate of the true cost.
    for _ in 0..iters / 10 + 1 { f(); }
    let mut best = f64::INFINITY;
    for _ in 0..5 {
        let t = Instant::now();
        for _ in 0..iters { f(); }
        let ns = t.elapsed().as_nanos() as f64 / iters as f64;
        if ns < best { best = ns; }
    }
    best
}

fn main() {
    let width: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(32);
    let net = Net::random(width, 7);

    // Real positions from random walks, not startpos: movegen and eval both scale with how
    // busy the board is, and startpos is not representative of a search's interior.
    let mut rng: u64 = 0xC0FFEE;
    let mut rnd = || { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; rng };
    let mut ps = Vec::new();
    while ps.len() < 64 {
        let mut p = Position::startpos();
        for _ in 0..(6 + rnd() % 24) {
            let l = p.legal_moves();
            if l.is_empty() { break; }
            p.make_move(l.as_slice()[(rnd() % l.len() as u64) as usize]);
        }
        if !p.legal_moves().is_empty() { ps.push(p); }
    }

    let mut i = 0usize;
    let mut next = || { i = (i + 1) % 64; i };

    let mut acc = 0i64;
    let t_arith = bench(200_000, || { acc = acc.wrapping_mul(3).wrapping_add(1); });
    std::hint::black_box(acc);

    let mut scratch = Vec::new();
    let t_eval_scratch = bench(2_000, || { std::hint::black_box(net.eval(&ps[next()], &mut scratch)); });

    // The INTERPRETER no longer evaluates from scratch: `apply` carries the accumulator forward
    // and `eval` is the output layer alone. Those are the costs the model must charge, because
    // they are what evolved programs actually pay.
    let nodes: Vec<interp::PosAcc> =
        ps.iter().map(|p| interp::PosAcc::fresh(&net, p.clone())).collect();
    let t_eval = bench(20_000, || { std::hint::black_box(nodes[next()].score(&net)); });
    let mut fb = interp::Delta::new();
    let t_apply_inc = bench(20_000, || {
        let n = &nodes[next()];
        let l = n.pos.legal_moves();
        if !l.is_empty() { std::hint::black_box(n.child(&net, l.as_slice()[0], &mut fb)); }
    });
    let t_moves = bench(20_000, || { std::hint::black_box(ps[next()].legal_moves().len()); });
    let t_key = bench(50_000, || { std::hint::black_box(ps[next()].zobrist()); });

    let t_apply = bench(20_000, || {
        let mut p = ps[next()].clone();
        let l = p.legal_moves();
        if !l.is_empty() { let m = l.as_slice()[0]; let u = p.make_move(m); p.unmake_move(m, u); }
    });
    let t_terminal = bench(20_000, || {
        let p = &ps[next()];
        std::hint::black_box(p.legal_moves().is_empty());
    });

    let unit = t_arith.max(1e-3);
    println!("# configs/cost.toml — MEASURED per-primitive cost, GRAMMAR 8");
    println!("# Relative to one integer arithmetic op = 1. Net width {width}.");
    println!("# Regenerate: cargo run --release --example cost_calibrate -p interp -- <width>");
    println!();
    let mut row = |name: &str, ns: f64| {
        println!("{name:<12} = {:>7}    # {:.1} ns", (ns / unit).round().max(1.0) as u64, ns);
    };
    row("arith", t_arith);
    row("cmp", t_arith);
    row("const", t_arith);
    row("var", t_arith);
    row("eval", t_eval);
    println!("# eval from-scratch was {:.1} ns; incremental output layer is {:.1} ns ({:.0}x)",
             t_eval_scratch, t_eval, t_eval_scratch / t_eval.max(1e-9));
    row("moves", t_moves);
    row("apply", t_apply_inc);
    println!("#   (bare make/unmake was {:.1} ns; apply now also carries the accumulator)", t_apply);
    row("terminal", t_terminal);
    row("key", t_key);
    println!();
    println!("# The number that matters for the ladder: eval / (probe-ish cheap op) = {:.0}x.",
             t_eval / unit);
    println!("# Under the current FLAT model that ratio is 1, so a transposition table's whole");
    println!("# trade -- spend a cheap probe, skip an expensive eval -- is priced at zero gain.");
}
