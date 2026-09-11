//! STATIC-VS-DEEP RESIDUAL — the diagnostic that two specification documents call load-bearing and
//! that has never existed in this repository.
//!
//!   MASTER_PLAN.md:273 (P1 kill criterion)
//!     "no iteration-over-iteration gain across iterations 4-8 AND the static-vs-deep residual is
//!      not shrinking -> pipeline bug; stop and find it"
//!
//!   FITNESS.md:270 (adversarial check, §8)
//!     "the candidate's explanation-layer confidence (PV stability, static-vs-deep residual class)
//!      must be LOW on at least 80% of positions where the candidate's move differs from its own
//!      32x-cost move"
//!
//! A grep for `residual` across the crates returns only profiling breakdowns and R^2 spreads. HALF
//! OF THE P1 KILL CRITERION HAS NEVER BEEN MEASURED. Tonight the other half — iteration-over-
//! iteration gain — was measured repeatedly and read as a plateau, and a kill was never fired
//! because the conjunction was never evaluable. This closes that.
//!
//! WHAT IT MEASURES. For a fixed position set: how far the net's STATIC eval sits from what a real
//! SEARCH from the same position returns. A learning eval increasingly predicts what search finds,
//! so the residual shrinks. A stuck eval's residual stays flat however much its weights move —
//! which is the exact signature tonight's lr result found in weight space (the control travelled
//! 1.73x farther and gained nothing) but could not confirm without games.
//!
//! WHY IT IS WORTH BUILDING RATHER THAN RUNNING MORE MATCHES. It needs NO GAMES. `netmatch`'s
//! between-seed sd is 0.047, which exceeds its own within-run ci95 of 0.030 at 224 pairs — one seed
//! is frequently a lottery and this project has said so about its own promotion. A per-position
//! regression has no opening book, no pairing, and no seed lottery: the same positions are put to
//! every net.
//!
//! ---------------------------------------------------------------------------------------------
//! THE FRAME PROBLEM, AND WHY THERE IS A HARD GUARD RATHER THAN A COMMENT
//!
//! `Net::eval` documents itself as MOVER-relative ("evaluate from WHITE's point of view, then flip
//! for the mover"). `Searcher::best_move` is negamax, so its score is mover-relative too. If that
//! reading is wrong for either one, every residual below is |a - (-a)| = 2|a| and the instrument
//! reports garbage that LOOKS like a large residual — a wrong number, not a crash.
//!
//! So the frames are not assumed. Both sides are converted to WHITE's point of view and the
//! correlation between them is computed and GATED: a frame flip inverts the sign of the
//! correlation, turning a strong positive into a strong negative, which no amount of noise does.
//! Below the floor the run ABORTS instead of printing.
//!
//! The depth-0 identity would have been the tighter check — the leaf returns the static eval, so a
//! depth-0 search must reproduce it exactly — but `best_move` computes `depth - 1` on a u32 at the
//! root, so depth 0 underflows there. The correlation gate is the check that this API admits.
//!
//! ---------------------------------------------------------------------------------------------
//! TWO CONFOUNDS, BOTH REPORTED RATHER THAN ASSUMED AWAY
//!
//! 1. A NET THAT SHRINKS ITS OUTPUT RANGE FAKES A SHRINKING RESIDUAL. An eval that collapses toward
//!    a constant has a small residual against a small target and has learned nothing. So `sd(static)`
//!    is printed beside the residual: if it falls in step, the "improvement" is collapse. The
//!    scale-free column `rms/sd(deep)` is the one to read — 1.00 means "no better than predicting
//!    the mean", which is exactly what a collapsed eval scores.
//!
//! 2. MATE SCORES ARE NOT EVAL ERRORS. A search returning +-MATE differs from any static eval by
//!    ~30,000, and a handful of them would swamp a mean of a few hundred. They are EXCLUDED and
//!    COUNTED, never silently dropped: a changing exclusion count is itself a finding (a net whose
//!    positions are increasingly decided by search is a different sample, not a better eval).
//!
//! CONTROL BUILT IN, following `eval_anatomy`: an untrained random net is always measured. If a
//! trained net's residual is not clearly below the random net's, this instrument cannot see
//! learning and none of its readings mean anything.
//!
//! USAGE:  static_deep_residual <n_pos> <depth> <seed> [net.net ...]

use board::{Color, Position};
use nnue::Net;
use pipeline::datagen::Rng;
use pipeline::search::{Searcher, MATE};

