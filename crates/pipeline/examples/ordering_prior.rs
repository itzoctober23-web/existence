//! HOW BIG WAS THE UNDECLARED MOVE-ORDERING PRIOR?
//!
//! MASTER_PLAN "Iteration zero" declares the seed program searches "children in emission order
//! (SHUFFLED)". `crates/pipeline/src/search.rs` -- the searcher that produces every datagen
//! label and runs every gate -- did not shuffle, so it inherited the movegen's emission order.
//! The movegen emits by piece type with `gen_pawns` first, which is a move-ordering heuristic:
//! alpha-beta cuts off sooner when good moves come first, and "pawn moves first" is a real,
//! if crude, opinion about which moves are good.
//!
//! Move ordering is on the DISCOVERY list precisely because it is worth a lot. So the question
//! is not rhetorical: how many nodes was the unshuffled search saving? That number IS the size
//! of the prior that was sitting in the Given column undeclared.
//!
//! Both arms search the SAME positions to the SAME depth with the SAME net, so any node-count
//! difference is ordering and nothing else. (Traps file: "before believing ANY ratio, prove
//! both arms did the same work" -- here the work is defined by depth and position, and the
//! returned SCORES are asserted equal, which is alpha-beta's soundness guarantee: ordering
//! changes the node count, never the value.)

use board::Position;
use nnue::Net;
use pipeline::datagen::Rng;
use pipeline::search::Searcher;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let net = match args.iter().position(|a| a == "--net").and_then(|i| args.get(i + 1)) {
        Some(p) => Net::load(p).unwrap_or_else(|_| Net::random(256, 7)),
        None => Net::random(256, 7),
    };
    let depth: u32 = args.iter().position(|a| a == "--depth")
        .and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(4);
    let n_pos: usize = args.iter().position(|a| a == "--positions")
        .and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(200);

    // Positions from random walks, so this is not one cherry-picked opening.
    let mut rng = Rng(0xABCDEF);
    let mut positions = Vec::new();
    while positions.len() < n_pos {
        let mut p = Position::startpos();
        let plies = 4 + rng.below(20);
        let mut ok = true;
        for _ in 0..plies {
            let l = p.legal_moves();
            if l.is_empty() { ok = false; break; }
            p.make_move(l.as_slice()[rng.below(l.len())]);
        }
        if ok && !p.legal_moves().is_empty() { positions.push(p); }
    }

    let (mut n_shuf, mut n_emit, mut mismatches) = (0u64, 0u64, 0usize);
    for (i, p0) in positions.iter().enumerate() {
        let mut a = Searcher::with_seed(0x1234 + i as u64);
        let mut p = p0.clone();
        let (_, s_shuf) = a.best_move(&mut p, depth, &net);
        n_shuf += a.nodes;

        let mut b = Searcher::new();
        b.shuffle_children = false;
        let mut p = p0.clone();
        let (_, s_emit) = b.best_move(&mut p, depth, &net);
        n_emit += b.nodes;

        // Alpha-beta returns the minimax value regardless of ordering. If these disagree, the
        // difference is a BUG, not an ordering effect, and the node ratio below is meaningless.
        if s_shuf != s_emit { mismatches += 1; }
    }

    println!("positions {n_pos}  depth {depth}  hidden {}", net.n_hidden);
    println!("score mismatches: {mismatches}  (must be 0 -- ordering changes cost, never value)");
    println!("nodes, emission order (the old behaviour): {n_emit}");
    println!("nodes, shuffled     (what the plan declares): {n_shuf}");
    let ratio = n_shuf as f64 / n_emit.max(1) as f64;
    println!("shuffled / emission = {ratio:.3}x");
    if ratio > 1.02 {
        println!("=> emission order searched {:.1}% FEWER nodes. That saving was an undeclared\n\
                  move-ordering heuristic the seed program is not supposed to have.",
                 100.0 * (1.0 - 1.0 / ratio));
    } else if ratio < 0.98 {
        println!("=> emission order searched {:.1}% MORE nodes: it is an ANTI-ordering, worse\n\
                  than random. Pawn moves are emitted first and are rarely best, so trying them\n\
                  first delays every cutoff. The prior was real but pointed the other way -- it\n\
                  was taxing the search, not helping it.",
                 100.0 * (1.0 / ratio - 1.0));
    } else {
        println!("=> no measurable difference: emission order carried no ordering signal here.");
    }
}
