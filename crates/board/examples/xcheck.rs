//! Movegen cross-check against an external engine. The logic lives in `board::xcheck` so this
//! example and `tests/xcheck.rs` run IDENTICAL code.
//!
//! CRATE.md 2 requires BOTH halves of movegen verification:
//!   (a) a frozen perft fixture — `tests/perft.rs`
//!   (b) random full games walked to terminal, comparing the LEGAL MOVE SET at every ply
//!
//! Usage: cargo run --release --example xcheck -- [games] [max_plies]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let games: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(200);
    let max_plies: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(300);
    let path = board::xcheck::engine_path();

    let r = match board::xcheck::cross_check(&path, games, max_plies, 0x2026_09_07_1234_5678,
                                             |s| println!("{s}")) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("could not start reference engine '{path}': {e}");
            eprintln!("set XCHECK_ENGINE to its path");
            std::process::exit(2);
        }
    };

    for d in &r.detail {
        println!("DIVERGENCE {d}");
    }
    println!("\n{} games, {} plies compared, {} divergences", r.games, r.plies, r.divergences);
    println!(
        "terminals reached: ongoing(ply-cap) {}, checkmate {}, stalemate {}",
        r.terminals[0], r.terminals[1], r.terminals[2]
    );
    if r.divergences == 0 {
        println!("PASS — the move generators agree on every position visited.");
    } else {
        println!("FAIL — {} disagreement(s).", r.divergences);
        std::process::exit(1);
    }
}
