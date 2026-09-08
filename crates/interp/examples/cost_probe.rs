//! WHY DOES ONE FITNESS EVALUATION COST 4e8 UNITS AT DEPTH 3?
//!
//! The search track's generation time is ~100 s for 12 candidates over 25 positions. Working
//! backwards: the seed costs 9,930,290,911 units for 25 positions, so ~4e8 per position, and at
//! 1365 units per eval that is ~290,000 evals for a DEPTH 3 SEARCH. A depth-3 alpha-beta over ~35
//! legal moves should be a few thousand nodes. Two orders of magnitude is not a tuning gap, it is
//! a defect, and until it is explained every P2 experiment runs ~100x slower than it should.
//!
//! MEASURE, DO NOT INFER. This reports evals and cost per depth for the UNMUTATED seed, so the
//! effective branching factor falls out of the ratios:
//!   * evals(d+1)/evals(d) ~ 35 means NO pruning is happening at all -- full minimax width.
//!   * ~6 (sqrt of 35) means alpha-beta is pruning about as well as perfect ordering allows.
//!   * anything much above 35 means the search is deeper than it is being told to go, or eval is
//!     being called at interior nodes as well as leaves.
//!
//! It also reports cost/eval. If that is far above 1365, evals are NOT what the cost model is
//! actually charging for and the bottleneck is elsewhere in the primitive mix.
use board::Position;
use grammar::reference;
use interp::Interp;
use nnue::Net;

fn main() {
    let n: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(5);
    let maxd: i64 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(4);
    let net = Net::random(32, 20260907);
    let ab = reference::bare_alpha_beta();

    // Same construction the fitness set uses: random walks from startpos.
    let mut rng: u64 = 0xC0DE_F00D;
    let mut set = Vec::new();
    while set.len() < n {
        let mut p = Position::startpos();
        for _ in 0..(10 + rng % 30) {
            let l = p.legal_moves();
            if l.is_empty() { break; }
            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
            p.make_move(l.as_slice()[(rng % l.len() as u64) as usize]);
        }
        if !p.legal_moves().is_empty() { set.push(p); }
    }
    let avg_moves: f64 =
        set.iter().map(|p| p.legal_moves().len() as f64).sum::<f64>() / set.len() as f64;

    println!("=== seed alpha-beta cost by depth, {} positions, avg {avg_moves:.1} legal moves ===",
             set.len());
    println!("  {:>5} {:>14} {:>16} {:>12} {:>14}", "depth", "evals/pos", "cost/pos", "cost/eval",
             "evals ratio");
    // CONTROL: depth-one is 9 nodes and evaluates each root move exactly once, so at D=1 it MUST
    // score ~one eval per legal move. If it does and the seed does not, the seed is doing
    // something extra; if BOTH are inflated, the counter or the cost model is what is wrong. Two
    // wrong hypotheses in a row means the harness, so this run gets a control before I read
    // anything into the seed's numbers.
    for (name, prog) in [("depth-one (control)", reference::depth_one()),
                         ("bare alpha-beta (seed)", reference::bare_alpha_beta())] {
        println!("\n  --- {name} ---");
        let mut prev: f64 = 0.0;
        for d in 1..=maxd {
            let mut it = Interp::new(&net, vec![d, 32_000, 8]);
            let (mut ev, mut cost) = (0u64, 0u64);
            for p in &set {
                it.run(&prog, p, 16);
                ev += it.evals;
                cost += it.cost;
            }
            let e = ev as f64 / set.len() as f64;
            let c = cost as f64 / set.len() as f64;
            let ratio = if prev > 0.0 { e / prev } else { 0.0 };
            println!("  {d:>5} {e:>14.0} {c:>16.0} {:>12.1} {:>14}  full-width b^d = {:.0}",
                     c / e.max(1.0),
                     if ratio > 0.0 { format!("{ratio:.1}x") } else { "-".into() },
                     avg_moves.powi(d as i32));
            prev = e;
        }
    }
    let mut prev: f64 = 0.0;
    for d in 0..0 {
        let mut it = Interp::new(&net, vec![d, 32_000, 8]);
        let (mut ev, mut cost) = (0u64, 0u64);
        for p in &set {
            it.run(&ab, p, 16);
            // `run` RESETS cost and evals to 0 on entry (interp/src/lib.rs:448), so these are
            // already per-run totals. The first version of this probe subtracted a "before"
            // reading, which differences two independent absolutes and, via wrapping_sub,
            // manufactured a fake 400,000,079 that looked exactly like a 4e8 cost cap. The real
            // cap is 2e9. Measuring the instrument before believing it -- the harness was wrong,
            // not the subject.
            ev += it.evals;
            cost += it.cost;
        }
        let e = ev as f64 / set.len() as f64;
        let c = cost as f64 / set.len() as f64;
        let ratio = if prev > 0.0 { e / prev } else { 0.0 };
        println!("  {d:>5} {e:>14.0} {c:>16.0} {:>12.1} {:>14}", c / e.max(1.0),
                 if ratio > 0.0 { format!("{ratio:.1}x") } else { "-".into() });
        prev = e;
    }
    println!("\n  A ratio near {avg_moves:.0} means NO pruning -- full-width minimax.");
    println!("  A ratio near {:.0} means alpha-beta is pruning about as well as ordering allows.",
             avg_moves.sqrt());
    println!("  cost/eval far above 1365 means evals are not what the cost model is charging for.");
}
