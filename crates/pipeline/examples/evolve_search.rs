//! THE SEARCH TRACK. Evolve the search PROGRAM, judged by games.
//!
//! Everything before this evolved the EVAL. The search program has been the fixed bare
//! alpha-beta seed since day one, which means the project's actual claim — that the engine
//! discovers its own search — has never been exercised.
//!
//! The earlier `evolve.rs` mutated and scored on mates-per-cost over mate-in-1 puzzles with a
//! RANDOM net. That is not the search track; it is one surrogate of it, run against noise.
//! FITNESS 1 requires PROGRAM candidates to clear three things in order, and the two missing
//! pieces are the ones that make a result mean anything:
//!
//!   1. CORRECTNESS ORACLE (FITNESS 2)  — a candidate must agree with a full-width reference.
//!      Without it, "cheaper" is trivially achievable by searching less and playing worse, and
//!      mates-per-cost alone cannot tell that apart from a real gain.
//!   2. mates-per-cost surrogate (FITNESS 3) — cheap filter, proposes.
//!   3. GAME GATE — candidate program vs champion program, SAME net, EQUAL COST BUDGET,
//!      pentanomial. This is what decides. A program that survives 1 and 2 has earned the
//!      games, nothing more.
//!
//! Two things had to land before this could be honest, and both landed today:
//!   - a MEASURED per-primitive cost model (eval = 1365 units, not 1), so "equal cost" is
//!     equal WORK rather than equal AST-node visits;
//!   - a champion net that is actually stronger than random (0.816 +/- 0.038 at depth 3), so
//!     the games discriminate between search programs instead of between two noise sources.

use board::{Outcome, Position};
use grammar::mutate::{self, Op, Rng as MRng};
use std::collections::BTreeMap;
use grammar::{reference, Program};
use interp::Interp;
use nnue::Net;
use pipeline::gate::Score as Pent;

/// Full-width negamax, no pruning, no hash — FITNESS 2.1's reference. Deterministic, and
/// deliberately dumb: it is the test rig, not the engine.
fn reference_value(pos: &mut Position, depth: u32, net: &Net, scratch: &mut Vec<f32>) -> i32 {
    let list = pos.legal_moves();
    if list.is_empty() {
        return match pos.outcome() { Outcome::Loss => -30_000 + (64 - depth as i32), _ => 0 };
    }
    if depth == 0 { return net.eval(pos, scratch); }
    let mut best = i32::MIN + 1;
    for &m in list.as_slice() {
        let u = pos.make_move(m);
        let v = -reference_value(pos, depth - 1, net, scratch);
        pos.unmake_move(m, u);
        if v > best { best = v; }
    }
    best
}

/// Does the candidate pick a move the full-width reference agrees is optimal?
///
/// Ties are allowed: several moves can share the reference's best value, and a program that
/// picks a different one of them is not wrong. Requiring an exact move match would reject
/// correct programs for tie-breaking differently, which is a preference, not a correctness
/// criterion.
/// Budgets here are BOUNDED (100M cost units), not effectively-infinite. They were 1<<30 and
/// 1<<28, which mattered nothing while the interpreter ignored its budget entirely and matters
/// a great deal now that it enforces one: a mutant with an unbounded loop hung the first real
/// search-track run for four hours on a single candidate.
fn passes_oracle(prog: &Program, set: &[Position], depth: u32, net: &Net) -> (usize, usize) {
    let mut scratch = Vec::new();
    let (mut ok, mut n) = (0usize, 0usize);
    for p0 in set {
        let mut it = Interp::new(net, vec![depth as i64, 32_000, 8]);
        let mv = it.run(prog, p0, 100_000_000);
        let mut p = p0.clone();
        let list = p.legal_moves();
        if list.is_empty() { continue; }
        n += 1;
        let mut best = i32::MIN + 1;
        let mut chosen = i32::MIN + 1;
        for &m in list.as_slice() {
            let u = p.make_move(m);
            // DEPTH ALIGNMENT, and this is the off-by-one GRAMMAR 8 already records as the
            // cause of the bogus "0.14x" interpreter reading. The seed's `choose` expands the
            // ROOT ITSELF and then calls ab(apply(p,m), D, ...), so after a root move it
            // searches D MORE plies. A reference called with depth-1 here is a ply shallower
            // than the program it is judging, and the disagreements it reports are its own.
            // Caught because the SEED scored 7/10 against its own oracle, which is impossible
            // for a correct reference: bare alpha-beta returns the full-width minimax value.
            let v = -reference_value(&mut p, depth, net, &mut scratch);
            p.unmake_move(m, u);
            if v > best { best = v; }
            if m == mv { chosen = v; }
        }
        if chosen == best { ok += 1; }
    }
    (ok, n)
}


