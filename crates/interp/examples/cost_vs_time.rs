//! IS THE COST MODEL A FAITHFUL PROXY FOR WALL TIME? The declared check that was never run.
//!
//! FITNESS.md:116 specifies the cost unit and then, verbatim:
//!
//!   "Wall time on the declared hardware is recorded alongside as the reality check on the cost
//!    model (revisit trigger: cost-vs-time correlation < 0.95)."
//!
//! Two things are true about that sentence today. Wall time is NOT recorded alongside anywhere in
//! the fitness path -- `crates/pipeline` has no timing of a program run at all -- and the phrase
//! "cost-vs-time correlation" appears in no result file. It is a declared trigger that has never
//! been evaluated, so the cost model has never been checked against the reality it stands in for.
//!
//! WHY IT IS THE BINDING CONSTRAINT NOW. `grammar4_addfn_unpark_blocker.md` records that the other
//! two clauses blocking the AddFn unpark have moved: the divergence clause is satisfiable, and
//! acceptance-vs-retention was a conflation. What remains is the COST clause -- `cost_of` ends in
//! `_ => 2`, so any new node falls through to a charge of 2. An underpriced primitive is a standing
//! invitation for the search to spend everything on it for free. Before pricing a NEW node, the
//! honest first question is whether the EXISTING prices are right, and that is this measurement.
//!
//! THE DESIGN POINT THAT MAKES IT MEANINGFUL. A correlation taken over programs that share one
//! primitive mix is high by construction and says nothing: scaling the same program up moves cost
//! and time together whatever the per-primitive prices are. The cost model can only be WRONG about
//! the relative price of different primitives, so the program set must vary the MIX. `reference::all()`
//! does exactly that -- alpha-beta (eval-bound), UCT MCTS (apply/moves-bound), and proof-number
//! search, which FITNESS.md:110 itself singles out as proving mates "from `terminal()` with zero
//! `eval` calls". If eval is mispriced relative to terminal, those two land on opposite sides of the
//! line and the correlation falls. That is the failure this check exists to catch.
//!
//! METHOD, and the three ways it could lie to me:
//!
//!   * LOAD. Wall time on a box running a trainer is contended. Programs are therefore measured
//!     INTERLEAVED (all programs, then all programs again) rather than one program to completion,
//!     so a drift in load lands on every arm instead of on whichever ran during it. Times are
//!     reported as MEDIANS over the repeats, not means.
//!   * NOISE. A weak correlation is only evidence if the timer is tighter than the effect. The
//!     per-program relative spread across repeats is reported alongside, and any program whose own
//!     noise is large enough to move the verdict is named.
//!   * TRUNCATION. `Interp::cost_cap` defaults to 2e9. A program that hits it stops early, so its
//!     cost is CLAMPED to the cap while its time is whatever it took to get there -- a fake point
//!     that would drag the fit. Capped programs are detected and excluded from the fit, and said so.
//!
//! Cost is deterministic by construction, so the repeats double as a self-test: if a program's cost
//! differs between two runs on the same positions, the instrument is broken and the run says so
//! rather than reporting a correlation over numbers that do not reproduce.
//!
//! READING IT. The headline is Pearson r over programs. The more interpretable number is printed
//! beside it: cost per microsecond for each program. A faithful model makes that roughly constant;
//! the ratio between its largest and smallest value is the factor by which the model mis-prices one
//! mix against another, which is the quantity a re-calibration would have to fix.
use board::{Outcome, Position};
use grammar::reference;
use interp::Interp;
use nnue::Net;
use std::time::Instant;

fn median(v: &mut Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    if n == 0 { return 0.0; }
    if n % 2 == 1 { v[n / 2] } else { (v[n / 2 - 1] + v[n / 2]) / 2.0 }
}

fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    if n < 2.0 { return f64::NAN; }
    let mx = x.iter().sum::<f64>() / n;
    let my = y.iter().sum::<f64>() / n;
    let mut sxy = 0.0; let mut sxx = 0.0; let mut syy = 0.0;
    for i in 0..x.len() {
        let dx = x[i] - mx; let dy = y[i] - my;
        sxy += dx * dy; sxx += dx * dx; syy += dy * dy;
    }
    if sxx <= 0.0 || syy <= 0.0 { return f64::NAN; }
    sxy / (sxx * syy).sqrt()
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let getn = |flag: &str, dflt: i64| -> i64 {
        argv.iter().position(|a| a == flag)
            .and_then(|i| argv.get(i + 1)).and_then(|s| s.parse().ok()).unwrap_or(dflt)
    };
    let depth = getn("--depth", 3);
    let budget = getn("--budget", 800);
    let inf = getn("--inf", 1_000_000);
    let npos = getn("--positions", 40) as usize;
    let reps = getn("--reps", 5) as usize;

    let net = Net::random(32, 20260907);

    // Same MATE-1 mining as examples/ladder.rs, same seed, so this measures the cost model on the
    // workload FITNESS actually scores programs on rather than on a distribution invented here.
    let mut rng: u64 = 20260907;
    let mut mates: Vec<Position> = Vec::new();
    while mates.len() < npos {
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
            if winning { mates.push(p.clone()); break; }
            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
            let m = l.as_slice()[(rng % l.len() as u64) as usize];
            p.make_move(m);
        }
    }

    let progs = reference::all();
    let nprog = progs.len();
    println!("COST vs WALL TIME -- the FITNESS.md:116 revisit trigger, measured");
    println!("  depth {depth}, budget {budget}, {} positions, {reps} interleaved repeats, {nprog} programs",
             mates.len());
    println!("  trigger: revisit the cost model if Pearson r < 0.95\n");

    let mut costs: Vec<u64> = vec![0; nprog];
    let mut cost_seen: Vec<Vec<u64>> = vec![Vec::new(); nprog];
    let mut times: Vec<Vec<f64>> = vec![Vec::new(); nprog];
    // THE CAP IS PER RUN, NOT PER SWEEP. `Interp::run` sets `self.cost = 0` on entry, so the cap
    // applies to ONE position. Comparing the TOTAL over all positions against it -- which is what
    // this did on the first pass -- flags any program whose sweep total exceeds 2e9 as truncated,
    // and at 40 positions that is every expensive program. It excluded both MCTS variants and
    // proof-number search: precisely the different-mix programs this check needs, leaving a fit over
    // nine alpha-beta variants that all share one mix, where a high r is guaranteed and meaningless.
    // Capping is therefore detected INSIDE the position loop, per run.
    let cap = Interp::new(&net, vec![depth, inf, 8]).cost_cap;
    let mut capped_runs: Vec<u32> = vec![0; nprog];

    // INTERLEAVED: rep-major, so load drift is shared across programs instead of being charged to
    // whichever program happened to run while the box was busy.
    for _rep in 0..reps {
        for (pi, (_name, prog)) in progs.iter().enumerate() {
            let mut it = Interp::new(&net, vec![depth, inf, 8]);
            let mut cost = 0u64;
            let t0 = Instant::now();
            for p in &mates {
                let _mv = it.run(prog, p, budget);
                cost += it.cost;
                if it.cost >= cap { capped_runs[pi] += 1; }
            }
            let us = t0.elapsed().as_secs_f64() * 1e6;
            times[pi].push(us);
            cost_seen[pi].push(cost);
            costs[pi] = cost;
        }
    }

    // DETERMINISM SELF-TEST. Cost must reproduce exactly; if it does not, the instrument is broken
    // and no correlation computed from it means anything.
    let mut nondet = Vec::new();
    for (pi, seen) in cost_seen.iter().enumerate() {
        if seen.iter().any(|c| *c != seen[0]) { nondet.push(progs[pi].0); }
    }
    if !nondet.is_empty() {
        println!("  *** COST IS NON-DETERMINISTIC for: {:?}", nondet);
        println!("  *** The instrument is broken. Not reporting a correlation.");
        return;
    }
    println!("  cost reproduced exactly across all {reps} repeats for all {nprog} programs (self-test OK)\n");

    println!("  {:<30} {:>15} {:>12} {:>10} {:>9} {:>7}",
             "program", "cost", "time_us", "cost/us", "rel_sd", "capped");
    let mut fx: Vec<f64> = Vec::new();
    let mut fy: Vec<f64> = Vec::new();
    let mut names: Vec<&str> = Vec::new();
    let mut rates: Vec<(f64, &str)> = Vec::new();
    let mut ncapped = 0;
    for (pi, (name, _)) in progs.iter().enumerate() {
        let mut t = times[pi].clone();
        let med = median(&mut t);
        let mean = times[pi].iter().sum::<f64>() / times[pi].len() as f64;
        let var = times[pi].iter().map(|x| (x - mean) * (x - mean)).sum::<f64>()
            / (times[pi].len() as f64 - 1.0).max(1.0);
        let rel_sd = if mean > 0.0 { var.sqrt() / mean * 100.0 } else { 0.0 };
        // Capped iff at least one SINGLE-POSITION run hit the cap, counted in the loop above.
        let capped = capped_runs[pi] > 0;
        if capped { ncapped += 1; }
        let rate = costs[pi] as f64 / med.max(1e-9);
        println!("  {:<30} {:>15} {:>12.0} {:>10.0} {:>8.1}% {:>7}",
                 name, costs[pi], med, rate, rel_sd, if capped { "YES" } else { "-" });
        if !capped && costs[pi] > 0 {
            fx.push(costs[pi] as f64); fy.push(med); names.push(name);
            rates.push((rate, name));
        }
    }

    let r = pearson(&fx, &fy);
    println!("\n  n = {} programs in the fit ({} excluded as cost-capped)", fx.len(), ncapped);
    println!("  Pearson r (cost vs wall time) = {:.4}", r);

    rates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    if rates.len() >= 2 {
        let (lo, lo_n) = rates[0];
        let (hi, hi_n) = rates[rates.len() - 1];
        println!("  cost/us spread: {:.0} ({}) .. {:.0} ({}) = {:.1}x",
                 lo, lo_n, hi, hi_n, hi / lo.max(1e-9));
        println!("  -> the model charges '{}' {:.1}x more cost per real microsecond than '{}'.",
                 hi_n, hi / lo.max(1e-9), lo_n);
    }

    println!();
    if r.is_nan() {
        println!("  VERDICT: UNDEFINED -- not enough non-capped programs with nonzero cost.");
    } else if r < 0.95 {
        println!("  VERDICT: r = {:.4} < 0.95 -- THE DECLARED REVISIT TRIGGER IS MET.", r);
        println!("  FITNESS.md:116's own condition for re-examining the cost model is satisfied.");
    } else {
        println!("  VERDICT: r = {:.4} >= 0.95 -- the trigger is NOT met; the cost model tracks", r);
        println!("  wall time across these primitive mixes and stands as written.");
    }
}
