//! DOES NARROWING THE ALPHA-BETA WINDOW FAKE AN IMPROVEMENT ON THIS SURROGATE?
//!
//! The search track accepted two candidates at D=3 that kept 20/20 mates while costing ~12x less.
//! I judged them non-degenerate because the mate count held, INCLUDING the forced-mate-in-2
//! positions that a shallow program loses first. That reasoning has a hole: the mate-in-2 guard
//! catches SHALLOWNESS, not WINDOW NARROWING.
//!
//! Mate scores are +/-30000. A program that shrinks its initial (alpha, beta) window prunes
//! enormously and still finds every mate, because a mate is far outside any narrow window. It
//! would play terribly in normal positions, where the differences are tens of centipawns -- and
//! this surrogate contains no normal positions, so it cannot see that.
//!
//! Test on the UNMUTATED SEED, no mutation involved: sweep INF (table 1, the initial window) and
//! measure mates and cost. If narrowing it reproduces a large cost drop at 20/20 mates, then the
//! surrogate cannot distinguish that failure from a real improvement, and the search track's first
//! success needs re-examining rather than celebrating.
use board::{Outcome, Position};
use grammar::reference;
use interp::Interp;
use nnue::Net;

fn mate_set(n: usize) -> Vec<Position> {
    let mut rng: u64 = 0xC0DE_F00D;
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
            p.make_move(l.as_slice()[(rng % l.len() as u64) as usize]);
        }
    }
    out
}

fn main() {
    let depth: i64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(3);
    let net = Net::random(32, 20260907);
    let ab = reference::bare_alpha_beta();
    let set = mate_set(15);

    println!("bare alpha-beta (UNMUTATED), depth {depth}, {} mate-in-1 positions", set.len());
    println!("  {:>8} {:>7} {:>16} {:>14} {:>10}", "INF", "mates", "cost", "mates/Mcost", "vs 32000");
    let mut base = 0u64;
    for inf in [32_000i64, 8_000, 1_000, 200, 50] {
        let mut it = Interp::new(&net, vec![depth, inf, 8]);
        let (mut found, mut cost) = (0u32, 0u64);
        for p in &set {
            let mv = it.run(&ab, p, 16);
            cost += it.cost;
            if mv != board::types::MOVE_NONE {
                let mut q = p.clone();
                if let Some(m) = q.legal_moves().as_slice().iter().copied().find(|x| *x == mv) {
                    q.make_move(m);
                    if q.legal_moves().is_empty() && q.outcome() == Outcome::Loss { found += 1; }
                }
            }
        }
        if inf == 32_000 { base = cost; }
        let rate = found as f64 * 1e6 / cost.max(1) as f64;
        let ratio = if base > 0 { cost as f64 / base as f64 } else { 1.0 };
        println!("  {inf:>8} {found:>7} {cost:>16} {rate:>14.6} {ratio:>9.3}x");
    }
    println!("\n  If mates stay at {} while cost collapses, the window is a free lunch ON THIS SET", set.len());
    println!("  and the surrogate cannot tell it from a real search improvement. A program that");
    println!("  narrows its window still finds mates (they score +/-30000, outside any window) and");
    println!("  would still play badly wherever the difference is tens of centipawns -- which is");
    println!("  every position this set does not contain.");
}
