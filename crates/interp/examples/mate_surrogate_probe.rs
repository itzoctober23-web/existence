//! IS THE SEARCH TRACK'S SURROGATE GAMEABLE BY SEARCHING LESS?
//!
//! evolve.rs accepts a candidate when `f >= best_found && rate > best_rate` -- keep every mate,
//! get cheaper -- with no game gate in the loop. Its set is built by `mate_set`, which collects
//! positions that have a mate in ONE. On such a set no amount of shallowness can lose a mate,
//! so the "must not lose mates" guard cannot bite and the optimiser is free to drive cost to
//! zero. The first restarted run went 0.03 -> 8.73 mates/Mcost in a single type-preserving edit,
//! 333x cheaper at the same node count, which is what that failure looks like.
//!
//! That is a hypothesis about the MECHANISM, so it gets tested rather than asserted: if depth is
//! the lever, then the SEED program at depth 1 should keep all the mates and cost a fraction of
//! depth 2 -- no mutation required. If depth 1 instead loses mates, the hypothesis is wrong and
//! the 333x came from somewhere else.
//!
//! Also measures a MATE-IN-TWO set, because that is the proposed repair: on positions where the
//! win needs three plies, a shallow program loses mates and the existing guard starts working.
use board::{Outcome, Position};
use grammar::reference;
use interp::Interp;
use nnue::Net;

/// Positions with a mate in one (same construction as evolve.rs).
fn mate_in_one_set(n: usize) -> Vec<Position> {
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

/// Positions with NO mate in one, but where some move forces mate on the next turn: every
/// opponent reply loses. Three plies, so depth 1 and 2 cannot see it and depth 3 can.
///
/// Returns the FORCING MOVE with each position. The first version returned bare positions and
/// scored them with the mate-in-one checker, which credits a move only when it mates IMMEDIATELY
/// -- and a mate-in-two's first move never does. Depth 1, 2 AND 3 all scored 0, which is not a
/// property of the search: it was the checker returning zero by construction. The control caught
/// it (depth 3 scoring nothing on a set built to be solvable at depth 3 is not believable).
fn mate_in_two_set(n: usize, cap: usize) -> Vec<(Position, board::Move)> {
    let mut rng: u64 = 0x5EED_1234;
    let mut out = Vec::new();
    let mut tries = 0;
    while out.len() < n && tries < cap {
        tries += 1;
        let mut p = Position::startpos();
        for _ in 0..(10 + rng % 40) {
            let l = p.legal_moves();
            if l.is_empty() { break; }
            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
            p.make_move(l.as_slice()[(rng % l.len() as u64) as usize]);
        }
        if p.legal_moves().is_empty() { continue; }
        // reject anything with a mate in one -- otherwise depth 1 solves it and the set is not
        // testing depth at all
        let mut has_m1 = false;
        for &m in p.legal_moves().as_slice() {
            let u = p.make_move(m);
            if p.legal_moves().is_empty() && p.outcome() == Outcome::Loss { has_m1 = true; }
            p.unmake_move(m, u);
            if has_m1 { break; }
        }
        if has_m1 { continue; }
        // a forcing move: every reply leaves us a mate in one
        let moves: Vec<_> = p.legal_moves().as_slice().to_vec();
        for m in moves {
            let u = p.make_move(m);
            let replies: Vec<_> = p.legal_moves().as_slice().to_vec();
            let mut all_lose = !replies.is_empty();
            for r in replies {
                let u2 = p.make_move(r);
                let mut mates = false;
                for &m2 in p.legal_moves().as_slice() {
                    let u3 = p.make_move(m2);
                    if p.legal_moves().is_empty() && p.outcome() == Outcome::Loss { mates = true; }
                    p.unmake_move(m2, u3);
                    if mates { break; }
                }
                p.unmake_move(r, u2);
                if !mates { all_lose = false; break; }
            }
            p.unmake_move(m, u);
            if all_lose { out.push((p.clone(), m)); break; }
        }
    }
    out
}

fn score(prog: &grammar::Program, set: &[Position], net: &Net, d: i64) -> (u32, u64, f64) {
    let mut it = Interp::new(net, vec![d, 32_000, interp::UCT_EXPLORATION]);
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

/// Credit the program for playing THE forcing move, not for mating on this ply.
fn score_forcing(prog: &grammar::Program, set: &[(Position, board::Move)], net: &Net, d: i64)
    -> (u32, u64, f64) {
    let mut it = Interp::new(net, vec![d, 32_000, interp::UCT_EXPLORATION]);
    let (mut found, mut cost) = (0u32, 0u64);
    for (p, best) in set {
        let mv = it.run(prog, p, 16);
        cost += it.cost;
        if mv == *best { found += 1; }
    }
    (found, cost, found as f64 * 1e6 / cost.max(1) as f64)
}

fn main() {
    let net = Net::random(32, 20260907);
    let ab = reference::bare_alpha_beta();

    let m1 = mate_in_one_set(80);
    println!("MATE-IN-ONE set ({} positions) — what evolve.rs actually optimises on", m1.len());
    println!("  {:>5} {:>8} {:>16} {:>14}", "depth", "mates", "cost", "mates/Mcost");
    for d in 1..=3 {
        let (f, c, r) = score(&ab, &m1, &net, d);
        println!("  {d:>5} {f:>8} {c:>16} {r:>14.2}");
    }
    println!("  If depth 1 keeps all {} mates and costs far less, then 'do not lose mates' cannot", m1.len());
    println!("  bite on this set and the surrogate rewards searching less. That is the defect.\n");

    let m2 = mate_in_two_set(40, 400_000);
    if m2.is_empty() {
        println!("MATE-IN-TWO set: none found — cannot test the repair, and NOT claiming it works.");
        return;
    }
    println!("MATE-IN-TWO set ({} positions, no mate in one available) — the proposed repair", m2.len());
    println!("  {:>5} {:>8} {:>16} {:>14}", "depth", "mates", "cost", "mates/Mcost");
    for d in 1..=3 {
        let (f, c, r) = score_forcing(&ab, &m2, &net, d);
        println!("  {d:>5} {f:>8} {c:>16} {r:>14.2}");
    }
    println!("  Here shallower MUST cost mates. If depth 1 scores near zero and depth 3 solves");
    println!("  them, the existing f >= best_found guard starts doing the job it was written for.");
}
