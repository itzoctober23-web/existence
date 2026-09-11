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
    // THE SAME CALL, CONSUMED CHEAPLY. `black_box` on the returned MoveList forces the compiler to
    // materialise all 1032 bytes as observable memory at every call. The real search binds the
    // result to a local and never demands that, so the figure above may be measuring my own probe.
    // Here the moves are still fully generated -- `len` cannot be known without generating them, and
    // the checksum reads real entries -- but nothing forces a 1 KB store.
    let mg_cheap = best_of(reps, || {
        let mut acc = 0u64;
        let mut n = 0usize;
        for p in &ps {
            let l = p.legal_moves();
            acc ^= l.len() as u64 ^ l.as_slice().first().map_or(0, |m| m.0 as u64);
            n += 1;
        }
        std::hint::black_box(acc);
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

    // 5. THE SAME SHUFFLE WITHOUT THE DIVISION. Measurement only -- nothing in the engine changes.
    //    Fisher-Yates needs a uniform index in [0, i]; the current code gets it with `rng % (i+1)`,
    //    an integer division executed ~30 times per node. Lemire's multiply-shift computes the same
    //    range reduction with a 64-bit multiply and a shift. It is still a uniform shuffle and still
    //    denies the move-ordering prior; it simply draws a DIFFERENT permutation from a given seed,
    //    which is why adopting it is a deliberate change with a determinism cost (FITNESS 10), not a
    //    free win. This measures what that win would be worth before anyone spends it.
    let sh_nodiv = best_of(reps, || {
        let mut rng = 0x9E3779B97F4A7C15u64;
        let mut buf: Vec<board::Move> = Vec::new();
        let mut n = 0usize;
        for l in &lists {
            buf.clear();
            buf.extend_from_slice(l);
            for i in (1..buf.len()).rev() {
                rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
                let j = (((rng as u128) * (i as u128 + 1)) >> 64) as usize;
                buf.swap(i, j);
            }
            std::hint::black_box(&buf);
            n += 1;
        }
        n
    });

    // 6. INSIDE legal_moves(). It is the largest single cost (451 ns, 44% of a leaf), and "movegen
    //    is slow" is not an actionable statement. This is already a proper LEGAL generator -- not
    //    pseudo-legal plus a filter -- so the cost is in its precomputation, and there are two
    //    obvious candidates:
    //      * `attackers_to(ksq, them, all)`      -- which pieces give check
    //      * `attacks_by(them, occ_no_king)`     -- the full opponent DANGER map, needed only to
    //                                               mask king destinations
    //    The danger map is the suspicious one: it sweeps every enemy piece including sliders, at
    //    every node, to constrain at most 8 king targets. `pinned()` is private so it is obtained by
    //    subtraction rather than measured directly, and is reported as a residual, not a number.
    let ksqs: Vec<(u8, board::Color, u64)> = ps.iter()
        .map(|p| { let us = p.stm; (p.king_sq(us), us.flip(), p.all) }).collect();
    let atk_to = best_of(reps, || {
        let mut n = 0usize;
        for (p, (ksq, them, all)) in ps.iter().zip(&ksqs) {
            std::hint::black_box(p.attackers_to(*ksq, *them, *all)); n += 1;
        }
        n
    });
    let danger = best_of(reps, || {
        let mut n = 0usize;
        for (p, (ksq, them, _)) in ps.iter().zip(&ksqs) {
            let occ_no_king = p.all ^ (1u64 << *ksq);
            std::hint::black_box(p.attacks_by(*them, occ_no_king)); n += 1;
        }
        n
    });

    // 7. SLIDER ATTACKS, the suspect inside the 90% residual. attacks.rs uses the CLASSICAL ray
    //    method: per direction, load RAYS[dir][sq], mask by occupancy, find the nearest blocker with
    //    lsb/msb, then mask off the ray beyond it. That is O(1) per direction but a bishop costs 4
    //    such rays and a QUEEN costs 8 -- roughly 16 table loads -- where a magic bitboard is one
    //    multiply, one shift and one load for the whole piece.
    //
    //    Measured over the real slider squares in the corpus, so the occupancy patterns and the
    //    branch behaviour are the ones the search actually sees, not a synthetic best case.
    let mut slider_sites: Vec<(u8, u64)> = Vec::new();
    for p in &ps {
        for sq in 0u8..64 {
            if p.all & (1u64 << sq) != 0 { slider_sites.push((sq, p.all)); }
        }
    }
    let q = best_of(reps, || {
        for (sq, occ) in &slider_sites { std::hint::black_box(board::attacks::queen(*sq, *occ)); }
        slider_sites.len()
    });
    let b_ = best_of(reps, || {
        for (sq, occ) in &slider_sites { std::hint::black_box(board::attacks::bishop(*sq, *occ)); }
        slider_sites.len()
    });

    // 8. THE THIRD HYPOTHESIS, after two wrong ones -- and per the standing rule, two wrong
    //    hypotheses in a row means suspect the HARNESS or the API, not the subject.
    //
    //    `MoveList` is `{ moves: [Move; 256], len: usize }` with `Move(u32)` -- **1032 bytes**. Its
    //    `new()` writes `[MOVE_NONE; 256]`, zero-filling all 1024 bytes on EVERY call, to hold about
    //    30 moves = 120 bytes of actual payload. And `legal_moves()` returns it BY VALUE, so unless
    //    the compiler elides it there is a second 1032-byte move. Neither is move GENERATION; both
    //    are the container.
    let ml = best_of(reps, || {
        for _ in 0..slider_sites.len() {
            std::hint::black_box(board::MoveList::new());
        }
        slider_sites.len()
    });

    println!("  {:<26} {:>10}", "primitive", "ns/op");
    println!("  {:<26} {:>10.1}   <- black_box forces a 1032 B materialisation", "legal_moves() TOTAL", mg);
    println!("  {:<26} {:>10.1}   <- same work, consumed cheaply", "legal_moves() cheap-consume", mg_cheap);
    println!("  {:<26} {:>10.1}   ({:.0}% of movegen)", "  attackers_to (checkers)", atk_to, 100.0*atk_to/mg);
    println!("  {:<26} {:>10.1}   ({:.0}% of movegen)", "  attacks_by (danger map)", danger, 100.0*danger/mg);
    println!("  {:<26} {:>10.1}   ({:.0}% of movegen)", "  residual (pins + emit)", mg-atk_to-danger, 100.0*(mg-atk_to-danger)/mg);
    println!("  {:<26} {:>10.1}   (classical rays, {} sites)", "  attacks::queen", q, slider_sites.len());
    println!("  {:<26} {:>10.1}   (classical rays)", "  attacks::bishop", b_);
    println!("  {:<26} {:>10.1}   (zeroes 1024 B for ~120 B of moves)", "  MoveList::new()", ml);
    println!("  {:<26} {:>10.1}", "make + unmake (pair)", mu);
    println!("  {:<26} {:>10.1}", "eval", ev);
    println!("  {:<26} {:>10.1}", "shuffle + buffer copy", sh);
    println!("  {:<26} {:>10.1}   <- measurement only, engine unchanged", "  same, modulo-free", sh_nodiv);
    println!("  {:<26} {:>10.1} ns/node available", "  division cost", sh - sh_nodiv);

    // A node costs one legal_moves, one make/unmake as its parent's child, and -- at a leaf -- one
    // eval. Interior nodes skip the eval, so this brackets rather than pinpoints.
    // A LEAF DOES NOT SHUFFLE, and billing it for one understates eval's share.
    //
    // `pipeline/src/search.rs::ab()` in order: `nodes += 1`, then `legal_moves()`, then the
    // `depth == 0` early return with the eval -- and only AFTER that return does it take the
    // per-depth buffer and shuffle. So a leaf pays movegen (it runs before the check, to detect
    // mate/stalemate) and eval, and never reaches the shuffle. The old formula added `sh` to the
    // leaf anyway.
    //
    // make/unmake is billed to the child by convention: it is paid by the parent on this node's
    // behalf, once per node either way.
    let leaf = mg + mu + ev;
    let interior = mg + mu + sh;
    println!("\n  implied node cost:");
    println!("    interior (movegen + make/unmake)      {:>8.1} ns", interior);
    println!("    leaf     (+ eval)                     {:>8.1} ns", leaf);
    println!("    eval's share of a LEAF                {:>8.1}%", 100.0 * ev / leaf);

    // CROSS-CHECK, MEASURED HERE rather than quoted.
    //
    // This block used to be two `println!` lines asserting "width 16 = 796145 nps = 1256 ns/node".
    // That was true when written and became 3.3x wrong as the engine got faster (the O(changed)
    // accumulator delta, the shuffle-division fix) -- and because it was a STRING it could never
    // notice. A self-check that cannot fail is decoration: it printed its own falsification
    // condition ("if the primitives do not roughly bracket that, the attribution is INCOMPLETE")
    // while supplying a stale constant that made the check pass forever.
    //
    // It now runs the search and compares. The primitives are timed in ISOLATION, so each carries
    // its own loop and timing overhead and they are UPPER BOUNDS; a node inside a real search also
    // pipelines and hits warm caches. The expected relation is primitives >= actual; the
    // interesting quantity is BY HOW MUCH, because that gap is the part the shares do not explain.
    let mut bench_nodes = 0u64;
    let t0 = Instant::now();
    for p0 in ps.iter().take(20) {
        let mut p = p0.clone();
        let mut s = pipeline::search::Searcher::new();
        let _ = s.best_move(&mut p, 4, &net);
        bench_nodes += s.nodes;
    }
    let bench_s = t0.elapsed().as_secs_f64();
    let actual_ns = 1e9 * bench_s / bench_nodes.max(1) as f64;
    let ratio = leaf / actual_ns;
    println!("\n  CROSS-CHECK against a real search, measured NOW (not quoted):");
    println!("    depth 4, same net: {bench_nodes} nodes in {bench_s:.3}s = {:>9.0} nps = {actual_ns:.0} ns/node",
             bench_nodes as f64 / bench_s);
    println!("    primitives imply a LEAF of {leaf:.0} ns -> {ratio:.2}x the measured node cost");
    if ratio > 1.6 {
        println!("    ** ATTRIBUTION INCOMPLETE ** primitives over-predict by more than 1.6x.");
        println!("    Isolated timings are an UPPER BOUND, so part of this is loop/timing overhead");
        println!("    and warm caches -- but at this size the SHARES above are NOT safe to size an");
        println!("    optimisation against. Name the gap before quoting an Elo figure from them.");
    } else {
        println!("    within 1.6x -- the three primitives account for the node, shares are usable.");
    }
}
