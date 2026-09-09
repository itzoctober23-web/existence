//! Does the INVERTED exploration term explain the 50x cost anomaly? Pre-registered test.
//!
//! THE ANOMALY. STATE.md stopped this thread on an observation it could not explain: raising the UCT
//! exploration weight cost ~50x for the SAME playout count. Its reasoning was that raising
//! exploration makes the tree broad and shallow, which must be CHEAPER, so "the code and the
//! measurement disagree about the sign" — and that disagreement was called "the real reason to stop".
//!
//! THE EXPLANATION UNDER TEST. Raising the weight never raised exploration. `uct_mcts` reads table
//! slot 2 both as the scale inside the sqrt and as the weight of `Mix(q, u, c) = (q*c+u*(16-c))/16`,
//! so `u`'s coefficient is `16 - c`: zero at 16 and NEGATIVE above it. At c = 360_000 the program is
//! penalised for exploring, i.e. it prefers ALREADY-VISITED children. A playout stops at the first
//! unvisited node, so preferring visited children makes each descent go DEEPER before terminating.
//! Deep narrow descents, not broad shallow ones — which predicts MORE cost, the sign that was
//! measured.
//!
//! PRE-REGISTERED in STATE.md before this was written:
//!   * Mix at K=360000 costs substantially MORE than sum at K=360000  => explanation stands.
//!   * their costs are comparable                                     => explanation is WRONG and
//!     the anomaly is reopened. This must be reported, not quietly dropped.
//!
//! EQUAL WORK IS PROVEN, NOT ASSUMED. A cost ratio between two arms means nothing until both are
//! shown to have done the same amount of work — three stacked harness bugs in this project were
//! caught by exactly that check. Here the invariant is sharp: every playout terminates at exactly
//! ONE unvisited node, so `evals` must be IDENTICAL across encodings and weights. If evals differ,
//! the arms are not comparable and the cost ratio is not a descent-depth measurement.
use board::Position;
use grammar::reference;
use interp::Interp;
use nnue::Net;

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

/// Returns (evals, cost, over_budget_count).
///
/// OVER_BUDGET IS THE THIRD NUMBER AND IT IS NOT OPTIONAL. The first version of this probe reported
/// only evals and cost, and at budget 256 the Mix arm at K=360000 showed 301 evals against the sum
/// arm's 10239 with a 21x cost ratio. That ratio was meaningless: the cost totalled 80,000,048,048
/// over 40 positions, i.e. 2.0e9 each, which is EXACTLY `cost_cap`. Every one of those runs aborted
/// on the cap instead of finishing its playouts, so the arms had not done the same work and the
/// comparison measured where the cap bites, not how deep the descent goes. An aborted run also
/// silently looks like a cheap one if you only read evals.
fn measure(prog: &grammar::Program, net: &Net, ps: &[Position], k: i64, budget: i64)
    -> (u64, u64, usize, u64) {
    let (mut evals, mut cost, mut over, mut ceil) = (0u64, 0u64, 0usize, 0u64);
    for p in ps {
        let mut i = Interp::new(net, vec![2, 32_000, k]);
        i.run(prog, p, budget);
        evals += i.evals;
        cost += i.cost;
        ceil += i.ceiling_hits;
        if i.over_budget { over += 1; }
    }
    (evals, cost, over, ceil)
}

