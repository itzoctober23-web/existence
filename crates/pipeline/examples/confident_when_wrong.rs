//! FITNESS.md §8 — IS THE ENGINE CONFIDENT WHERE IT IS WRONG?
//!
//!   FITNESS.md:269-272
//!     "Check: on ADVERSARIAL, the candidate's explanation-layer confidence (PV stability,
//!      static-vs-deep residual class) must be LOW on at least 80% of positions where the
//!      candidate's move differs from its own 32x-cost move. A confident wrong answer is a
//!      regression in the property the project sells."
//!
//! Like the P1 residual, this check was specified and never implemented. Unlike the P1 residual, it
//! is WELL POSED as written, and `static_deep_residual_RESULT.md` says why: it asks a per-position
//! question WITHIN ONE NET — does this net know when its own cheap answer is unreliable — and uses
//! the answer to classify positions, never to rank nets. The self-coupling that makes the residual
//! useless for ranking (the search evaluates with the net under test, so an untrained net scores
//! corr 0.900) is not a defect here. It is the subject.
//!
//! ---------------------------------------------------------------------------------------------
//! WHAT "32x-cost" MEANS HERE — EXTRA DEPTH, WITH THE MULTIPLE MEASURED
//!
//! The obvious reading is a NODE budget: `cap` against `32 * cap` is an exact factor, where extra
//! depth would be some power of the branching factor and could not be held constant. That was the
//! first implementation and IT STARVED ON EVERY POSITION, reporting an empty table for all three
//! nets.
//!
//! Cause, found by reading `best_move_capped` rather than guessing a second time: it searches root
//! moves one at a time at FULL depth and `break`s on abort BEFORE updating `best_s`
//! (search.rs:310), returning `-INF` if the FIRST root move's subtree alone exceeds the cap. A
//! usable cap must therefore exceed one root subtree yet stay below the whole search — a window
//! that moves with every position and cannot be set from the command line.
//!
//! So the cheap and rich sides differ by PLIES, which never starves, and the resulting cost
//! multiple is measured per position from `Searcher::nodes` and reported as a median. +3 plies
//! lands near 32x at this branching factor, but the table prints what it actually was: an
//! approximation that is stated is honest, one that is hidden is not.
//!
//! ---------------------------------------------------------------------------------------------
//! DEFINING "CONFIDENCE" WITHOUT INVENTING A THRESHOLD
//!
//! Confidence is the static-vs-deep residual: a SMALL residual means the net's static opinion
//! already agrees with what search finds, i.e. it is confident. A large residual means search
//! substantially revised its opinion.
//!
//! An absolute cutoff would be arbitrary and would not survive a change of output scale (nets here
//! range over sd 237-420). So the threshold is the MEDIAN residual over all positions measured for
//! that same net. That is self-calibrating, and it comes with a control built in:
//!
//!   by construction exactly 50% of ALL positions sit above the median.
//!
//! So the base rate is 50% and the spec asks for 80%. The check is therefore asking whether
//! move-flip positions are ENRICHED for high residual relative to a coin flip. If the enrichment is
//! absent the reading is "this net's confidence carries no information about its own reliability",
//! which is a real and reportable finding rather than a pass.
//!
//! USAGE:  confident_when_wrong <n_pos> <depth> <extra_plies> <seed> [net.net ...]

use board::{Color, Position};
use nnue::Net;
use pipeline::datagen::Rng;
use pipeline::search::{Searcher, MATE};

const MATE_BAND: i32 = MATE - 1000;
/// FITNESS.md:270 — "LOW on at least 80% of positions where the candidate's move differs".
const REQUIRED: f64 = 0.80;