/// Scores this close to MATE are search verdicts, not evaluations. See confound 2 above.
const MATE_BAND: i32 = MATE - 1000;

/// A frame flip inverts this sign. Real noise does not take a genuine positive correlation
/// below this floor, so it separates "the frames disagree" from "the net is weak".
const CORR_FLOOR: f64 = 0.30;

struct Row {
    name: String,
    n_used: usize,
    n_mate: usize,
    mean_abs: f64,
    rms: f64,
    sd_static: f64,
    sd_deep: f64,
    corr: f64,
    /// RMS residual in TANH SPACE — the space the trainer actually works in.
    ///
    /// `trainer.rs:44-47` fits `tanh(out)` to `(1-blend)*z + blend*tanh(root/net.scale)`, and
    /// `datagen.rs:201` writes `Sample.root` = the search score at `--depth`. So at blend 1.0 this
    /// column IS the training objective's error term, not an analogy for it. The raw-centipawn
    /// columns are a MONOTONE BUT NONLINEAR relative: past |score| ~ scale the tanh saturates and a
    /// large raw residual costs the trainer almost nothing, so the two can rank nets differently.
    rms_tanh: f64,
}

fn mean(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

fn sd(v: &[f64]) -> f64 {
    let m = mean(v);
    (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / v.len() as f64).sqrt()
}

fn corr(a: &[f64], b: &[f64]) -> f64 {
    let (ma, mb) = (mean(a), mean(b));
    let (sa, sb) = (sd(a), sd(b));
    if sa == 0.0 || sb == 0.0 {
        return 0.0;
    }
    let cov = a
        .iter()
        .zip(b)
        .map(|(x, y)| (x - ma) * (y - mb))
        .sum::<f64>()
        / a.len() as f64;
    cov / (sa * sb)
}

fn measure(name: &str, net: &Net, ps: &[Position], depth: u32, seed: u64) -> Row {
    let mut s = Searcher::with_seed(seed);
    let mut scratch = Vec::new();
    let (mut st, mut dp) = (Vec::new(), Vec::new());
    let mut n_mate = 0usize;

    for p in ps {
        let mut q = p.clone();
        let (_, deep_mover) = s.best_move(&mut q, depth, net);
        if deep_mover.abs() >= MATE_BAND {
            n_mate += 1;
            continue;
        }
        let static_mover = net.eval(p, &mut scratch);
        // BOTH to white's point of view. Doing this to both sides is what makes the residual
        // frame-consistent; the correlation gate below is what proves the assumption held.
        let flip = if p.stm == Color::White { 1.0 } else { -1.0 };
        st.push(static_mover as f64 * flip);
        dp.push(deep_mover as f64 * flip);
    }

    let resid: Vec<f64> = st.iter().zip(&dp).map(|(a, b)| b - a).collect();
    // Same residual, put through the trainer's own squashing so it is comparable across nets with
    // different output scales. `net.scale` is the divisor trainer.rs uses, not a constant I chose.
    let sc = net.scale as f64;
    let rt: Vec<f64> = st
        .iter()
        .zip(&dp)
        .map(|(a, b)| (b / sc).tanh() - (a / sc).tanh())
        .collect();
    Row {
        rms_tanh: (rt.iter().map(|x| x * x).sum::<f64>() / rt.len() as f64).sqrt(),
        name: name.to_string(),
        n_used: resid.len(),
        n_mate,
        mean_abs: mean(&resid.iter().map(|x| x.abs()).collect::<Vec<_>>()),
        rms: (resid.iter().map(|x| x * x).sum::<f64>() / resid.len() as f64).sqrt(),
        sd_static: sd(&st),
        sd_deep: sd(&dp),
        corr: corr(&st, &dp),
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let n_pos: usize = a.first().and_then(|s| s.parse().ok()).unwrap_or(600);
    let depth: u32 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(4);
    let seed: u64 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(20260911);
    let files: Vec<String> = a.iter().skip(3).cloned().collect();

    // netmatch rejects a depth outside 1..=12 after a seed landed in the depth slot and searched to
    // depth 911,911. Same guard, same reason. Depth 0 additionally underflows at the root.
    if depth == 0 || depth > 12 {
        eprintln!("depth {depth} is outside 1..=12 -- a SEED in the depth slot is the usual cause");
        std::process::exit(2);
    }

    // ONE shared position set: any difference between nets must be the NET, never the sample.
    let mut rng = Rng(seed | 1);
    let mut ps: Vec<Position> = Vec::with_capacity(n_pos);
    while ps.len() < n_pos {
        let mut p = Position::startpos();
        let plies = 4 + rng.below(50);
        let mut ok = true;
        for _ in 0..plies {
            let l = p.legal_moves();
            if l.is_empty() {
                ok = false;
                break;
            }
            p.make_move(l.as_slice()[rng.below(l.len())]);
        }
        if ok && !p.legal_moves().is_empty() {
            ps.push(p);
        }
    }

    println!("static_deep_residual: {n_pos} positions, depth {depth}, seed {seed}");
    println!("  residual = deep_search_value - static_eval, both in WHITE's point of view\n");

    let mut rows = vec![measure("origin(random)", &Net::random(16, 20260907), &ps, depth, seed)];
    for f in &files {
        match Net::load(f) {
            Ok(n) => rows.push(measure(f, &n, &ps, depth, seed)),
            Err(e) => println!("  {f}: LOAD FAILED ({e})"),
        }
    }

    // THE GATE. A frame flip shows up here and nowhere else.
    if let Some(bad) = rows.iter().find(|r| r.corr < CORR_FLOOR) {
        eprintln!(
            "\nABORT: {} correlates {:.3} between static and deep, below the {CORR_FLOOR} floor.",
            bad.name, bad.corr
        );
        eprintln!("A point-of-view mismatch between Net::eval and Searcher::best_move inverts this");
        eprintln!("sign. Do not read the residuals above -- they would be |a-(-a)| = 2|a|.");
        std::process::exit(3);
    }

    println!(
        "  {:<30} {:>5} {:>5} {:>8} {:>8} {:>8} {:>8} {:>6} {:>6} {:>9}",
        "net", "used", "mate", "mean|r|", "rms r", "sd stat", "sd deep", "rms/sd", "corr", "rms TANH"
    );
    for r in &rows {
        println!(
            "  {:<30} {:>5} {:>5} {:>8.1} {:>8.1} {:>8.1} {:>8.1} {:>6.3} {:>6.3} {:>9.4}",
            r.name, r.n_used, r.n_mate, r.mean_abs, r.rms, r.sd_static, r.sd_deep,
            r.rms / r.sd_deep, r.corr, r.rms_tanh
        );
    }

    println!("\n  rms/sd is the column to read: 1.000 = no better than predicting the mean.");
    println!("  A residual that falls WITH sd static is an eval collapsing toward a constant,");
    println!("  not an eval learning. Compare every row against origin(random).");

    // THE CONTROL'S VERDICT, STATED RATHER THAN LEFT TO THE READER.
    //
    // An earlier version of this block compared only `rms/sd_deep` and printed "instrument
    // separates trained from untrained" -- a CONFIDENTLY WRONG verdict, because that one column
    // happens to favour the trained nets while the two columns added afterwards both favour the
    // UNTRAINED one. A check that reports a pass on a subset of the evidence is worse than no
    // check. It now fires on the two failure modes that actually occur:
    if rows.len() > 1 {
        let rnd = &rows[0];
        let scale_dominated = rows[1..].iter().all(|r| r.rms_tanh > rnd.rms_tanh);
        let coupled = rows.iter().all(|r| r.corr > 0.80);

        println!("\n  CONTROL VERDICT");
        if scale_dominated {
            println!(
                "  * SCALE-DOMINATED: origin(random) has the SMALLEST tanh residual ({:.4}) of every",
                rnd.rms_tanh
            );
            println!("    net here. It has not learned anything -- its eval is nearly CONSTANT");
            println!(
                "    (sd {:.1}), so search cannot move it far. Ranking nets by residual therefore",
                rnd.sd_static
            );
            println!("    rewards a SMALL OUTPUT RANGE, not a good evaluation.");
        }
        if coupled {
            println!(
                "  * SELF-COUPLED: every net correlates >0.80 static-vs-deep, random included ({:.3}).",
                rnd.corr
            );
            println!("    The search EVALUATES WITH THE SAME NET (datagen.rs:187), so the two sides are");
            println!("    one function at two depths. High agreement is mechanical, not evidence.");
        }
        if scale_dominated || coupled {
            println!("\n  => NOT USABLE AS A CROSS-NET STRENGTH OR HEALTH RANKING. Valid uses remain:");
            println!("     per-position, WITHIN one net (FITNESS.md:270 asks exactly that -- whether a");
            println!("     net anticipates its OWN deeper search), and within a single lineage once");
            println!("     normalised by that lineage's own output scale.");
        } else {
            println!("  * no scale domination and no mechanical coupling detected; ranking is readable.");
        }
    }
}
