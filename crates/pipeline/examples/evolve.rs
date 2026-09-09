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
use grammar::ast::Node;
use grammar::{reference, Program};
use interp::Interp;
use pipeline::gate;
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

/// Positions where the seed's answer CHANGES when the search window is narrowed.
///
/// THE SECOND EXPLOIT, and it appeared the moment the first was closed. With the disagreement set
/// blocking the depth cheat, the search found this instead (evolved_gen12.prog, verified by diff
/// against the seed): it replaced the root call's alpha argument, `neg(INF)`, with the constant 8.
/// That raises the initial alpha from -32000 to +8, pruning every move worth under 8 centipawns.
///
/// It is a legitimate alpha-beta technique, not a defect -- and it is SAFE ONLY ON THIS SET,
/// because every position here is decided by an evaluation worth about +/-30000, so the true best
/// move is never below alpha. In ordinary play, where the best move is often worth -50, it fails
/// low and returns whatever the move generator emitted first.
///
/// The disagreement set fixed the DEPTH axis. This fixes the SCORE-MAGNITUDE axis, by the same
/// self-calibrating principle: alpha is `neg(INF)` and INF is table 1, so running the seed with a
/// SMALL INF narrows the window, and a position whose answer changes under that narrowing is one
/// where the window cannot be narrowed for free. A program that raises alpha scores zero on these,
/// exactly as a shallower program scores zero on the disagreement positions.
///
/// (An earlier probe swept INF and measured only 13%, and I concluded the window was not
/// exploitable. That swept the MAGNITUDE bound while this mutation moved the LOWER bound of the
/// window. Same table, different question, and the conclusion did not transfer.)
fn window_sensitive_set(n: usize, depth: i64, net: &Net, narrow: i64, cap: usize)
    -> Vec<(Position, Option<board::Move>)> {
    let ab = reference::bare_alpha_beta();
    let mut rng: u64 = 0xA1FA_5EED;
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
        let mut full = Interp::new(net, vec![depth, 32_000, 8]);
        let a = full.run(&ab, &p, 16);
        let mut tight = Interp::new(net, vec![depth, narrow, 8]);
        let b = tight.run(&ab, &p, 16);
        if a != board::types::MOVE_NONE && a != b {
            out.push((p.clone(), Some(a)));   // the FULL-window answer is the correct one
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
fn fitness(prog: &Program, set: &[(Position, Option<board::Move>)], net: &Net, depth: i64,
           budget: i64)
    -> (u32, u64, f64) {
    let mut it = Interp::new(net, vec![depth, 32_000, 8]);
    let (mut found, mut cost) = (0u32, 0u64);
    for (p, forcing) in set {
        // BUDGET IS PER LINEAGE. Alpha-beta ignores it and recurses on the depth table; UCT
        // spends playouts against it. A single global 16 was therefore tuned for the lineage that
        // does not read it, and scored the UCT seed at 1/25 for 1.6% of the cost.
        let mv = it.run(prog, p, budget);
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

/// IS THE ONE VERIFIED RUNG REACHABLE BY A HILL CLIMB AT ALL? `evolve valley [n1 n2 n3 depth]`
///
/// This lives in `evolve.rs` on purpose. The question is whether a path exists under THIS loop's
/// fitness, so it reuses THIS loop's `fitness`, `mate_set`, `disagreement_set` and
/// `window_sensitive_set` verbatim. A standalone probe would be a second harness free to drift
/// from the first, and this project has already produced three different numbers for one quantity
/// exactly that way.
///
/// PRE-REGISTERED READING, written before the numbers exist:
///   * VALLEY CONFIRMED if BOTH halves score below the seed. Then the only rung ever measured as
///     fitter is unreachable by strict hill climbing NO MATTER which operators exist, because
///     acceptance requires `rate > best_rate` at every step. The bottleneck would be the SEARCH,
///     not the grammar, and the operator work is then not the thing to do first.
///   * REFUTED if either half is >= the seed. Then a monotone path may exist and the half that
///     passes names the operator to add first. I expect a valley; being wrong here is cheap and
///     immediately actionable, which is why it is worth measuring rather than reasoning about.
///   * The FULL rung is measured too, as a CONTROL. If it does not reproduce its 0.98x on this
///     set then nothing else in the table means anything and the 0.98x is what needs
///     re-examining -- so it is re-measured here rather than carried in from a comment.

/// How many transposition-table primitives does a program contain? Probe/Store/Key/Field.
///
/// WHY THIS EXISTS. The plateau tolerance was reported as "carrying the first step into the
/// valley" on the strength of a RATE COINCIDENCE: the carried member sits at 0.9972x the seed and
/// the store-only reference measures 0.9968x. Close is not the same, and "a cheap variant that
/// happens to cost about what a store costs" is a completely different claim from "a store".
/// Counting the primitives answers it directly instead of inferring it from a number that merely
/// looks right.

/// Read the DECLARED search-track parameters. Panics rather than defaulting: see the config file.

/// IS MCTS WEAK, OR JUST UNDER-BUDGETED? `evolve mctsbudget [n1 n2 n3 depth]`
///
/// The valley probe measured UCT at 1/25 mates for 157M cost against alpha-beta's 25/25 for
/// 9.93e9 -- but that is 63x LESS COMPUTE, so it is not a comparison, it is the same equal-cost
/// error as scoring each net width against its own origin. `run(prog, pos, budget)` passes 16, and
/// the UCT program spends playouts against `Budget`; alpha-beta ignores it and recurses on the
/// depth table. So the two seeds were never given the same resources.
///
/// This sweeps the budget and reports mates AND cost, so the question becomes the right one: at
/// the cost alpha-beta actually spends, how many of the 25 does MCTS get? If it approaches 25 the
/// second lineage is viable and its guard is meaningful. If it stays near 1 even at matched cost,
/// then seeding a lineage with it creates a population whose mate guard is `f >= 1` -- vacuous --
/// and that is a reason to say so rather than to build it and watch it degenerate.
fn mcts_budget() {
    let a = |i: usize, d: i64| std::env::args().nth(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (n1, n2, n3, depth) = (a(2, 15) as usize, a(3, 5) as usize, a(4, 5) as usize, a(5, 3));
    let net = Net::random(32, 20260907);
    let mut set = mate_set(n1);
    set.extend(disagreement_set(n2, depth, &net, 4_000));
    set.extend(window_sensitive_set(n3, depth, &net, 8, 4_000));
    let ab = reference::bare_alpha_beta();
    let (abf, abc, abr) = fitness(&ab, &set, &net, depth, 16);
    println!("=== MCTS budget sweep, {} positions at depth {depth} ===", set.len());
    println!("  reference: bare alpha-beta {abf}/{} mates, {abc} cost, {abr:.6} mates/Mcost",
             set.len());
    println!("\n  {:>10} {:>7} {:>16} {:>14} {:>12}", "budget", "mates", "cost", "mates/Mcost",
             "cost vs AB");
    let mcts = reference::uct_mcts();
    // 512 and 2048 added after the exploration-term fix moved the matched-cost point:
    // budget 256 fell from 0.858x to 0.413x of alpha-beta's cost, so the value that justified
    // budget_mcts = 256 no longer holds and the parity point has to be re-found, not interpolated.
    for b in [16i64, 64, 256, 512, 1024, 2048, 4096] {
        let mut it = Interp::new(&net, vec![depth, 32_000, 8]);
        let (mut found, mut cost) = (0u32, 0u64);
        for (p, forcing) in &set {
            let mv = it.run(&mcts, p, b);
            cost += it.cost;
            match forcing {
                Some(best) => { if mv == *best { found += 1; } }
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
        let rate = found as f64 * 1e6 / cost.max(1) as f64;
        println!("  {b:>10} {found:>7} {cost:>16} {rate:>14.6} {:>11.3}x",
                 cost as f64 / abc.max(1) as f64);
    }
    println!("\n  Approaching {}/{} at cost ~1.0x AB => the lineage is viable and its guard bites.",
             set.len(), set.len());
    println!("  Stuck near 1 at matched cost => a lineage seeded here has a VACUOUS mate guard");
    println!("  (f >= 1), which is the degenerate-optimiser regime the guards exist to prevent.");
}


/// A5 -- THE VALLEY PROBE AS A STANDING TEST. `evolve valleyall [n1 n2 n3 depth]`
///
/// The one-off `valley` mode answered the question for hash reuse only. This asks it of EVERY
/// reference program in GRAMMAR 6: is this rung monotone-reachable from the seed, or does it sit
/// behind a conjunctive valley that a strict hill climb cannot cross?
///
/// A rung is MONOTONE if it is fitter than the seed and can be approached by fitter steps; it is
/// CONJUNCTIVE if the whole is fitter than the seed while its parts are not. Hash reuse is the
/// worked example -- 1.024x whole, 0.991x and 0.997x in halves -- and the decomposition that
/// exposed it (probe-only / store-only) is specific to a transposition table. There is no generic
/// "single-primitive decomposition" of an arbitrary program, so this reports what CAN be measured
/// for all of them (fitness vs the seed, and node distance) and the conjunctive test only where a
/// decomposition exists. Claiming a general decomposition would be inventing an instrument.
///
/// Run this against the population loop after the lineage and crossover work: the success
/// criterion is hash reuse ASSEMBLED by the population without a gadget operator. If it is not,
/// that is a result about the prior, not a failure to fix -- and this is the measurement that
/// says so with numbers rather than an impression from a log.
fn valley_all() {
    let a = |i: usize, d: i64| std::env::args().nth(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (n1, n2, n3, depth) = (a(2, 15) as usize, a(3, 5) as usize, a(4, 5) as usize, a(5, 3));
    let net = Net::random(32, 20260907);
    let mut set = mate_set(n1);
    set.extend(disagreement_set(n2, depth, &net, 4_000));
    set.extend(window_sensitive_set(n3, depth, &net, 8, 4_000));

    let seed = reference::bare_alpha_beta();
    let (sf, _sc, sr) = fitness(&seed, &set, &net, depth, 16);
    let sn = seed.size() as i64;
    println!("=== GRAMMAR 6 ladder: monotone or conjunctive? {} positions at depth {depth} ===",
             set.len());
    println!("  seed bare alpha-beta: {sf}/{} mates, {sr:.6} mates/Mcost, {sn} nodes\n",
             set.len());
    // THE CONTROL THE HARD SET NEEDS: is it winnable by ANY program we have? An unsaturated
    // dimension is useless if nothing reachable can score on it. capture_extension is the specific
    // hope -- it searches deeper on tactical lines, which is exactly what these positions require.
    let hard = harder_set(8, depth, &net, 3_000);
    let (seed_hard, _, _) = fitness(&seed, &hard, &net, depth, 16);
    println!("  HARD set: {} positions, seed scores {seed_hard}/{} (0 expected -- by construction)",
             hard.len(), hard.len());
    println!("  {:<34} {:>6} {:>7} {:>13} {:>9} {:>6}  {}", "program", "nodes", "mates",
             "mates/Mcost", "vs seed", "hard", "verdict");
    for (name, prog) in reference::all() {
        // UCT is scored at ITS declared budget, for the same reason the lineage is: it spends
        // playouts against `budget` and alpha-beta ignores it. Scoring it at 16 would report the
        // 1/25 artifact as though it were the program's strength.
        let bud = if name.contains("MCTS") { 256 } else { 16 };
        let (f, _c, r) = fitness(&prog, &set, &net, depth, bud);
        let ratio = r / sr.max(1e-12);
        let verdict = if f < sf {
            "loses answers -- not a rung at this depth"
        } else if ratio > 1.0 {
            "FITTER than the seed"
        } else {
            "not fitter"
        };
        let (hf, _, _) = fitness(&prog, &hard, &net, depth, bud);
        println!("  {name:<34} {:>+6} {f:>7} {r:>13.6} {ratio:>8.3}x {hf:>6}  {verdict}",
                 prog.size() as i64 - sn);
    }

    println!("\n  CONJUNCTIVE TEST -- only where a decomposition exists (hash reuse):");
    let halves = [
        ("probe only (never stores)", reference::ab_probe_only()),
        ("store only (never probes)", reference::ab_store_only()),
        ("hash reuse (both halves)", reference::ab_hash()),
    ];
    let mut whole = 0.0;
    let mut parts_max: f64 = 0.0;
    for (name, prog) in halves {
        let (f, _c, r) = fitness(&prog, &set, &net, depth, 16);
        let ratio = r / sr.max(1e-12);
        println!("    {name:<30} {f:>3} mates  {ratio:.3}x");
        if name.starts_with("hash") { whole = ratio; } else { parts_max = parts_max.max(ratio); }
    }
    if whole > 1.0 && parts_max < 1.0 {
        println!("    => CONJUNCTIVE: whole {whole:.3}x fitter, best part {parts_max:.3}x is not.");
        println!("       A strict `rate > best_rate` climb cannot take the first step. This is the");
        println!("       measurement the plateau tolerance exists to answer.");
    } else if whole > 1.0 {
        println!("    => MONOTONE: a part is already fitter, so the rung is reachable by hill climbing.");
    } else {
        println!("    => NOT A RUNG at this depth: the whole is not fitter than the seed.");
    }
    println!("\n  No generic single-primitive decomposition is attempted for the other rungs.");
    println!("  probe-only/store-only is specific to a transposition table; inventing an");
    println!("  equivalent for ID or capture extension would be inventing an instrument.");
}


/// DOES ALPHA-BETA'S EXACTNESS ACTUALLY HOLD HERE? `evolve moveagree [n depth]`
///
/// search_track_WHY_NOTHING.md claims the MAIN lineage has no correctness gradient BECAUSE
/// alpha-beta is exact: every correct variant at the same depth returns the same move, so
/// correctness cannot discriminate and only cost varies. That claim is currently INFERRED -- from
/// theory, plus the coincidence that every exact variant scores 25/25 on the guard set and 0/8 on
/// the hard set.
///
/// Inference is not measurement, and this is the strongest claim of the session, so it gets the
/// control it deserves: run every reference program on the same positions and compare the MOVE it
/// returns against the seed's, position by position.
///
/// PRE-REGISTERED:
///   * CONFIRMED if the exact variants (hash reuse, ID, hash+ID) agree with the seed on 100% of
///     positions. Then behavioural identity is measured, not argued, and the account holds.
///   * REFUTED if any of them disagrees anywhere. Then "identical" is too strong -- the
///     interpreter's hash table, integer arithmetic or move ordering breaks exactness somewhere --
///     and the conclusion needs weakening to "nearly always identical", which is a different and
///     weaker claim about why the fitness cannot discriminate.
///   * The INEXACT variants (capture extension, table reduction) are expected to disagree
///     SOMETIMES. If they never disagree, they are not changing the search at all and their 0.985x
///     and 0.993x are pure overhead, which would be worth knowing separately.
fn move_agree() {
    let a = |i: usize, d: i64| std::env::args().nth(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (n, depth) = (a(2, 40) as usize, a(3, 3));
    let net = Net::random(32, 20260907);
    let mut rng: u64 = 0x51A7_E5EE;
    let mut set = Vec::new();
    while set.len() < n {
        let mut p = Position::startpos();
        for _ in 0..(10 + rng % 34) {
            let l = p.legal_moves();
            if l.is_empty() { break; }
            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
            p.make_move(l.as_slice()[(rng % l.len() as u64) as usize]);
        }
        if !p.legal_moves().is_empty() { set.push(p); }
    }
    // RAISE THE PER-RUN COST CAP. The default is 2e9 and a depth-4 search costs ~1.67e9 per
    // position, so at depth 4 the ceiling truncated 17-19 of 20 searches and `run` returned
    // MOVE_NONE. The first version of this instrument counted MOVE_NONE == MOVE_NONE as AGREEMENT,
    // which is how it reported ab_hash at 18/20 and nearly had me file a transposition-table
    // soundness bug. The 2 "disagreements" were positions where ab_hash COMPLETED and the seed did
    // not -- because ab_hash is cheaper. A point in its favour, read as a leak.
    let cap: u64 = 50_000_000_000;
    let seed = reference::bare_alpha_beta();
    let seed_moves: Vec<board::Move> = set.iter().map(|p| {
        let mut it = Interp::new(&net, vec![depth, 32_000, 8]);
        it.cost_cap = cap;
        it.run(&seed, p, 16)
    }).collect();

    let seed_cost: u64 = set.iter().map(|p| {
        let mut it = Interp::new(&net, vec![depth, 32_000, 8]);
        it.cost_cap = cap;
        it.run(&seed, p, 16);
        it.cost
    }).sum();
    println!("=== move agreement with the seed, {n} positions at depth {depth} ===");
    println!("  EXACT variants must agree 100% if alpha-beta's exactness holds in this interpreter.\n");
    println!("  {:<34} {:>10} {:>9} {:>6} {:>12} {:>8}  {}", "program", "agree", "pct", "noMove",
             "evals", "cost", "class");
    for (name, prog) in reference::all() {
        let bud = if name.contains("MCTS") { 512 } else { 16 };
        // COUNT MOVE_NONE SEPARATELY. Without this a program that hit the 2e9 per-run COST CAP
        // and returned no move at all was scored as a "disagreement", which is a resource artefact
        // and not a difference of opinion. At depth 4 the seed costs ~1.67e9 per position against
        // that 2e9 ceiling, so it is close enough for variance to push individual positions over --
        // and this instrument reported ab_hash at 18/20 and had me an inch from filing a
        // transposition-table soundness bug that the value check then could not reproduce.
        let (mut agree, mut none) = (0usize, 0usize);
        for (p, sm) in set.iter().zip(&seed_moves) {
            let mut it = Interp::new(&net, vec![depth, 32_000, 8]);
            it.cost_cap = cap;
            let mv = it.run(&prog, p, bud);
            if mv == board::types::MOVE_NONE { none += 1; }
            else if mv == *sm { agree += 1; }
        }
        let class = if name.contains("hash") || name.contains("deepening") || name.contains("main seed") {
            "EXACT -- must be 100%"
        } else if name.contains("capture") || name.contains("reduction") {
            "inexact -- may differ"
        } else { "different paradigm" };
        // EVALS AND COST, because "agrees 100%" and "costs 1.5% more" together are suspicious for
        // a CAPTURE EXTENSION. A real one re-searches every capture at the horizon and should move
        // node counts substantially in midgame positions. If its eval count is within a whisker of
        // the seed's, it is barely firing -- which would make GRAMMAR 6's "faithful" label wrong
        // and would explain the 100% agreement as a non-event rather than a finding about
        // extensions.
        let (mut ev, mut cost) = (0u64, 0u64);
        for p in &set {
            let mut it = Interp::new(&net, vec![depth, 32_000, 8]);
            it.cost_cap = cap;
            it.run(&prog, p, bud);
            ev += it.evals;
            cost += it.cost;
        }
        println!("  {name:<34} {agree:>7}/{n:<3} {:>8.1}% {none:>6} {:>12} {:>7.3}x  {class}",
                 100.0 * agree as f64 / n as f64, ev, cost as f64 / seed_cost.max(1) as f64);
    }
    println!("\n  100% for the exact variants => behavioural identity MEASURED, and correctness");
    println!("  genuinely cannot discriminate among them at fixed depth.");
    println!("  Anything below 100% => 'identical' is too strong and the account needs weakening.");
}


/// THE REAL ALARM: is the TT's move disagreement a VALUE disagreement? `evolve ttvalue [n depth]`
///
/// Two independent instruments now agree on the symptom. `evolve moveagree` at depth 4: ab_hash
/// returns a different move from the seed on 2 of 20 positions, while every other exact variant
/// agrees 20/20. `tt_pressure`: 60/60 at depths 2 and 3, **59/60 at depth 4**, with collisions
/// rising 691 -> 159,208 -> 1,879,089.
///
/// Neither settles what matters, and tt_pressure says so itself: "agreement < n does NOT
/// automatically mean a leak: alpha-beta can return a DIFFERENT best move of EQUAL value, and a TT
/// changes which one is found first. A value disagreement would be the real alarm; this checks the
/// cheaper proxy." Nobody has run the real alarm, and it decides whether the ONE rung measured as
/// fitter than the seed -- and therefore the whole valley analysis built on it -- rests on a sound
/// program or on a leak.
///
/// The check: where the two disagree, score BOTH moves with an exact oracle and compare values.
/// `Searcher::best_move_capped` returns (Move, Score) and is hand-written exact alpha-beta, so
/// v(m) = -search(apply(p, m), depth-1) is the value of playing m.
///
/// PRE-REGISTERED:
///   * SOUND (tie-breaking) if the two moves have EQUAL value everywhere they differ. Then the TT
///     is fine, alpha-beta simply returns a different member of an equal-valued set once cutoff
///     order changes, and the 1.024x rung stands.
///   * UNSOUND (a leak) if any disagreement is a value difference. Then ab_hash returns a WORSE
///     move, its cheapness is partly bought by being wrong, and every result resting on it --
///     the ladder's only fitter rung, ladder_valley_RESULT.md, the conjunctive-valley claim, and
///     the plateau tolerance justified by it -- needs re-examining.
fn tt_value() {
    let a = |i: usize, d: i64| std::env::args().nth(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (n, depth) = (a(2, 40) as usize, a(3, 4));
    let net = Net::random(32, 20260907);
    let mut rng: u64 = 0x51A7_E5EE;
    let mut set = Vec::new();
    while set.len() < n {
        let mut p = Position::startpos();
        for _ in 0..(10 + rng % 34) {
            let l = p.legal_moves();
            if l.is_empty() { break; }
            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
            p.make_move(l.as_slice()[(rng % l.len() as u64) as usize]);
        }
        if !p.legal_moves().is_empty() { set.push(p); }
    }
    let seed = reference::bare_alpha_beta();
    let hash = reference::ab_hash();
    // Exact oracle for the VALUE of a root move: -search(child, depth-1), uncapped.
    let value_of = |p: &Position, m: board::Move| -> i32 {
        let mut q = p.clone();
        q.make_move(m);
        let mut s = pipeline::search::Searcher::with_seed(1);
        let (_, sc) = s.best_move_capped(&mut q, (depth as u32).saturating_sub(1), &net, u64::MAX, 1);
        -(sc as i32)
    };

    println!("=== TT value check: {n} positions at depth {depth} ===");
    let (mut diffs, mut value_diffs) = (0usize, 0usize);
    for p in &set {
        let mut ia = Interp::new(&net, vec![depth, 32_000, 8]);
        let ma = ia.run(&seed, p, 16);
        let mut ib = Interp::new(&net, vec![depth, 32_000, 8]);
        let mb = ib.run(&hash, p, 16);
        if ma == mb || ma == board::types::MOVE_NONE || mb == board::types::MOVE_NONE { continue; }
        diffs += 1;
        let (va, vb) = (value_of(p, ma), value_of(p, mb));
        let verdict = if va == vb { "EQUAL VALUE -- tie-break, sound" } else { "VALUE DIFFERS -- LEAK" };
        if va != vb { value_diffs += 1; }
        println!("  disagreement {diffs}: seed move value {va:+}, hash move value {vb:+}  ({verdict})");
    }
    println!("\n  move disagreements: {diffs}/{n}");
    println!("  of those, VALUE disagreements: {value_diffs}");
    if diffs == 0 {
        println!("  => no disagreement at all in this sample; nothing to judge.");
    } else if value_diffs == 0 {
        println!("  => SOUND. Every disagreement is an equal-valued alternative, which is exactly");
        println!("     what a TT does to cutoff order. The 1.024x rung and the valley result stand.");
    } else {
        println!("  => UNSOUND. ab_hash returns a move of DIFFERENT value, so part of its cheapness");
        println!("     is bought by being wrong. Everything resting on that rung needs re-examining.");
    }
}

fn read_declared(path: &str) -> (usize, f64, i64, i64) {
    let txt = std::fs::read_to_string(path).unwrap_or_else(|e| {
        panic!("declared parameters missing at {path}: {e}. This file is part of the Given \
                column; running without it would silently substitute a default for a choice that \
                is supposed to be visible.")
    });
    let mut mu = None;
    let mut eps = None;
    let mut bmain = None;
    let mut bmcts = None;
    for line in txt.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() { continue; }
        let (k, v) = match line.split_once('=') { Some(kv) => kv, None => continue };
        match k.trim() {
            "mu" => mu = Some(v.trim().parse().unwrap_or_else(|e| panic!("mu does not parse: {e}"))),
            "eps" => eps = Some(v.trim().parse().unwrap_or_else(|e| panic!("eps does not parse: {e}"))),
            "budget_main" => bmain = Some(v.trim().parse().unwrap_or_else(|e| panic!("budget_main does not parse: {e}"))),
            "budget_mcts" => bmcts = Some(v.trim().parse().unwrap_or_else(|e| panic!("budget_mcts does not parse: {e}"))),
            _ => {}
        }
    }
    (mu.expect("configs/search_track.conf declares no `mu`"),
     eps.expect("configs/search_track.conf declares no `eps`"),
     bmain.expect("configs/search_track.conf declares no `budget_main`"),
     bmcts.expect("configs/search_track.conf declares no `budget_mcts`"))
}


/// Positions the SEED GETS WRONG at the fitness depth — the gradient the fitness has never had.
///
/// WHY THIS EXISTS. search_track_WHY_NOTHING.md measured the MAIN lineage as having no usable
/// gradient at all: the seed scores 25/25 on the existing set, the guard is `f >= best_found`, so
/// every survivor also scores exactly 25 and MATES CANNOT DISCRIMINATE. Selection then falls
/// entirely to cost, and across 93 generations with survivors, ZERO were ever cheaper than the
/// champion. One dimension saturated, the other blocked.
///
/// `disagreement_set` already finds positions whose answer changes with depth, but it records the
/// answer at the FITNESS depth, so the seed is right on them by construction — it is a guard, not
/// a gradient. This records the answer one ply DEEPER, so the seed is WRONG on them by
/// construction, and a candidate that searches better can be right.
///
/// THAT IS THE POINT: it rewards SEARCHING BETTER rather than merely searching cheaper. A capture
/// extension resolves a tactical line the flat-depth seed truncates, so it can convert here; a
/// program that just prunes more cannot. Rung 6 of the GRAMMAR 9 ladder becomes reachable as an
/// improvement instead of scoring 0.985x and being discarded.
///
/// Still rules-derived and chess-blind: the "correct" answer is the SEED's own answer at depth+1.
/// No human labels, no engine oracle, no chess knowledge — the same self-calibrating construction
/// as the depth and window guards, which is what makes it legitimate under the tabula-rasa rule.
fn harder_set(n: usize, depth: i64, net: &Net, cap: usize)
    -> Vec<(Position, Option<board::Move>)> {
    let ab = reference::bare_alpha_beta();
    let mut rng: u64 = 0x4A8D_3117;
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
        let mut here = Interp::new(net, vec![depth, 32_000, 8]);
        let a = here.run(&ab, &p, 16);
        let mut deeper = Interp::new(net, vec![depth + 1, 32_000, 8]);
        let b = deeper.run(&ab, &p, 16);
        // Both real answers, and they must DIFFER: the deeper one is recorded as correct, so the
        // seed at the fitness depth scores zero here.
        if a != board::types::MOVE_NONE && b != board::types::MOVE_NONE && a != b {
            out.push((p.clone(), Some(b)));
        }
    }
    out
}

fn tt_prims(p: &Program) -> usize {
    fn walk(n: &Node) -> usize {
        use Node::*;
        let here = matches!(n, Probe(_) | Store(..) | Key(_) | Field(..)) as usize;
        here + match n {
            Budget | Const(_) | Var(_) | OutcomeLit(_) | Nop => 0,
            Moves(a) | Terminal(a) | Key(a) | Eval(a) | Ret(a) | Probe(a) | Field(a, _)
            | Set(_, a) => walk(a),
            Apply(a, b) | Max(a, b) | Min(a, b) | Avg(a, b) | ScoreOf(a, b) | Cmp(a, b, _)
            | Pred(a, b, _) | Loop(a, b) | Store(a, _, b) => walk(a) + walk(b),
            Mix(a, b, c) => walk(a) + walk(b) + walk(c),
            Foreach(a, _, b) | Argmax(a, _, b) | Sort(a, _, b) | Sample(a, _, b)
            | Let(_, a, b) => walk(a) + walk(b),
            If(c, t, e) => walk(c) + walk(t) + e.as_ref().map_or(0, |x| walk(x)),
            Call(_, args) | Arith(_, args) | TRead(_, args) => args.iter().map(walk).sum(),
        }
    }
    p.funcs.iter().map(|f| walk(&f.body)).sum()
}

fn valley() {
    let a = |i: usize, d: i64| std::env::args().nth(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (n1, n2, n3, depth) = (a(2, 15) as usize, a(3, 5) as usize, a(4, 5) as usize, a(5, 3));
    let net = Net::random(32, 20260907);
    let mut set = mate_set(n1);
    set.extend(disagreement_set(n2, depth, &net, 4_000));
    set.extend(window_sensitive_set(n3, depth, &net, 8, 4_000));
    println!("=== ladder valley probe: {} positions ({n1} mate-in-1, {n2} depth-requiring, \
              {n3} window-sensitive) at depth {depth} ===", set.len());

    let progs: Vec<(&str, Program)> = vec![
        ("bare alpha-beta (seed)", reference::bare_alpha_beta()),
        ("probe only (never stores)", reference::ab_probe_only()),
        ("store only (never probes)", reference::ab_store_only()),
        ("hash reuse (both halves)", reference::ab_hash()),
        // THE SECOND LINEAGE'S SEED, measured before any machinery is built around it. GRAMMAR 6
        // records UCT as PARTIAL -- 20/23 forced mates, not 23/23 -- and this set demands the
        // exact answer on all 25. If MCTS scores far below the seed's 25 here, a lineage seeded
        // with it starts with its own best_found and can still climb, but it cannot be compared
        // to MAIN on mates and that has to be known in advance rather than discovered as a
        // confusing log line.
        ("UCT MCTS (2nd lineage seed)", reference::uct_mcts()),
    ];
    let (_, _, base_rate) = fitness(&progs[0].1, &set, &net, depth, 16);
    let base_nodes = progs[0].1.size() as i64;

    println!("\n  {:<28} {:>6} {:>7} {:>16} {:>14} {:>10}",
             "program", "nodes", "mates", "cost", "mates/Mcost", "vs seed");
    for (name, p) in &progs {
        let (found, cost, rate) = fitness(p, &set, &net, depth, 16);
        let nodes = p.size() as i64;
        println!("  {name:<28} {:>+6} {found:>7} {cost:>16} {rate:>14.6} {:>9.3}x",
                 nodes - base_nodes, rate / base_rate.max(1e-12));
    }
    println!("\n  'vs seed' is mates-per-cost RELATIVE TO THE SEED: >1.000 is FITTER, and fitter is");
    println!("  the only thing `evolve` accepts (it requires rate > best_rate, STRICTLY).");
    println!("  Both halves below 1.000 => the rung sits at the bottom of a VALLEY, and no mutation");
    println!("  operator can make it reachable by this search. Fitness is deterministic here, so");
    println!("  these ratios are exact for this set rather than estimates with error bars.");
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("valley") {
        return valley();
    }
    if std::env::args().nth(1).as_deref() == Some("mctsbudget") {
        return mcts_budget();
    }
    if std::env::args().nth(1).as_deref() == Some("valleyall") {
        return valley_all();
    }
    if std::env::args().nth(1).as_deref() == Some("moveagree") {
        return move_agree();
    }
    if std::env::args().nth(1).as_deref() == Some("ttvalue") {
        return tt_value();
    }
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
    // Third component: window-sensitive positions, closing the raised-alpha exploit.
    let n3: usize = std::env::args().nth(6).and_then(|s| s.parse().ok()).unwrap_or(5);
    // GAME-GATE PAIRS. Small on purpose: a game at fitness depth is ~200x a single fitness
    // evaluation, so this is the expensive half and it only runs on a surrogate improvement.
    // 6 pairs = 12 games resolves a large effect, which is the only kind worth promoting here;
    // it CANNOT resolve a 2% edge and is not asked to. It is a veto on unplayable programs.
    let gate_pairs: usize = std::env::args().nth(7).and_then(|s| s.parse().ok()).unwrap_or(6);
    let win = window_sensitive_set(n3, depth, &net, 8, 4_000);
    // THE UNSATURATED DIMENSION, measured before anything is built on it. The seed is WRONG on
    // these by construction (the recorded answer is its own at depth+1), so unlike the 25/25 guard
    // set they can DISCRIMINATE. Scored and reported per generation; acceptance is NOT changed
    // yet, because the claim "a better-searching candidate can win these" is exactly the sort of
    // thing that should be measured before a fitness is restructured around it.
    let hard = harder_set(8, depth, &net, 3_000);
    let n_win = win.len();
    let n_deep = deep.len();
    set.extend(deep);
    set.extend(win);

    // ---- DECLARED PARAMETERS FIRST: everything below depends on them.
    let (mu, eps, budget_main, budget_mcts) = read_declared("configs/search_track.conf");
    let (MU, EPS) = (mu, eps);

    // ---- TWO LINEAGES, each with its own seed, budget and population.
    //
    // GRAMMAR 6 records both as declared seeds with the same status: bare alpha-beta (71 nodes)
    // and UCT MCTS (130). NEITHER is a hybrid -- seeding one would answer the question the search
    // track exists to ask. If a hybrid appears it must be built by crossover or mutation.
    //
    // BUDGETS DIFFER BY LINEAGE AND THAT IS THE POINT. Alpha-beta ignores `budget` and recurses on
    // the depth table; UCT spends playouts against it. A single global 16 scored the UCT seed at
    // 1/25 for 1.6% of alpha-beta's cost -- and 2.525x its mates-per-cost, i.e. the fittest thing
    // on the board by being cheap and wrong. At the declared 256 the seeds spend comparable cost
    // and UCT scores 12/25, giving that lineage a mate guard of f >= 12 rather than a vacuous
    // f >= 1.
    //
    // EACH LINEAGE'S MATE GUARD IS ITS OWN SEED'S SCORE. Holding MCTS to alpha-beta's 25 would
    // freeze it permanently; holding alpha-beta to 12 would gut its guard. Cross-lineage
    // comparison by RATE is therefore meaningless and is never done -- only the game gate
    // promotes, and it plays programs against each other on a board.
    struct Lineage {
        name: &'static str,
        budget: i64,
        popn: Vec<(Program, u32, f64)>,
        champ: Program,
        best_found: u32,
        best_rate: f64,
        accepted: usize,
    }
    let mut lineages: Vec<Lineage> = Vec::new();
    for (name, seed_prog, bud) in [
        ("MAIN", reference::bare_alpha_beta(), budget_main),
        ("MCTS", reference::uct_mcts(), budget_mcts),
    ] {
        let (f, c, r) = fitness(&seed_prog, &set, &net, depth, bud);
        println!("  lineage {name:<5} seed {:>3} nodes, budget {bud:<5} -> {f}/{} mates, {c} cost, \
{r:.6} mates/Mcost", seed_prog.size(), set.len());
        lineages.push(Lineage {
            name,
            budget: bud,
            popn: vec![(seed_prog.clone(), f, r); MU],
            champ: seed_prog,
            best_found: f,
            best_rate: r,
            accepted: 0,
        });
    }
    let (seed_hard, _, _) = fitness(&reference::bare_alpha_beta(), &hard, &net, depth, budget_main);
    println!("  HARD set: {} positions the seed FAILS by construction; seed scores {seed_hard}/{} \
(a real gradient, unlike the saturated 25/25 guard set)", hard.len(), hard.len());
    println!("  population MU={MU}, lambda={pop}, plateau tolerance EPS={EPS:.3} \
(deepest measured valley half is 0.009)");

    // RECORD panics, do not silence them. The first version of this hook discarded the message
    // entirely, and the very next crash was therefore INVISIBLE -- the run died with no diagnostic
    // at all, which is strictly worse than the backtrace spam it was avoiding. Append one line per
    // panic to a file instead: out of the results, still on disk.
    std::panic::set_hook(Box::new(|info| {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true).append(true).open("search_track_panics.log")
        {
            let _ = writeln!(f, "{info}");
        }
    }));

    // COST CEILING DELIBERATELY OFF, and the reason is a finding rather than an omission.
    //
    // I added this ceiling to make the game gate reward cost-efficiency, because the surrogate
    // scores mates-per-COST while the gate gave both sides the same BUDGET -- so a candidate could
    // be 4.4x cheaper, max the surrogate, and play identical games. Setting it to 4e8 did nothing:
    // the alpha-beta seed costs 3.972e8 per position and the MCTS seed 3.843e8, so neither ever
    // reached the ceiling. The mechanism was INERT, and I nearly reported it as working.
    //
    // LOWERING IT UNTIL IT BINDS IS WORSE, NOT BETTER. interp/src/lib.rs:501 unwinds the whole
    // program on a ceiling hit and `run` reports MOVE_NONE, which play_progs treats as a FORFEIT.
    // So a binding ceiling does not hand the cheaper program more search -- it makes whichever
    // program crosses the line first LOSE THE GAME OUTRIGHT. That converts the gate into a pure
    // cost race, which is exactly the surrogate failure mode the gate exists to counteract.
    //
    // THE REAL REASON COST WILL NOT CONVERT: a DEPTH-LIMITED program cannot spend a saving. The
    // alpha-beta seed searches to the depth in table 0 and ignores `Budget` entirely, so costing
    // half as much means finishing sooner with the IDENTICAL move, not searching twice as far.
    // Efficiency only becomes strength for a BUDGET-AWARE program -- one that searches until its
    // allowance is gone. That is what iterative deepening is, and it is rung 5 of the GRAMMAR 9
    // ladder, currently measured at 0.914x (a loss) on the surrogate.
    //
    // So FITNESS 3's cost term is rewarding a property that cannot become playing strength for the
    // programs this track actually evolves. That is a statement about the FITNESS, not about the
    // gate, and it is recorded in FITNESS.md rather than patched over here. The plumbing stays
    // because it is correct for a budget-aware seed; the value is off so nothing pretends to work.
    const COST_PER_MOVE: u64 = u64::MAX;

    let mut rng = Rng::new(0xE0FFEE);
    for g in 1..=gens {
        // Snapshot every lineage's programs BEFORE this generation, so crossover donors are drawn
        // from a fixed set rather than from populations mutating underneath the loop -- otherwise
        // whether a graft is possible depends on lineage order, which is not a property anyone
        // declared.
        let donors: Vec<Program> = lineages
            .iter()
            .flat_map(|l| l.popn.iter().map(|(p, _, _)| p.clone()))
            .collect();

        for li in 0..lineages.len() {
            let (bud, best_found, best_rate) =
                (lineages[li].budget, lineages[li].best_found, lineages[li].best_rate);
            let popsnap = lineages[li].popn.clone();

            // MUTATE OR CROSS, then evaluate in parallel. One in four proposals is a crossover:
            // the hybrid, if it exists, is only reachable this way, but crossover between two
            // programs that already work is far more destructive than a single mutation, so it
            // does not get to crowd out the operator set.
            let cands: Vec<Program> = (0..pop)
                .filter_map(|i| {
                    let parent = &popsnap[i % popsnap.len()].0;
                    let mut r = Rng::new((g as u64) << 20 ^ (li as u64) << 16 ^ i as u64 ^ 0xBEEF);
                    if i % 4 == 3 && donors.len() > 1 {
                        let d = &donors[(r.next() as usize) % donors.len()];
                        mutate::crossover(parent, d, &mut r)
                    } else {
                        mutate::mutate_program(parent, &mut r)
                    }
                })
                .collect();
            let ill = pop - cands.len();
            const THREADS: usize = 3;
            let chunk = cands.len().div_ceil(THREADS).max(1);
            let scored: Vec<(Program, u32, f64, u32)> = std::thread::scope(|sc| {
                let handles: Vec<_> = cands
                    .chunks(chunk)
                    .map(|part| {
                        let (set, net, hard) = (&set, &net, &hard);
                        sc.spawn(move || {
                            part.iter()
                                .map(|c| {
                                    // BELT AND BRACES, AND THE BRACES ARE INERT HERE.
                                    //
                                    // This catch_unwind CANNOT catch anything under the workspace's
                                    // `panic = "abort"` release profile (Cargo.toml): the process
                                    // aborts before unwinding. I added it believing it fixed the
                                    // crashes and reported it as working; it never ran. The real
                                    // fix is that the interpreter's Pos accessors are now TOTAL,
                                    // so there is no panic to catch.
                                    //
                                    // Kept because it costs nothing, it is correct if the profile
                                    // ever changes to unwind, and deleting it would remove the
                                    // record of why it is not the fix.
                                    //
                                    // A CANDIDATE THAT PANICS SCORES ZERO. It does not kill the run.
                                    //
                                    // Crossover produces programs that pass the TYPE CHECKER and
                                    // still violate a runtime invariant -- an unbound variable
                                    // types as Unit and then reaches an accessor expecting a Pos.
                                    // Two such crashes in one hour took the whole track down:
                                    // "make_move: empty from-square", then "type error: expected
                                    // Pos".
                                    //
                                    // Guarding each accessor as it is discovered is whack-a-mole
                                    // against a search whose entire job is to generate programs
                                    // nobody anticipated. This is the general form: whatever
                                    // invariant a candidate violates, it is caught here, scored
                                    // as the worst possible program, and rejected by the mate
                                    // guard on the next line. The search continues.
                                    //
                                    // Scoring 0 mates is exactly right rather than merely safe --
                                    // a program that cannot complete an evaluation has, in fact,
                                    // answered nothing.
                                    let r = std::panic::catch_unwind(
                                        std::panic::AssertUnwindSafe(|| fitness(c, set, net, depth, bud)),
                                    );
                                    match r {
                                        Ok((f, _cst, rate)) => {
                                            let (hf, _, _) = fitness(c, hard, net, depth, bud);
                                            (c.clone(), f, rate, hf)
                                        }
                                        Err(_) => (c.clone(), 0, 0.0, 0),
                                    }
                                })
                                .collect::<Vec<_>>()
                        })
                    })
                    .collect();
                handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
            });

            let n_scored = scored.len();
            // HARD-SET RANGE across candidates that pass the correctness guard. If this never
            // varies, the gradient does not exist and restructuring acceptance around it would
            // have achieved nothing -- which is why it is measured first.
            let hard_scores: Vec<u32> =
                scored.iter().filter(|(_, f, _, _)| *f >= best_found).map(|(_, _, _, h)| *h).collect();
            let (hlo, hhi) = (hard_scores.iter().min().copied().unwrap_or(0),
                              hard_scores.iter().max().copied().unwrap_or(0));
            let scored: Vec<(Program, u32, f64)> =
                scored.into_iter().map(|(p, f, r, _)| (p, f, r)).collect();
            let mate_ok = scored.iter().filter(|(_, f, _)| *f >= best_found).count();
            let rel: Vec<f64> = scored
                .iter()
                .filter(|(_, f, _)| *f >= best_found)
                .map(|(_, _, r)| r / best_rate.max(1e-12))
                .collect();
            let (rlo, rhi) = rel.iter().fold((f64::MAX, 0.0f64), |(a, b), x| (a.min(*x), b.max(*x)));
            let offspring: Vec<(Program, u32, f64)> =
                scored.into_iter().filter(|(_, f, _)| *f >= best_found).collect();

            let mut pool = popsnap.clone();
            pool.extend(offspring);
            pool.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
            let top = pool[0].2;
            pool.retain(|x| x.2 >= top * (1.0 - EPS));
            let mut seen = std::collections::HashSet::new();
            pool.retain(|x| seen.insert(format!("{:?}", x.0)));
            pool.truncate(MU);
            lineages[li].popn = pool;

            let popn = &lineages[li].popn;
            let (spread_lo, spread_hi) = (popn.last().unwrap().2, popn[0].2);
            let tt: Vec<usize> = popn.iter().map(|(p, _, _)| tt_prims(p)).collect();
            if popn[0].2 > best_rate {
                let (c, f, rate) = popn[0].clone();
                // THE GAME GATE RUNS EVOLVED PROGRAMS ON A BOARD, so it is exactly as exposed
                // to a malformed candidate as the fitness call is, and it was NOT wrapped. A
                // candidate that survives fitness can still violate an invariant once it is asked
                // to play 200 plies against another program.
                let gsc = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    gate::match_progs(&c, &lineages[li].champ, &net,
                                      vec![depth, 32_000, 8], bud, gate_pairs,
                                      0xC0FFEE ^ g as u64 ^ (li as u64) << 8, 4,
                                      COST_PER_MOVE)
                })) {
                    Ok(sc) => sc,
                    Err(_) => {
                        // Cannot finish a game => cannot be promoted. Same rule as fitness: the
                        // candidate scores as the worst possible program and the run continues.
                        println!("  gen {g:>3} {:<5} gate PANIC -- candidate cannot play, rejected",
                                 lineages[li].name);
                        lineages[li].best_rate = rate;
                        continue;
                    }
                };
                let resolved_up = gsc.pent_rate() - gsc.ci95() > 0.5;
                if !resolved_up {
                    println!("  gen {g:>3} {:<5} gate REJECT {:.3}+/-{:.3} ({} games)  surrogate {rate:.6}",
                             lineages[li].name, gsc.pent_rate(), gsc.ci95(), gsc.games());
                    lineages[li].best_rate = rate;
                    continue;
                }
                println!("  gen {g:>3} {:<5} ACCEPT  {f} mates  {rate:.6} ({} nodes, was {:.6})  gate {:.3}",
                         lineages[li].name, c.size(), best_rate, gsc.pent_rate());
                lineages[li].champ = c.clone();
                lineages[li].best_found = f;
                lineages[li].best_rate = rate;
                lineages[li].accepted += 1;
                let _ = std::fs::write(
                    format!("evolved_{}_gen{g}.prog", lineages[li].name),
                    format!("// {f} mates, {rate:.6} mates/Mcost, {} nodes, gen {g}\n{:#?}\n",
                            c.size(), c));
            } else {
                let span = if rel.is_empty() { "none".to_string() }
                           else { format!("{rlo:.3}-{rhi:.3}x") };
                println!("  gen {g:>3} {:<5} ..none ({n_scored} cand, {ill} ill, mate-ok {mate_ok}, \
rates {span}, hard {hlo}-{hhi})  pop {} spread {:.6}-{:.6} tt{:?}",
                         lineages[li].name, popn.len(), spread_lo, spread_hi, tt);
            }
        }
    }

    let _ = rng.next();
    let _ = rng.next();
    println!("\n=== per-lineage summary over {gens} generations ===");
    for l in &lineages {
        println!("  {:<5} {} accepted   final {} mates {:.6} mates/Mcost ({} nodes)",
                 l.name, l.accepted, l.best_found, l.best_rate, l.champ.size());
    }
    println!("\n  Cross-lineage RATE comparison is meaningless and is not printed: the two seeds");
    println!("  run at different budgets against different mate guards (MAIN f>=25, MCTS f>=12).");
    println!("  Only the game gate compares programs, and it plays them on a board.");
}
