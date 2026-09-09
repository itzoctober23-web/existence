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
    // CHALLENGER IS SELECTABLE. This was hardcoded to ab_hash for the transposition-table alarm;
    // the same question -- "are the moves it plays DIFFERENTLY actually BETTER?" -- is now the live
    // one for capture extension, which is the first alpha-beta-family program measured to play
    // different chess (8/10 agreement at 1.679x cost, once `pred` was implemented and the rung was
    // moved to the horizon).
    let which = std::env::args().nth(4).unwrap_or_else(|| "hash".into());
    let (chall_name, hash) = match which.as_str() {
        "capture" => ("capture extension", reference::capture_extension()),
        "id" => ("iterative deepening", reference::ab_id()),
        _ => ("hash reuse", reference::ab_hash()),
    };
    println!("  challenger: {chall_name}");
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
        // For an EXACT variant a value difference is a leak. For an INEXACT one (an extension)
        // it is the whole point -- a different effective depth SHOULD change the value, and the
        // sign says whether the change is an improvement.
        let verdict = if va == vb { "EQUAL VALUE" }
                      else if vb > va { "CHALLENGER BETTER" }
                      else { "CHALLENGER WORSE" };
        if va != vb { value_diffs += 1; }
        println!("  disagreement {diffs}: seed {va:+}, challenger {vb:+}  ({verdict})");
    }
    println!("\n  move disagreements: {diffs}/{n}");
    println!("  of those, VALUE disagreements: {value_diffs}");
    if diffs == 0 {
        println!("  => no disagreement at all in this sample; nothing to judge.");
    } else if value_diffs == 0 {
        println!("  => SOUND. Every disagreement is an equal-valued alternative, which is exactly");
        println!("     what a TT does to cutoff order. The 1.024x rung and the valley result stand.");
    } else {
        println!("  => VALUE DIFFERENCES PRESENT. For an EXACT variant (hash reuse, ID) that is a");
        println!("     LEAK. For an INEXACT one (an extension) it is expected -- a different");
        println!("     effective depth SHOULD change values -- and this oracle CANNOT judge it:");
        println!("     it searches depth-1, SHALLOWER than the program it is judging, so it is");
        println!("     blind to exactly the tactics an extension exists to see. Use games.");
    }
}


