//! RUN the two reference programs that have never been run.
//!
//! GRAMMAR 6 lists five reference programs, all labelled `faithful`, all MEASURED. One of them
//! was not: `alpha-beta + hash reuse` returned the constant 0 without ever calling `eval`, and it
//! sat in the reference set calibrating the prior, labelled faithful, under a line reading "no
//! sketches remain". It was caught by RUNNING it (tt_pressure.rs), not by reading it.
//!
//! `uct_mcts` and `proof_number` are referenced exactly once each in the whole tree -- from
//! `reference::all()`, which examples/prior.rs uses to COUNT NODES. Nothing executes them. So
//! their "faithful" label rests on counting and reading, which is the precise state the hash-reuse
//! program was in when it was wrong.
//!
//! THE CHECKS ARE NOT THE SAME FOR BOTH, deliberately. The hash-reuse tell was `evals == 0`, but
//! proof_number's own doc comment says it "is terminal-driven and calls `eval` nowhere" -- so
//! zero evals is CORRECT there and flagging it would be a false alarm from a borrowed criterion.
//! What each program must do:
//!
//!   uct_mcts       must call eval (its leaf case is Node::Eval), must return a legal move, and
//!                  must actually spend its budget -- cost has to grow with simulations, or the
//!                  loop is not looping.
//!   proof_number   must return a legal move and must SOLVE: on a position with a mate in one it
//!                  has to produce the mating move. A prover that cannot find a forced mate at
//!                  depth one is not a prover.
//!
//! Both must also beat the only bar that matters for a reference program: returning a move that
//! is actually in the legal move list. A program returning garbage still counts nodes fine.
//!
//! CALIBRATE THE HARNESS AGAINST THE CONTROL FIRST. The first version of this audit ran at depth
//! 4 with budget 200 and reported that MCTS returned a legal move 13/60 and PN found 0/23 mates
//! -- but bare alpha-beta, the SEED program, scored 23/60 and 7/23 on the same harness. A control
//! that fails means the instrument is wrong, so none of those numbers were about the programs.
//! The cause: cost_cap is 2e9, those settings cost ~1.67e9 PER POSITION, and an over-budget run
//! returns Value::Unit whose .mv() is a default -- an illegal move, indistinguishable from a
//! program that answered wrongly. So the depth is now chosen BY the control: the deepest setting
//! at which alpha-beta is clean, and over_budget is counted and printed rather than silently
//! becoming a wrong answer.
use board::{Outcome, Position};
use grammar::reference;
use interp::Interp;
use nnue::Net;

/// Positions from random walks, and separately a set that has a mate in one.
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

/// A position is "mate in one" if some legal move leaves the opponent checkmated.
fn mate_in_one(p: &Position) -> Option<board::Move> {
    for m in p.legal_moves().as_slice() {
        let mut q = p.clone();
        q.make_move(*m);
        if q.outcome() == Outcome::Loss { return Some(*m); }
    }
    None
}

/// Run one program over a set, reporting legality, evals, cost and BUDGET EXHAUSTION separately.
fn probe(prog: &grammar::Program, net: &Net, ps: &[Position], d: i64, b: i64)
    -> (usize, u64, u64, usize) {
    let (mut legal, mut evals, mut cost, mut over) = (0usize, 0u64, 0u64, 0usize);
    let mut ceil = 0u64;
    for p in ps {
        let mut i = Interp::new(net, vec![d, 32_000, interp::UCT_EXPLORATION]);
        let m = i.run(prog, p, b);
        evals += i.evals; cost += i.cost;
        if i.over_budget { over += 1; }
        ceil += i.ceiling_hits;
        if p.legal_moves().as_slice().contains(&m) { legal += 1; }
    }
    if ceil > 0 { println!("      [recursion ceiling hit {ceil} times — a call unwound as 0]"); }
    (legal, evals, cost, over)
}

