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

fn mate_set(n: usize) -> Vec<(Position, Option<board::Move>)> {
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
            if winning { out.push((p.clone(), None)); break; }
            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
            p.make_move(l.as_slice()[(rng % l.len() as u64) as usize]);
        }
    }
    out
}


/// Positions with NO mate in one, where some move FORCES mate next turn (every reply loses).
///
/// WHY THIS SET EXISTS. The loop accepts on `f >= best_found && rate > best_rate` -- keep every
/// mate, get cheaper -- with no game gate. On a mate-in-ONE-only set no amount of shallowness can
/// lose a mate, so the "must not lose mates" guard could never bite and the optimiser was free to
/// drive cost to zero. It did: the first restarted run went 0.03 -> 8.73 mates/Mcost in ONE
/// type-preserving edit, 333x cheaper at an unchanged node count.
///
/// MEASURED (examples/mate_surrogate_probe.rs), seed alpha-beta, no mutation involved:
///     mate-in-1 set   depth 1: 80/80 mates at 246M cost   depth 2: 80/80 at 3049M
///                     -> shallower keeps every mate and costs 12x less. Guard cannot bite.
///     mate-in-2 set   depth 1: 17/40 forcing moves        depth 2: 40/40
///                     -> shallower LOSES 23 mates. Guard bites, as written.
/// So the repair is the SET, not the rule: the rule was always right and had nothing to enforce.
fn forced_mate_set(n: usize, cap: usize) -> Vec<(Position, Option<board::Move>)> {
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
        let mut has_m1 = false;
        for &m in p.legal_moves().as_slice() {
            let u = p.make_move(m);
            if p.legal_moves().is_empty() && p.outcome() == Outcome::Loss { has_m1 = true; }
            p.unmake_move(m, u);
            if has_m1 { break; }
        }
        if has_m1 { continue; }   // a mate in one here would let depth 1 solve it
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
            if all_lose { out.push((p.clone(), Some(m))); break; }
        }
    }
    out
}

/// Positions where the SEED answers differently at depth D-1 and depth D.
///
/// THE GUARD THAT MATE-DISTANCE COULD NOT PROVIDE. The forced-mate-in-2 set was added to stop a
/// candidate from simply searching less, and it worked at fitness depth 2. At fitness depth 3 it
/// has no teeth: that set is solved 40/40 AT DEPTH 2 (measured -- the forcing move is also the
/// eval-best move), so cutting 3 -> 2 costs nothing on it. The search track promptly found exactly
/// that: `Const(0)` -> `Const(1)` in the horizon guard `if d == 0: ret eval(p)`, one ply shallower,
/// 11x cheaper, all 20 mates intact.
///
/// A depth guard must require the FULL fitness depth, and a mate-in-N does not imply N plies of
/// search. Disagreement does, by construction: if the seed returns a different move at D-1 than at
/// D, then D-1 is provably insufficient FOR THIS POSITION, whatever D happens to be. Self-
/// calibrating -- change the fitness depth and the guard follows.
fn disagreement_set(n: usize, depth: i64, net: &Net, cap: usize)
    -> Vec<(Position, Option<board::Move>)> {
    let ab = reference::bare_alpha_beta();
    let mut rng: u64 = 0xD15A_6EED;
    let mut out = Vec::new();
    let mut tries = 0;
    while out.len() < n && tries < cap {
        tries += 1;
        let mut p = Position::startpos();
        for _ in 0..(10 + rng % 34) {
            let l = p.legal_moves();
            if l.is_empty() { break; }
            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
            p.make_move(l.as_slice()[(rng % l.len() as u64) as usize]);
        }
        if p.legal_moves().is_empty() { continue; }
        let mut shallow = Interp::new(net, vec![depth - 1, 32_000, 8]);
        let a = shallow.run(&ab, &p, 16);
        let mut deep = Interp::new(net, vec![depth, 32_000, 8]);
        let b = deep.run(&ab, &p, 16);
        // Both must be real answers, and they must differ: that is the whole criterion.
        if a != board::types::MOVE_NONE && b != board::types::MOVE_NONE && a != b {
            out.push((p.clone(), Some(b)));
        }
    }
    out
}

/// mates per million cost units, and the mate count (a program that finds fewer mates more
/// cheaply is NOT better -- unsound pruning's characteristic failure is a missed forced mate).
/// FITNESS DEPTH IS A PARAMETER, and D=2 was the wrong value.
///
/// The GRAMMAR 9 ladder has exactly one verified ascent step from the seed: alpha-beta + hash
/// reuse, 0.98x the seed's cost for the identical 120 mates. That step exists at **D=3**. At D=2
/// the same rung measures 1.01x -- a LOSS -- because iteration 1 of ID stores entries at depth 1
/// and iteration 2 rejects every probe, so a transposition table has no reuse to find and pays
/// only probe/store cost. ladder.rs says this outright in its own comment.
///
/// This loop was evaluating every candidate at D=2, i.e. at the one depth where the single known
/// improvement is invisible. ~690 candidates across two runs, zero accepts. That is the leading
/// explanation, and this makes it testable instead of assumed.
///
/// D=3 costs roughly 11x D=2 per position (50.9e9 vs 4.6e9 over 120 positions), so the set has to
/// shrink to keep a generation affordable. That is an acceptable trade because the fitness is
/// DETERMINISTIC -- fixed positions, fixed net, no sampling -- so a 2% cost difference is exact at
/// any set size; a smaller set measures a smaller sample of positions, not a noisier number.
fn fitness(prog: &Program, set: &[(Position, Option<board::Move>)], net: &Net, depth: i64)
    -> (u32, u64, f64) {
    let mut it = Interp::new(net, vec![depth, 32_000, 8]);
    let (mut found, mut cost) = (0u32, 0u64);
    for (p, forcing) in set {
        let mv = it.run(prog, p, 16);
        cost += it.cost;
        match forcing {
            // MATE IN TWO: credit the FORCING move. The first move of a mate in two never mates
            // on this ply, so the immediate-mate test below scores it 0 by construction -- that
            // bug made depth 1, 2 and 3 all read 0 until the control caught it.
            Some(best) => { if mv == *best { found += 1; } }
            // MATE IN ONE: any move that mates now, since there may be several.
            None => {
                if mv != board::types::MOVE_NONE {
                    let mut q = p.clone();
                    if let Some(m) = q.legal_moves().as_slice().iter().copied().find(|x| *x == mv) {
                        q.make_move(m);
                        if q.legal_moves().is_empty() && q.outcome() == Outcome::Loss { found += 1; }
                    }
                }
            }
        }
    }
    (found, cost, found as f64 * 1e6 / cost.max(1) as f64)
}

