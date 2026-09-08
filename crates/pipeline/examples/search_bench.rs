//! Search throughput. Reported as nodes/sec at fixed depth from a fixed position set, so the
//! before/after of any eval change is comparable. Node counts must MATCH across variants --
//! a speed number for two different trees is meaningless (that mistake cost three stacked
//! harness bugs in bench_interp).
use board::Position;
use nnue::Net;
use pipeline::search::Searcher;
use std::time::Instant;

fn main() {
    let depth: u32 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(4);
    let hidden: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(32);
    let net = Net::random(hidden, 20260907);
    let fens = [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
    ];
    let mut total_nodes = 0u64;
    let t0 = Instant::now();
    for f in fens {
        let mut p = Position::from_fen(f).unwrap();
        let mut s = Searcher::new();
        let _ = s.best_move(&mut p, depth, &net);
        total_nodes += s.nodes;
    }
    let el = t0.elapsed().as_secs_f64();
    println!("  hidden {hidden:>4}  {:<9}  nodes {total_nodes}  {:.3}s  {:>9.0} nps", if std::env::var("EXISTENCE_FULL_REFRESH").is_ok() {"refresh"} else {"incr"}, el, total_nodes as f64 / el);
}