/// FITNESS 3's mates-per-cost surrogate. CHEAP, and it runs BEFORE the games.
///
/// The gate is the expensive stage: 12 pairs is 24 games of up to 160 plies, each move a full
/// budgeted search. Spending that on a candidate that cannot find a mate-in-1 is waste. The
/// surrogate proposes; the gate disposes.
///
/// Denominated in COST UNITS, not evaluations, because proof-number search proves mates with
/// zero eval calls and an eval-count denominator would divide by zero for it (FITNESS 3).
fn mates_per_cost(prog: &Program, set: &[Position], depth: i64, net: &Net) -> (u32, f64, u32) {
    let mut it = Interp::new(net, vec![depth, 32_000, 8]);
    let (mut found, mut cost, mut forfeits) = (0u32, 0u64, 0u32);
    for p in set {
        let mv = it.run(prog, p, 100_000_000);
        cost += it.cost;
        if it.over_budget { forfeits += 1; }
        let mut q = p.clone();
        if q.legal_moves().as_slice().contains(&mv) {
            q.make_move(mv);
            if q.legal_moves().is_empty() && q.outcome() == Outcome::Loss { found += 1; }
        }
    }
    (found, found as f64 * 1e6 / cost.max(1) as f64, forfeits)
}

/// Positions where the side to move has a mate in one. Rules-derived: a position is MATE-1 iff
/// some legal move ends the game as a win. No chess knowledge enters.
fn mate_set(n: usize, rnd: &mut impl FnMut() -> u64) -> Vec<Position> {
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
            p.make_move(l.as_slice()[(rnd() % l.len() as u64) as usize]);
        }
    }
    out
}

/// One game, program vs program, SAME net, each move capped at `budget` COST UNITS.
/// Returns Some(true) if A won.
fn play(a: &Program, b: &Program, a_white: bool, start: &Position, net: &Net,
        depth: i64, budget: i64) -> Option<bool> {
    let mut pos = start.clone();
    for _ in 0..160 {
        let list = pos.legal_moves();
        if list.is_empty() {
            return match pos.outcome() {
                Outcome::Loss => Some((pos.stm == board::Color::Black) == a_white),
                _ => None,
            };
        }
        if pos.halfmove >= 100 { return None; }
        let is_a = (pos.stm == board::Color::White) == a_white;
        let prog = if is_a { a } else { b };
        let mut it = Interp::new(net, vec![depth, 32_000, 8]);
        let mv = it.run(prog, &pos, budget);
        // A program that returns no legal move forfeits: it failed to play chess.
        if !list.as_slice().contains(&mv) { return Some(!is_a); }
        pos.make_move(mv);
    }
    None
}