/// PLAY TWO REFERENCE PROGRAMS AGAINST EACH OTHER. `evolve refmatch <pairs> <depth> <name>`
///
/// The value-oracle check cannot judge an EXTENSION. `ttvalue` scores a move with
/// `best_move_capped` at depth-1, which is SHALLOWER than the program being judged -- and a
/// capture extension exists precisely to see tactics a flat search misses, so a flatter judge is
/// blind to its whole purpose. Measured anyway and reported as inconclusive: 5/20 disagreements,
/// 2 better, 2 worse, 1 equal, which is what a blind judge produces.
///
/// Games need no oracle. Whoever wins, wins. This is the same `match_progs` the search track's
/// gate uses, so the number means the same thing as an in-loop gate result.
fn ref_match() {
    let a = |i: usize, d: i64| std::env::args().nth(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (pairs, depth) = (a(2, 24) as usize, a(3, 3));
    let which = std::env::args().nth(4).unwrap_or_else(|| "capture".into());
    let (name, chall) = match which.as_str() {
        "hash" => ("hash reuse", reference::ab_hash()),
        "id" => ("iterative deepening", reference::ab_id()),
        _ => ("capture extension", reference::capture_extension()),
    };
    let net = Net::random(32, 20260907);
    let seed = reference::bare_alpha_beta();
    println!("=== {name} vs bare alpha-beta, {pairs} pairs at depth {depth} ===");
    println!("  Same net and same budget both sides, so this measures the PROGRAM.");
    // GENEROUS BUT FINITE cost ceiling. u64::MAX let a single move run away: with quiescence, a
    // capture chain has no depth bound, and the first attempt at this match never finished a game.
    // 5e9 is ~12x the seed's cost for a whole position, so an ordinary move completes easily and
    // only a pathological one aborts.
    //
    // A capped program that runs out forfeits, and forfeits fall on the EXPENSIVE side
    // systematically -- capture extension costs 1.679x -- so a score built on them measures cost,
    // not play. The count is printed and the verdict is void if it is non-zero.
    let cap: u64 = std::env::args().nth(5).and_then(|s| s.parse().ok()).unwrap_or(5_000_000_000);
    gate::FORFEITS.store(0, std::sync::atomic::Ordering::Relaxed);
    let sc = gate::match_progs(&chall, &seed, &net, vec![depth, 32_000, 8], 16, pairs,
                               0x9E2D_1A77, 4, cap);
    let forfeits = gate::FORFEITS.load(std::sync::atomic::Ordering::Relaxed);
    println!("\n  {}W-{}D-{}L   rate {:.3} +/- {:.3}", sc.wins, sc.draws, sc.losses,
             sc.pent_rate(), sc.ci95());
    println!("  forfeits (ran out of budget): {forfeits}");
    if forfeits > 0 {
        println!("  => VERDICT VOID. Forfeits fall on the expensive side, so this measured COST.");
        println!("     Raise the ceiling and re-run before reading anything into the score.");
        return;
    }
    let up = sc.pent_rate() - sc.ci95() > 0.5;
    let down = sc.pent_rate() + sc.ci95() < 0.5;
    println!("  => {}", if up { "CHALLENGER STRONGER, resolved" }
                        else if down { "CHALLENGER WEAKER, resolved" }
                        else { "UNRESOLVED at this pair count -- needs more games, not a conclusion" });
    println!("\n  NOTE ON COST: this is a fixed-DEPTH match, so the extension's extra work is not");
    println!("  charged. capture extension costs 1.679x the seed, so a win here is a win per NODE,");
    println!("  not per unit of time. Both readings matter and they are different questions.");
}


/// CAN A SINGLE MUTATION CHANGE HOW THE PROGRAM PLAYS, WITHOUT BREAKING IT? `evolve stepdiff [n]`
///
/// This is the question the whole search track rests on and it has never been measured directly.
/// Everything so far measured OUTCOMES -- 93 generations with no survivor cheaper than the
/// champion, mates saturated at 25/25, the whole alpha-beta family playing identically. None of it
/// asked the prior question: does the operator set contain a step that is BOTH correctness-
/// preserving AND behaviour-changing?
///
/// If the answer is zero, no fitness can help. Selection needs candidates that differ in PLAY, and
/// a fitness cannot reward a difference that the mutation operators never produce. That would move
/// the blocker from FITNESS (where I have been putting it) to the OPERATOR SET.
///
/// Three buckets, and the middle one is the search's actual working material:
///   * BROKEN     -- loses at least one of the 25 guard answers. Correctly rejected.
///   * IDENTICAL  -- keeps all 25 and returns the SAME move everywhere. Passes the guard and is
///                   invisible to any play-based measure; only cost distinguishes it, and cost is
///                   unconvertible for a depth-limited program (docs/FITNESS.md).
///   * DIFFERENT  -- keeps all 25 and plays differently somewhere. THE ONLY USEFUL KIND.
fn step_diff() {
    let n: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(200);
    let depth: i64 = std::env::args().nth(3).and_then(|s| s.parse().ok()).unwrap_or(3);
    let net = Net::random(32, 20260907);
    let seed = reference::bare_alpha_beta();

    // Small position set: this runs hundreds of candidates, so it is deliberately cheap. It only
    // has to detect "plays differently ANYWHERE", not measure strength.
    let mut rng: u64 = 0x5D1F_F00D;
    let mut set = Vec::new();
    while set.len() < 8 {
        let mut p = Position::startpos();
        for _ in 0..(10 + rng % 24) {
            let l = p.legal_moves();
            if l.is_empty() { break; }
            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
            p.make_move(l.as_slice()[(rng % l.len() as u64) as usize]);
        }
        if !p.legal_moves().is_empty() { set.push(p); }
    }
    let base: Vec<board::Move> = set.iter().map(|p| {
        let mut it = Interp::new(&net, vec![depth, 32_000, 8]);
        it.cost_cap = 20_000_000_000;
        it.run(&seed, p, 16)
    }).collect();

    // THE REAL GUARD SET, not just "did it return a move". The first version of this instrument
    // sorted on whether a candidate returned MOVE_NONE, and its own doc comment claimed the
    // buckets meant "keeps all answers" -- they did not. The loop's guard is `f >= best_found` on
    // the 25-position mate/disagreement/window set, and the search track's measured mate-ok is
    // about 2/12, so most candidates that return a move still FAIL it. Reporting 35% as "the
    // useful kind" would have overstated the useful bucket several-fold.
    // TARGETED guard: alpha-sensitive instead of window-sensitive. The genuine-loss distribution
    // MUST be measured on the SAME guard the exploit losses were measured on, or the comparison is
    // cross-protocol -- the error this session has already produced four times. The exploit check
    // reports the alpha exploit losing 5 on this guard against 2 on the old one, so a "genuine
    // minimum of 2" carried over from the old guard would prove nothing about the new one.
    let mut guard = mate_set(15);
    guard.extend(disagreement_set(5, depth, &net, 4_000));
    guard.extend(alpha_sensitive_set(5, depth, &net, 8, 4_000));
    let (gf0, _, _) = fitness(&seed, &guard, &net, depth, 16);
    println!("  guard set (TARGETED, alpha-sensitive): {} positions, seed scores {gf0}/{}",
             guard.len(), guard.len());

    let (_, seed_guard_cost, _) = fitness(&seed, &guard, &net, depth, 16);
    let mut identical_cheaper = 0usize;
    let (mut ill, mut broken, mut identical, mut different) = (0usize, 0usize, 0usize, 0usize);
    let mut guard_ok_diff = 0usize;
    let mut diff_ops: std::collections::BTreeMap<String, usize> = Default::default();
    let mut lost: std::collections::BTreeMap<u32, usize> = Default::default();
    for k in 0..n {
        let mut r = Rng::new((k as u64) << 12 ^ 0xA5A5);
        // ONE edit, not the loop's usual 1-3: the question is what a SINGLE step can do.
        let (cand, ops) = match mutate::mutate_program_n(&seed, &mut r, 1) {
            Some(x) => x,
            None => { ill += 1; continue }
        };
        let mut same = true;
        let mut ok = true;
        for (p, b) in set.iter().zip(&base) {
            let mut it = Interp::new(&net, vec![depth, 32_000, 8]);
            it.cost_cap = 20_000_000_000;
            let mv = it.run(&cand, p, 16);
            if mv == board::types::MOVE_NONE { ok = false; break; }
            if mv != *b { same = false; }
        }
        if !ok { broken += 1; }
        else if same {
            identical += 1;
            // IS THE SPEEDUP PATH REACHABLE? It accepts a candidate that plays IDENTICALLY and
            // costs LESS, with no game required. That is the path that would let hash reuse
            // through. But 93 generations produced ZERO survivors cheaper than the champion, so
            // the path may be correct and EMPTY -- a fourth inert feature. Counting it here
            // instead of waiting to find out.
            let (_, cst, _) = fitness(&cand, &guard, &net, depth, 16);
            if cst < seed_guard_cost { identical_cheaper += 1; }
        }
        else {
            different += 1;
            // AND does it survive the loop's actual correctness guard? This is the bucket that
            // matters: behaviour-changing AND correctness-preserving.
            let (gf, _, _) = fitness(&cand, &guard, &net, depth, 16);
            // HOW MANY guard positions does it lose? Measured as a DISTRIBUTION, because the fix
            // suggested by "0 of 33 pass an all-or-nothing guard" is a small TOLERANCE -- and
            // whether that works depends entirely on whether the losses are bimodal.
            //
            // The guards were built so EXPLOITS score ZERO on a whole subset: the depth exploit
            // (Const(0)->Const(1)) loses all 5 disagreement positions by construction, and the
            // raised-alpha exploit loses all 5 window-sensitive ones. If genuine behavioural
            // changes lose only 1-2, a tolerance of 1-2 separates the two classes cleanly. If they
            // also lose 5+, no tolerance can distinguish them and the idea is dead.
            *lost.entry(gf0.saturating_sub(gf)).or_default() += 1;
            if gf >= gf0 {
                guard_ok_diff += 1;
                *diff_ops.entry(format!("{:?}", ops)).or_default() += 1;
            }
        }
        // PROGRESS, because a long measurement with no output is indistinguishable from a hang.
        // run_search_track.sh records this project learning that once already: "the loop printed
        // only every 10th barren generation so 55 minutes of silence looked identical to a hang".
        // This one printed NOTHING for its whole run and I could only tell it was alive by reading
        // /proc.
        if (k + 1) % 25 == 0 {
            // EVERY number the summary reports, in the progress line. The previous version
            // printed only three of them and the run was killed by its own timeout at 75/100 --
            // 75 candidates of work and NO answer to the question it was launched for, because
            // `identical_cheaper` only appeared after the loop. A long measurement must be
            // informative when truncated, not all-or-nothing.
            println!("  ..{}/{n}  broken {broken}  identical {identical} (cheaper {identical_cheaper})  \
DIFFERENT {different} (guard-ok {guard_ok_diff})", k + 1);
        }
    }
    println!("=== single-edit mutants of the seed, {n} attempts, {} positions at depth {depth} ===",
             set.len());
    println!("  ill-typed / inapplicable : {ill}");
    println!("  BROKEN    (no move)      : {broken}");
    println!("  IDENTICAL (same play)    : {identical}   of which CHEAPER: {identical_cheaper}");
    println!("      ^ cheaper AND identical = the speedup path. Zero means that path is dead code.");
    println!("  DIFFERENT (plays differently)                  : {different}");
    println!("  ...AND passes the {}-position guard (THE USEFUL KIND): {guard_ok_diff}", guard.len());
    if !lost.is_empty() {
        println!("\n  guard positions LOST by behaviour-changing candidates (of {gf0}):");
        for (k, v) in &lost { println!("    lost {k:>2}: {v:>3} candidates"); }
        let small: usize = lost.iter().filter(|(k, _)| **k <= 2).map(|(_, v)| *v).sum();
        println!("  losing <=2: {small}  -- a tolerance of 2 would admit these");
        println!("  MEASURED on THIS guard: DEPTH exploit loses 8, ALPHA exploit loses 5.");
        println!("  A tolerance of 2-4 rejects both exploits. Whether it admits anything genuine");
        println!("  is the distribution above -- and both numbers now come from the same guard.");
    }
    if different > 0 {
        println!("\n  operators producing a behaviour change that ALSO passes the guard:");
        for (o, c) in &diff_ops { println!("    {o:<28} {c}"); }
    } else {
        println!("\n  ZERO behaviour-changing single edits. If this holds at larger n, the blocker");
        println!("  is the OPERATOR SET, not the fitness -- no fitness can reward a difference the");
        println!("  operators never produce.");
    }
}


/// DO THE KNOWN EXPLOITS LOSE MORE GUARD POSITIONS THAN GENUINE CHANGES? `evolve exploitcheck`
///
/// The guard-loss distribution says a tolerance of 2 would admit 3 of 33 behaviour-changing single
/// edits where the current all-or-nothing guard admits ZERO. That fix is only safe if the known
/// EXPLOITS lose MORE than the tolerance -- otherwise loosening the guard re-admits exactly what it
/// was built to stop.
///
/// I asserted they lose 5 "by construction" (the disagreement and window subsets are 5 positions
/// each, and an exploit fails its whole subset). Asserting is not measuring, and 3 candidates in
/// the distribution already sit at exactly 5, so the margin is thin enough to need the number.
///
/// Both exploits are reconstructed directly rather than loaded: the saved .prog files are `{:#?}`
/// Debug dumps, readable but not parseable back. These are the two the search track actually found:
///   * DEPTH   -- `Const(0)` -> `Const(1)` in the horizon guard `if d == 0: ret eval(p)`, so it
///                searches one ply less. 11x cheaper, all mates intact.
///   * ALPHA   -- the root call's `neg(INF)` -> `Const(8)`, raising initial alpha and pruning every
///                move worth under 8 centipawns.
fn exploit_check() {
    use grammar::ast::{ArithOp, Rel};
    let depth: i64 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(3);
    let net = Net::random(32, 20260907);
    let mut guard = mate_set(15);
    guard.extend(disagreement_set(5, depth, &net, 4_000));
    guard.extend(window_sensitive_set(5, depth, &net, 8, 4_000));
    let seed = reference::bare_alpha_beta();
    let (gf0, _, r0) = fitness(&seed, &guard, &net, depth, 16);
    println!("=== known exploits vs the {}-position guard, depth {depth} ===", guard.len());
    println!("  seed: {gf0}/{} at {r0:.6} mates/Mcost", guard.len());

    // Rewrite the FIRST node matching a predicate, anywhere in the tree.
    fn rewrite(n: &Node, f: &dyn Fn(&Node) -> Option<Node>) -> Node {
        if let Some(r) = f(n) { return r; }
        use Node::*;
        match n {
            Arith(o, a) => Arith(*o, a.iter().map(|x| rewrite(x, f)).collect()),
            Call(i, a) => Call(*i, a.iter().map(|x| rewrite(x, f)).collect()),
            TRead(i, a) => TRead(*i, a.iter().map(|x| rewrite(x, f)).collect()),
            If(c, t, e) => If(Box::new(rewrite(c, f)), Box::new(rewrite(t, f)),
                              e.as_ref().map(|x| Box::new(rewrite(x, f)))),
            Cmp(a, b, r) => Cmp(Box::new(rewrite(a, f)), Box::new(rewrite(b, f)), *r),
            Let(s2, a, b) => Let(s2.clone(), Box::new(rewrite(a, f)), Box::new(rewrite(b, f))),
            Foreach(a, s2, b) => Foreach(Box::new(rewrite(a, f)), s2.clone(), Box::new(rewrite(b, f))),
            Argmax(a, s2, b) => Argmax(Box::new(rewrite(a, f)), s2.clone(), Box::new(rewrite(b, f))),
            Set(s2, a) => Set(s2.clone(), Box::new(rewrite(a, f))),
            Ret(a) => Ret(Box::new(rewrite(a, f))),
            Max(a, b) => Max(Box::new(rewrite(a, f)), Box::new(rewrite(b, f))),
            other => other.clone(),
        }
    }

    // DEPTH exploit: the horizon guard's Const(0) becomes Const(1).
    let mut dex = seed.clone();
    dex.funcs[1].body = rewrite(&seed.funcs[1].body, &|n| match n {
        Node::Cmp(a, b, Rel::Eq) => match (&**a, &**b) {
            (Node::Var(v), Node::Const(0)) if v == "d" =>
                Some(Node::Cmp(a.clone(), Box::new(Node::Const(1)), Rel::Eq)),
            _ => None,
        },
        _ => None,
    });
    // ALPHA exploit: the root call's neg(INF) becomes Const(8).
    let mut aex = seed.clone();
    aex.funcs[0].body = rewrite(&seed.funcs[0].body, &|n| match n {
        Node::Arith(ArithOp::Neg, a) if a.len() == 1 => match &a[0] {
            Node::TRead(1, _) => Some(Node::Const(8)),
            _ => None,
        },
        _ => None,
    });

    // The TARGETED guard: mate + depth-disagreement + ALPHA-sensitive, replacing the window set.
    let mut guard2 = mate_set(15);
    guard2.extend(disagreement_set(5, depth, &net, 4_000));
    let alpha_set = alpha_sensitive_set(5, depth, &net, 8, 4_000);
    let n_alpha = alpha_set.len();
    guard2.extend(alpha_set);
    let (g2f0, _, _) = fitness(&seed, &guard2, &net, depth, 16);
    println!("  targeted guard: {} positions ({n_alpha} alpha-sensitive), seed {g2f0}/{}",
             guard2.len(), guard2.len());

    for (name, prog) in [("DEPTH exploit (d==0 -> d==1)", dex), ("ALPHA exploit (neg INF -> 8)", aex)] {
        let changed = format!("{:?}", prog) != format!("{:?}", seed);
        let (gf, _, r) = fitness(&prog, &guard, &net, depth, 16);
        let (g2f, _, _) = fitness(&prog, &guard2, &net, depth, 16);
        println!("  {name:<32} built={changed}  OLD guard loses {:>2}  TARGETED guard loses {:>2}  rate {:.2}x",
                 gf0.saturating_sub(gf), g2f0.saturating_sub(g2f), r / r0.max(1e-12));
    }
    println!("\n  Genuine behaviour-changing single edits lose a MINIMUM of 2 (measured, n=33).");
    println!("  If both exploits lose strictly more than 2, a tolerance of 2 separates the classes");
    println!("  and is safe. If either loses 2 or fewer, loosening the guard re-admits the exploit");
    println!("  and the idea is dead.");
}

fn read_declared(path: &str) -> (usize, f64, i64, i64, u32) {
    let txt = std::fs::read_to_string(path).unwrap_or_else(|e| {
        panic!("declared parameters missing at {path}: {e}. This file is part of the Given \
                column; running without it would silently substitute a default for a choice that \
                is supposed to be visible.")
    });
    let mut mu = None;
    let mut eps = None;
    let mut bmain = None;
    let mut bmcts = None;
    let mut gtol = None;
    for line in txt.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() { continue; }
        let (k, v) = match line.split_once('=') { Some(kv) => kv, None => continue };
        match k.trim() {
            "mu" => mu = Some(v.trim().parse().unwrap_or_else(|e| panic!("mu does not parse: {e}"))),
            "eps" => eps = Some(v.trim().parse().unwrap_or_else(|e| panic!("eps does not parse: {e}"))),
            "budget_main" => bmain = Some(v.trim().parse().unwrap_or_else(|e| panic!("budget_main does not parse: {e}"))),
            "budget_mcts" => bmcts = Some(v.trim().parse().unwrap_or_else(|e| panic!("budget_mcts does not parse: {e}"))),
            "guard_tolerance" => gtol = Some(v.trim().parse().unwrap_or_else(|e| panic!("guard_tolerance does not parse: {e}"))),
            _ => {}
        }
    }
    (mu.expect("configs/search_track.conf declares no `mu`"),
     eps.expect("configs/search_track.conf declares no `eps`"),
     bmain.expect("configs/search_track.conf declares no `budget_main`"),
     bmcts.expect("configs/search_track.conf declares no `budget_mcts`"),
     gtol.expect("configs/search_track.conf declares no `guard_tolerance`"))
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


/// Build the alpha-raised variant of a program: the root call's `neg(INF)` becomes `Const(k)`.
/// Shared by the exploit check and by `alpha_sensitive_set`, so the guard is built against the
/// SAME transformation it is meant to catch rather than a proxy for it.
fn raise_alpha(seed: &Program, k: i8) -> Program {
    use grammar::ast::ArithOp;
    fn rw(n: &Node, k: i8) -> Node {
        use Node::*;
        if let Arith(ArithOp::Neg, a) = n {
            if a.len() == 1 {
                if let TRead(1, _) = &a[0] { return Const(k); }
            }
        }
        match n {
            Arith(o, a) => Arith(*o, a.iter().map(|x| rw(x, k)).collect()),
            Call(i, a) => Call(*i, a.iter().map(|x| rw(x, k)).collect()),
            TRead(i, a) => TRead(*i, a.iter().map(|x| rw(x, k)).collect()),
            If(c, t, e) => If(Box::new(rw(c, k)), Box::new(rw(t, k)),
                              e.as_ref().map(|x| Box::new(rw(x, k)))),
            Cmp(a, b, r) => Cmp(Box::new(rw(a, k)), Box::new(rw(b, k)), *r),
            Let(s2, a, b) => Let(s2.clone(), Box::new(rw(a, k)), Box::new(rw(b, k))),
            Foreach(a, s2, b) => Foreach(Box::new(rw(a, k)), s2.clone(), Box::new(rw(b, k))),
            Argmax(a, s2, b) => Argmax(Box::new(rw(a, k)), s2.clone(), Box::new(rw(b, k))),
            Set(s2, a) => Set(s2.clone(), Box::new(rw(a, k))),
            Ret(a) => Ret(Box::new(rw(a, k))),
            Max(a, b) => Max(Box::new(rw(a, k)), Box::new(rw(b, k))),
            other => other.clone(),
        }
    }
    let mut p = seed.clone();
    p.funcs[0].body = rw(&seed.funcs[0].body, k);
    p
}

/// Positions where RAISING THE INITIAL ALPHA changes the seed's answer.
///
/// THE EXISTING WINDOW GUARD DOES NOT TARGET THE EXPLOIT IT WAS BUILT FOR. `window_sensitive_set`
/// varies table 1 (INF) between 32000 and 8, and since alpha is `neg(INF)` that produces a
/// SYMMETRIC window of [-8, +8]. The exploit the search actually found raises the initial alpha to
/// +8 while beta stays at INF -- asymmetric, a different transformation. Measured consequence: the
/// alpha exploit loses only 2 of the 25 guard positions, catching it incidentally rather than by
/// design, and it still posts 1.29x the seed's mates-per-cost.
///
/// This builds the guard from the transformation ITSELF: run the seed and the alpha-raised variant,
/// keep the positions where they disagree, and record the SEED's answer as correct. The exploit
/// then scores ZERO on every one of them by construction -- the same self-calibrating shape as the
/// depth-disagreement set, which compares the seed against itself one ply shallower.
fn alpha_sensitive_set(n: usize, depth: i64, net: &Net, raised: i8, cap: usize)
    -> Vec<(Position, Option<board::Move>)> {
    let ab = reference::bare_alpha_beta();
    let hi = raise_alpha(&ab, raised);
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
        let mut a_it = Interp::new(net, vec![depth, 32_000, 8]);
        let a = a_it.run(&ab, &p, 16);
        let mut b_it = Interp::new(net, vec![depth, 32_000, 8]);
        let b = b_it.run(&hi, &p, 16);
        if a != board::types::MOVE_NONE && a != b {
            out.push((p.clone(), Some(a)));
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
    if std::env::args().nth(1).as_deref() == Some("refmatch") {
        return ref_match();
    }
    if std::env::args().nth(1).as_deref() == Some("stepdiff") {
        return step_diff();
    }
    if std::env::args().nth(1).as_deref() == Some("exploitcheck") {
        return exploit_check();
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
    // ALPHA-SENSITIVE, not window-sensitive. The window set varies INF (a symmetric window) while
    // the exploit raises the initial ALPHA (asymmetric), so it caught that exploit only
    // incidentally -- measured at 2 of 25 lost. The alpha-sensitive set is built from the
    // transformation itself, moving it to 5, which is what makes a tolerance possible at all.
    let win = alpha_sensitive_set(n3, depth, &net, 8, 4_000);
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

    // ---- REFUSE TO RUN ON A SHORT GUARD SET. RESTORED after the lineage rewrite silently dropped
    // these, which a `unused variable: n_win` warning revealed. Reading the warning was the only
    // reason it was caught: a dropped guard produces no symptom at all -- the loop runs happily
    // with a set too small to bite, which is exactly the failure these exist to prevent, and it is
    // how the depth exploit survived the first time.
    if n_win < n3 {
        println!("  REFUSING TO RUN: found {n_win} window-sensitive positions, wanted {n3}.");
        println!("  Without them a candidate can raise alpha and buy cost for free, which is what");
        println!("  happened the moment the depth exploit was closed.");
        return;
    }
    if n_deep < n2 {
        println!("  REFUSING TO RUN: found {n_deep} depth-requiring positions, wanted {n2}.");
        println!("  Without them the 'do not lose mates' guard cannot bite and this loop optimises");
        println!("  toward searching one ply less -- which is what it did when the guard was built");
        println!("  from mate distance instead of disagreement.");
        return;
    }

    // ---- DECLARED PARAMETERS FIRST: everything below depends on them.
    let (mu, eps, budget_main, budget_mcts, guard_tolerance) =
        read_declared("configs/search_track.conf");
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
        /// ABSOLUTE guard floor for this lineage, anchored to its SEED's score. Not relative to
        /// the current best: `f >= best_found - tolerance` lets the champion ratchet DOWN, since
        /// every accept could lose another `tolerance` positions and after k accepts it would sit
        /// k*tolerance below the seed with the guard decayed to nothing. Anchoring to the seed
        /// caps total drift at `tolerance`, permanently.
        guard_floor: u32,
        best_rate: f64,
        accepted: usize,
    }
    let mut lineages: Vec<Lineage> = Vec::new();
    for (name, seed_prog, bud) in [
        ("MAIN", reference::bare_alpha_beta(), budget_main),
        ("MCTS", reference::uct_mcts(), budget_mcts),
    ] {
        let (f, c, r) = fitness(&seed_prog, &set, &net, depth, bud);
        println!("  lineage {name:<5} seed {:>3} nodes, budget {bud:<5} -> {f}/{} mates (floor {}), {c} cost, \
{r:.6} mates/Mcost", seed_prog.size(), set.len(), f.saturating_sub(guard_tolerance));
        lineages.push(Lineage {
            name,
            budget: bud,
            popn: vec![(seed_prog.clone(), f, r); MU],
            champ: seed_prog,
            best_found: f,
            guard_floor: f.saturating_sub(guard_tolerance),
            best_rate: r,
            accepted: 0,
        });
    }
    let (seed_hard, _, _) = fitness(&reference::bare_alpha_beta(), &hard, &net, depth, budget_main);
    println!("  HARD set: {} positions the seed FAILS by construction; seed scores {seed_hard}/{} \
(a real gradient, unlike the saturated 25/25 guard set)", hard.len(), hard.len());
    println!("  population MU={MU}, lambda={pop}, EPS={EPS:.3}, guard tolerance {guard_tolerance} \
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
            let (bud, best_found, best_rate, guard_floor) =
                (lineages[li].budget, lineages[li].best_found, lineages[li].best_rate,
                 lineages[li].guard_floor);
            let _ = best_found;
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
                                        Ok((f, cst, rate)) => {
                                            let (hf, hcst, _) = fitness(c, hard, net, depth, bud);
                                            // EXISTENCE_HARD_FITNESS=1 folds the hard set into the
                                            // surrogate. Default OFF, so nothing changes unless set.
                                            //
                                            // WHY THIS IS NOW JUSTIFIED. evolve.rs deferred it
                                            // explicitly: "acceptance is NOT changed yet, because
                                            // the claim 'a better-searching candidate can win
                                            // these' is exactly the sort of thing that should be
                                            // measured before a fitness is restructured around
                                            // it", and the range check below says "if this never
                                            // varies, the gradient does not exist".
                                            //
                                            // MEASURED across 39 lineage-generations in two runs:
                                            // 20 of 39 (51%) contain a member scoring above zero,
                                            // best 2/8, where the seed is 0/8 by construction. It
                                            // varies. The precondition the code set is met.
                                            //
                                            // WHY IT MATTERS: mates are SATURATED at 25/25, so the
                                            // surrogate can only improve via cost -- and 0 of 30
                                            // identical-playing mutants are cheaper. Hence the max
                                            // candidate rate is exactly 1.000x in all 39
                                            // lineage-generations and never above. Folding in a
                                            // dimension where the seed scores ZERO is the only way
                                            // the surrogate can rise at all.
                                            // COST INCLUDES BOTH SETS. Pairing (f + hf) with the
                                            // guard set's cost alone would count the hard-set
                                            // solves in the numerator while charging nothing for
                                            // the work that produced them -- and the cost term is
                                            // the only thing stopping program bloat. A candidate
                                            // that wins hard positions by searching enormously
                                            // must pay for it.
                                            let rate = if std::env::var("EXISTENCE_HARD_FITNESS").is_ok() {
                                                (f + hf) as f64 * 1e6 / cst.saturating_add(hcst).max(1) as f64
                                            } else { rate };
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
                scored.iter().filter(|(_, f, _, _)| *f >= guard_floor).map(|(_, _, _, h)| *h).collect();
            let (hlo, hhi) = (hard_scores.iter().min().copied().unwrap_or(0),
                              hard_scores.iter().max().copied().unwrap_or(0));
            let scored: Vec<(Program, u32, f64)> =
                scored.into_iter().map(|(p, f, r, _)| (p, f, r)).collect();
            let mate_ok = scored.iter().filter(|(_, f, _)| *f >= guard_floor).count();
            let rel: Vec<f64> = scored
                .iter()
                .filter(|(_, f, _)| *f >= guard_floor)
                .map(|(_, _, r)| r / best_rate.max(1e-12))
                .collect();
            let (rlo, rhi) = rel.iter().fold((f64::MAX, 0.0f64), |(a, b), x| (a.min(*x), b.max(*x)));
            // RATE HISTOGRAM over guard-passing candidates. min-max cannot answer the question that
            // matters: the MAX is always a neutral twin scoring 1.000x, so "best >= 0.98" is true
            // every generation and says nothing about whether informative candidates exist in the
            // band EPS discards. I ran exactly that wrong statistic and nearly reported it as a
            // refutation. Buckets, in units of best_rate:
            //   >=0.98  survives EPS (eps = 0.02)   -- in practice the neutral twins
            //   .90-.98 CUT by EPS                  -- the band the diagnosis is about
            //   .50-.90 / <.50                      -- damaged and correctly cut
            let hist = {
                let (mut a, mut b, mut c, mut d) = (0usize, 0usize, 0usize, 0usize);
                for x in &rel {
                    if *x >= 0.98 { a += 1 } else if *x >= 0.90 { b += 1 }
                    else if *x >= 0.50 { c += 1 } else { d += 1 }
                }
                (a, b, c, d)
            };
            // Also count how many guard-passers are rate-DISTINCT from the incumbent best. If this
            // is 0 while `mate_ok` is large, the operators are producing only neutral rewrites and
            // no selection policy can help -- which is a different problem from EPS cutting them.
            let distinct = rel.iter().filter(|x| (**x - 1.0).abs() > 1e-9).count();
            let offspring: Vec<(Program, u32, f64)> =
                // guard_floor, NOT best_found. This line is the ACTUAL selection filter; the
                // three above it are diagnostics. When I reverted a misplaced floor definition I
                // reverted this one with it, so the tolerance changed only what was PRINTED --
                // mate-ok read 3 and rates read 1.099-1.143x while the population stayed at 1 and
                // nothing was ever accepted. Third inert feature today (cost ceiling, catch_unwind
                // under panic=abort, this), and all three looked correct in the diff.
                scored.into_iter().filter(|(_, f, _)| *f >= guard_floor).collect();

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
                // ---- TWO ACCEPTANCE PATHS, because one gate cannot judge both kinds of change.
                //
                // MEASURED: the game gate accepts only when `rate - ci95 > 0.5`, and at 6 pairs
                // that needs a 0.689 win rate -- roughly +138 Elo. A rout.
                //
                // Worse, it can NEVER confirm the one rung ever measured as fitter. Hash reuse is
                // exact alpha-beta: it agrees with the seed on 40/40 positions at depth 3 and 12/12
                // at depth 4, so its game rate is exactly 0.5 and the gate rejects it permanently.
                // The surrogate SEES it (1.024x) and cannot tell it from an exploit; the games can
                // tell exploits apart and are blind to it. Requiring games for every promotion
                // therefore blocks the only improvement anyone has found.
                //
                // PATH 1 -- BEHAVIOUR-PRESERVING SPEEDUP. If the candidate returns the SAME move as
                // the champion on every guard position and costs less, it is a pure speedup:
                // identical play, fewer resources. No game is needed because there is nothing to
                // decide -- the two would play the same games.
                //
                // NO EXPLOIT CAN TAKE THIS PATH, which is what makes it safe. Both known exploits
                // change play, by construction and by measurement: the depth exploit loses 8 guard
                // positions and the alpha exploit 5. A program that changes no move on any guard
                // position has not searched less; it has done the same search for less.
                // IDENTITY CHECKED ON THE GUARD SET **AND** THE HARD SET.
                //
                // This path grants acceptance with NO game, so a false positive promotes an
                // unexamined behaviour change. Identity across 25 guard positions is strong
                // evidence but not proof: a candidate can match there and differ elsewhere.
                //
                // The hard set is 8 positions built by DEPTH DISAGREEMENT -- the seed answers them
                // differently at depth D and D+1 -- so they are exactly where a search that
                // changed its effective depth shows up. Adding them costs 8 more searches against
                // a 12-game gate that costs minutes, which is a trade worth making on the one path
                // that skips the games entirely.
                //
                // It is still not proof, and the honest bound is: identical on 33 positions chosen
                // to be maximally sensitive to depth, window and mate behaviour.
                let same_play = {
                    let mut ic = Interp::new(&net, vec![depth, 32_000, 8]);
                    let mut ih = Interp::new(&net, vec![depth, 32_000, 8]);
                    set.iter().chain(hard.iter()).all(|(p, _)| {
                        ic.run(&c, p, bud) == ih.run(&lineages[li].champ, p, bud)
                    })
                };
                if same_play {
                    println!("  gen {g:>3} {:<5} ACCEPT speedup: play IDENTICAL on all {} guard \
positions, {rate:.6} was {:.6}", lineages[li].name, set.len() + hard.len(), best_rate);
                    lineages[li].champ = c.clone();
                    lineages[li].best_found = f;
                    lineages[li].best_rate = rate;
                    lineages[li].accepted += 1;
                    let _ = std::fs::write(
                        format!("evolved_{}_gen{g}.prog", lineages[li].name),
                        format!("// SPEEDUP {f} mates, {rate:.6} mates/Mcost, {} nodes, gen {g}\n{:#?}\n",
                                c.size(), c));
                    continue;
                }

                // ---- PATH 2: THE GAME GATE, for candidates that change play.
                // It RUNS EVOLVED PROGRAMS ON A BOARD, so it is exactly as exposed
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
rates {span} [>=.98:{} .90-.98:{} .50-.90:{} <.50:{} distinct:{}], hard {hlo}-{hhi})  pop {} spread {:.6}-{:.6} tt{:?}",
                         lineages[li].name, hist.0, hist.1, hist.2, hist.3, distinct,
                         popn.len(), spread_lo, spread_hi, tt);
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
