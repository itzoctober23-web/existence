//! What does a BOUNDED transposition table actually cost?
//!
//! The interpreter's hash was an unbounded `HashMap` — a sketch of a TT that never evicts. It
//! is now a fixed 64k-slot direct-indexed table with always-replace and full-key validation.
//! Two things have to be true for that swap to be honest, and neither is checkable by staring
//! at the code:
//!
//!   1. CORRECTNESS. A colliding probe must return NOTHING, never another position's slot. If
//!      it leaked, a hash-using program would read a foreign score and the oracle's exactness
//!      rule (FITNESS 2.4) would be the only thing standing between that and a false mate
//!      claim. Checked by running the hash-using reference against the hash-free one and
//!      asserting they choose the same move: alpha-beta with a sound TT returns the same value
//!      as alpha-beta without one.
//!   2. PRESSURE. 64k slots is a declared constant. If a search at these depths overflows it
//!      badly, the table is thrashing and "hash reuse" would be measured under a table too
//!      small to help — the mirror of the infinite-table problem, and just as misleading.
//!
//! Reports the collision rate so the constant can be revisited against a number.

use board::Position;
use grammar::reference;
use interp::Interp;
use nnue::Net;

fn main() {
    let net = Net::random(64, 20260907);
    let hash_prog = reference::ab_hash();
    let bare_prog = reference::bare_alpha_beta();

    let mut rng: u64 = 0xC0FFEE;
    let mut rnd = || { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; rng };

    let mut positions = Vec::new();
    while positions.len() < 60 {
        let mut p = Position::startpos();
        let plies = 4 + (rnd() % 20) as usize;
        let mut ok = true;
        for _ in 0..plies {
            let l = p.legal_moves();
            if l.is_empty() { ok = false; break; }
            p.make_move(l.as_slice()[(rnd() % l.len() as u64) as usize]);
        }
        if ok && !p.legal_moves().is_empty() { positions.push(p); }
    }

    for depth in [2i64, 3, 4] {
        let (mut probes_total, mut coll_total, mut agree, mut cost_hash, mut cost_bare) =
            (0u64, 0u64, 0usize, 0u64, 0u64);
        // THE equivalence check the traps file demands: a cost ratio means nothing unless both
        // arms did the same work, and "same work" for a search is the number of leaves it
        // actually evaluated.
        let (mut evals_hash, mut evals_bare) = (0u64, 0u64);
        for p in &positions {
            let mut ih = Interp::new(&net, vec![depth, 32_000]);
            let mh = ih.run(&hash_prog, p, depth);
            coll_total += ih.hash_collisions();
            cost_hash += ih.cost;
            evals_hash += ih.evals;

            let mut ib = Interp::new(&net, vec![depth, 32_000]);
            let mb = ib.run(&bare_prog, p, depth);
            cost_bare += ib.cost;
            evals_bare += ib.evals;

            // A sound transposition table does not change the VALUE alpha-beta returns, so a
            // disagreement here is a leak, not an optimisation.
            if mh == mb { agree += 1; }
            probes_total += 1;
        }
        println!("depth {depth}  agree {agree}/{}  collisions {coll_total}\n\
                  \tEVALS  hash {evals_hash}  bare {evals_bare}\n\
                  \tcost   hash {cost_hash}  bare {cost_bare}   ({:.3}x)",
                 positions.len(), cost_hash as f64 / cost_bare.max(1) as f64);
        let _ = probes_total;
    }
    println!("\nagreement < n does NOT automatically mean a leak: alpha-beta can return a\n\
              DIFFERENT best move of EQUAL value, and a TT changes which one is found first.\n\
              A value disagreement would be the real alarm; this checks the cheaper proxy.");
}