fn main() {
    let gens: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(30);
    let pop: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(24);
    // SET SIZES ARE ARGUMENTS, because they are the cost knob and hardcoding them meant a REBUILD
    // to resize -- which is impossible while another job holds ./target/release.
    //
    // MEASURED 2026-09-08: at 80 mate-in-1 + 40 mate-in-2 and pop 24, ONE generation takes 330
    // seconds, so the 400-generation run I launched would have taken 37 HOURS. Not broken, just
    // sized wrong, and invisible because the loop only printed every 10th barren generation.
    // Cost is per candidate-evaluation: pop x (n1 + n2) positions, ~37M cost units each.
    let n1: usize = std::env::args().nth(3).and_then(|s| s.parse().ok()).unwrap_or(40);
    let n2: usize = std::env::args().nth(4).and_then(|s| s.parse().ok()).unwrap_or(20);
    let depth: i64 = std::env::args().nth(5).and_then(|s| s.parse().ok()).unwrap_or(3);
    let net = Net::random(32, 20260907);
    // MIXED on purpose: mate-in-1 alone made the surrogate maximisable by searching less.
    let mut set = mate_set(n1);
    // Built by DISAGREEMENT at the fitness depth, not by mate distance. See disagreement_set.
    let deep = disagreement_set(n2, depth, &net, 4_000);
    let n_deep = deep.len();
    set.extend(deep);

    let mut champ = reference::bare_alpha_beta();
    let (f0, c0, r0) = fitness(&champ, &set, &net, depth);
    println!("  surrogate set {} positions ({} mate-in-1, {} depth-{}-REQUIRING by disagreement), fitness depth {}",
             set.len(), set.len() - n_deep, n_deep, depth, depth);
    if n_deep < n2 {
        println!("  REFUSING TO RUN: found {n_deep} depth-requiring positions, wanted {n2}.");
        println!("  Without them the 'do not lose mates' guard cannot bite and this loop");
        println!("  optimises toward searching one ply less -- which is exactly what it did");
        println!("  when the guard was built from mate distance instead of disagreement.");
        return;
    }
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
            let (f, _c, rate) = fitness(&cand, &set, &net, depth);
            // must not LOSE mates, and must improve cost-efficiency
            if f >= best_found && rate > best_rate {
                if best.as_ref().map_or(true, |(_, _, br)| rate > *br) { best = Some((cand, f, rate)); }
            }
        }
        match best {
            Some((c, f, rate)) => {
                // SIX decimals, not two. The seed scores 0.0024 mates/Mcost and an accepted
                // candidate scored 0.03 -- at {:.2} both the before and after of the SECOND accept
                // printed as "0.03", so a real improvement was indistinguishable from none. The
                // whole output of this loop is these lines; rounding them away hides the result.
                println!("  gen {g:>3}  ACCEPT  {f} mates  {rate:.6} mates/Mcost  ({} nodes, was {:.6})",
                    c.size(), best_rate);
                champ = c; best_found = f; best_rate = rate; accepted += 1;
                // PERSIST IT. Until now the evolved program existed only in memory: the run that
                // found two improvements at D=3 left nothing to inspect, reproduce or gate, so a
                // discovery was unfalsifiable and unusable in the same breath. Debug is a lossless
                // round-trip of the AST for these purposes -- the point is to be able to READ what
                // the search found and diff it against the seed.
                if let Err(e) = std::fs::write(
                    format!("evolved_gen{g}.prog"),
                    format!("// {f} mates, {rate:.6} mates/Mcost, {} nodes, generation {g}\n{:#?}\n",
                            champ.size(), champ)) {
                    eprintln!("  WARNING: could not save the evolved program: {e}");
                }
            }
            // EVERY generation, not every 10th. At 5.5 min/generation a 10-generation gap is
            // 55 minutes of silence, which is indistinguishable from a hang -- I could not tell
            // whether the track was progressing without timing a generation by hand.
            None => println!("  gen {g:>3}  ..no improvement ({pop} candidates, {ill} ill-typed/inapplicable)"),
        }
    }
    let _ = rng.next();
    println!("\n  {accepted} accepted over {gens} generations");
    println!("  final: {best_found} mates  {best_rate:.6} mates/Mcost  ({} nodes)", champ.size());
    println!("  seed was: {f0} mates  {r0:.6} mates/Mcost  ({} nodes)", reference::bare_alpha_beta().size());
    if accepted > 0 {
        println!("  improvement over the seed: {:.2}x on mates-per-cost", best_rate / r0.max(1e-12));
        println!("  saved: evolved_gen*.prog  (read them; a rate this loop cannot explain is a bug, not a discovery)");
    }
}
