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

/// Can the side to move FORCE mate within `n` moves (2n-1 plies)?
///
/// WHY THIS EXISTS. FITNESS §3 specifies the mate set as **500 positions at each of MATE-1, MATE-2,
/// MATE-3 and MATE-4**, stratified and reported per N. `mate_set` below builds **mate-in-ONE only**,
/// so the spec's entire depth ladder has been missing. That is not a cosmetic gap:
/// `fitness_set_composition_RESULT.md` measures a program doing 0.0366% of the seed's search — a
/// null search — scoring **2934.933x** on a mate-1-only set while keeping 25/25 mates, because a
/// mate-in-one is a ONE-PLY CHECK and needs no search to find. The depth ladder is what makes the
/// surrogate able to tell a search from a no-op.
///
/// SOUNDNESS, stated precisely: this is EXHAUSTIVE within `cap` nodes and gives up otherwise. On
/// budget exhaustion it returns FALSE — "not proven", never "proven absent". For SET CONSTRUCTION
/// that is the safe direction: an unproven position is skipped, so the set contains only positions
/// whose mate distance was actually demonstrated. It would NOT be safe to use this as an oracle for
/// "no mate exists".
fn mate_within(p: &mut Position, n: u32, nodes: &mut u64, cap: u64) -> bool {
    if n == 0 || *nodes > cap { return false; }
    let l = p.legal_moves();
    if l.is_empty() { return false; }
    for &m in l.as_slice() {
        *nodes += 1;
        if *nodes > cap { return false; }
        let u = p.make_move(m);
        let opp = p.legal_moves();
        // Same mate test `mate_set` uses: after our move it is the opponent to move, so no legal
        // replies plus Outcome::Loss is checkmate rather than stalemate.
        let delivered = opp.is_empty() && p.outcome() == Outcome::Loss;
        let ok = if delivered {
            true
        } else if n == 1 || opp.is_empty() {
            false
        } else {
            // EVERY opponent reply must still lose within n-1. One escape refutes the move, so this
            // breaks early and the effective branching is far below the legal-move count.
            let mut all = true;
            for &r in opp.as_slice() {
                let u2 = p.make_move(r);
                let sub = mate_within(p, n - 1, nodes, cap);
                p.unmake_move(r, u2);
                if !sub { all = false; break; }
            }
            all
        };
        p.unmake_move(m, u);
        if ok { return true; }
    }
    false
}

/// The move that forces mate within `n`, if one is proven. `None` when unproven within `cap`.
///
/// NEEDED BECAUSE OF HOW `fitness` SCORES, and I got this wrong once already. `fitness` credits a
/// position two different ways (`fitness`, the `match forcing` arm):
///
///   * `None`       -> MATE IN ONE: any move that mates ON THIS PLY.
///   * `Some(best)` -> credit the FORCING move, compared by equality.
///
/// The first move of a mate-in-two does NOT mate on its own ply, so labelling a MATE-2 position
/// `None` scores it **0 by construction, for every program forever**. The codebase had already found
/// and documented exactly this — "that bug made depth 1, 2 and 3 all read 0 until the control caught
/// it" — and my first `mate_set_n` reintroduced it by pushing `None` for every stratum. The symptom
/// was the SEED scoring 12 of 24 on a 12 MATE-1 + 12 MATE-2 set: every MATE-1 solved, every MATE-2
/// impossible. A stratum the champion cannot score is constant-zero and adds no discrimination at
/// all, which is the failure this whole ladder exists to remove.
///
/// AMBIGUITY, stated: a position may have several mate-forcing moves and this returns the first in
/// move order. `fitness` compares by equality, so a program finding a DIFFERENT sound forcing move
/// scores 0. That is the existing mate-in-two convention rather than something new, but it means
/// these strata measure "finds THIS forcing move", not "finds A forcing move".
fn forcing_move(p: &mut Position, n: u32, cap: u64) -> Option<board::Move> {
    let l = p.legal_moves();
    for &m in l.as_slice() {
        let u = p.make_move(m);
        let opp = p.legal_moves();
        let delivered = opp.is_empty() && p.outcome() == Outcome::Loss;
        let ok = if delivered {
            n >= 1
        } else if n <= 1 || opp.is_empty() {
            false
        } else {
            let mut all = true;
            for &r in opp.as_slice() {
                let u2 = p.make_move(r);
                let mut nodes = 0u64;
                let sub = mate_within(p, n - 1, &mut nodes, cap);
                p.unmake_move(r, u2);
                if !sub { all = false; break; }
            }
            all
        };
        p.unmake_move(m, u);
        if ok { return Some(m); }
    }
    None
}

