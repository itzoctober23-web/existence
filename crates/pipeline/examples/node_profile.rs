//! WHERE DOES A SEARCH NODE'S TIME GO? The measurement `throughput_RESULT.md` said should come next.
//!
//! THE STANDING CLAIM THIS TESTS. The loop brief ranks "incremental NNUE accumulator" first, on the
//! grounds that eval "is a dense 256x782 forward pass at every leaf and caps the engine at ~10k nps".
//! Both halves are already refuted: `search_bench` measures 796,145 nps at the champion's width 16,
//! and scaling width 16 -> 256 (16x the eval work) costs only 3.06x the nps, not 16x, which puts eval
//! at roughly 14% of a 1.256 us node. That leaves ~86% unattributed, and "unattributed" is not a
//! place to start optimising.
//!
//! WHAT IT MEASURES. The three primitives a node actually spends time in, on the same position set,
//! with the same net the champion uses:
//!   * `legal_moves()`            -- once per interior node
//!   * `make_move` + `unmake_move` -- once per child visited
//!   * `eval`                      -- once per leaf
//! and reports each as ns/op plus its implied share of a node.
//!
//! BEST-OF-N, NOT MEAN, AND THAT IS THE WHOLE REASON THIS CAN RUN NOW. All four background cores are
//! busy with time-boxed experiments, and a mean would measure the neighbour as much as the code. The
//! MINIMUM over repeats is the run that suffered least interference, so it is the closest available
//! estimate of the uncontended cost and it degrades gracefully rather than silently. This is the same
//! technique `interp/examples/alloc_probe.rs` used to resolve a 13.9 ns difference.
//!
//! THE SELF-CHECK, which is what makes this more than three unrelated numbers: the primitives are
//! measured independently and then compared against the INDEPENDENTLY measured node cost from
//! `search_bench`. If they do not roughly add up, the attribution is incomplete and says so, rather
//! than presenting a tidy pie chart that omits whatever was missed.
use board::Position;
use nnue::Net;
use std::time::Instant;

fn corpus(n: usize, seed: u64) -> Vec<Position> {
    let mut rng = seed | 1;
    let mut rnd = move || { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; rng };
    let mut out = Vec::new();
    while out.len() < n {
        let mut p = Position::startpos();
        for _ in 0..(6 + rnd() % 26) {
            let l = p.legal_moves();
            if l.is_empty() { break; }
            p.make_move(l.as_slice()[(rnd() % l.len() as u64) as usize]);
        }
        if !p.legal_moves().is_empty() { out.push(p); }
    }
    out
}

/// Best-of-`reps`: returns ns per operation from the fastest repeat.
fn best_of<F: FnMut() -> usize>(reps: usize, mut f: F) -> f64 {
    let mut best = f64::MAX;
    for _ in 0..reps {
        let t = Instant::now();
        let ops = f();
        let ns = t.elapsed().as_nanos() as f64 / ops.max(1) as f64;
        if ns < best { best = ns; }
    }
    best
}

fn main() {
    let width: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(16);
    let reps: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(9);
    let net = Net::random(width, 20260907);
    let ps = corpus(200, 0xC0FFEE);
    println!("node profile — width {width}, {} positions, best of {reps}\n", ps.len());

    // 1. legal_moves(): once per interior node.
    let mg = best_of(reps, || {
        let mut n = 0usize;
        for p in &ps { std::hint::black_box(p.legal_moves()); n += 1; }
        n
    });

    // 2. make + unmake: once per child visited. Measured as the PAIR, because the search always
    //    does both and unmake's cost is meaningless alone.
    let mu = best_of(reps, || {
        let mut n = 0usize;
        for p in &ps {
            let l = p.legal_moves();
            let mut q = p.clone();
            for m in l.as_slice() {
                let u = q.make_move(*m);
                q.unmake_move(*m, u);
                n += 1;
            }
        }
        n
    });

    // 3. eval: once per leaf. Scratch is reused, as the hot path does -- allocating per call was
    //    measured at 13.9 ns of overhead and fixed, so measuring it with a fresh Vec would report a
    //    cost the engine does not pay.
    let mut scratch = Vec::new();
    let ev = best_of(reps, || {
        let mut n = 0usize;
        for p in &ps { std::hint::black_box(net.eval(p, &mut scratch)); n += 1; }
        n
    });

    // 4. The SHUFFLE, which the seed search does at every node. MASTER_PLAN puts move ordering on
    //    the DISCOVERY list, so `pipeline/src/search.rs` shuffles children to deny alpha-beta an
    //    undeclared "try pawn moves first" prior inherited from movegen emission order. That denial
    //    is deliberate and correct -- but it is not free, and nobody has measured what it costs.
    //    Includes the per-node buffer copy the real search also pays.
    //    MOVE LISTS ARE PRE-COMPUTED, and the first version of this did NOT do that: it called
    //    p.legal_moves() inside the timed loop, so the shuffle figure silently included the whole
    //    movegen cost and came out at 647 ns -- LARGER than movegen itself, which is impossible for
    //    a Fisher-Yates over ~30 elements. Same class as the eval-count equivalence check: an arm
    //    that does extra work reports a cost that is not its own.
    let lists: Vec<Vec<board::Move>> =
        ps.iter().map(|p| p.legal_moves().as_slice().to_vec()).collect();
    let sh = best_of(reps, || {
        let mut rng = 0x9E3779B97F4A7C15u64;
        let mut buf: Vec<board::Move> = Vec::new();
        let mut n = 0usize;
        for l in &lists {
            buf.clear();
            buf.extend_from_slice(l);
            for i in (1..buf.len()).rev() {
                rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
                let j = (rng % (i as u64 + 1)) as usize;
                buf.swap(i, j);
            }
            std::hint::black_box(&buf);
            n += 1;
        }
        n
    });

    println!("  {:<26} {:>10}", "primitive", "ns/op");
    println!("  {:<26} {:>10.1}", "legal_moves()", mg);
    println!("  {:<26} {:>10.1}", "make + unmake (pair)", mu);
    println!("  {:<26} {:>10.1}", "eval", ev);
    println!("  {:<26} {:>10.1}", "shuffle + buffer copy", sh);

    // A node costs one legal_moves, one make/unmake as its parent's child, and -- at a leaf -- one
    // eval. Interior nodes skip the eval, so this brackets rather than pinpoints.
    let leaf = mg + mu + ev + sh;
    let interior = mg + mu + sh;
    println!("\n  implied node cost:");
    println!("    interior (movegen + make/unmake)      {:>8.1} ns", interior);
    println!("    leaf     (+ eval)                     {:>8.1} ns", leaf);
    println!("    eval's share of a LEAF                {:>8.1}%", 100.0 * ev / leaf);

    println!("\n  CROSS-CHECK against search_bench, measured independently:");
    println!("    width 16 = 796145 nps = 1256 ns/node; width 256 = 260334 nps = 3841 ns/node.");
    println!("    If the primitives above do not roughly bracket that, the attribution is");
    println!("    INCOMPLETE -- something outside these three is taking the time, and the honest");
    println!("    answer is to name that rather than to optimise one of these.");
}