fn main() {
    let net = Net::random(16, 7);
    let ps = corpus(60, 0xC0FFEE);

    // Mate-in-one positions are rare in a uniform random walk, so search a wider pool for them.
    let mut mates: Vec<(Position, board::Move)> = Vec::new();
    for p in corpus(4000, 0xBEEF) {
        if let Some(m) = mate_in_one(&p) { mates.push((p, m)); }
        if mates.len() >= 25 { break; }
    }

    println!("reference audit — programs that reference::all() counts but nothing ever ran\n");

    // ---- CALIBRATION: the control picks the settings -------------------------------------
    let ab = reference::bare_alpha_beta();
    println!("CALIBRATION — deepest depth at which the SEED program is clean");
    let mut depth = 0i64;
    for d in 1..=4 {
        let (legal, _e, cost, over) = probe(&ab, &net, &ps, d, d);
        println!("  depth {d}: legal {legal}/{}  over_budget {over}/{}  cost {cost}",
                 ps.len(), ps.len());
        if legal == ps.len() && over == 0 { depth = d; }
    }
    if depth == 0 {
        println!("\n  ABORT: alpha-beta is not clean at ANY depth — the harness is broken, and");
        println!("  nothing below it would be a statement about the reference programs.");
        return;
    }
    println!("  -> auditing at depth {depth}\n");

    let (ab_legal, ab_evals, _abc, ab_over) = probe(&ab, &net, &ps, depth, depth);
    let mut ab_solved = 0usize;
    for (p, best) in &mates {
        let mut i = Interp::new(&net, vec![depth, 32_000, interp::UCT_EXPLORATION]);
        if i.run(&ab, p, depth) == *best { ab_solved += 1; }
    }
    println!("CONTROL — bare alpha-beta @depth {depth}");
    println!("  legal {ab_legal}/{}  over_budget {ab_over}  evals {ab_evals}  mate-in-1 {ab_solved}/{}",
             ps.len(), mates.len());
    if ab_solved != mates.len() {
        println!("\n  ABORT: the control cannot find every mate in one, so it cannot be the bar");
        println!("  for anything else. Fix the harness before auditing the reference programs.");
        return;
    }

    // ---- UCT MCTS -------------------------------------------------------------------------
    let mcts = reference::uct_mcts();
    let (m_legal, m_ev_s, m_cost_s, m_over_s) = probe(&mcts, &net, &ps, depth, 8);
    let (m_legal2, m_ev_b, m_cost_b, m_over_b) = probe(&mcts, &net, &ps, depth, 64);
    println!("\nUCT MCTS");
    println!("  @sims 8   legal {m_legal}/{}  over {m_over_s}  evals {m_ev_s}  cost {m_cost_s}",
             ps.len());
    println!("  @sims 64  legal {m_legal2}/{}  over {m_over_b}  evals {m_ev_b}  cost {m_cost_b}",
             ps.len());
    // SAME BAR AS PN, and it was not applied here first time round. The original MCTS verdict
    // asked only: legal move, evals > 0, cost scales with simulations. PN was additionally
    // required to SOLVE. Holding two reference programs to different standards is how a
    // degenerate program keeps a "faithful" label -- exactly what happened to hash-reuse. The
    // ladder sweep (ladder_sweep.log) independently reports MCTS finding 0 mates out of 120.
    let mut m_solved = [0usize; 3];
    // EXPLORATION-WEIGHT SWEEP. Mix(q,u,k) = (q*k + u*(16-k))/16, and the audit passes
    // tables[2] = 1 -- which weights the EXPLOITATION term at 1/16 and is close to pure random
    // exploration. That is a harness choice about a LEARNED table, not a property of the program,
    // so a shortfall at k=1 must not be reported as the encoding failing.
    for kw in [1i64, 4, 8, 16] {
        let mut sv = 0usize;
        for (q, best) in &mates {
            let mut i = Interp::new(&net, vec![depth, 32_000, kw]);
            if i.run(&mcts, q, 256) == *best { sv += 1; }
        }
        println!("  exploration weight {kw:>3} (budget 256): mate-in-one {sv}/{}", mates.len());
    }
    for (k, bgt) in [64i64, 256, 1024].iter().enumerate() {
        for (q, best) in &mates {
            let mut i = Interp::new(&net, vec![depth, 32_000, interp::UCT_EXPLORATION]);
            if i.run(&mcts, q, *bgt) == *best { m_solved[k] += 1; }
        }
        println!("  budget {bgt:>5}: mate-in-one {}/{}", m_solved[k], mates.len());
    }
    // Same diagnostic that separated "weak search" from "never selects" for PN.
    let (mut m_first, mut m_distinct) = (0usize, std::collections::HashSet::new());
    for (q, _best) in &mates {
        let mut i = Interp::new(&net, vec![depth, 32_000, interp::UCT_EXPLORATION]);
        let mv = i.run(&mcts, q, 256);
        if q.legal_moves().as_slice().first() == Some(&mv) { m_first += 1; }
        m_distinct.insert(format!("{mv:?}"));
    }
    println!("  returns the FIRST legal move  {m_first}/{}", mates.len());
    println!("  distinct moves over {} positions {}", mates.len(), m_distinct.len());
    let mcts_ok = m_legal2 == ps.len() && m_over_b == 0 && m_ev_b > 0 && m_cost_b > m_cost_s
                  && m_solved[2] == mates.len();
    println!("  VERDICT: {}", if mcts_ok { "FAITHFUL — runs, evaluates, spends its budget, and SOLVES" }
                              else { "FAILS the solving bar — the same bar PN is held to" });

    // ---- PROOF-NUMBER SEARCH ---------------------------------------------------------------
    let pn = reference::proof_number();
    // 256, not 64: the budget sweep below shows 64 is simply under-resourced (21/23 -> 23/23
    // at 256), and judging a prover at a budget it cannot finish in measures the budget.
    let (p_legal, p_ev, _pc, p_over) = probe(&pn, &net, &ps, depth, 256);
    // BUDGET SWEEP. A prover that finds 21/23 is not degenerate, it is under-resourced or
    // slightly wrong, and those are different diagnoses. If the count rises with budget it is
    // resource; if it plateaus below 23 there is a residual defect.
    for bgt in [64, 256, 1024] {
        let mut soln = 0usize;
        for (q, best) in &mates {
            let mut i = Interp::new(&net, vec![depth, 32_000, interp::UCT_EXPLORATION]);
            if i.run(&pn, q, bgt) == *best { soln += 1; }
        }
        println!("  budget {bgt:>5}: mate-in-one {soln}/{}", mates.len());
    }
    let mut solved = 0usize;
    for (p, best) in &mates {
        let mut i = Interp::new(&net, vec![depth, 32_000, interp::UCT_EXPLORATION]);
        if i.run(&pn, p, 256) == *best { solved += 1; }
    }
    println!("\nproof-number search");
    println!("  legal {p_legal}/{}  over_budget {p_over}  evals {p_ev} (0 is CORRECT, PN is terminal-driven)",
             ps.len());
    println!("  mate-in-one found {solved}/{}", mates.len());
    // IS IT SEARCHING AT ALL, or just returning the first legal move? A prover that fails to
    // prove and a program whose argmax never moves off element zero look identical from the
    // mate count alone, and they are different defects: the first is a weak search, the second
    // is a sketch that never selects.
    let (mut first_move, mut same_as_ab) = (0usize, 0usize);
    for (p, _best) in &mates {
        let mut i = Interp::new(&net, vec![depth, 32_000, interp::UCT_EXPLORATION]);
        let m = i.run(&pn, p, 256);
        let l = p.legal_moves();
        if l.as_slice().first() == Some(&m) { first_move += 1; }
        let mut j = Interp::new(&net, vec![depth, 32_000, interp::UCT_EXPLORATION]);
        if j.run(&ab, p, depth) == m { same_as_ab += 1; }
    }
    println!("  returns the FIRST legal move  {first_move}/{}", mates.len());
    println!("  agrees with alpha-beta        {same_as_ab}/{}", mates.len());
    let pn_ok = p_legal == ps.len() && p_over == 0 && solved == mates.len();
    println!("  VERDICT: {}", if pn_ok { "FAITHFUL — runs and solves forced mates" }
                              else { "FAILS — a prover that misses a mate in one is not a prover" });
}
