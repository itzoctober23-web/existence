//! P2, first run: EVOLVE the search program.
//!
//! Mutate the seed, discard ill-typed candidates by tree walk (GRAMMAR 3), and score the
//! survivors on mates-per-cost (FITNESS 3) over a fixed MATE-1 set. Keep anything that beats
//! the champion. This is the search track's inner loop; the gate is what would confirm a
//! winner on the clock, but a candidate that cannot even improve mates-per-cost has no
//! business consuming gate time.
//!
//! Everything here is rules-derived: the mate set comes from terminal conditions, the cost
//! from the interpreter's own accounting. No chess knowledge enters.
use board::{Outcome, Position};
use grammar::mutate::{self, Rng};
use grammar::{reference, Program};
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

/// mates per million cost units, and the mate count (a program that finds fewer mates more
/// cheaply is NOT better -- unsound pruning's characteristic failure is a missed forced mate).
fn fitness(prog: &Program, set: &[Position], net: &Net) -> (u32, u64, f64) {
    let mut it = Interp::new(net, vec![2, 32_000, 8]);
    let (mut found, mut cost) = (0u32, 0u64);
    for p in set {
        let mv = it.run(prog, p, 16);
        cost += it.cost;
        if mv != board::types::MOVE_NONE {
            let mut q = p.clone();
            if let Some(m) = q.legal_moves().as_slice().iter().copied().find(|x| *x == mv) {
                q.make_move(m);
                if q.legal_moves().is_empty() && q.outcome() == Outcome::Loss { found += 1; }
            }
        }
    }
    (found, cost, found as f64 * 1e6 / cost.max(1) as f64)
}

fn main() {
    let gens: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(30);
    let pop: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(24);
    let net = Net::random(32, 20260907);
    let set = mate_set(80);

    let mut champ = reference::bare_alpha_beta();
    let (f0, c0, r0) = fitness(&champ, &set, &net);
    println!("  MATE-1 set {} positions", set.len());
    println!("  seed: bare alpha-beta  {f0} mates  {c0} cost  {r0:.2} mates/Mcost  ({} nodes)", champ.size());
    let (mut best_found, mut best_rate) = (f0, r0);

    let mut rng = Rng::new(0xE0FFEE);
    let mut accepted = 0;
    for g in 1..=gens {
        let mut ill = 0;
        let mut best: Option<(Program, u32, f64)> = None;
        for i in 0..pop {
            let mut r = Rng::new((g as u64) << 20 ^ i as u64 ^ 0xBEEF);
            let cand = match mutate::mutate_program(&champ, &mut r) { Some(c) => c, None => { ill += 1; continue } };
            let (f, _c, rate) = fitness(&cand, &set, &net);
            // must not LOSE mates, and must improve cost-efficiency
            if f >= best_found && rate > best_rate {
                if best.as_ref().map_or(true, |(_, _, br)| rate > *br) { best = Some((cand, f, rate)); }
            }
        }
        match best {
            Some((c, f, rate)) => {
                println!("  gen {g:>3}  ACCEPT  {f} mates  {rate:.2} mates/Mcost  ({} nodes, was {:.2})",
                    c.size(), best_rate);
                champ = c; best_found = f; best_rate = rate; accepted += 1;
            }
            None => if g % 10 == 0 {
                println!("  gen {g:>3}  ..no improvement ({pop} candidates, {ill} ill-typed/inapplicable)");
            },
        }
    }
    let _ = rng.next();
    println!("\n  {accepted} accepted over {gens} generations");
    println!("  final: {best_found} mates  {best_rate:.2} mates/Mcost  ({} nodes)", champ.size());
    println!("  seed was: {f0} mates  {r0:.2} mates/Mcost  ({} nodes)", reference::bare_alpha_beta().size());
}