fn match_programs(a: &Program, b: &Program, net: &Net, pairs: usize, depth: i64,
                  budget: i64, seed: u64) -> Pent {
    let mut sc = Pent::default();
    let mut rng = seed | 1;
    let mut rnd = || { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; rng };
    for _ in 0..pairs {
        let mut open = Position::startpos();
        for _ in 0..4 {
            let l = open.legal_moves();
            if l.is_empty() { break; }
            open.make_move(l.as_slice()[(rnd() % l.len() as u64) as usize]);
        }
        let mut half = 0usize;
        for a_white in [true, false] {
            match play(a, b, a_white, &open, net, depth, budget) {
                Some(true) => { sc.wins += 1; half += 2; }
                Some(false) => { sc.losses += 1; }
                None => { sc.draws += 1; half += 1; }
            }
        }
        sc.pent[half.min(4)] += 1;
    }
    sc
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let get = |k: &str, d: usize| -> usize {
        args.iter().position(|a| a == k).and_then(|i| args.get(i + 1))
            .and_then(|v| v.parse().ok()).unwrap_or(d)
    };
    let gens = get("--gens", 12);
    let pop = get("--pop", 24);
    let pairs = get("--pairs", 24);
    let depth = get("--depth", 3) as i64;
    let budget = get("--budget", 3_000_000) as i64;   // COST UNITS, measured model
    let oracle_n = get("--oracle-set", 12);
    let oracle_depth = get("--oracle-depth", 2) as u32;
    let net_path = args.iter().position(|a| a == "--net").and_then(|i| args.get(i + 1))
        .cloned().unwrap_or_else(|| "champion_cap40.net".into());

    let net = match Net::load(&net_path) {
        Ok(n) => { println!("net {net_path} (hidden {})", n.n_hidden); n }
        Err(e) => { eprintln!("no champion net at {net_path}: {e}"); std::process::exit(2); }
    };

    // Oracle set: positions from random walks. FITNESS 2.2 wants champion self-play; this is
    // the rules-derived stand-in until a position store exists, and it is declared as such.
    let mut rng: u64 = 0x5EED_5EED;
    let mut rnd = || { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; rng };
    let mut oracle_set = Vec::new();
    while oracle_set.len() < oracle_n {
        let mut p = Position::startpos();
        for _ in 0..(6 + rnd() % 20) {
            let l = p.legal_moves();
            if l.is_empty() { break; }
            p.make_move(l.as_slice()[(rnd() % l.len() as u64) as usize]);
        }
        if !p.legal_moves().is_empty() { oracle_set.push(p); }
    }

    let mates = mate_set(40, &mut rnd);
    let mut champ = reference::bare_alpha_beta();
    let (ok, n) = passes_oracle(&champ, &oracle_set, oracle_depth, &net);
    let surrogate_depth = get("--surrogate-depth", 2) as i64;
    let (seed_mates, seed_rate, seed_ff) = mates_per_cost(&champ, &mates, surrogate_depth, &net);
    println!("seed: bare alpha-beta, {} nodes; oracle {ok}/{n}; surrogate {seed_mates}/{} mates at {seed_rate:.3} per Mcost ({seed_ff} forfeits)",
             champ.size(), mates.len());
    assert!(seed_mates > 0, "the SEED finds no mates on the mate-in-1 set -- the surrogate is \
             inert and would filter nothing (it scored {seed_mates}/{}, {seed_ff} over budget)",
             mates.len());
    println!("gate: candidate vs champion PROGRAM, same net, {budget} cost units/move, {pairs} pairs\n");

    let edits = get("--edits", 1);
    println!("mutations per candidate: {edits}\n");
    // Per-operator tally: (proposed, survived-oracle, survived-surrogate, reached-games).
    // 128 rejected candidates teach nothing without this; with it the same run reports which
    // operators produce viable programs at all.
    let mut tally: BTreeMap<String, [u32; 4]> = BTreeMap::new();
    let mut accepted = 0;
    for g in 1..=gens {
        let (mut tried, mut ill, mut oracle_fail, mut surrogate_fail) = (0, 0, 0, 0);
        let mut best: Option<(Program, Pent)> = None;
        for i in 0..pop {
            let mut r = MRng::new((g as u64) << 24 ^ i as u64 ^ 0xBEEF);
            let (cand, ops) = match mutate::mutate_program_n(&champ, &mut r, edits) {
                Some(c) => c, None => { ill += 1; continue }
            };
            let key = ops.iter().map(|o| format!("{o:?}")).collect::<Vec<_>>().join("+");
            tally.entry(key.clone()).or_insert([0; 4])[0] += 1;
            tried += 1;
            if i % 4 == 0 { println!("  gen {g} cand {i}/{pop}..."); use std::io::Write; let _ = std::io::stdout().flush(); }
            // 1. ORACLE. A program that plays worse chess can always be cheaper; this is what
            //    stops "cheaper" from being confused with "better".
            let (co, cn) = passes_oracle(&cand, &oracle_set, oracle_depth, &net);
            if cn == 0 || co * 100 < cn * 95 { oracle_fail += 1; continue; }
            tally.get_mut(&key).unwrap()[1] += 1;
            // 2. SURROGATE (FITNESS 3). Must not LOSE mates -- a program that finds fewer
            //    mates more cheaply is not better, and missing a forced mate is unsound
            //    pruning's characteristic failure. Cheap, so it runs before the games.
            let (cm, _cr, _cf) = mates_per_cost(&cand, &mates, surrogate_depth, &net);
            if cm < seed_mates { surrogate_fail += 1; continue; }
            tally.get_mut(&key).unwrap()[2] += 1;
            tally.get_mut(&key).unwrap()[3] += 1;
            // 3. GAMES. The only thing that decides.
            let sc = match_programs(&cand, &champ, &net, pairs, depth, budget,
                                    0xA11CE ^ (g as u64) << 8 ^ i as u64);
            if sc.pent_rate() - sc.ci95() > 0.5 {
                if best.as_ref().map_or(true, |(_, b)| sc.pent_rate() > b.pent_rate()) {
                    best = Some((cand, sc));
                }
            }
        }
        match best {
            Some((c, sc)) => {
                println!("gen {g:>3}  ACCEPT  {} nodes  {}W-{}D-{}L  {:.3}+/-{:.3}  \
                          ({tried} typed, {ill} ill-typed, {oracle_fail} oracle, {surrogate_fail} surrogate)",
                         c.size(), sc.wins, sc.draws, sc.losses, sc.pent_rate(), sc.ci95());
                champ = c;
                accepted += 1;
            }
            None => println!("gen {g:>3}  --      ({tried} typed, {ill} ill-typed, {oracle_fail} oracle, \
                              {surrogate_fail} surrogate, none beat the champion)"),
        }
    }

    println!("\n--- operator survival (proposed -> oracle -> surrogate) ---");
    let mut rows: Vec<_> = tally.iter().collect();
    rows.sort_by_key(|(_, v)| std::cmp::Reverse(v[1]));
    for (k, v) in rows.iter().take(14) {
        println!("  {:<34} {:>4} -> {:>4} -> {:>4}", k, v[0], v[1], v[2]);
    }
    let tot: u32 = tally.values().map(|v| v[0]).sum();
    let sur: u32 = tally.values().map(|v| v[1]).sum();
    println!("  {:<34} {:>4} -> {:>4}  ({:.0}% survive the oracle)", "TOTAL", tot, sur,
             100.0 * sur as f64 / tot.max(1) as f64);

    println!("\n{accepted} program(s) accepted over {gens} generations");
    println!("final: {} nodes (seed was {})", champ.size(), reference::bare_alpha_beta().size());
}
