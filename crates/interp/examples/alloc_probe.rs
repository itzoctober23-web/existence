//! Does the per-call `Vec::new()` on the narrow-net eval path actually cost anything?
//!
//! `PosAcc::score` falls back to `net.eval(&self.pos, &mut Vec::new())` below
//! INCREMENTAL_MIN_WIDTH, and the comment justifies the fresh allocation with "this path is only
//! taken below width 64 where the gather is small". The gather being small is true and
//! irrelevant: a malloc/free pair is a FIXED cost per call, so it does not shrink with the
//! feature count. The champion in play is width 16, so this is the path every interpreter eval
//! actually takes -- the excuse and the hot path are the same line.
//!
//! Both arms must produce identical scores, and this asserts that rather than assuming it: an
//! earlier speed ratio on this project survived review only because a separate equivalence check
//! caught that the two arms were not doing the same work.
use board::Position;
use nnue::Net;
use std::time::Instant;

fn bench(iters: usize, mut f: impl FnMut()) -> f64 {
    let mut best = f64::MAX;
    for _ in 0..7 {
        let t = Instant::now();
        for _ in 0..iters { f(); }
        let ns = t.elapsed().as_nanos() as f64 / iters as f64;
        if ns < best { best = ns; }
    }
    best
}

fn main() {
    let width: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(16);
    let net = Net::random(width, 7);

    // Same position construction as cost_calibrate: random walks, not startpos, because eval
    // scales with how busy the board is and startpos is not representative of a search interior.
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

    // EQUIVALENCE FIRST. If the two arms ever disagree the timings are meaningless.
    let mut shared = Vec::new();
    for p in &ps {
        let fresh = net.eval(p, &mut Vec::new());
        let reused = net.eval(p, &mut shared);
        assert_eq!(fresh, reused, "arms disagree — the comparison would be meaningless");
    }

    let mut i = 0usize;
    let mut next = || { i = (i + 1) % 64; i };

    let t_fresh = bench(20_000, || {
        std::hint::black_box(net.eval(&ps[next()], &mut Vec::new()));
    });
    let mut scratch = Vec::new();
    let t_reused = bench(20_000, || {
        std::hint::black_box(net.eval(&ps[next()], &mut scratch));
    });

    let delta = t_fresh - t_reused;
    println!("width {width}, 64 random-walk positions, best-of-7");
    println!("  eval, fresh Vec::new() per call : {t_fresh:8.1} ns   <- the shipping narrow path");
    println!("  eval, reused scratch buffer     : {t_reused:8.1} ns");
    println!("  allocation cost                 : {delta:8.1} ns  ({:.1}% of the fresh path)",
             100.0 * delta / t_fresh);
    println!("  (both arms verified to return identical scores on all 64 positions)");
}