/// Positions with a proven forced mate in EXACTLY `n` — `mate_within(n)` and not `mate_within(n-1)`.
///
/// The "and not n-1" clause is what makes the strata disjoint, and it is not optional: without it a
/// MATE-3 set silently contains every MATE-1 and MATE-2 position, the depth ladder collapses back to
/// mate-in-one, and the per-N thresholds FITNESS §3 asks for would be computed over the wrong
/// populations.
///
/// MATE-1 is labelled `None` (any mating move counts, matching the shipped `mate_set`); every deeper
/// stratum is labelled with its forcing move, because `None` would score it zero forever. See
/// `forcing_move`.
fn mate_set_n(count: usize, n: u32, cap: u64) -> Vec<(Position, Option<board::Move>)> {
    let mut rng: u64 = 0xC0DE_F00D ^ (n as u64) << 32;
    let mut out = Vec::new();
    let mut tries: u64 = 0;
    while out.len() < count && tries < 400_000 {
        tries += 1;
        let mut p = Position::startpos();
        let plies = 20 + (rng % 45) as usize;
        rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
        let mut ok = true;
        for _ in 0..plies {
            let l = p.legal_moves();
            if l.is_empty() { ok = false; break; }
            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
            p.make_move(l.as_slice()[(rng % l.len() as u64) as usize]);
        }
        if !ok || p.legal_moves().is_empty() { continue; }
        let mut nodes = 0u64;
        if !mate_within(&mut p, n, &mut nodes, cap) { continue; }
        if n > 1 {
            let mut n2 = 0u64;
            if mate_within(&mut p, n - 1, &mut n2, cap) { continue; }   // shallower => wrong stratum
        }
        // Label the position the way `fitness` will read it. MATE-1 keeps `None` (any mating move
        // counts, as the shipped `mate_set` does); deeper strata MUST carry their forcing move or
        // they score 0 for every program forever.
        let label = if n == 1 { None } else {
            match forcing_move(&mut p, n, cap) {
                Some(m) => Some(m),
                None => continue,   // proven by mate_within but the move could not be recovered
            }
        };
        out.push((p.clone(), label));
    }
    out
}

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
        let mut shallow = Interp::new(net, vec![depth - 1, 32_000, interp::uct_exploration()]);
        let a = shallow.run(&ab, &p, 16);
        let mut deep = Interp::new(net, vec![depth, 32_000, interp::uct_exploration()]);
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
        let mut full = Interp::new(net, vec![depth, 32_000, interp::uct_exploration()]);
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
    let mut it = Interp::new(net, vec![depth, 32_000, interp::uct_exploration()]);
    // COST CAP, and it is the reason this harness stops at depth 3.
    //
    // `Interp::new` defaults `cost_cap` to 2e9 PER POSITION (`interp/src/lib.rs:527`), and `fitness`
    // has never overridden it, while other call sites in this file use 20e9. Measured on a
    // 12 MATE-1 + 12 MATE-2 ladder, the SEED's cost per position runs:
    //
    //     depth 3 ->   480,601,359     24/24 mates
    //     depth 4 -> 1,953,485,828      2/24 mates   (just under the cap, already starving)
    //     depth 5 -> 2,000,000,800      0/24 mates   (exactly the cap)
    //     depth 6 -> 2,000,001,072      0/24 mates   (exactly the cap)
    //
    // So a DEEPER search scores FEWER mates, which reads as a broken seed and is actually a
    // truncated one. FITNESS §3's MATE-3 stratum needs 5 plies, so the whole upper ladder is
    // unreachable while this ceiling stands.
    //
    // Default is UNCHANGED at 2e9, so every running arm and every recorded measurement stays
    // comparable; the env var exists so the ceiling can be measured rather than argued about.
    if let Ok(v) = std::env::var("EXISTENCE_COST_CAP") {
        if let Ok(n) = v.parse::<u64>() { it.cost_cap = n; }
    }
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
    // BOTH ENCODINGS, each at the weight it actually works at. `uct_mcts` selects on
    // Mix(q, u, c) = (q*c + u*(16-c))/16, so slot 2 is BOTH the exploration scale and the blend
    // weight; its coefficient on u is (16 - c), which is zero at 16 and negative above. Measured on
    // the 23 mate-in-one set at budget 256: 20/23 at c=1 down to 14/23 at c=16 (pure greed) and
    // 0/23 at c=360000. Its declared 8 is mid-slope. `uct_mcts_sum` selects on q + u, so slot 2 is
    // a pure exploration constant, and it reaches 23/23 from K=600 -- the net's declared eval scale.
    //
    // Comparing them at a single shared K would be a rigged test: the same number means different
    // things in the two encodings. Each is run at its own working value and the value is printed.
    let arms: [(&str, grammar::Program, i64); 2] = [
        // Pinned to 8, NOT uct_exploration(): that constant is now 600 for the sum encoding,
        // and the Mix form is pathological there (1604 ceiling hits vs the sum form's 1).
        ("Mix", reference::uct_mcts_mix(), 8),
        ("sum", reference::uct_mcts(), 600),
    ];
    // 512 and 2048 added after the exploration-term fix moved the matched-cost point:
    // budget 256 fell from 0.858x to 0.413x of alpha-beta's cost, so the value that justified
    // budget_mcts = 256 no longer holds and the parity point has to be re-found, not interpolated.
    for (name, mcts, kw) in &arms {
        println!("  -- encoding {name} at slot2 = {kw} --");
        for b in [16i64, 64, 256, 512, 1024, 2048, 4096] {
            let mut it = Interp::new(&net, vec![depth, 32_000, *kw]);
            let (mut found, mut cost) = (0u32, 0u64);
            for (p, forcing) in &set {
                let mv = it.run(mcts, p, b);
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
/// FITNESS 3 SPECIFIES MATE-N FOR N IN {1,2,3,4}, REPORTED **PER N**. The loop scores a single
/// pooled ratio over `mate_set` (MATE-1 only) plus two disagreement sets, and `forced_mate_set`
/// -- the MATE-2 generator -- is defined at line 57 and never used by any fitness set.
///
/// That is not a cosmetic difference. FITNESS 10's table of degenerate solutions opens with
///     | Prune everything / return eval | mates-per-cost filter (3); ladder (7) |
/// and the per-N split IS how filter (3) catches it: a program that does not search still finds
/// MATE-1, because mate-in-one is a one-ply check that costs nothing, but it cannot find MATE-2.
/// Pooling the two into one number destroys exactly the signal that separates them, which is why
/// the captured exploits score 18 of 25 while playing at 0.208.
///
/// This measures the claim instead of asserting it: every reference program, scored on MATE-1 and
/// MATE-2 SEPARATELY. `depth-one` is the hand-written non-searcher and is the one to watch.
fn mate_split() {
    let a = |i: usize, d: i64| std::env::args().nth(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (n1, n2, depth) = (a(2, 20) as usize, a(3, 20) as usize, a(4, 3));
    let net = Net::random(32, 20260907);
    let m1 = mate_set(n1);
    let m2 = forced_mate_set(n2, 20_000);
    println!("=== MATE-1 vs MATE-2, scored separately (FITNESS 3 asks for per-N) ===");
    println!("  MATE-1 set: {} positions   MATE-2 set: {} positions   depth {depth}\n",
             m1.len(), m2.len());
    println!("  {:<30} {:>12} {:>12}   {}", "program", "MATE-1", "MATE-2", "verdict");
    for (name, prog) in reference::all() {
        let bud = if name.contains("MCTS") { 256 } else { 16 };
        let (f1, _, _) = fitness(&prog, &m1, &net, depth, bud);
        let (f2, _, _) = fitness(&prog, &m2, &net, depth, bud);
        let (p1, p2) = (f1 as f64 / m1.len().max(1) as f64, f2 as f64 / m2.len().max(1) as f64);
        // A NON-SEARCHER is the signature: high on MATE-1, near zero on MATE-2. Anything that
        // holds up on both is doing real work.
        let v = if p1 >= 0.5 && p2 <= 0.1 { "NON-SEARCHER: aces MATE-1, fails MATE-2" }
                else if p2 >= 0.5 { "searches" }
                else if p1 <= 0.1 { "fails both" }
                else { "partial" };
        println!("  {name:<30} {f1:>5}/{:<6} {f2:>5}/{:<6}   {v}", m1.len(), m2.len());
    }
    println!("\n  If depth-one aces MATE-1 and fails MATE-2 while the seed holds both, the per-N");
    println!("  split separates non-searchers from searchers and the pooled ratio does not.");
}

fn valley_all() {
    let a = |i: usize, d: i64| std::env::args().nth(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (n1, n2, n3, depth) = (a(2, 15) as usize, a(3, 5) as usize, a(4, 5) as usize, a(5, 3));
    let net = Net::random(32, 20260907);
    let mut set = mate_set(n1);
    set.extend(disagreement_set(n2, depth, &net, 4_000));
    set.extend(window_sensitive_set(n3, depth, &net, 8, 4_000));
    // Same EXISTENCE_MATE2 arm as the main loop, so the ORACLE can score the set change. The
    // question this answers: `matesplit` showed capture extension scores 17/20 on MATE-1 and 18/20
    // on MATE-2 -- it searches -- while this set rates it 0.340x, the worst of any rung. If the set
    // is what is wrong, adding the MATE-2 rung should move the ladder ordering toward the truth.
    if std::env::var("EXISTENCE_MATE2").is_ok() {
        let before = set.len();
        set.extend(forced_mate_set(12, 20_000));
        println!("  EXISTENCE_MATE2: set {} -> {} positions", before, set.len());
    }

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
    // HARD SET SIZE, settable. The ranking defect needs an UNSATURATED dimension, and the hard set
    // is the only one there is: the seed scores 0/8 by construction. But 8 positions with capture
    // extension scoring 1 is not yet a gradient -- it is one position, and one position is as
    // consistent with luck as with a real signal. If the score scales with the set (about 5 of 40),
    // the dimension is real and can carry a ranking. If it stays at 1, the "hard set" is a single
    // lucky position wearing a plural.
    let n_hard: usize = std::env::var("EXISTENCE_HARD_N").ok()
        .and_then(|x| x.parse().ok()).unwrap_or(8);
    let hard = harder_set(n_hard, depth, &net, 3_000 * (n_hard as usize / 8).max(1));
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
        let mut it = Interp::new(&net, vec![depth, 32_000, interp::uct_exploration()]);
        it.cost_cap = cap;
        it.run(&seed, p, 16)
    }).collect();

    let seed_cost: u64 = set.iter().map(|p| {
        let mut it = Interp::new(&net, vec![depth, 32_000, interp::uct_exploration()]);
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
            let mut it = Interp::new(&net, vec![depth, 32_000, interp::uct_exploration()]);
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
            let mut it = Interp::new(&net, vec![depth, 32_000, interp::uct_exploration()]);
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
        let mut ia = Interp::new(&net, vec![depth, 32_000, interp::uct_exploration()]);
        let ma = ia.run(&seed, p, 16);
        let mut ib = Interp::new(&net, vec![depth, 32_000, interp::uct_exploration()]);
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
    let sc = gate::match_progs(&chall, &seed, &net, vec![depth, 32_000, interp::uct_exploration()], 16, pairs,
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
    // EDIT COUNT, arg 4. Was hardcoded to 1, which answers "what can ONE step do" -- the wrong
    // question for a conjunctive valley.
    //
    // Corrected 2026-09-10: Op::ProbeRead emits Field(Probe(Key(Var "p")), f) in a SINGLE edit and
    // Op::StoreHere emits a Store, so hash reuse is TWO well-placed edits, inside the loop's own 1-3
    // budget. Each half alone is worse (probe 0.991x, store 0.997x) and only the pair pays (1.024x),
    // so a 1-edit probe can only ever measure the valley FLOOR. Whether a 2-edit mutant installs both
    // halves at once and lands PATH-1 acceptable has never been measured, and it is the difference
    // between "the valley is impassable" and "the loop's own edit budget already jumps it".
    let edits: usize = std::env::args().nth(4).and_then(|s| s.parse().ok()).unwrap_or(1);
    println!("  edits per candidate: {edits}");
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
        let mut it = Interp::new(&net, vec![depth, 32_000, interp::uct_exploration()]);
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
    let mut both_halves = 0usize;
    let mut guard_ok_diff = 0usize;
    let mut diff_ops: std::collections::BTreeMap<String, usize> = Default::default();
    let mut lost: std::collections::BTreeMap<u32, usize> = Default::default();
    for k in 0..n {
        let mut r = Rng::new((k as u64) << 12 ^ 0xA5A5);
        // ONE edit, not the loop's usual 1-3: the question is what a SINGLE step can do.
        let (cand, ops) = match mutate::mutate_program_n(&seed, &mut r, edits) {
            Some(x) => x,
            None => { ill += 1; continue }
        };
        // BOTH halves of the rung? Probe without Store (or the reverse) is the valley floor by
        // construction; only the pair can pay.
        let ttc = tt_counts(&cand);
        if ttc[0] > 0 && ttc[1] > 0 { both_halves += 1; }
        let mut same = true;
        let mut ok = true;
        for (p, b) in set.iter().zip(&base) {
            let mut it = Interp::new(&net, vec![depth, 32_000, interp::uct_exploration()]);
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
    println!("  carrying BOTH TT halves (Probe AND Store) : {both_halves}");
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
    // ENV OVERRIDES for the two knobs the ladder measurement actually implicates, so a treatment
    // and a control can run SIMULTANEOUSLY against one config file instead of being separated by
    // an edit between launches. Both still default to the config, so an unset environment is
    // byte-identical to the previous behaviour.
    //
    // WHY THESE TWO, sized from `valleyall` rather than guessed:
    //   guard_tolerance 4 -> 7   the mates floor rises from 21 to 18, which admits capture
    //                            extension (18/25 mates, and the ONLY reference program that
    //                            scores on the hard set). It does NOT admit any exploit: UCT is
    //                            10, depth-one 5, proof-number 4, all still under 18.
    //   eps 0.02 -> 0.10         iterative deepening sits at 0.914x and needs 0.086 of tolerance
    //                            to survive into the population; the current band reaches 0.98.
    // Parsed at each knob's OWN type: eps is f64 and guard_tolerance is u32, so one generic
    // closure cannot serve both -- the first attempt inferred f64 from eps and failed to compile
    // against gtol. Typed separately rather than coerced, so a malformed value is a parse failure
    // here instead of a silently truncated tolerance later.
    let env_f64 = |k: &str| std::env::var(k).ok().and_then(|s| s.parse::<f64>().ok());
    let env_u32 = |k: &str| std::env::var(k).ok().and_then(|s| s.parse::<u32>().ok());
    (mu.expect("configs/search_track.conf declares no `mu`"),
     env_f64("EXISTENCE_EPS").unwrap_or_else(|| eps.expect("configs/search_track.conf declares no `eps`")),
     bmain.expect("configs/search_track.conf declares no `budget_main`"),
     bmcts.expect("configs/search_track.conf declares no `budget_mcts`"),
     env_u32("EXISTENCE_GUARD_TOL").unwrap_or_else(|| gtol.expect("configs/search_track.conf declares no `guard_tolerance`")))
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
        let mut here = Interp::new(net, vec![depth, 32_000, interp::uct_exploration()]);
        let a = here.run(&ab, &p, 16);
        let mut deeper = Interp::new(net, vec![depth + 1, 32_000, interp::uct_exploration()]);
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
        let mut a_it = Interp::new(net, vec![depth, 32_000, interp::uct_exploration()]);
        let a = a_it.run(&ab, &p, 16);
        let mut b_it = Interp::new(net, vec![depth, 32_000, interp::uct_exploration()]);
        let b = b_it.run(&hi, &p, 16);
        if a != board::types::MOVE_NONE && a != b {
            out.push((p.clone(), Some(a)));
        }
    }
    out
}

/// TT primitives split by KIND: `[Probe, Store, Key, Field]`.
///
/// WHY THIS EXISTS RATHER THAN JUST A COUNT. `tt_prims` pools all four kinds into one integer, and
/// on 2026-09-09 that pooling produced a retracted claim. Two population members each showing
/// `tt 3` were written up as "both halves of the rung retained simultaneously, neither acceptable
/// alone" -- a statement about COMPOSITION read off a metric that cannot express composition. `tt 3`
/// is equally `Key+Field+Probe` (a probe half), `Key+Field+Store` (a store half), or three `Key`s.
/// The two members may well have carried the SAME half, in which case the crossover precondition
/// was NOT observed and the conclusion inverts. See `ladder_valley_RESULT.md`.
///
/// The valley is CONJUNCTIVE (probe-only 0.991x, store-only 0.997x, both 1.024x), so "which half"
/// is the entire question. A counter that cannot answer it is decoration on the one measurement
/// that matters.
///
/// ONE TRAVERSAL, shared with `tt_prims`, so the count and the composition can never disagree --
/// duplicating these match arms would let the two fields drift silently, which is the same class of
/// defect this function exists to fix.
/// Count reads of the VALIDITY field. `Slot::flag` is what distinguishes a stored entry from an
/// empty one; interp's `probe()` returns `Slot::default()` on a miss, so `Field(Probe(..), Score)`
/// silently yields the CONSTANT 0 unless something tests `flag`. That is the third ingredient the
/// 2026-09-10 split measurement named, and this is how a child is checked for carrying it.
/// Count flag reads that are actually TESTED -- a `Field(_, Flag)` appearing as an operand of a
/// comparison, or as an `If` condition. This is the distinction that decides whether the three-way
/// experiment measures what it claims.
///
/// `flag_reads` counts the field being READ. A read is not a guard: `Field(Probe(k), Flag)` used as
/// a value simply substitutes one garbage number for another. The ingredient `ab_hash` actually
/// needs is `flag != 0` GATING the use of the stored score. Supplying a bare read and finding no
/// improvement would refute the validity-marker hypothesis for the wrong reason -- the instrument
/// would have withheld the thing under test.
fn flag_tests(p: &Program) -> usize {
    fn is_flag(n: &Node) -> bool {
        matches!(n, Node::Field(_, f) if *f == grammar::ast::FieldId::Flag)
    }
    fn walk(n: &Node, out: &mut usize) {
        use Node::*;
        match n {
            Cmp(a, b, _) | Pred(a, b, _) => { if is_flag(a) || is_flag(b) { *out += 1; } }
            If(c, _, _) => { if is_flag(c) { *out += 1; } }
            _ => {}
        }
        match n {
            Budget | Const(_) | Var(_) | OutcomeLit(_) | Nop => {}
            Moves(a) | Terminal(a) | Key(a) | Eval(a) | Ret(a) | Probe(a) | Field(a, _)
            | Set(_, a) => walk(a, out),
            Apply(a, b) | Max(a, b) | Min(a, b) | Avg(a, b) | ScoreOf(a, b) | Cmp(a, b, _)
            | Pred(a, b, _) | Loop(a, b) | Store(a, _, b) => { walk(a, out); walk(b, out) }
            Mix(a, b, c) => { walk(a, out); walk(b, out); walk(c, out) }
            Foreach(a, _, b) | Argmax(a, _, b) | Sort(a, _, b) | Sample(a, _, b)
            | Let(_, a, b) => { walk(a, out); walk(b, out) }
            If(c, t, e) => { walk(c, out); walk(t, out); if let Some(x) = e { walk(x, out) } }
            Call(_, args) | Arith(_, args) | TRead(_, args) => { for a in args { walk(a, out) } }
        }
    }
    let mut n = 0usize;
    for f in &p.funcs { walk(&f.body, &mut n); }
    n
}

fn flag_reads(p: &Program) -> usize {
    fn walk(n: &Node, out: &mut usize) {
        use Node::*;
        if let Field(_, f) = n { if *f == grammar::ast::FieldId::Flag { *out += 1; } }
        match n {
            Budget | Const(_) | Var(_) | OutcomeLit(_) | Nop => {}
            Moves(a) | Terminal(a) | Key(a) | Eval(a) | Ret(a) | Probe(a) | Field(a, _)
            | Set(_, a) => walk(a, out),
            Apply(a, b) | Max(a, b) | Min(a, b) | Avg(a, b) | ScoreOf(a, b) | Cmp(a, b, _)
            | Pred(a, b, _) | Loop(a, b) | Store(a, _, b) => { walk(a, out); walk(b, out) }
            Mix(a, b, c) => { walk(a, out); walk(b, out); walk(c, out) }
            Foreach(a, _, b) | Argmax(a, _, b) | Sort(a, _, b) | Sample(a, _, b)
            | Let(_, a, b) => { walk(a, out); walk(b, out) }
            If(c, t, e) => { walk(c, out); walk(t, out); if let Some(x) = e { walk(x, out) } }
            Call(_, args) | Arith(_, args) | TRead(_, args) => { for a in args { walk(a, out) } }
        }
    }
    let mut n = 0usize;
    for f in &p.funcs { walk(&f.body, &mut n); }
    n
}

fn tt_counts(p: &Program) -> [usize; 4] {
    fn walk(n: &Node, out: &mut [usize; 4]) {
        use Node::*;
        match n {
            Probe(_) => out[0] += 1,
            Store(..) => out[1] += 1,
            Key(_) => out[2] += 1,
            Field(..) => out[3] += 1,
            _ => {}
        }
        match n {
            Budget | Const(_) | Var(_) | OutcomeLit(_) | Nop => {}
            Moves(a) | Terminal(a) | Key(a) | Eval(a) | Ret(a) | Probe(a) | Field(a, _)
            | Set(_, a) => walk(a, out),
            Apply(a, b) | Max(a, b) | Min(a, b) | Avg(a, b) | ScoreOf(a, b) | Cmp(a, b, _)
            | Pred(a, b, _) | Loop(a, b) | Store(a, _, b) => { walk(a, out); walk(b, out) }
            Mix(a, b, c) => { walk(a, out); walk(b, out); walk(c, out) }
            Foreach(a, _, b) | Argmax(a, _, b) | Sort(a, _, b) | Sample(a, _, b)
            | Let(_, a, b) => { walk(a, out); walk(b, out) }
            If(c, t, e) => { walk(c, out); walk(t, out); if let Some(x) = e { walk(x, out) } }
            Call(_, args) | Arith(_, args) | TRead(_, args) => { for a in args { walk(a, out) } }
        }
    }
    let mut out = [0usize; 4];
    for f in &p.funcs { walk(&f.body, &mut out); }
    out
}

/// Composition tag for one member: `P2S1K1`, or `-` when it carries no TT primitive at all.
///
/// Read it against the valley table: a member tagged with `P` but no `S` is a probe half, `S`
/// without `P` is a store half, and only a member (or a crossover of two) carrying BOTH can be the
/// +2.4% rung. That distinction is invisible in the pooled count.
fn tt_kind_tag(p: &Program) -> String {
    let c = tt_counts(p);
    if c.iter().all(|&x| x == 0) { return "-".to_string(); }
    let mut s = String::new();
    for (ch, n) in ["P", "S", "K", "F"].iter().zip(c.iter()) {
        if *n > 0 { s.push_str(&format!("{ch}{n}")); }
    }
    s
}

/// Total TT primitives. Delegates to `tt_counts` so the pooled number and the per-kind tag are the
/// SAME traversal by construction -- a second copy of these match arms could drift from the first
/// without any test noticing, and a silently-disagreeing pair of fields is worse than one field.
fn tt_prims(p: &Program) -> usize { tt_counts(p).iter().sum() }

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

/// POSITIVE CONTROL for the `ttk` field. An instrument that cannot separate a probe half from a
/// store half on programs KNOWN to be one or the other is decoration, and the pooled `tt` count it
/// replaces already produced one retracted claim by being trusted without this check.
///
/// Prints the tag for every reference rung and ASSERTS the two that matter: `ab_probe_only` must
/// show `P` and no `S`, `ab_store_only` the reverse, and `ab_hash` must show both. A silent pass
/// here is what licenses reading `ttk` off a live run.
fn tt_kinds_control() {
    println!("=== ttk positive control: can the tag separate the valley halves? ===");
    let cases = [
        ("bare alpha-beta (seed)", reference::bare_alpha_beta()),
        ("probe only (never stores)", reference::ab_probe_only()),
        ("store only (never probes)", reference::ab_store_only()),
        ("hash reuse (both halves)", reference::ab_hash()),
        ("UCT MCTS", reference::uct_mcts()),
    ];
    for (name, p) in &cases {
        println!("  {:<28} tt {:>2}  ttk {}", name, tt_prims(p), tt_kind_tag(p));
    }
    let probe = tt_kind_tag(&reference::ab_probe_only());
    let store = tt_kind_tag(&reference::ab_store_only());
    let both = tt_kind_tag(&reference::ab_hash());
    assert!(probe.contains('P') && !probe.contains('S'),
            "probe-only tagged {probe:?} -- the tag cannot identify a probe half, so ttk is unusable");
    assert!(store.contains('S') && !store.contains('P'),
            "store-only tagged {store:?} -- the tag cannot identify a store half, so ttk is unusable");
    assert!(both.contains('P') && both.contains('S'),
            "hash-reuse tagged {both:?} -- the tag cannot identify the united rung, so ttk is unusable");
    // The count must equal the sum of the kinds, or the two printed fields disagree on the same run.
    for (name, p) in &cases {
        assert_eq!(tt_prims(p), tt_counts(p).iter().sum::<usize>(),
                   "{name}: pooled count and per-kind counts disagree");
    }
    println!("  PASS: probe={probe}  store={store}  hash={both}  (halves are distinguishable)");
}

/// Score REAL crossover children that carry both TT halves, on the valley set.
///
/// WHY. `crossover_can_carry_the_tt_rung_from_mcts` measured that 5.6% of `ab <- uct` grafts arrive
/// with `Probe` AND `Store`, which settles REACHABILITY and settles nothing else. Well-typed is not
/// correct, and correct is not fitter: `mutate.rs:437` records 90 of 106 well-typed candidates
/// rejected by the correctness oracle, and a UCT subtree grafted into alpha-beta is a far more
/// violent edit than the 1-3 mutations that produced those rejects.
///
/// The three outcomes are genuinely different findings and the run reports all three:
///   * children LOSE mates          -> the oracle is what stops the rung, not the surrogate
///   * children keep mates, ratio<1 -> a single graft does not pay what the 10-call-site hand-built
///                                    `ab_hash` pays; coverage is the missing ingredient
///   * children keep mates, ratio>1 -> the rung is reachable AND acceptable, and the search's
///                                    failure to find it is a question about the LIVE run, not the
///                                    grammar
///
/// Same `fitness`, same `mate_set`/`disagreement_set`/`window_sensitive_set`, same net and depth as
/// `valley`, so the ratios are directly comparable to the table at the top of
/// `ladder_valley_RESULT.md` rather than being a second harness with its own biases.
fn tt_graft() {
    let a = |i: usize, d: i64| std::env::args().nth(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (want, n1, n2, n3, depth) =
        (a(2, 40) as usize, a(3, 15) as usize, a(4, 5) as usize, a(5, 5) as usize, a(6, 3));
    // arg 7: LADDER MODE. 0 (default) = the shipped mixed set (mate-in-1 + disagreement + window).
    // 1 = FITNESS §3's stratified ladder, n1 at MATE-1, n2 at MATE-2, n3 at MATE-3.
    //
    // A MATE-n position needs 2n-1 plies to solve, so `depth` MUST be >= 2n-1 for the stratum to be
    // solvable at all: depth 3 covers MATE-1 and MATE-2, and MATE-3 needs depth 5. Running a MATE-3
    // stratum at depth 3 would report the SEED failing it, which reads as a broken seed rather than
    // as a set built past the search horizon -- so the mismatch is checked and announced.
    let ladder = a(7, 0) == 1;
    let net = Net::random(32, 20260907);
    let mut set;
    if ladder {
        set = mate_set_n(n1, 1, 300_000);
        if n2 > 0 { set.extend(mate_set_n(n2, 2, 300_000)); }
        if n3 > 0 { set.extend(mate_set_n(n3, 3, 300_000)); }
        let deepest = if n3 > 0 { 3 } else if n2 > 0 { 2 } else { 1 };
        let need = 2 * deepest - 1;
        println!("  LADDER MODE: {n1} MATE-1 + {n2} MATE-2 + {n3} MATE-3 = {} positions", set.len());
        if depth < need {
            println!("  ⚠ depth {depth} < {need} plies needed for MATE-{deepest}: that stratum is BEYOND");
            println!("    the search horizon and the seed will fail it. Not a broken seed -- a set");
            println!("    built past what the configured depth can see.");
        }
    } else {
        set = mate_set(n1);
        set.extend(disagreement_set(n2, depth, &net, 4_000));
        set.extend(window_sensitive_set(n3, depth, &net, 8, 4_000));
    }

    let ab = reference::bare_alpha_beta();
    let uct = reference::uct_mcts();
    let (base_found, base_cost, base_rate) = fitness(&ab, &set, &net, depth, 16);
    // PATH-1 BASELINE: the seed's chosen move on every position, computed ONCE.
    //
    // WHY THIS AND NOT THE MATE COUNT. `evolve` has two acceptance routes and they test different
    // things. The mate guard (scored above) asks "did it keep the forced wins". PATH 1 asks
    // "does it play the SAME MOVE everywhere, and cost less" -- and PATH 1 accepts with NO GAMES
    // AT ALL. `moveagree` measured that hash reuse plays 40/40 identical at 0.971x cost, so the
    // rung is PATH-1 shaped; `stepdiff` measured that 0 of 100 behaviour-preserving SINGLE EDITS
    // are cheaper, so single mutation never delivers one. Crossover has never been tested against
    // this route -- only against the mate guard, where it scored 0 of 40. Those are not the same
    // question and a graft could in principle pass one while failing the other.
    let base_moves: Vec<board::types::Move> = set.iter().map(|(p, _)| {
        let mut it = Interp::new(&net, vec![depth, 32_000, interp::uct_exploration()]);
        it.cost_cap = 20_000_000_000;
        it.run(&ab, p, 16)
    }).collect();
    println!("=== TT graft scoring: {} positions at depth {depth}, {want} both-halves children ===",
             set.len());
    println!("  seed: {base_found} mates, cost {base_cost}, rate {base_rate:.6}");
    println!("  reference hand-built rung `ab_hash` scores 1.024x on this set (10 call sites).\n");
    // POSITIVE CONTROL FOR THE PATH-1 TEST, run BEFORE any child is scored.
    //
    // A `0 / N` on "plays identically" is exactly the kind of clean zero that is indistinguishable
    // from a broken comparison, and `moveagree` already establishes the ground truth independently:
    // ab_hash plays 40/40 identical to the seed at 0.971x cost. So ab_hash MUST read same-play here.
    // If it does not, `base_moves` is wrong and every number below is noise.
    {
        let hash = reference::ab_hash();
        let mut ctl_same = true;
        for ((pp, _), b) in set.iter().zip(&base_moves) {
            let mut it = Interp::new(&net, vec![depth, 32_000, interp::uct_exploration()]);
            it.cost_cap = 20_000_000_000;
            if it.run(&hash, pp, 16) != *b { ctl_same = false; break; }
        }
        let (_, hcost, _) = fitness(&hash, &set, &net, depth, 16);
        println!("  CONTROL ab_hash: same-play {}  cost {} vs seed {}  -> PATH-1 acceptable: {}",
                 ctl_same, hcost, base_cost, ctl_same && hcost < base_cost);
        if !ctl_same {
            println!("  *** CONTROL FAILED: the seed's own known-identical rung does not match.");
            println!("  *** base_moves is wrong; ignore every PATH-1 number below.");
        }
    }
    println!("  {:<5} {:>6} {:>7} {:>16} {:>10}  {}", "#", "nodes", "mates", "cost", "vs seed", "ttk");

    let (mut tried, mut kept_mates, mut fitter, mut scored) = (0usize, 0usize, 0usize, 0usize);
    let (mut p1_same, mut p1_cheaper) = (0usize, 0usize);
    let mut best: Option<(f64, String)> = None;
    for k in 0..200_000u64 {
        if scored >= want { break; }
        tried += 1;
        let mut rng = mutate::Rng::new(k ^ 0x9E37_79B9);
        let Some(child) = mutate::crossover(&ab, &uct, &mut rng) else { continue };
        let c = tt_counts(&child);
        if c[0] == 0 || c[1] == 0 { continue; }          // need BOTH halves, not one
        scored += 1;
        let (found, cost, rate) = fitness(&child, &set, &net, depth, 16);
        let ratio = rate / base_rate.max(1e-12);
        if found >= base_found { kept_mates += 1; }
        // PATH-1 TEST on this child: identical play everywhere, and cheaper?
        let mut same_play = true;
        for ((p, _), b) in set.iter().zip(&base_moves) {
            let mut it = Interp::new(&net, vec![depth, 32_000, interp::uct_exploration()]);
            it.cost_cap = 20_000_000_000;
            if it.run(&child, p, 16) != *b { same_play = false; break; }
        }
        if same_play {
            p1_same += 1;
            if cost < base_cost { p1_cheaper += 1; }
        }
        if found >= base_found && ratio > 1.0 {
            fitter += 1;
            if best.as_ref().is_none_or(|(b, _)| ratio > *b) {
                best = Some((ratio, tt_kind_tag(&child)));
            }
        }
        println!("  {scored:<5} {:>+6} {found:>7} {cost:>16} {ratio:>9.3}x  {}",
                 child.size() as i64 - ab.size() as i64, tt_kind_tag(&child));
    }

    println!("\n  === summary ===");
    println!("  attempts to collect {scored} both-halves children : {tried}");
    println!("  kept all {base_found} mates          : {kept_mates} / {scored}");
    println!("  kept mates AND rate > seed  : {fitter} / {scored}");
    println!("  --- PATH 1 (accepts with NO GAMES: identical play + cheaper) ---");
    println!("  play IDENTICALLY to the seed : {p1_same} / {scored}");
    println!("  ...AND cheaper (PATH-1 ACCEPTABLE) : {p1_cheaper} / {scored}");
    match &best {
        Some((r, tag)) => println!("  best fitter child: {r:.3}x  ttk {tag}"),
        None => println!("  NO child was both mate-preserving and fitter."),
    }
    println!("\n  Read this against the three pre-registered outcomes in the doc comment: mates lost");
    println!("  indicts the ORACLE, mates kept with ratio<1 indicts single-site COVERAGE, and any");
    println!("  fitter child moves the question to why the LIVE search has not found one.");
}

/// Build FITNESS §3's MATE-1..4 ladder and report yield, cost and DISJOINTNESS.
///
/// Measured before being wired into `fitness`, because a stratified set whose strata overlap is
/// worse than no stratification: a MATE-3 set that silently contains mate-in-ones would report a
/// per-N threshold computed over the wrong population, and nothing downstream would reveal it.
///
/// POSITIVE CONTROL FIRST: every position `mate_set` produces must satisfy `mate_within(_, 1)`. If
/// the new solver disagrees with the shipped mate-in-1 builder, the solver is wrong and every number
/// below is noise. This is the check that the `ttk` field taught me to write first rather than last.
fn mate_ladder() {
    let a = |i: usize, d: i64| std::env::args().nth(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (count, maxn, cap) = (a(2, 8) as usize, a(3, 3) as u32, a(4, 300_000) as u64);
    println!("=== FITNESS §3 mate ladder: {count} positions at each of MATE-1..{maxn}, cap {cap} nodes ===");

    print!("  control: shipped mate_set(12) positions all solve as mate_within(1) ... ");
    let base = mate_set(12);
    let mut bad = 0;
    for (p, _) in &base {
        let mut q = p.clone();
        let mut nodes = 0u64;
        if !mate_within(&mut q, 1, &mut nodes, cap) { bad += 1; }
    }
    if bad > 0 {
        println!("FAIL — {bad} of {} disagree. The solver is wrong; ignore everything below.", base.len());
        return;
    }
    println!("PASS ({} of {})", base.len(), base.len());

    let mut sets: Vec<(u32, Vec<(Position, Option<board::Move>)>, f64)> = Vec::new();
    for n in 1..=maxn {
        let t0 = std::time::Instant::now();
        let s = mate_set_n(count, n, cap);
        let secs = t0.elapsed().as_secs_f64();
        println!("  MATE-{n}: built {:>3} of {count} in {secs:>7.1}s  ({:.2}s per position)",
                 s.len(), if s.is_empty() { 0.0 } else { secs / s.len() as f64 });
        sets.push((n, s, secs));
    }

    // DISJOINTNESS: a MATE-n position must NOT be solvable in n-1. mate_set_n enforces this at
    // construction; this re-checks it independently, because a filter that is also its own test
    // proves nothing.
    println!("\n  disjointness (a MATE-n position must NOT be mate-in-(n-1)):");
    for (n, s, _) in &sets {
        if *n < 2 || s.is_empty() { continue; }
        let mut leak = 0;
        for (p, _) in s {
            let mut q = p.clone();
            let mut nodes = 0u64;
            if mate_within(&mut q, n - 1, &mut nodes, cap) { leak += 1; }
        }
        println!("    MATE-{n}: {leak} of {} also solve in {} — {}",
                 s.len(), n - 1, if leak == 0 { "DISJOINT" } else { "LEAKING, strata are not clean" });
    }
    println!("\n  Yield falls off with N because these are mined by random play, not by §3's");
    println!("  retrograde walk from real endings. Cost per position is the number to watch: it");
    println!("  decides whether 500-per-stratum is affordable or whether the walk is required.");
}

/// How often do N edits produce a candidate carrying BOTH TT halves? Mutation only, NO fitness.
///
/// WHY SEPARATE FROM `stepdiff`. The both-halves rate is the one number in that sweep that n=40
/// cannot resolve: `ALL_OPS` has 11 operators drawn uniformly, so P(a run of 2 edits includes both
/// ProbeRead and StoreHere) = 2*(1/11)^2 = 0.0165, giving an expected 0.66 hits in 40 -- a zero
/// there measures the sample size, not the search space (`editcount_power_PREREG.md`).
///
/// The fix is not a longer stepdiff. Its cost is entirely the FITNESS evaluation, ~13.5s per
/// candidate; the mutation and the kind-count are microseconds. Dropping fitness makes n=5000
/// trivial and answers the reachability question exactly, leaving stepdiff to answer the
/// correctness/cost questions it is actually powered for.
///
/// Reports the ceiling too: mutations that produce a PROBE-side and a STORE-side separately. If the
/// pair rate matches 2*(1/11)^2 the operators compose freely and only DRAW limits them; if it is
/// far below, placement is refusing the combination and that is a different problem.
fn tt_reach() {
    let a = |i: usize, d: i64| std::env::args().nth(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (n, maxe) = (a(2, 5000) as usize, a(3, 3) as usize);
    let seed = reference::bare_alpha_beta();
    println!("=== TT-primitive reachability by edit count, {n} mutations each, NO fitness ===");
    println!("  expected pair rate if only DRAW limits: edits=2 -> {:.4}, edits=3 -> {:.4}",
             2.0 / 121.0, 1.0 - (2.0 * (10.0f64 / 11.0).powi(3) - (9.0f64 / 11.0).powi(3)));
    println!("  {:<7} {:>8} {:>10} {:>10} {:>10} {:>12}",
             "edits", "welltyped", "probe-side", "store-side", "BOTH", "both-rate");
    for e in 1..=maxe {
        let (mut wt, mut ps, mut ss, mut both) = (0usize, 0usize, 0usize, 0usize);
        for k in 0..n {
            let mut r = Rng::new((k as u64) << 12 ^ 0xA5A5 ^ (e as u64) << 40);
            let Some((cand, _)) = mutate::mutate_program_n(&seed, &mut r, e) else { continue };
            wt += 1;
            let c = tt_counts(&cand);
            if c[0] > 0 { ps += 1; }
            if c[1] > 0 { ss += 1; }
            if c[0] > 0 && c[1] > 0 { both += 1; }
        }
        println!("  {:<7} {:>8} {:>10} {:>10} {:>10} {:>11.4}",
                 e, wt, ps, ss, both, both as f64 / wt.max(1) as f64);
    }
    println!("\n  A pair rate at or near the expected line means the operators COMPOSE and only the");
    println!("  DRAW limits them -- so the loop's 1-3 budget does install both halves, and the");
    println!("  barrier is downstream (the valley: probe-only 0.991x, store-only 0.997x, pair 1.024x).");
    println!("  A rate far BELOW it means placement is refusing the combination, which is a");
    println!("  different problem needing targeted operators rather than plateau tolerance.");
}

/// Do the TT primitives mutation installs ever actually HIT? `evolve tthits <n> <edits> <depth>`
///
/// `editcount_RESULT.md` concluded the barrier is SEMANTIC PLACEMENT: mutation installs a Probe and
/// a Store together at exactly the rate operator draw predicts (0.0182 observed vs 0.0165 expected
/// at 2 edits), yet none of those candidates is ever cheaper or guard-passing. The explanation --
/// that an arbitrarily-placed probe and store are not a transposition table -- was an INFERENCE from
/// those two facts. This measures it: a working table produces HITS, a decorative pair produces none.
///
/// POSITIVE CONTROL FIRST, and it is the whole point. `ab_hash` is a hand-built working TT, so it
/// MUST register hits under this harness. If it does not, `probe_stats` is wired wrong and every
/// zero below is meaningless -- the same trap as reading `0/12` before checking that a known-good
/// program reads non-zero.
fn tt_hits() {
    let a = |i: usize, d: i64| std::env::args().nth(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (n, edits, depth) = (a(2, 400) as usize, a(3, 3) as usize, a(4, 3));
    let net = Net::random(32, 20260907);
    let seed = reference::bare_alpha_beta();
    let set = mate_set(6);

    let run = |prog: &Program| -> (u64, u64, u64, u64, u64) {
        let (mut c, mut h, mut st, mut zp, mut zs) = (0u64, 0u64, 0u64, 0u64, 0u64);
        for (p, _) in &set {
            let mut it = Interp::new(&net, vec![depth, 32_000, interp::uct_exploration()]);
            it.cost_cap = 20_000_000_000;
            let _ = it.run(prog, p, 16);
            let (cc, hh, ss) = it.probe_stats();
            let (zzp, zzs) = it.zero_key_stats();
            c += cc; h += hh; st += ss; zp += zzp; zs += zzs;
        }
        (c, h, st, zp, zs)
    };

    println!("=== do installed TT primitives ever HIT? {n} mutants at {edits} edits, depth {depth} ===");
    let (sc, sh, ss, szp, szs) = run(&reference::ab_hash());
    println!("  CONTROL ab_hash (hand-built, working TT): {sc} probes, {sh} HITS ({:.1}%), {ss} stores, {:.1} probes/store",
             100.0 * sh as f64 / sc.max(1) as f64, sc as f64 / ss.max(1) as f64);
    println!("           zero-key: {szp} probes ({:.1}%), {szs} stores ({:.1}%)",
             100.0 * szp as f64 / sc.max(1) as f64, 100.0 * szs as f64 / ss.max(1) as f64);
    if sh == 0 {
        println!("  *** CONTROL FAILED: a known-working table registered zero hits.");
        println!("  *** probe_stats is wired wrong; every number below is meaningless.");
        return;
    }
    let (bc, bh, bs, _, _) = run(&seed);
    println!("  seed bare_alpha_beta (no TT at all)     : {bc} probes, {bh} hits, {bs} stores  (must be 0/0/0)");

    let (mut both, mut probed, mut any_hit, mut tot_c, mut tot_h) = (0usize, 0usize, 0usize, 0u64, 0u64);
    let mut tot_s = 0u64;
    let (mut tot_zp, mut tot_zs) = (0u64, 0u64);
    for k in 0..n {
        let mut r = Rng::new((k as u64) << 12 ^ 0x77AA ^ (edits as u64) << 40);
        let Some((cand, _)) = mutate::mutate_program_n(&seed, &mut r, edits) else { continue };
        let c = tt_counts(&cand);
        if c[0] == 0 || c[1] == 0 { continue; }
        both += 1;
        let (cc, hh, ss, zzp, zzs) = run(&cand);
        tot_c += cc; tot_h += hh; tot_s += ss; tot_zp += zzp; tot_zs += zzs;
        if cc > 0 { probed += 1; }
        if hh > 0 { any_hit += 1; }
    }
    println!("\n  mutants carrying BOTH halves      : {both} of {n}");
    println!("  ...that actually EXECUTE a probe  : {probed}");
    println!("  ...that ever get a HIT            : {any_hit}");
    println!("  total across them                 : {tot_c} probes, {tot_h} hits, {tot_s} stores");
    println!("  PROBES PER STORE                  : {:.1}   (ab_hash control: {:.1})",
             tot_c as f64 / tot_s.max(1) as f64, sc as f64 / ss.max(1) as f64);
    println!("  ZERO-KEY (the `_ => 0` fallback)   : {tot_zp} probes ({:.1}%), {tot_zs} stores ({:.1}%)",
             100.0 * tot_zp as f64 / tot_c.max(1) as f64,
             100.0 * tot_zs as f64 / tot_s.max(1) as f64);
    println!("  Node::Probe and Node::Store both do `match val!(k) {{ Value::Key(x) => x, _ => 0 }}`,");
    println!("  so a key expression that is not a Key silently collapses to slot ZERO. If these");
    println!("  percentages are high, every store writes one slot and every probe reads it -- which");
    println!("  explains a 1:1 store/probe ratio and a high hit rate from a SINGLE cause.");
    println!("  A real TT stores about as often as it probes -- each new node probes, misses,");
    println!("  searches, stores. A memo cell stores a few times and probes millions.");
    // CLOSING TEXT CORRECTED 2026-09-10, BY THIS TOOL'S OWN FIRST RUN. It previously read "A pair
    // that never hits is not a transposition table", which is what I expected and is FALSE: 8 of 10
    // both-halves mutants DO hit, at a 62.1% hit rate against ab_hash's 1.0%. Asserting the expected
    // conclusion in the output is how a tool stops being able to surprise you.
    println!("\n  READ THE HIT RATE, NOT JUST THE HIT COUNT. A real transposition table mostly MISSES:");
    println!("  every new node is a position not seen before, which is why the hand-built ab_hash");
    println!("  sits at ~1%. A HIGH hit rate means the probe keeps returning the SAME slot -- one key,");
    println!("  read over and over -- which is a scratch variable, not a transposition table. And at");
    println!("  12 cost units per probe it is an expensive one.");
}

/// CAN crossover unite the two halves into a WORKING table? `evolve ttunion <n> <depth>`
///
/// The live arms hold both halves simultaneously -- `gate_composition_s2` gen 4-6 carries `P1K1F1`
/// probe halves and `S1K1` store halves as separate members -- but a union has never appeared, and
/// the power says that is a coin flip: P(a union is even ATTEMPTED per generation) ~ 0.20, so
/// P(none in the three generations of coexistence) = 0.51. Waiting 19 more generations would answer
/// it at 0.99, and this answers it now.
///
/// `ab_probe_only` and `ab_store_only` ARE the two halves, hand-built and full-coverage. Crossing
/// them isolates the question the arms are sampling slowly: given both halves in hand, does the
/// crossover operator produce a child that behaves like a transposition table?
///
/// CONTROLS, both printed and both gating:
///   * `ab_hash` is the hand-built union -- it MUST read PATH-1 acceptable and ~14 probes/store.
///   * `ab_probe_only` alone MUST show probes and ZERO stores; `ab_store_only` the reverse. If the
///     halves are not actually complementary, nothing below means anything.
fn tt_union() {
    let a = |i: usize, d: i64| std::env::args().nth(i).and_then(|s| s.parse().ok()).unwrap_or(d);
    let (n, depth) = (a(2, 400) as usize, a(3, 3));
    // arg 4: MINIMAL mode. 0 (default) crosses the HAND-BUILT halves ab_probe_only (P10K10F10) and
    // ab_store_only (S5K5). 1 crosses MUTATION-GENERATED minimal halves -- P1K1F1 and S1K1, one call
    // site each -- which is what the LIVE population actually contains.
    //
    // This is the limit the hand-built run named and could not test. `gate_composition_s2` gens 4-6
    // hold `P1K1F1` probe halves and `S1K1` store halves as separate members; ab_hash's entire 2.4%
    // gain comes from TEN call sites at a 1% hit rate, so coverage plainly matters and a union of two
    // one-site halves need not reproduce the hand-built 5.5%.
    let mode = a(4, 0);
    let minimal = mode >= 1;   // 1 = minimal halves, 2 = minimal + validity guard
    let three_way = mode >= 2;
    let need_test = mode == 3;   // 3 = third half must TEST the flag, not merely read it
    let net = Net::random(32, 20260907);
    let seed = reference::bare_alpha_beta();
    let set = mate_set(6);

    let run = |prog: &Program| -> (u64, u64, u64) {
        let (mut c, mut h, mut st) = (0u64, 0u64, 0u64);
        for (p, _) in &set {
            let mut it = Interp::new(&net, vec![depth, 32_000, interp::uct_exploration()]);
            it.cost_cap = 20_000_000_000;
            let _ = it.run(prog, p, 16);
            let (cc, hh, ss) = it.probe_stats();
            c += cc; h += hh; st += ss;
        }
        (c, h, st)
    };
    let base_moves: Vec<board::types::Move> = set.iter().map(|(p, _)| {
        let mut it = Interp::new(&net, vec![depth, 32_000, interp::uct_exploration()]);
        it.cost_cap = 20_000_000_000;
        it.run(&seed, p, 16)
    }).collect();
    let plays_same = |prog: &Program| -> bool {
        set.iter().zip(&base_moves).all(|((p, _), b)| {
            let mut it = Interp::new(&net, vec![depth, 32_000, interp::uct_exploration()]);
            it.cost_cap = 20_000_000_000;
            it.run(prog, p, 16) == *b
        })
    };

    println!("=== can crossover UNITE the halves into a working table? {n} attempts, depth {depth} ===");
    let (_, bc, _) = fitness(&seed, &set, &net, depth, 16);
    for (name, prog) in [("ab_probe_only", reference::ab_probe_only()),
                         ("ab_store_only", reference::ab_store_only()),
                         ("ab_hash (the UNION)", reference::ab_hash())] {
        let (c, h, st) = run(&prog);
        let (_, cost, _) = fitness(&prog, &set, &net, depth, 16);
        println!("  CONTROL {:<20} {:>9} probes {:>9} hits {:>9} stores  {:>6} p/s  same-play {}  cheaper {}",
                 name, c, h, st,
                 if st == 0 { "inf".to_string() } else { format!("{:.1}", c as f64 / st as f64) },
                 plays_same(&prog), cost < bc);
    }

    // Build the MINIMAL halves the way the population does: single mutations off the seed, kept when
    // they carry exactly one side. Reported so the comparison is against a stated pair, not a guess.
    let (mut min_probe, mut min_store): (Option<Program>, Option<Program>) = (None, None);
    let mut min_guard: Option<Program> = None;
    if minimal {
        for k in 0..20_000u64 {
            if min_probe.is_some() && min_store.is_some() { break; }
            let mut r = Rng::new(k << 12 ^ 0x3417);
            let Some((c2, _)) = mutate::mutate_program_n(&seed, &mut r, 1) else { continue };
            let c = tt_counts(&c2);
            if c[0] > 0 && c[1] == 0 && min_probe.is_none() { min_probe = Some(c2); continue; }
            if c[1] > 0 && c[0] == 0 && min_store.is_none() { min_store = Some(c2); }
        }
        match (&min_probe, &min_store) {
            (Some(pp), Some(ss)) => println!("  MINIMAL halves from single mutation: probe {} / store {}",
                                             tt_kind_tag(pp), tt_kind_tag(ss)),
            _ => { println!("  ABORT: could not build both minimal halves by single mutation"); return; }
        }
        if three_way {
            for k in 0..60_000u64 {
                if min_guard.is_some() { break; }
                let mut r = Rng::new(k << 12 ^ 0x9F3B);
                let Some((c2, _)) = mutate::mutate_program_n(&seed, &mut r, 1) else { continue };
                let ok = if need_test { flag_tests(&c2) > 0 } else { flag_reads(&c2) > 0 };
                if ok { min_guard = Some(c2); }
            }
            match &min_guard {
                Some(g) => println!("  THIRD half ({}): {} with {} flag read(s), {} of them TESTED",
                                    if need_test { "validity TEST" } else { "bare flag read" },
                                    tt_kind_tag(g), flag_reads(g), flag_tests(g)),
                None => { println!("  ABORT: no single mutation produced the required flag {} in 60k draws \
-- that ingredient is NOT single-edit reachable, which is itself the answer",
                                   if need_test { "TEST" } else { "read" }); return; }
            }
        }
    }
    let (mut wt, mut both, mut same, mut acceptable) = (0usize, 0usize, 0usize, 0usize);
    let (mut tc, mut th, mut ts) = (0u64, 0u64, 0u64);
    // SPLIT THE TABLE STATS BY SOUNDNESS. The pooled numbers cannot answer the question the n=1000
    // result raised: 77% of unions change the answer, and the ones that do NOT could be sound for
    // either of two opposite reasons -- their table works, or their probe never hits and the pair is
    // inert. Those predict opposite hit rates, so the split settles it and an argument cannot.
    let (mut sc, mut sh, mut ss_) = (0u64, 0u64, 0u64); // sound (plays identically)
    let (mut uc, mut uh, mut us) = (0u64, 0u64, 0u64); // unsound (changes the answer)
    let (mut n_sound_nohit, mut n_unsound_nohit) = (0usize, 0usize);
    for k in 0..n {
        let mut r = Rng::new((k as u64) << 12 ^ 0x5E11);
        let (rec, don) = if minimal {
            match (&min_probe, &min_store) {
                (Some(pp), Some(ss)) if k % 2 == 0 => (pp.clone(), ss.clone()),
                (Some(pp), Some(ss))               => (ss.clone(), pp.clone()),
                _ => break,
            }
        } else if k % 2 == 0 { (reference::ab_probe_only(), reference::ab_store_only()) }
          else               { (reference::ab_store_only(), reference::ab_probe_only()) };
        let Some(child) = mutate::crossover(&rec, &don, &mut r) else { continue };
        // THREE-WAY: fold the validity guard into the probe-store child. The split measurement says
        // a miss injects the constant 0, so a union without a flag test is unsound by construction;
        // this asks whether supplying the third ingredient is what raises soundness.
        let child = if three_way {
            match &min_guard {
                Some(g) => match mutate::crossover(&child, g, &mut r) { Some(c) => c, None => continue },
                None => continue,
            }
        } else { child };
        wt += 1;
        let c = tt_counts(&child);
        if c[0] == 0 || c[1] == 0 { continue; }
        both += 1;
        let (cc, hh, ss) = run(&child);
        tc += cc; th += hh; ts += ss;
        let (_, cost, _) = fitness(&child, &set, &net, depth, 16);
        let sp = plays_same(&child);
        if sp {
            sc += cc; sh += hh; ss_ += ss;
            if hh == 0 { n_sound_nohit += 1; }
            same += 1;
            if cost < bc { acceptable += 1; }
        } else {
            uc += cc; uh += hh; us += ss;
            if hh == 0 { n_unsound_nohit += 1; }
        }
    }
    println!("\n  well-typed children      : {wt} of {n}");
    println!("  carrying BOTH halves     : {both}");
    println!("  ...that play IDENTICALLY : {same}");
    println!("  ...AND cheaper (PATH-1)  : {acceptable}");
    if both > 0 {
        println!("  table behaviour across them: {tc} probes, {th} hits ({:.1}%), {ts} stores, {:.1} probes/store",
                 100.0 * th as f64 / tc.max(1) as f64, tc as f64 / ts.max(1) as f64);
        println!("  compare ab_hash, a working table: ~1% hits, ~14 probes/store");
        // The discriminator, both arms on the same lines so they read against each other.
        let pct = |h: u64, c: u64| 100.0 * h as f64 / c.max(1) as f64;
        let per = |c: u64, s: u64| c as f64 / s.max(1) as f64;
        println!("\n  SPLIT BY SOUNDNESS (the question: do the sound ones REUSE, or merely not fire?)");
        println!("    sound   (plays identically): {sh} hits / {sc} probes = {:.1}%,  {:.1} probes/store,  {n_sound_nohit} of {same} never hit at all",
                 pct(sh, sc), per(sc, ss_));
        println!("    unsound (changes the answer): {uh} hits / {uc} probes = {:.1}%,  {:.1} probes/store,  {n_unsound_nohit} of {} never hit at all",
                 pct(uh, uc), per(uc, us), both - same);
        println!("    READING: sound ~0% hits  -> they are sound because the pair is INERT, and PATH-1");
        println!("             would be accepting no-ops. Sound hits ~= unsound hits -> soundness is");
        println!("             about WHAT is stored, not whether it is read, and a validity/bound");
        println!("             marker is the missing ingredient (ab_hash needs flag != 0 + bound type).");
    }
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("ttunion") {
        return tt_union();
    }
    if std::env::args().nth(1).as_deref() == Some("tthits") {
        return tt_hits();
    }
    if std::env::args().nth(1).as_deref() == Some("ttreach") {
        return tt_reach();
    }
    if std::env::args().nth(1).as_deref() == Some("mateladder") {
        return mate_ladder();
    }
    if std::env::args().nth(1).as_deref() == Some("ttk") {
        return tt_kinds_control();
    }
    if std::env::args().nth(1).as_deref() == Some("ttgraft") {
        return tt_graft();
    }
    if std::env::args().nth(1).as_deref() == Some("valley") {
        return valley();
    }
    if std::env::args().nth(1).as_deref() == Some("mctsbudget") {
        return mcts_budget();
    }
    if std::env::args().nth(1).as_deref() == Some("matesplit") {
        mate_split();
        return;
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
    // n3 AND gate_pairs ARE PARSED HERE, BEFORE ANY SET IS BUILT, so the echo below can run first.
    // They used to be read further down, interleaved with construction; parsing is side-effect-free
    // so hoisting them changes nothing except WHEN the values are known.
    let n3: usize = std::env::args().nth(6).and_then(|s| s.parse().ok()).unwrap_or(5);
    let gate_pairs: usize = std::env::args().nth(7).and_then(|s| s.parse().ok()).unwrap_or(6);
    // ECHO THE DECODED POSITIONAL ARGS *BEFORE* THE EXPENSIVE WORK.
    //
    // The header used to report only the ENV knobs (HARD_FITNESS, SPEC_FILTER) and none of the
    // POSITIONAL ones, so two runs with completely different set composition and search depth
    // printed byte-identical headers. That cost a real arm on 2026-09-09: the order is NOT
    // contiguous --
    //     evolve <gens> <pop> <n1> <n2> <DEPTH> <n3> <gate_pairs>
    // because n3 was appended after depth, while the `ttgraft` subcommand in this same binary takes
    // (want, n1, n2, n3, depth), which IS. I carried the ttgraft order over and launched
    // `25 8 4 10 10 3` intending depth 3 / n3 10, and actually ran DEPTH 10 / n3 3. Every position
    // then hit the 2e9 per-position cost cap and the arm burned 268 seconds emitting nothing.
    //
    // POSITION MATTERS AS MUCH AS CONTENT. My first attempt at this fix printed the same line from
    // the existing header block -- which runs AFTER set construction -- so at depth 10 it would
    // never have printed at all, and the mistake would have stayed just as invisible. A diagnostic
    // that only appears once the expensive work succeeds cannot diagnose the expensive work.
    println!("  set {}+{}+{}={} positions (mate-in-1 {:.0}%), depth {}, gate {} pairs  \
[args: gens pop n1 n2 DEPTH n3 gate_pairs — depth is arg 5, n3 is arg 6]",
             n1, n2, n3, n1 + n2 + n3,
             100.0 * n1 as f64 / (n1 + n2 + n3).max(1) as f64, depth, gate_pairs);
    let net = Net::random(32, 20260907);
    // MIXED on purpose: mate-in-1 alone made the surrogate maximisable by searching less.
    let mut set = mate_set(n1);
    // Built by DISAGREEMENT at the fitness depth, not by mate distance. See disagreement_set.
    let deep = disagreement_set(n2, depth, &net, 4_000);
    // Third component: window-sensitive positions, closing the raised-alpha exploit.
    // GAME-GATE PAIRS. Small on purpose: a game at fitness depth is ~200x a single fitness
    // evaluation, so this is the expensive half and it only runs on a surrogate improvement.
    // 6 pairs = 12 games resolves a large effect, which is the only kind worth promoting here;
    // it CANNOT resolve a 2% edge and is not asked to. It is a veto on unplayable programs.
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
    // PER-N PROBES, scored on every captured exploit so the corpus records the dimension FITNESS 3
    // specifies. `matesplit` measured that MATE-2 separates a shallow searcher from a real one --
    // UCT falls 11/20 to 2/20 while every alpha-beta variant holds 20/20 -- and this loop's own set
    // is 15/25 MATE-1 with no MATE-2 at all. Without these two numbers, "the exploit only finds
    // shallow mates" stays an INFERENCE about specimens that cannot be reloaded: the dumps are
    // `{:#?}` and there is no text format for Program. With them it is a measurement taken while
    // the program is still in hand. Built once; scored only on a capture, not per candidate.
    let m1_probe = mate_set(12);
    let m2_probe = forced_mate_set(12, 20_000);
    let n_win = win.len();
    let n_deep = deep.len();
    set.extend(deep);
    set.extend(win);
    // EXISTENCE_MATE2: add the MATE-2 rung FITNESS 3 asks for and this loop has never had.
    //
    // THE EXPERIMENT THIS ENABLES. `EXISTENCE_GUARD_TOL=7` reproduces an exploit reliably -- two
    // specimens from two independent seeds, both landing on exactly 18 mates and 0.208 games -- so
    // it is an instrument rather than an anecdote. Running that same configuration WITH MATE-2 in
    // the set asks whether the spec's per-N design is what stops the exploit class. If exploits
    // keep appearing, the per-N story is wrong and the defence is somewhere else.
    //
    // `matesplit` already showed MATE-2 discriminates: UCT scores 11/20 on MATE-1 and 2/20 on
    // MATE-2, while every alpha-beta variant holds 20/20. A program that finds mates one ply away
    // cannot find mates two plies away without searching, and the current set never asks it to.
    //
    // Off by default: this changes the guard floor, the surrogate scale and every banked number, so
    // it is an A/B arm and not a silent redefinition of the fitness.
    if std::env::var("EXISTENCE_MATE2").is_ok() {
        let n_before = set.len();
        set.extend(m2_probe.iter().cloned());
        println!("  EXISTENCE_MATE2: set {} -> {} positions ({} MATE-2 added)",
                 n_before, set.len(), m2_probe.len());
    }

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
        /// Programs already sent to the ladder. Used ONLY by the spec filter: under the strict
        /// rule, re-proposal is prevented by raising `best_rate` to a rejected candidate's rate,
        /// but under a TOLERANCE filter that same update ratchets the bar DOWNWARD, because every
        /// rejection lowers the reference the next 0.9x is measured against. The filter therefore
        /// records what it has already tried and leaves `best_rate` to the ACCEPT paths alone.
        gated: std::collections::HashSet<String>,
    }
    let mut lineages: Vec<Lineage> = Vec::new();
    for (name, seed_prog, bud) in [
        ("MAIN", reference::bare_alpha_beta(), budget_main),
        // SUM ENCODING AS THE SEED, changed 2026-09-09 on measurement. The Mix form reads table
        // slot 2 both as the exploration scale and as the weight of Mix(q,u,c)=(q*c+u*(16-c))/16,
        // so u's coefficient is (16-c): zero at 16, negative above. That is not a tuning problem,
        // it caps the encoding. Measured at matched cost (~1.0x bare alpha-beta) on the 25-position
        // mate set: Mix 11/25 at budget 512 (0.968x), sum 17/25 at budget 1024 (1.085x). On
        // mate-in-one, Mix tops out at 20/23 at ANY weight while sum reaches 23/23 from K=600.
        // The lineage was seeded with a program whose exploration term is partly cancelled by its
        // own blend weight, and every one of its gates reads exactly 0.500 (STATE.md:1483).
        // EXISTENCE_MCTS_SEED=mix restores the historical blend encoding, so the reseed can be
        // measured as an A/B instead of asserted. Without a switch the claim "the seed is why every
        // MCTS gate reads 0.500" would be untestable: the old behaviour would no longer exist to
        // compare against. Defaults to the declared (sum) program.
        ("MCTS",
         if std::env::var("EXISTENCE_MCTS_SEED").as_deref() == Ok("mix") {
             reference::uct_mcts_mix()
         } else {
             reference::uct_mcts()
         },
         budget_mcts),
    ] {
        let (f, c, r) = fitness(&seed_prog, &set, &net, depth, bud);
        // THE INCUMBENT MUST USE THE SAME FORMULA AS THE CANDIDATES. With
        // EXISTENCE_HARD_FITNESS the candidate rate is (f + hf) / (cst + hcst); leaving the seed's
        // rate at f / cst compares the two arms on different work, and since the seed scores hf = 0
        // the new formula only enlarges its denominator -- so EVERY candidate scored below an
        // incumbent measured a different way.
        //
        // Observed exactly that before this fix: "rates 0.894-0.927x [>=.98:0 ...] pop 1". The max
        // candidate rate fell BELOW 1.000x, nothing cleared EPS, and the population collapsed to a
        // single member. The flag looked like it made the search strictly worse; it had only made
        // the comparison invalid.
        let (f, c, r) = if std::env::var("EXISTENCE_HARD_FITNESS").is_ok() {
            let (shf, shc, _) = fitness(&seed_prog, &hard, &net, depth, bud);
            let cc = c.saturating_add(shc);
            (f, cc, (f + shf) as f64 * 1e6 / cc.max(1) as f64)
        } else { (f, c, r) };
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
            gated: Default::default(),
        });
    }
    let (seed_hard, _, _) = fitness(&reference::bare_alpha_beta(), &hard, &net, depth, budget_main);
    println!("  HARD set: {} positions the seed FAILS by construction; seed scores {seed_hard}/{} \
(a real gradient, unlike the saturated 25/25 guard set)", hard.len(), hard.len());
    // THE HEADER MUST NAME THE ARM'S CONFIGURATION, because the analysis reads the HEADER and not
    // the filename. `ab_report.py` was detecting HARD_FITNESS from the seed's own surrogate falling
    // to 0.002084 -- which is correct for whether the flag is set, and blind to the WEIGHT, because
    // the seed scores hf=0 and w*0 = 0 at every weight. A weight-1 and a weight-4 arm therefore have
    // byte-identical headers and would be pooled as one condition. That is the same class of error
    // as counting duplicate trajectories as independent observations.
    // SPEC_FILTER MUST BE IN THE HEADER TOO. Without it a spec-filter arm and a strict-rule arm
    // print byte-identical headers, so `ab_report.py` pools them and the comparison vanishes into
    // an average. That is the SIXTH time tonight a key could not express a varied dimension; the
    // fix is the same each time -- if the header cannot say it, ADD A FIELD, do not infer it from
    // a filename. Absent field == off, which is correct for every arm that predates this line.
    let sf_cfg = if std::env::var("EXISTENCE_SPEC_FILTER").is_ok() { ", SPEC_FILTER on" }
                 else { ", SPEC_FILTER off" };
    let hard_cfg = if std::env::var("EXISTENCE_HARD_FITNESS").is_ok() {
        let w: f64 = std::env::var("EXISTENCE_HARD_WEIGHT")
            .ok().and_then(|s| s.parse().ok()).unwrap_or(1.0);
        format!(", HARD_FITNESS on weight {w:.2}")
    } else {
        ", HARD_FITNESS off".to_string()
    };
    println!("  population MU={MU}, lambda={pop}, EPS={EPS:.3}, guard tolerance {guard_tolerance}{hard_cfg}{sf_cfg} \
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

    // SEED IS SETTABLE (EXISTENCE_EVOLVE_SEED), defaulting to the original constant so an
    // unset environment reproduces every prior run bit-for-bit.
    //
    // WHY: this was hardcoded, so every invocation of this track was the SAME RUN. I aggregated
    // search_track.log (34 lineage-generations) and hist_probe.log (5) as 39 independent
    // observations; gen-3 MAIN is field-for-field identical between them, so the true n was 34 and
    // the probe contributed no new trajectory at all.
    //
    // That matters more here than it would elsewhere. Today's central statistical result is that
    // single-seed conclusions FLIP SIGN on a second seed -- blend at z = 3.3, epochs at z = 3.6 --
    // so a track that cannot produce a second trajectory cannot distinguish a finding from its own
    // one run. Every claim this file has emitted is single-trajectory by construction.
    //
    // THE FIRST ATTEMPT AT THIS BOUND NOTHING. It wrapped the variable in an `Rng` whose only two
    // uses in the whole file were `let _ = rng.next()` -- both discarded. Seed 777 and the default
    // therefore emitted byte-identical generations, which is exactly what the two-way check caught:
    //     seedchk_default  gen 1 MAIN gate REJECT 0.417+/-0.103  surrogate 0.002794
    //     seedchk_777      gen 1 MAIN gate REJECT 0.417+/-0.103  surrogate 0.002794
    // The actual mutation draw is seeded per SLOT, from (g, li, i) and a hardcoded 0xBEEF, so the
    // run seed has to be mixed in THERE or it does nothing. Attaching a knob to the wrong object
    // and confirming only that it parsed is the same failure as reading a speed ratio without
    // checking both arms did equal work.
    //
    // MIXED MULTIPLICATIVELY SO THE DEFAULT IS THE IDENTITY: seed 0 (unset) gives seed_mix 0, and
    // xor-ing 0 leaves every prior trajectory bit-for-bit intact. That is deliberate -- the banked
    // results have to stay reproducible or the comparison against them is worthless.
    let run_seed: u64 = std::env::var("EXISTENCE_EVOLVE_SEED").ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let seed_mix = run_seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    println!("run seed {run_seed} (mix {seed_mix:#x}){}",
             if run_seed == 0 { "  -- default trajectory, reproduces the bank" } else { "  -- NEW trajectory" });
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
                    // seed_mix is 0 unless EXISTENCE_EVOLVE_SEED is set, so the default draw here is
                    // unchanged. This is the ONLY place the run seed can enter: the draw is derived
                    // entirely from the slot indices, so without it every run proposes the identical
                    // program at every (g, li, i) forever.
                    let mut r = Rng::new(
                        (g as u64) << 20 ^ (li as u64) << 16 ^ i as u64 ^ 0xBEEF ^ seed_mix);
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
                                            // EXISTENCE_HARD_WEIGHT: what ONE hard-set solve is worth,
                                            // in mates. Default 1.0, so every existing run is
                                            // unchanged.
                                            //
                                            // WHY A WEIGHT IS NEEDED AT ALL. At weight 1 the fix is
                                            // probably too weak to do its job, and the arithmetic
                                            // says so before any gate has reported. Recovered from
                                            // the two arms' seed surrogates: C_m = 9.24e9 and
                                            // C_h = 1.80e9, so the hard set costs 19.5% of the mate
                                            // set. The best hard score ever achieved is 2 of 8, so
                                            // the largest numerator boost available is
                                            // (23+2)/23 = +8.7%. But cutting cost moves the
                                            // DENOMINATOR: a 10% mate-set cost cut is already +9.1%,
                                            // and the cost-driven gains actually observed were
                                            // +13.1%, +17.4% and +22.3% -- all of them bigger. So a
                                            // cost-cutter outbids a hard-set solver every time and
                                            // selection never changes.
                                            //
                                            // CHOOSING THE VALUE, from the measured gains rather
                                            // than taste: to make ONE solve competitive with the
                                            // weakest observed cost gain (+13.1%) needs
                                            // (23+w)/23 >= 1.131, i.e. w >= 3.0. Weight 4 puts one
                                            // solve at +17.4% and two at +34.8%, which clears the
                                            // whole observed range.
                                            //
                                            // This does NOT relax the bloat guard: cost still counts
                                            // both sets, so a candidate that buys hard solves with
                                            // enormous search still pays for it in the denominator.
                                            let rate = if std::env::var("EXISTENCE_HARD_FITNESS").is_ok() {
                                                let w: f64 = std::env::var("EXISTENCE_HARD_WEIGHT")
                                                    .ok().and_then(|s| s.parse().ok()).unwrap_or(1.0);
                                                (f as f64 + w * hf as f64) * 1e6
                                                    / cst.saturating_add(hcst).max(1) as f64
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
            // ABOVE: guard-passers STRICTLY beating the incumbent -- the acceptance condition
            // (`rate > best_rate`) itself.
            //
            // IT MUST BE REPORTED ON THE GATE LINE, NOT THE `..none` LINE. `..none` is printed in
            // the ELSE of `if popn[0].2 > best_rate`, and popn is parents-union-offspring sorted by
            // rate, so if any offspring beat the incumbent we are in the IF branch by construction.
            // ABOVE is therefore TAUTOLOGICALLY 0 wherever `..none` prints it, and I read exactly
            // that tautology as though it were evidence that nothing ever beats the incumbent. The
            // logs say the opposite: 7 of 15 generations reached the gate, which is only reachable
            // when a candidate DID beat it. Fourth inert diagnostic in this file, and the first one
            // whose output I published a conclusion from.
            let above = rel.iter().filter(|x| **x > 1.0 + 1e-9).count();
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
            // Per-KIND composition beside the pooled count. The count alone cannot say whether two
            // TT-carrying members hold COMPLEMENTARY halves or the same one, and the valley is
            // conjunctive, so "which half" is the whole question. See `tt_counts`.
            let ttk: Vec<String> = popn.iter().map(|(p, _, _)| tt_kind_tag(p)).collect();
            // FITNESS 3 SPECIFIES A FILTER, NOT A CLIMB: "(a) a filter -- a PROGRAM candidate
            // must score >= 0.9x the champion on MATE-{1,2} and >= 0.8x on MATE-{3,4} to reach the
            // ladder". This loop requires `rate > best_rate` STRICTLY -- an IMPROVEMENT in the
            // surrogate -- where the spec asks only that a candidate not be much WORSE and lets the
            // ladder decide. Measured against the reference rungs (`evolve valleyall`):
            //     hash reuse            1.024x   spec PASS    strict PASS
            //     table reduction       0.992x   spec PASS    strict REJECT
            //     hash + ID             0.933x   spec PASS    strict REJECT
            //     iterative deepening   0.914x   spec PASS    strict REJECT
            //     capture extension     0.340x   spec reject  strict REJECT
            // The spec admits FOUR rungs to the ladder; the strict rule admits ONE. That is
            // MASTER_PLAN P2's kill criterion -- "no program improves on the seed -> grammar or
            // fitness is wrong; fix those" -- localised in the fitness.
            //
            // Env-gated: unset is byte-identical to today, so the two are A/B comparable.
            let spec_filter = std::env::var("EXISTENCE_SPEC_FILTER").is_ok();
            let pick = if spec_filter {
                popn.iter().find(|(pr, _, r)| {
                    *r >= 0.9 * best_rate && !lineages[li].gated.contains(&format!("{pr:?}"))
                }).cloned()
            } else {
                // STRICT RULE, now with the SAME anti-re-proposal mechanism the spec filter uses.
                //
                // It used to be `if popn[0].2 > best_rate { Some(popn[0].clone()) }`, with
                // re-proposal prevented by RAISING `best_rate` to a rejected candidate's rate in the
                // reject path. That ratchets the bar upward using programs the gate just measured as
                // WORSE, and it demonstrably cost a gate call: after a gen-3 candidate was rejected
                // at 0.002924 with VERIFY 0.422 (resolved worse), gen 4's candidates read 0.947x and
                // 0.961x of that inherited bar and produced NO gate -- yet against the CHAMPION's
                // actual 0.002490 they are 1.112x and 1.129x and would have gated.
                //
                // WHY IT HAD TO CHANGE NOW rather than after the current A/B: the ratchet's SIZE
                // scales with EXISTENCE_HARD_WEIGHT. A rejected hf=2 candidate raises the bar by
                // +8.7% at weight 1 and by +34.8% at weight 4, so the heavier arm would freeze after
                // one rejection while the lighter one kept gating. That is a difference in gate
                // FREQUENCY produced by the instrument rather than by selection quality -- a
                // confound aligned exactly with the treatment variable, which is the one kind that
                // cannot be left in.
                //
                // `gated` already exists and already does this correctly on the other branch; the
                // struct comment there explains it was introduced because a bar-raise misbehaves
                // under a tolerance filter. It never argued the bar-raise was right here.
                popn.iter()
                    .find(|(pr, _, r)| *r > best_rate
                          && !lineages[li].gated.contains(&format!("{pr:?}")))
                    .cloned()
            };
            // WHY WAS THE PICK NONE? The log printed a bare `..none`, and that one word covers
            // two situations with opposite meanings:
            //
            //   (a) nothing beat best_rate           -> the SEARCH found nothing. Real failure.
            //   (b) something did, but it is `gated` -> already tried and REJECTED by the game
            //                                           gate. Working as designed, not failure.
            //
            // Measured 2026-09-09: the control arm's gen-5 MCTS line reads `..none` with
            // `rates 1.079-1.078679x` -- a candidate 7.9% ABOVE best_rate that passed the mate
            // guard -- and the log gives no way to tell which case it is. The generation before
            // gated a candidate and took `REJECT llr -3.31`, so (b) is likely; but "likely" is
            // exactly what an instrument exists to replace. Counting is free: same predicate,
            // split in two.
            let n_above = popn.iter().filter(|(_, _, r)| *r > best_rate).count();
            let n_gated_skip = popn.iter()
                .filter(|(pr, _, r)| *r > best_rate
                        && lineages[li].gated.contains(&format!("{pr:?}")))
                .count();
            if let Some((c, f, rate)) = pick {
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
                    let mut ic = Interp::new(&net, vec![depth, 32_000, interp::uct_exploration()]);
                    let mut ih = Interp::new(&net, vec![depth, 32_000, interp::uct_exploration()]);
                    set.iter().chain(hard.iter()).all(|(p, _)| {
                        ic.run(&c, p, bud) == ih.run(&lineages[li].champ, p, bud)
                    })
                };
                // PATH 1 MUST RE-CHECK COST, and until now it did not.
                //
                // Its own comment defines the path as "returns the SAME move as the champion on every
                // guard position AND COSTS LESS ... a pure speedup". The code only tested `same_play`;
                // the "costs less" half was silently guaranteed by the caller, because the strict
                // filter picks only when `popn[0].2 > best_rate`.
                //
                // EXISTENCE_SPEC_FILTER breaks that unstated invariant: it picks on `r >= 0.9 *
                // best_rate`, so `rate` may be EQUAL or WORSE. The very first generation of the SPEC
                // cells promoted on `0.002490 was 0.002490` -- identical play at identical cost,
                // recorded as a "speedup". That is a no-op replacing the champion, and it would have
                // been read as the SPEC filter working when it is the guard failing.
                //
                // Restoring the documented condition is a no-op under the strict filter (which already
                // guarantees it), so the control and veto cells are unaffected and stay comparable.
                if same_play && rate > best_rate {
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
                    // Recoverable sibling: the .prog above is a `{:#?}` dump and cannot be read
                    // back, which made every champion this project evolved unrecoverable.
                    let _ = std::fs::write(format!("evolved_{}_gen{g}.sexp", lineages[li].name),
                                           grammar::sexp::to_string(&c));
                    continue;
                }

                // NO-OP VETO: identical play, NOT cheaper -> skip the gate entirely.
                //
                // PATH 1 above accepts identical play that is CHEAPER. The case it leaves is identical play
                // at the same or worse cost -- a program behaviourally indistinguishable from the champion.
                // Sending that to the game gate spends 96 VERIFY pairs and ~60 games to discover that two
                // identical programs draw.
                //
                // MEASURED 2026-09-10 on gate_specfilter_s1, which spent its ENTIRE budget this way:
                //   gen 1 MAIN gate INCONCLUSIVE llr +0.00 (60 games W-D-L 8-44-8) mates 23 surrogate 0.002490
                //   gen 1 MCTS gate INCONCLUSIVE llr +0.00 (60 games W-D-L 2-56-2) mates 15 surrogate 0.001406
                //   gen 2 MAIN gate INCONCLUSIVE llr +0.00 (60 games W-D-L 8-44-8) mates 23 surrogate 0.002487
                // The seed's own header is `23/23 mates, 0.002490 mates/Mcost` -- so those carry the seed's
                // exact mate count and exact surrogate, and the games return perfectly symmetric at 0.500.
                // That arm reached generation 2 while the control reached 6 and both composition arms 5-6.
                //
                // WHY THE STRICT FILTER NEVER NEEDED THIS: it picks only on `rate > best_rate`, which excludes
                // equality by construction, so a no-op cannot be picked at all. EXISTENCE_SPEC_FILTER picks on
                // `r >= 0.9 * best_rate` and admits them. This veto is therefore INERT on the default path and
                // the control/veto/composition arms stay byte-comparable -- the same property that made the
                // PATH 1 repair safe.
                //
                // It is a VETO, not a rejection: the candidate is not recorded in `gated`, because nothing was
                // learned about it. It simply never should have cost games.
                if same_play {
                    println!("  gen {g:>3} {:<5} ..no-op VETO: plays IDENTICALLY on all {} guard positions at \
{rate:.6} vs champion {:.6} -- gate skipped, 0 games spent",
                             lineages[li].name, set.len() + hard.len(), best_rate);
                    continue;
                }

                // ---- PATH 2: THE GAME GATE, for candidates that change play.
                // It RUNS EVOLVED PROGRAMS ON A BOARD, so it is exactly as exposed
                // to a malformed candidate as the fitness call is, and it was NOT wrapped. A
                // candidate that survives fitness can still violate an invariant once it is asked
                // to play 200 plies against another program.
                  // SEQUENTIAL, as FITNESS 7 specifies. `EXISTENCE_GATE_SPRT` selects it; unset is
                  // byte-identical to the fixed-pair gate every prior measurement used.
                  //
                  // FITNESS 7.2: "a candidate near a bound gets thousands of pairs, an obvious dud a
                  // few hundred; nobody picks the count, the evidence does." This loop picked 6.
                  // Measured over every log here: 0 accepts in 203 decisions, 46.8% with ZERO
                  // observed variance -- at a ~2/3 draw rate six pairs frequently tie every one, and
                  // no fixed count can spend more games on the close cases.
                  //
                  // BOUNDS FROM THE SPEC: alpha = beta = 0.05 (LLR_BOUND 2.944), width e1-e0 = 2,
                  // bootstrap e1 = 5 until 20 acceptances exist -> [3, 5]. Env-overridable for
                  // experiments; the DEFAULT is what FITNESS declares.
                  let sprt_gate = std::env::var("EXISTENCE_GATE_SPRT").is_ok();
                  let elo1: f64 = std::env::var("EXISTENCE_GATE_ELO1").ok()
                      .and_then(|s| s.parse().ok()).unwrap_or(5.0);
                  let elo0: f64 = std::env::var("EXISTENCE_GATE_ELO0").ok()
                      .and_then(|s| s.parse().ok()).unwrap_or(elo1 - 2.0);
                  let sprt_max: usize = std::env::var("EXISTENCE_GATE_MAXPAIRS").ok()
                      .and_then(|s| s.parse().ok()).unwrap_or(400);
                  let (gsc, sprt_verdict, sprt_llr) = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                      if sprt_gate {
                          // FITNESS 7.3: random-ply openings until the unbalanced book exists. Same
                          // policy the fixed gate used, so ONLY the stopping rule changes here.
                          let mut orng = grammar::mutate::Rng::new(
                              (0xC0FFEE ^ g as u64 ^ (li as u64) << 8) | 1);
                          let openings: Vec<board::Position> = (0..sprt_max).map(|_| {
                              let mut o = board::Position::startpos();
                              for _ in 0..4 {
                                  let l = o.legal_moves();
                                  if l.is_empty() { break }
                                  let m = l.as_slice()[(orng.next() as usize) % l.as_slice().len()];
                                  let _ = o.make_move(m);
                              }
                              o
                          }).collect();
                          let (v, sc, llr) = gate::match_progs_sprt(
                              &c, &lineages[li].champ, &net,
                              vec![depth, 32_000, interp::uct_exploration()], bud,
                              0xC0FFEE ^ g as u64 ^ (li as u64) << 8, &openings,
                              COST_PER_MOVE, elo0, elo1, sprt_max);
                          (sc, Some(v), llr)
                      } else {
                          (gate::match_progs(&c, &lineages[li].champ, &net,
                                             vec![depth, 32_000, interp::uct_exploration()], bud,
                                             gate_pairs,
                                             0xC0FFEE ^ g as u64 ^ (li as u64) << 8, 4,
                                             COST_PER_MOVE), None, 0.0)
                      }
                  })) {
                    Ok(t) => t,
                    Err(_) => {
                        // Cannot finish a game => cannot be promoted. Same rule as fitness: the
                        // candidate scores as the worst possible program and the run continues.
                        println!("  gen {g:>3} {:<5} gate PANIC -- candidate cannot play, rejected",
                                 lineages[li].name);
                        // RECORD, do not raise the bar. Both branches now use `gated`: a
                        // rejected program must not be re-proposed, and it must not become
                        // the reference the next candidate is measured against.
                        lineages[li].gated.insert(format!("{c:?}"));
                        continue;
                    }
                };
                // EXISTENCE_GATE_VERIFY=<pairs>: a DIAGNOSTIC re-match at a higher pair count.
                //
                // The A/B between the two acceptance rules can show that they DECIDE differently --
                // it cannot show which decision was RIGHT, because the gate itself is 6 pairs and
                // ci95 there is up to 0.250. A promotion at "gate 0.500" might be a real improvement
                // the strict rule wrongly rejected, or noise the veto wrongly admitted, and the gate
                // score cannot distinguish those.
                //
                // The saved `.prog` files cannot answer it either: they are `{:#?}` dumps, not
                // loadable, as this file already records at line ~927. So the verification has to
                // happen HERE, while the candidate is still in memory.
                //
                // It runs the SAME match_progs against the SAME champion with a different pair count
                // and a DIFFERENT seed (so it is an independent sample, not a longer version of the
                // same one), prints the result, and CHANGES NOTHING. The decision above is untouched
                // -- this is an observer, not a second gate. Default 0 = off, so every existing run
                // is byte-identical.
                //
                // At 96 pairs ci95 is ~0.047, which separates a true 0.6 from a true 0.5; the 6-pair
                // gate cannot. Cost is ~16x the gate, paid only when a gate call happens.
                let verify_pairs: usize = std::env::var("EXISTENCE_GATE_VERIFY")
                    .ok().and_then(|s| s.parse().ok()).unwrap_or(0);
                if verify_pairs > 0 {
                    if let Ok(vsc) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        gate::match_progs(&c, &lineages[li].champ, &net,
                                          vec![depth, 32_000, interp::uct_exploration()], bud,
                                          verify_pairs,
                                          0x5EEDBEEF ^ g as u64 ^ (li as u64) << 8, 4,
                                          COST_PER_MOVE)
                    })) {
                        println!("  gen {g:>3} {:<5} VERIFY {:.3}+/-{:.3} ({} pairs, independent seed)",
                                 lineages[li].name, vsc.pent_rate(), vsc.ci95(), verify_pairs);
                    }
                }

                // EXISTENCE_GATE_VETO=1 implements the rule this gate's own comment DESCRIBES.
                //
                // evolve.rs documents the game gate as "a veto on unplayable programs" that
                // "CANNOT resolve a 2% edge and is not asked to". The shipped code asks it to do
                // exactly that: `pent_rate - ci95 > 0.5` demands the candidate be resolved BETTER,
                // so at 6 pairs (ci95 up to 0.250) it must win ~60-69% of pairs. A veto would
                // reject only what is resolved WORSE. STATE.md:116-144 records the contradiction
                // and declines to change it pending one thing: "measuring how many promotions the
                // rule costs."
                //
                // MEASURED, over the 17 game-gate calls in the two completed mcts_ab arms:
                //     implemented (resolved_up)        0/17 promotions
                //     documented veto (resolved_down)  8/17 promotions
                // and the difference DISCRIMINATES rather than waving everything through: all 8
                // flips are ties (0.458-0.542), while every MAIN-lineage call (0.292, 0.333, 0.375)
                // is rejected under BOTH rules because it is resolved worse. Zero promotions under
                // the shipped rule means the search track cannot advance at all.
                //
                // Drift is already bounded independently: `guard_floor` is anchored to the SEED's
                // score, not the current best, so admitting ties cannot ratchet the champion down.
                //
                // Default is UNCHANGED. This is the experimental apparatus, and it is switched, not
                // replaced, so the two rules can be run as an A/B on the same seed.
                let veto_only = std::env::var("EXISTENCE_GATE_VETO").as_deref() == Ok("1");
                  // THE SPRT VERDICT IS THE DECISION when the sequential gate ran. Accept only on
                  // Accept: Inconclusive means the cost ceiling was hit before the evidence decided,
                  // which is NOT a rejection and is logged as itself -- "the evidence never ruled it
                  // out" recorded as "the evidence ruled it out" is how a real candidate disappears.
                  let resolved_up = match sprt_verdict {
                      Some(gate::Sprt::Accept) => true,
                      Some(_) => false,
                      None if veto_only => gsc.pent_rate() + gsc.ci95() >= 0.5,
                      None => gsc.pent_rate() - gsc.ci95() > 0.5,
                  };
                if !resolved_up {
                    // ABOVE and the ACCEPTANCE BAR both belong here. `resolved_up` demands
                    // pent_rate - ci95 > 0.5, so at these pair counts the candidate must score
                    // above 0.5 + ci95 -- printed so the bar is never inferred from memory.
                    // W-D-L IS LOGGED BECAUSE `games` ALONE CANNOT EXPLAIN A DEAD HEAT. Of the 203
                    // logged decisions, 95 (46.8%) carry ci95 == 1.5/6 exactly -- `gate.rs`'s
                    // ZERO-VARIANCE fallback -- with pent_rate exactly 0.500. An all-middle
                    // pentanomial has two causes needing OPPOSITE fixes, and the rate cannot tell
                    // them apart: MIRRORED (candidate plays like the champion, so each pair is one
                    // win and one mirrored loss, draws == 0) means the operator produced an INERT
                    // candidate and no pair count can ever help; ALL-DRAWN (wins == losses == 0)
                    // means the match is not producing decisive games and sample size was never the
                    // issue. Logging draws makes the REAL gate population self-reporting, rather
                    // than needing a probe whose mutants are not the ones that reach the gate.
                    // `hard {hlo}-{hhi}` IS PRINTED HERE BECAUSE A NEGATIVE RESULT IS OTHERWISE
                    // UNREADABLE. Under EXISTENCE_HARD_FITNESS the surrogate is
                    // (f + hf) / (cost + hard_cost). If NO candidate scores on the hard set then
                    // hf = 0, the rate is (23+0)/(cost+hard_cost), and the only way to improve it
                    // is still to cut cost -- identical in incentive to the control's 23/cost. The
                    // fix has then not engaged, and "the treatment is still resolved worse" is not
                    // evidence against the saturation diagnosis; it is no evidence at all.
                    //
                    // The non-gated `..none` line already carries this field, but a GATED
                    // generation prints only VERIFY and this line, so it was missing for exactly
                    // the candidates that reach a gate. `hhi == 0` identifies the uninterpretable
                    // case outright.
                    //
                    // This is the range over guard-passing CANDIDATES, not the winner's own hf --
                    // that one is dropped at the population boundary (`popn` is a 3-tuple). The
                    // range is enough for the reading that matters: hhi == 0 means the winner had
                    // hf = 0 too, so the fix demonstrably did not engage.
                    //
                    // PRINT-ONLY: consumes no RNG and touches no state, so an arm built with this
                    // must reproduce its trajectory exactly. That is checked on restart.
                    // `mates {f}` IS THE DIRECT TEST OF THE SATURATION MECHANISM, and it costs one
                    // format argument because `f` is already bound by `if let Some((c, f, rate))`.
                    //
                    // The whole diagnosis is numerator-vs-denominator: a surrogate gain is either a
                    // real mate gain (numerator) or a cost cut (denominator), and only the second is
                    // suspected of costing strength. Until now the gate line printed the RATE, which
                    // cannot tell them apart, so every attribution had to be inferred arithmetically
                    // from the seed's rate -- and for MCTS, whose seed is 15/23 and NOT saturated,
                    // that inference is genuinely ambiguous. A surrogate of 0.001506 is consistent
                    // BOTH with f=16 at unchanged cost (16/15 = 1.067) and with f=15 at a 6.7%
                    // cheaper cost. Those are opposite findings under this hypothesis, and that
                    // reading happens to be the only VERIFY above 0.500 on record (0.505).
                    //
                    // For MAIN this is expected to read a constant 23 -- that IS the saturation, made
                    // visible instead of argued. If it ever reads anything else, the premise this
                    // whole document rests on is wrong and should fail loudly.
                    // POP AND DISTINCT ON THE GATE LINE TOO. They were printed only on the `..none` path, so an
                    // arm that gates every generation reports its population NOWHERE. Measured 2026-09-10: the
                    // control had 2 `..none` lines and 4 gate lines while a composition arm had 6 and 0 -- so when
                    // both composition seeds showed the population climbing 2 -> 6 and 7, at the exact stage
                    // `search_has_no_choice_RESULT.md` identifies as binding, the control could not be compared
                    // there at all. Not measured-as-2: UNMEASURED.
                    //
                    // An observable that vanishes precisely when a candidate is interesting enough to GATE is the
                    // worst place for a blind spot, and it cost a comparison two seeds had already earned.
                    let g_pop = popn.len();
                    let g_distinct = {
                        let mut v: Vec<String> = popn.iter().map(|(_, _, r)| format!("{r:.9}")).collect();
                        v.sort(); v.dedup(); v.len()
                    };
                    println!("  gen {g:>3} {:<5} gate {} {:.3}+/-{:.3} ({} games W-D-L {}-{}-{})  mates {f}  hard {hlo}-{hhi}  surrogate \
{rate:.6}  ABOVE:{above}  needed >{:.3}  pop {g_pop} distinct:{g_distinct}",
                             lineages[li].name,
                               match sprt_verdict {
                                   Some(gate::Sprt::Inconclusive) => format!("INCONCLUSIVE llr {sprt_llr:+.2}"),
                                   Some(gate::Sprt::Reject) => format!("REJECT llr {sprt_llr:+.2}"),
                                   Some(gate::Sprt::Accept) => format!("ACCEPT llr {sprt_llr:+.2}"),
                                   None => "REJECT".to_string(),
                               },
                               gsc.pent_rate(), gsc.ci95(), gsc.games(),
                               gsc.wins, gsc.draws, gsc.losses,
                             // THE PRINTED BAR MUST MATCH THE ACTIVE RULE. This hardcoded `0.5 + ci95`, the
                             // resolved_up threshold; under EXISTENCE_GATE_VETO the criterion is
                             // `rate >= 0.5 - ci95`, so a veto REJECT would print a bar it was never judged
                             // against. That is the defect class this tree keeps finding -- the gate comment
                             // describing a veto while the code demanded resolution, the anchor comment
                             // claiming 'same seed family, same openings' when it did not. Adding a third
                             // while fixing the first would be poor.
                             if veto_only { 0.5 - gsc.ci95() } else { 0.5 + gsc.ci95() });
                    // EXPLOIT CAPTURE. A candidate whose surrogate is orders above the incumbent
                    // while its GAMES are far below parity is, by definition, a program that beats
                    // the fitness function without playing better. Those are the only examples that
                    // can test whether a REPLACEMENT surrogate is exploit-resistant.
                    //
                    // WHY THIS IS NEEDED, from a failure this file already recorded: the
                    // `valleyall` oracle ranks nine HAND-WRITTEN reference programs, and a
                    // guard-tolerance change that scored 4/6 on it produced a 1314x-rate exploit in
                    // the loop within three generations. An offline ranking test over known-good
                    // programs cannot validate a filter whose job is rejecting unknown-BAD ones,
                    // and the reference set does not span the space mutation actually reaches.
                    // Hand-written exploits are the ones we already thought of.
                    //
                    // Thresholds are deliberately loose (10x surrogate, sub-0.4 games) because the
                    // cost of a false capture is one small file and the cost of a miss is losing a
                    // machine-found counterexample that cannot be reconstructed.
                    let ratio = rate / best_rate.max(1e-12);
                    if ratio > 10.0 && gsc.pent_rate() < 0.4 {
                        let _ = std::fs::write(
                            format!("exploit_{}_gen{g}_{:.0}x.prog", lineages[li].name, ratio),
                            format!("// CAPTURED EXPLOIT\n// surrogate {rate:.6} vs incumbent \
{best_rate:.6} = {ratio:.1}x\n// games {:.3} +/- {:.3} over {} -- far below parity\n// {f} mates, \
{} nodes, generation {g}, lineage {}\n{:#?}\n",
                                    gsc.pent_rate(), gsc.ci95(), gsc.games(), c.size(),
                                    lineages[li].name, c));
                        // Recoverable sibling: the .prog above is a `{:#?}` dump and cannot be read
                        // back, which made every champion this project evolved unrecoverable.
                        let _ = std::fs::write(format!("exploit_{}_gen{g}_{:.0}x.sexp", lineages[li].name, ratio),
                                               grammar::sexp::to_string(&c));
                        println!("         ^ captured as an exploit: {ratio:.0}x surrogate, \
{:.3} games", gsc.pent_rate());
                        // MACHINE-READABLE LEDGER alongside the program dump.
                        //
                        // The .prog file is `{:#?}` output: readable by a person, NOT reloadable by
                        // the program. There is no text format for Program -- 1 occurrence of
                        // "parse" in 1760 lines of the grammar crate and no serde anywhere -- so a
                        // captured specimen cannot be replayed through a candidate fitness.
                        //
                        // It does not need to be. What makes a specimen a THREAT is entirely
                        // captured by (mates, cost, game rate): any surrogate that is a function of
                        // mates and cost can be asked "would you admit 18 mates at 5.4M cost when
                        // the champion is 25 at 10.2B?" arithmetically. That turns the corpus into
                        // a regression test without needing a parser, which is the difference
                        // between an instrument that exists and one that is merely planned.
                        //
                        // Cost is recomputed from the rate rather than plumbed through: rate is
                        // mates*1e6/cost by construction, so cost = mates*1e6/rate exactly.
                        let (e1, _, _) = fitness(&c, &m1_probe, &net, depth, bud);
                        let (e2, _, _) = fitness(&c, &m2_probe, &net, depth, bud);
                        let ex_cost = f as f64 * 1e6 / rate.max(1e-12);
                        let champ_cost = lineages[li].best_found as f64 * 1e6
                            / best_rate.max(1e-12);
                        let line = format!(
                            "{}\t{}\t{}\t{:.0}\t{:.6}\t{}\t{:.0}\t{:.6}\t{:.3}\t{:.3}\t{}\t\
{}/{}\t{}/{}\n",
                            lineages[li].name, g, f, ex_cost, rate,
                            lineages[li].best_found, champ_cost, best_rate,
                            gsc.pent_rate(), gsc.ci95(), c.size(),
                            e1, m1_probe.len(), e2, m2_probe.len());
                        use std::io::Write;
                        if let Ok(mut fh) = std::fs::OpenOptions::new()
                            .create(true).append(true).open("exploits.tsv") {
                            if fh.metadata().map(|m| m.len() == 0).unwrap_or(false) {
                                let _ = fh.write_all(b"lineage\tgen\tex_mates\tex_cost\tex_rate\t\
champ_mates\tchamp_cost\tchamp_rate\tgames\tci95\tnodes\tmate1\tmate2\n");
                            }
                            let _ = fh.write_all(line.as_bytes());
                        }
                    }
                    // RECORD, do not raise the bar -- see the pick block. A gate REJECTION
                    // used to set best_rate to the rejected candidate's rate, which is how a
                    // program measured WORSE became the bar its successors had to clear.
                    lineages[li].gated.insert(format!("{c:?}"));
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
                // Recoverable sibling: the .prog above is a `{:#?}` dump and cannot be read
                // back, which made every champion this project evolved unrecoverable.
                let _ = std::fs::write(format!("evolved_{}_gen{g}.sexp", lineages[li].name),
                                       grammar::sexp::to_string(&c));
            } else {
                // rhi at SIX decimals: at three, a candidate strictly above the incumbent is
                // indistinguishable from a tie, which is exactly the ambiguity `above` exists to end.
                let span = if rel.is_empty() { "none".to_string() }
                           else { format!("{rlo:.3}-{rhi:.6}x") };
                println!("  gen {g:>3} {:<5} ..none[above {n_above}, gated-skip {n_gated_skip}] ({n_scored} cand, {ill} ill, mate-ok {mate_ok}, \
rates {span} [>=.98:{} .90-.98:{} .50-.90:{} <.50:{} distinct:{}], hard {hlo}-{hhi})  pop {} spread {:.6}-{:.6} tt{:?} ttk{:?}",
                         lineages[li].name, hist.0, hist.1, hist.2, hist.3, distinct,
                         popn.len(), spread_lo, spread_hi, tt, ttk);
            }
        }
    }

    println!("\n=== per-lineage summary over {gens} generations ===");
    for l in &lineages {
        println!("  {:<5} {} accepted   final {} mates {:.6} mates/Mcost ({} nodes)",
                 l.name, l.accepted, l.best_found, l.best_rate, l.champ.size());
    }
    println!("\n  Cross-lineage RATE comparison is meaningless and is not printed: the two seeds");
    println!("  run at different budgets against different mate guards (MAIN f>=25, MCTS f>=12).");
    println!("  Only the game gate compares programs, and it plays them on a board.");
}