fn median(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if v.is_empty() { return 0.0; }
    v[v.len() / 2]
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let n_pos: usize = a.first().and_then(|s| s.parse().ok()).unwrap_or(400);
    let depth: u32 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(8);
    let extra: u32 = a.get(2).and_then(|s| s.parse().ok()).unwrap_or(3);
    let seed: u64 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(20260911);
    let files: Vec<String> = a.iter().skip(4).cloned().collect();

    if depth == 0 || depth > 12 {
        eprintln!("depth {depth} is outside 1..=12 -- a SEED in the depth slot is the usual cause");
        std::process::exit(2);
    }

    let mut rng = Rng(seed | 1);
    let mut ps: Vec<Position> = Vec::with_capacity(n_pos);
    while ps.len() < n_pos {
        let mut p = Position::startpos();
        let plies = 4 + rng.below(50);
        let mut ok = true;
        for _ in 0..plies {
            let l = p.legal_moves();
            if l.is_empty() { ok = false; break; }
            p.make_move(l.as_slice()[rng.below(l.len())]);
        }
        if ok && !p.legal_moves().is_empty() { ps.push(p); }
    }

    println!("confident_when_wrong: {n_pos} positions, depth {depth} vs {} (+{extra} plies), seed {seed}",
             depth + extra);
    println!("  FITNESS.md:270 asks: is confidence LOW on >= {:.0}% of move-flip positions?",
             REQUIRED * 100.0);
    println!("  confidence = SMALL static-vs-deep residual. Threshold = this net's own median.");
    println!("  Base rate is 50% by construction, so the question is enrichment over a coin flip.\n");

    println!("  {:<26} {:>7} {:>7} {:>9} {:>10} {:>8} {:>8}",
             "net", "used", "flips", "flip rate", "low-conf %", "cost", "verdict");

    for f in &files {
        let net = match Net::load(f) {
            Ok(n) => n,
            Err(e) => { println!("  {f}: LOAD FAILED ({e})"); continue; }
        };
        let mut s = Searcher::with_seed(seed);
        let mut scratch = Vec::new();
        let mut resid: Vec<f64> = Vec::new();
        let mut flip: Vec<bool> = Vec::new();
        let mut n_mate = 0usize;
        let mut ratios: Vec<f64> = Vec::new();

        for p in &ps {
            // DEPTH, NOT A NODE CAP -- and the cost multiple is MEASURED rather than assumed.
            //
            // The node-cap version of this check starved on every single position and reported an
            // empty table. Cause, after reading `best_move_capped` instead of guessing a second
            // time: it searches root moves one at a time at FULL depth and `break`s on abort
            // BEFORE updating `best_s` (search.rs:310), so if the FIRST root move's subtree exceeds
            // the cap it returns -INF. A usable cap must exceed one root subtree yet stay under the
            // whole search -- roughly 10%-100% of the full node count, a window that moves with the
            // position and cannot be fixed from the command line.
            //
            // Depth never starves. The cost ratio is then not exactly 32x, so it is MEASURED per
            // position from `Searcher::nodes` and the median is reported: an approximation that is
            // stated is honest, one that is hidden is not.
            // NODE COUNTS ARE DELTAS, because `best_move` does NOT reset the counter.
            //
            // `best_move_capped` sets `self.nodes = 0` (search.rs:303); `best_move` resets `ply`
            // and the accumulator but leaves `nodes` ALONE. Reading `s.nodes` after each call
            // therefore returns a RUNNING TOTAL over every position so far, and dividing two
            // running totals gives ~1.0 — which is exactly what the first version printed for a
            // depth-3 against depth-6 comparison, a ratio that is impossible on its face.
            let n0 = s.nodes;
            let mut q = p.clone();
            let (cheap_mv, _) = s.best_move(&mut q, depth, &net);
            let cheap_nodes = s.nodes.saturating_sub(n0).max(1);
            let n1 = s.nodes;
            let mut q2 = p.clone();
            let (rich_mv, rich_sc) = s.best_move(&mut q2, depth + extra, &net);
            let rich_nodes = s.nodes.saturating_sub(n1).max(1);
            ratios.push(rich_nodes as f64 / cheap_nodes as f64);

            // A mate verdict is a search result, not an evaluation error; excluded as elsewhere.
            if rich_sc.abs() >= MATE_BAND { n_mate += 1; continue; }
            let st = net.eval(p, &mut scratch) as f64;
            // Both are mover-relative already, so the residual needs no POV flip -- and taking the
            // absolute value makes the frame irrelevant regardless.
            resid.push((rich_sc as f64 - st).abs());
            flip.push(cheap_mv != rich_mv);
        }

        if resid.is_empty() {
            println!("  {f}: NO USABLE POSITIONS -- {n_mate} mate-scored of {}.", ps.len());
            continue;
        }
        let med_ratio = median(&mut ratios.clone());
        // SANITY GATE ON THE INSTRUMENT ITSELF. Searching +{extra} extra plies cannot cost the same
        // as not searching them; a ratio at or below 1 means the node counter is not measuring what
        // this code thinks it is, and the first version printed exactly 1.0x for depth 3 vs 6.
        if med_ratio <= 1.0 {
            eprintln!("ABORT: median cost ratio {med_ratio:.2}x for +{extra} plies is impossible.");
            eprintln!("The node counter is not being read as a per-search delta. Do not read the table.");
            std::process::exit(5);
        }
        let thr = median(&mut resid.clone());
        let n_flip = flip.iter().filter(|x| **x).count();
        // "LOW confidence" = residual ABOVE the median: search revised the static opinion a lot.
        let low_conf = resid.iter().zip(&flip)
            .filter(|(_, fl)| **fl)
            .filter(|(r, _)| **r > thr)
            .count();
        let pct = if n_flip > 0 { low_conf as f64 / n_flip as f64 } else { f64::NAN };

        let verdict = if n_flip < 20 {
            "TOO FEW"
        } else if pct >= REQUIRED {
            "PASS"
        } else if pct > 0.55 {
            "enriched, below spec"
        } else {
            "NO SIGNAL"
        };
        println!("  {:<26} {:>7} {:>7} {:>8.1}% {:>9.1}% {:>7.1}x {:>8}",
                 f, resid.len(), n_flip,
                 100.0 * n_flip as f64 / resid.len() as f64, 100.0 * pct, med_ratio, verdict);
    }

    println!("\n  low-conf % near 50 means confidence carries NO information about reliability:");
    println!("  the net is exactly as sure of itself on positions its cheap search gets wrong.");
    println!("  That is a finding, not a pass -- FITNESS.md:271 calls a confident wrong answer");
    println!("  \"a regression in the property the project sells\".");
}