fn main() {
    let net = Net::random(16, 7);
    let ps = corpus(40, 0xC0FFEE);
    let mix = reference::uct_mcts();
    let sum = reference::uct_mcts_sum();

    // BUDGET CHOSEN BY THE DATA, not picked. At 256 every Mix run at K=360000 hits cost_cap, so the
    // arms abort at different points and no ratio between them is interpretable. Sweep upward and
    // report the largest budget at which NOTHING is over budget; that is the only regime where a
    // cost comparison is a statement about the programs rather than about the cap.
    let mut budget = 0i64;
    println!("BUDGET CALIBRATION — largest playout budget at which no arm hits cost_cap");
    for b in [4i64, 8, 16, 32, 64, 128, 256] {
        let mut worst = 0usize;
        for (_, prog) in [("Mix", &mix), ("sum", &sum)] {
            for k in [8i64, 600, 360_000] {
                worst = worst.max(measure(prog, &net, &ps, k, b).2);
            }
        }
        println!("  budget {b:>4}: {worst} of {} runs over budget", ps.len());
        if worst == 0 { budget = b; }
    }
    if budget == 0 {
        println!("\n  ABORT: every budget caps somewhere. No cost comparison is possible.");
        return;
    }
    println!("\n  -> comparing at budget {budget}\n");

    println!("  {:<8} {:>9}  {:>12}  {:>14}  {:>11}  {:>5}  {:>12}",
             "encoding", "K", "evals", "cost", "cost/eval", "over", "ceiling hits");

    let mut rows = Vec::new();
    for (name, prog) in [("Mix", &mix), ("sum", &sum)] {
        for k in [8i64, 600, 360_000] {
            let (e, c, o, ch) = measure(prog, &net, &ps, k, budget);
            println!("  {name:<8} {k:>9}  {e:>12}  {c:>14}  {:>11.1}  {o:>5}  {ch:>12}",
                     if e > 0 { c as f64 / e as f64 } else { 0.0 });
            rows.push((name, k, e, c, ch));
        }
    }

    // ---- the equal-work check, before any ratio is quoted ----
    let evals: Vec<u64> = rows.iter().map(|r| r.2).collect();
    let same = evals.iter().all(|e| *e == evals[0]);
    println!("\n  eval counts identical across all six arms: {}", if same { "YES" } else { "NO" });
    if !same {
        println!("  Evals are NOT equal -- and here that is the RESULT, not a broken harness.");
        println!("  A playout ends at the first unvisited node, so it should produce exactly one");
        println!("  eval. Producing FEWER means playouts are ending some other way: the descent");
        println!("  hits MAX_CALL_DEPTH (128) and unwinds as 0 without ever reaching a leaf.");
        println!("  The ceiling-hit column is the direct measurement of that, and it is what the");
        println!("  cost ratio was only an indirect proxy for.");
    }

    // ---- the pre-registered comparison ----
    let get = |n: &str, k: i64| rows.iter().find(|r| r.0 == n && r.1 == k).map(|r| r.3).unwrap_or(0);
    let getc = |n: &str, k: i64| rows.iter().find(|r| r.0 == n && r.1 == k).map(|r| r.4).unwrap_or(0);
    let (m_hi, s_hi) = (get("Mix", 360_000), get("sum", 360_000));
    let (m_lo, s_lo) = (get("Mix", 8), get("sum", 8));
    println!("\n  PRE-REGISTERED: at K=360000, Mix should cost substantially MORE than sum.");
    println!("    Mix {m_hi}   sum {s_hi}   ratio {:.2}x", m_hi as f64 / s_hi.max(1) as f64);
    println!("    (at K=8, where Mix's coefficient on u is still POSITIVE: Mix {m_lo}  sum {s_lo}  ratio {:.2}x)",
             m_lo as f64 / s_lo.max(1) as f64);
    println!("\n  DIRECT TEST -- recursion-ceiling hits, which measure descent depth without any");
    println!("  cost accounting at all. Deep descents hit MAX_CALL_DEPTH; shallow ones cannot.");
    println!("    Mix  K=8 {:>10}   K=600 {:>10}   K=360000 {:>10}",
             getc("Mix", 8), getc("Mix", 600), getc("Mix", 360_000));
    println!("    sum  K=8 {:>10}   K=600 {:>10}   K=360000 {:>10}",
             getc("sum", 8), getc("sum", 600), getc("sum", 360_000));
    println!("\n  Ceiling hits rising with K for Mix and staying at zero for sum confirms the");
    println!("  mechanism: the cost tracks the SIGN of u's blend coefficient, not the weight's size.");
    println!("  Mix hitting the ceiling no more than sum REFUTES it and the anomaly is reopened.");
}
