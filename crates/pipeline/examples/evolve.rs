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
    ];
    let (_, _, base_rate) = fitness(&progs[0].1, &set, &net, depth);
    let base_nodes = progs[0].1.size() as i64;

    println!("\n  {:<28} {:>6} {:>7} {:>16} {:>14} {:>10}",
             "program", "nodes", "mates", "cost", "mates/Mcost", "vs seed");
    for (name, p) in &progs {
        let (found, cost, rate) = fitness(p, &set, &net, depth);
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
    let n_win = win.len();
    let n_deep = deep.len();
    set.extend(deep);
    set.extend(win);

    let mut champ = reference::bare_alpha_beta();
    let (f0, c0, r0) = fitness(&champ, &set, &net, depth);
    println!("  surrogate set {} positions ({} mate-in-1, {} depth-requiring, {} window-sensitive), fitness depth {}",
             set.len(), set.len() - n_deep - n_win, n_deep, n_win, depth);
    if n_win < n3 {
        println!("  REFUSING TO RUN: found {n_win} window-sensitive positions, wanted {n3}.");
        println!("  Without them a candidate can raise alpha and buy cost for free, which is");
        println!("  exactly what happened once the depth exploit was closed.");
        return;
    }
    if n_deep < n2 {
        println!("  REFUSING TO RUN: found {n_deep} depth-requiring positions, wanted {n2}.");
        println!("  Without them the 'do not lose mates' guard cannot bite and this loop");
        println!("  optimises toward searching one ply less -- which is exactly what it did");
        println!("  when the guard was built from mate distance instead of disagreement.");
        return;
    }
    println!("  seed: bare alpha-beta  {f0} mates  {c0} cost  {r0:.2} mates/Mcost  ({} nodes)", champ.size());
    let (mut best_found, mut best_rate) = (f0, r0);

    // ---- (MU + LAMBDA) WITH PLATEAU TOLERANCE, replacing the strict (1+lambda) hill climb.
    //
    // THE HILL CLIMB WAS PROVABLY UNABLE TO REACH THE ONE RUNG WE KNOW IS FITTER. Measured in
    // ladder_valley_RESULT.md, with this same fitness on this same set: hash reuse is 1.024x the
    // seed, but its halves are 0.991x (probe with nothing stored -- every probe a guaranteed miss)
    // and 0.997x (store nothing reads). The payoff is CONJUNCTIVE, so acceptance on
    // `rate > best_rate` can never take the first step, and the rung sits at the bottom of a
    // ~0.9% valley. That is not a tuning problem; no pair count or depth fixes it.
    //
    // EPS = 0.03 is chosen against that measurement, not by taste: the deepest half is 0.9% down,
    // so the window has to exceed 0.009 to admit it, and 0.03 clears it with margin while still
    // discarding anything meaningfully worse. Stated as a number so it can be argued with.
    //
    // THE MATE GUARD STAYS STRICT -- `f >= best_found`, never relaxed. Plateau tolerance applies
    // to COST ONLY. This matters more than it sounds: the two exploits this track has already
    // found (searching one ply shallower, raising the initial alpha) both work by giving up
    // correctness for cost, and both are caught by the mate/disagreement/window guards scoring
    // ZERO rather than "fewer". Relaxing the cost bar does not weaken any of them.
    const MU: usize = 4;
    const EPS: f64 = 0.03;
    let mut popn: Vec<(Program, u32, f64)> = vec![(champ.clone(), f0, r0); MU];
    println!("  population MU={MU}, lambda={pop}, plateau tolerance EPS={EPS:.3} \
(deepest measured valley half is 0.009)");

    let mut rng = Rng::new(0xE0FFEE);
    let mut accepted = 0;
    for g in 1..=gens {
        // MUTATE FIRST, EVALUATE IN PARALLEL. Mutation is microseconds; fitness is the whole
        // generation.
        //
        // WHY THIS IS FREE AND NOT A TRADE. Fitness is DETERMINISTIC -- fixed position set, fixed
        // net, no sampling -- so a candidate's score does not depend on which thread computes it
        // or in what order. The results are bit-identical to the sequential loop; only the wall
        // clock changes. Order is preserved by chunking rather than by a work queue, so even the
        // tie-breaking in the sort below is unchanged.
        //
        // MEASURED WASTE: the process ran at 99.5% CPU -- ONE core -- while pinned by
        // run_search_track.sh to cores 12-14. Two of three cores sat idle through a ~3 minute
        // generation. A depth-3 fitness is a 4-PLY search (choose applies the root move, then
        // recurses with the FULL D, so the tree is D+1 plies) at ~144k evals per position, which
        // is genuinely expensive and cannot be cut without losing the one rung that only pays at
        // 4 plies. This is the part that was pure waste.
        let cands: Vec<(usize, Program)> = (0..pop)
            .filter_map(|i| {
                // Parent chosen round-robin across the population, so every member breeds.
                // Sampling uniformly at random would let a member die without ever being tried,
                // which defeats the point of keeping a worse-but-different program alive.
                let parent = &popn[i % popn.len()].0;
                let mut r = Rng::new((g as u64) << 20 ^ i as u64 ^ 0xBEEF);
                mutate::mutate_program(parent, &mut r).map(|c| (i, c))
            })
            .collect();
        let ill = pop - cands.len();
        const THREADS: usize = 3;
        let chunk = cands.len().div_ceil(THREADS).max(1);
        let scored: Vec<(Program, u32, f64)> = std::thread::scope(|sc| {
            let handles: Vec<_> = cands
                .chunks(chunk)
                .map(|part| {
                    let (set, net) = (&set, &net);
                    sc.spawn(move || {
                        part.iter()
                            .map(|(_, c)| {
                                let (f, _cst, rate) = fitness(c, set, net, depth);
                                (c.clone(), f, rate)
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            handles.into_iter().flat_map(|h| h.join().unwrap()).collect()
        });
        // WHY THE POPULATION COLLAPSES, measured rather than guessed. First run of the plateau
        // tolerance reported `pop 1`: everything died and the diagnostic could not say WHY,
        // because the mate guard runs before any rate is looked at. Two very different causes
        // produce the same collapse -- offspring losing mates, or offspring being far worse than
        // EPS -- and they call for opposite fixes. So both are counted.
        //
        // If most candidates fail the MATE guard, single mutations of a search program are simply
        // destructive and the population needs a gentler operator, not a wider window. If they
        // pass the mate guard but sit far below EPS, then EPS is the binding constraint and the
        // 0.9% valley figure -- taken from two hand-built reference programs -- is not
        // representative of what mutation actually produces.
        let n_scored = scored.len();
        let mate_ok = scored.iter().filter(|(_, f, _)| *f >= best_found).count();
        let rel: Vec<f64> = scored
            .iter()
            .filter(|(_, f, _)| *f >= best_found)
            .map(|(_, _, r)| r / best_rate.max(1e-12))
            .collect();
        let (rlo, rhi) = rel.iter().fold((f64::MAX, 0.0f64), |(a, b), x| (a.min(*x), b.max(*x)));
        let offspring: Vec<(Program, u32, f64)> =
            scored.into_iter().filter(|(_, f, _)| *f >= best_found).collect();

        // (MU + LAMBDA): parents and offspring compete together, so the best-so-far can never be
        // lost -- elitist, which keeps the drift from becoming a random walk.
        let mut pool = popn.clone();
        pool.extend(offspring);
        pool.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        let top = pool[0].2;
        pool.retain(|x| x.2 >= top * (1.0 - EPS));
        // DEDUPE BY STRUCTURE, and this is load-bearing rather than tidy.
        //
        // The population starts as MU IDENTICAL copies of the seed. (MU+LAMBDA) is elitist and
        // ties are kept in sort order, so those clones occupied all MU slots and every offspring
        // was truncated away -- the population could never diversify and this silently degenerated
        // into the exact hill climb it was written to replace. MEASURED on the first run: `pop 4
        // spread 0.002518-0.002518`, four members at one rate, which is precisely what the spread
        // diagnostic was added to expose. It reported the defect on generation 1.
        //
        // Comparing by rate alone would be wrong: two structurally different programs can cost the
        // same, and collapsing them would throw away the diversity this exists to keep. The Debug
        // string is a faithful rendering of the AST, and at MU+LAMBDA = 16 items per ~3-minute
        // generation its cost is irrelevant.
        let mut seen = std::collections::HashSet::new();
        pool.retain(|x| seen.insert(format!("{:?}", x.0)));
        pool.truncate(MU);
        popn = pool;

        let (spread_lo, spread_hi) = (popn.last().unwrap().2, popn[0].2);
        if popn[0].2 > best_rate {
            let (c, f, rate) = popn[0].clone();
            // ---- THE GAME GATE. The surrogate proposes; games decide.
            //
            // MASTER_PLAN:276 makes the gate the arbiter and the search track never had one. It
            // promoted on mates-per-cost alone, and that surrogate has been exploited TWICE by
            // programs strictly worse at chess -- one searched a ply shallower, one raised the
            // initial alpha to +8 -- each keeping every mate while being unplayable. Each was
            // caught by a guard written AFTER the fact, and the next exploit will be found the
            // same way. Games close the whole class: a program that prunes real moves loses, and
            // no property of the position set can hide it.
            //
            // Only reached when the surrogate says the candidate is better, so games are spent on
            // candidates that earned them -- "a candidate that cannot even improve mates-per-cost
            // has no business consuming gate time".
            let gsc = gate::match_progs(&c, &champ, &net, vec![depth, 32_000, 8], 16,
                                        gate_pairs, 0xC0FFEE ^ g as u64, 4);
            let resolved_up = gsc.pent_rate() - gsc.ci95() > 0.5;
            if !resolved_up {
                // NOT promoted, but NOT discarded either: it stays in the population, so the
                // search can keep building on it. A candidate that is cheaper on the surrogate
                // and merely UNPROVEN on the board is exactly what the plateau tolerance exists
                // to carry -- half a transposition table looks like this.
                println!("  gen {g:>3}  gate REJECT  {:.3}+/-{:.3} ({} games)  surrogate said {rate:.6}",
                         gsc.pent_rate(), gsc.ci95(), gsc.games());
                // RAISE THE SURROGATE BAR ANYWAY, without promoting the champion. Otherwise
                // `popn[0].2 > best_rate` stays true forever and this candidate is re-gated every
                // generation for the rest of the run -- 12 games each time, on a question already
                // answered. best_rate now tracks the SURROGATE frontier and champ tracks the last
                // program that actually won on the board; they are different questions and this
                // makes that explicit rather than conflating them.
                best_rate = rate;
                continue;
            }
            println!("  gen {g:>3}  ACCEPT  {f} mates  {rate:.6} mates/Mcost  ({} nodes, was {:.6})  \
gate {:.3}+/-{:.3}", c.size(), best_rate, gsc.pent_rate(), gsc.ci95());
            champ = c; best_found = f; best_rate = rate; accepted += 1;
            if let Err(e) = std::fs::write(
                format!("evolved_gen{g}.prog"),
                format!("// {f} mates, {rate:.6} mates/Mcost, {} nodes, generation {g}\n{:#?}\n",
                        champ.size(), champ)) {
                eprintln!("  WARNING: could not save the evolved program: {e}");
            }
        } else {
            // The population SPREAD is the diagnostic that matters now. All members at an
            // identical rate means the plateau tolerance is admitting nothing and this has
            // silently degenerated back into the hill climb it replaced -- which would look
            // exactly like healthy "no improvement" output without this number.
            let span = if rel.is_empty() { "none".to_string() }
                       else { format!("{rlo:.3}-{rhi:.3}x seed") };
            println!("  gen {g:>3}  ..no improvement ({n_scored} cand, {ill} ill-typed, \
mate-ok {mate_ok}, rates {span})  pop {} spread {:.6}-{:.6} tt{:?}",
                     popn.len(), spread_lo, spread_hi,
                     popn.iter().map(|(p, _, _)| tt_prims(p)).collect::<Vec<_>>());
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
