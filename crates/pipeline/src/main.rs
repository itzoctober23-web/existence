//! The learning loop.
//!
//!   datagen (self-play with the champion)  ->  train a candidate  ->  GATE it vs the champion
//!   accept only if it wins the match; otherwise keep the champion and try again.
//!
//! The training loss is NOT evidence. Only the gate decides (MASTER_PLAN: SPRT decides,
//! surrogate proposes). Loss falling while strength does nothing is the exact failure this
//! project has already seen elsewhere.

use nnue::Net;
use pipeline::arch::{self, WIDTH_MENU};
use pipeline::datagen::{self, Rng, Sample};
use pipeline::gate;
use pipeline::ledger::{Entry, GateEvidence, Ledger, Reason};
use pipeline::trainer::Trainer;
use board::{Color, Position};

/// Train a candidate at `width`, starting either from scratch or from `seed_net` widened.
///
/// How long training takes is a property of the width, so it is MEASURED (early stopping on the
/// held-out split) rather than fixed at a number that would quietly advantage whichever width
/// the constant happened to suit.
///
/// The paragraph that used to stand here said "an ARCH candidate cannot inherit the champion's
/// weights -- it is a different shape -- so it starts from random init". That premise is what
/// this function now disproves: a wider net CAN inherit them, function-preservingly.
///
/// `seed_net` is what makes an ARCH proposal winnable. Starting from random, a wider net's
/// held-out loss at birth is far worse than a champion with dozens of generations behind it,
/// so the FITNESS 5 surrogate filter rejected every widening before a single game was played
/// (three times in one run: "ARCH w 16 -> w 32 ... surrogate filter, no gate"). Capacity could
/// never increase, and the origin control sat flat at 0.831/0.808/0.825 across generations
/// 25/50/75 while that was true. Widened function-preservingly, the candidate starts at exactly
/// the champion's loss and is judged on what the extra capacity ADDS.
fn train_from(
    width: usize, seed_net: Option<&Net>, data: &[Sample], held: &[&Sample], tr: &Trainer,
    seed: u64, max_epochs: usize,
) -> (Net, f64, usize) {
    let mut net = match seed_net {
        Some(c) if width >= c.n_hidden => c.widen(width, seed),
        _ => Net::random(width, seed),
    };
    let (mut best_net, mut best_loss, mut best_ep) = (net.clone(), f64::INFINITY, 0usize);
    let mut stale = 0;
    for e in 0..max_epochs {
        tr.epoch(&mut net, data, seed ^ (e as u64) << 32);
        let hl = tr.loss(&net, held);
        if hl < best_loss {
            best_loss = hl;
            best_net = net.clone();
            best_ep = e + 1;
            stale = 0;
        } else {
            stale += 1;
            // Generic early-stopping patience, declared as methodology. Not a chess choice.
            if stale >= 3 { break; }
        }
    }
    (best_net, best_loss, best_ep)
}

/// Paired sign-agreement counts: positions the champion got right and the candidate got wrong,
/// and vice versa. McNemar's statistic is built from exactly these two.
fn paired_sign(champ: &Net, cand: &Net, held: &[&Sample]) -> (u32, u32) {
    let mut s = Vec::new();
    let (mut b_only, mut a_only) = (0u32, 0u32);
    let ok = |net: &Net, p: &Position, z: f32, s: &mut Vec<f32>| {
        let mover = net.eval(p, s) as f32;
        let white = if p.stm == Color::White { mover } else { -mover };
        (white > 0.0) == (z > 0.0)
    };
    for h in held {
        let p = match Position::from_fen(&h.fen) { Ok(p) => p, Err(_) => continue };
        match (ok(champ, &p, h.z, &mut s), ok(cand, &p, h.z, &mut s)) {
            (true, false) => b_only += 1,
            (false, true) => a_only += 1,
            _ => {}
        }
    }
    (b_only, a_only)
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let arg = |k: &str, d: usize| -> usize {
        a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(d)
    };
    let gens = arg("--gens", 5);
    let games = arg("--games", 60);
    let depth = arg("--depth", 2) as u32;
    let epochs = arg("--epochs", 3);
    let gate_pairs = arg("--gate-pairs", 40);
    // NET WIDTH IS NOT A FLAG ANY MORE. It is a position on a declared menu (arch::WIDTH_MENU)
    // and the ARCH arm moves it by measurement. `--rung` only says where to START; the loop is
    // free to walk away from it, and the run prints where it ended up.
    let start_rung = arg("--rung", 0).min(WIDTH_MENU.len() - 1);
    let arch_every = arg("--arch-every", 5);
    // ARCH gets its OWN pair count, and the number is derived rather than picked. At 32 pairs
    // the ARCH gates measured +/-0.096 to +/-0.119, against the <0.05 needed for `resolves`, so
    // every architecture decision fell through to the surrogate. A pentanomial interval scales
    // as 1/sqrt(n), so reaching 0.05 from 0.11 needs (0.11/0.05)^2 = 4.8x the pairs.
    // An ARCH step changes the champion's shape and is judged only every `arch_every`
    // generations, so it can afford the games a per-generation NET gate cannot.
    //
    // 160 -> 224, because that derivation aimed at EXACTLY the threshold and left no margin.
    // Every ARCH gate since has landed just the wrong side of it: measured ci95 across four
    // gates in two runs was 0.050, 0.051, 0.052, 0.053 -- so `resolves` was false EVERY time,
    // by 0.001 to 0.003, and the strict branch it guards has never once executed. A check that
    // structurally cannot fire is not a check.
    //
    // 224 = 160 x 1.4, putting the expected interval at 0.0515 / sqrt(1.4) = 0.0435. That is
    // below 0.05 with room for the run-to-run spread actually observed, rather than aimed at
    // the line again. Cost is 40% more games on a step taken once every `arch_every`
    // generations; the NET gate's per-generation budget is untouched.
    let arch_pairs = arg("--arch-pairs", 224);
    // FITNESS 6 declares the fixed cost budget as "the cost of ~20k seed-program evaluations on
    // the seed net". 20k at P1 scale makes a single gate take minutes, so the budget is smaller
    // here and RECORDED rather than silently different; the ratio, not the absolute, is what
    // makes the clock gate mean anything.
    // 0 = DERIVE IT from the measured full-tree size at the cap depth (the default). A fixed
    // constant is wrong here because tree size is NET-DEPENDENT: different seeds give different
    // random nets, which give different alpha-beta cutoffs. Measured, a 4000-node budget covered
    // 57% of one seed's depth-4 tree and 33% of another's, and the coverage guard correctly
    // aborted 4 of 6 arms of a 3-seed experiment. Deriving it makes the budget mean the same
    // thing for every net instead of silently meaning something different for each.
    let cost_nodes_arg = arg("--cost-nodes", 0) as u64;
    // Depth for budgeted gates is a CEILING, not the thing that stops the search — the node
    // budget is. A depth-limited gate cannot charge a wide net for being slow, which is the
    // whole point of the ARCH clock gate (FITNESS 10: "bigger net that wins fixed-cost-budget,
    // loses on clock").
    // MEASURED, not guessed. Full-tree cost from startpos: depth 3 = 1,921 nodes, depth 4 =
    // 3,145, depth 5 = 140,009, depth 6 = 328,495. This defaulted to 6, so a 4,000-node budget
    // bought 1.29% of the tree -- the search never finished its FIRST root move and simply
    // returned whichever shuffled move it had reached. Both sides did that, so every budgeted
    // gate was two random movers drawing 120 of 128 games, and its 0.500 was NO EVIDENCE rather
    // than a negative result. It made the origin control read "no learning" while the same nets
    // scored 0.816 +/- 0.038 at plain depth 3, and it made every ARCH decision surrogate-driven.
    // At depth 4 the budget covers the tree at startpos and binds in the midgame, which is where
    // a wide net SHOULD be charged for its cost per node.
    let gate_depth_cap = arg("--gate-depth-cap", 4) as u32;
    // Datagen worker threads. Defaults to 4 because background work on this box is pinned to
    // cores 12-15; taking more would take his.
    let threads = arg("--threads", 4);
    // Fixed gradient-step budget per generation; 0 keeps the old epochs-over-the-slice
    // behaviour so the two can be compared as a single variable.
    let steps_per_gen = arg("--steps-per-gen", 0);
    let seed = arg("--seed", 20260907) as u64;
    let ctrl_every = arg("--control-every", 10);
    let out = a.iter().position(|x| x == "--out").and_then(|i| a.get(i + 1)).cloned()
        .unwrap_or_else(|| "champion.net".to_string());
    // Resume from a saved champion instead of starting at iteration zero. See the use site.
    let init_net = a.iter().position(|x| x == "--init").and_then(|i| a.get(i + 1)).cloned();
    // Cap on the widening horizon. Measured 2026-09-07: labels far from the terminal are
    // ANTI-signal while play is weak (sign acc 0.452 -> 0.441 when training on all decided
    // positions). An unbounded schedule reaches 205 plies by gen 40, i.e. no filter at all,
    // which would reintroduce exactly that. This tests whether the plateau is self-inflicted.
    // MEASURED 2026-09-07, paired arms on the same seed. The default was 1000, i.e. the
    // safeguard was switched off, and the schedule 10+5*(g-1) reached h95 by generation 18.
    //   uncapped   control vs origin:  0.664 -> 0.648 -> 0.508 (FAILED at gen 15)
    //   h<=40      control vs origin:  0.552 -> 0.567 -> 0.570 -> 0.694 (all pass)
    // Uncapped, the per-generation gate collapsed to 0.352-0.430 once the horizon passed 60
    // plies -- eight straight generations of candidates WORSE than the champion, four of them
    // flagged `regression` in the ledger. Capped, it holds ~0.49 and the champion compounds.
    // SWEPT 2026-09-08 (examples/hyper_ab.rs --horizons 10,20,40,80,1000, 10 replicates per
    // arm on one shared dataset). Mean gate rate against the same champion:
    //     h10   5,052 samples   0.5527 [0.5470, 0.5585]
    //     h20   9,418 samples   0.5660 [0.5599, 0.5721]   <- optimum
    //     h40  17,266 samples   0.5371 [0.5289, 0.5454]
    //     h80  27,303 samples   0.5320 [0.5208, 0.5433]
    //     h1000 30,151 samples  0.5188 [0.5084, 0.5291]
    // Unimodal, peak at 20. 20 vs 40 is 0.0289 +/- 0.0102 at 95%, excluding zero, so the
    // previous default of 40 was measurably worse -- and it was MY pick, flagged "not tuned".
    // Note h20 beats h40 on 45% fewer samples: quality, not quantity.
    //
    // DECLARED LIMIT: measured against a RANDOM champion, i.e. early in a run. The schedule
    // widens the horizon with generation precisely because the label becomes informative
    // further back as play improves, so the optimum should MOVE. This sets the cap the early
    // generations run into; it is not a claim about a strong champion.
    //
    // CAVEAT ADDED 2026-09-08, and it applies to me as much as to the number: that comparison
    // was n=1 PER ARM. The step-budget experiment has since measured run-to-run variance on
    // this very control metric at 0.151 (epochs-3 scored 0.641 and 0.792 on two seeds at
    // IDENTICAL settings) -- larger than most effects being tested here. The within-run
    // evidence was stronger than a single endpoint (uncapped showed EIGHT consecutive
    // generations below 0.5 with four flagged `regression` in the ledger, which is a pattern),
    // but the cross-arm claim rests on one run each and I said so nowhere at the time.
    // SUPERSEDED AGAIN 2026-09-08, and this time the SCHEDULE is vindicated rather than the
    // cap. Both sweeps re-run at the shipped blend of 0.75 (they had been run at blend 0, where
    // the target ignores the search score and the answer is meaningless):
    //
    //   RANDOM champion   h10 0.5633  h20 0.5906  h40 0.5980  h1000 0.5801   -> peak near 40
    //   TRAINED champion  h10 0.5352  h20 0.5539  h40 0.5988  h1000 0.6242   -> no cap best
    //
    // The optimum MOVES OUTWARD as the champion improves, which is exactly what
    // `horizon = 10 + 5*(g-1)` does: it reaches 40 at generation 7 and 160 at generation 31.
    // The schedule was right; the CAP was strangling it. At the previous default of 20 the
    // schedule stopped ramping at generation 3.
    //
    // So the cap becomes a safety rail, not a tuning knob. 1000 is above the longest game
    // (h160 and h1000 select the SAME 58,734 positions, so nothing lies beyond 160 plies).
    //
    // PREVIOUS NOTE, kept because the correction matters more than the conclusion:
    // re-tested on a controlled fixed-data A/B (examples/hyper_ab.rs
    // --compare horizon, 10 replicates per arm). horizon 40 scored 0.5371 +/- 0.0042 against
    // uncapped 0.5188 +/- 0.0053; difference 0.0183 +/- 0.0133 at 95%, excluding zero. The
    // capped arm wins on 43% FEWER samples, so it is data QUALITY and not quantity. The
    // default now rests on evidence that survives the n>=3 standard, and the loop-based
    // re-run is unnecessary -- the loop has 25x worse resolution for this question.
    let horizon_cap = arg("--horizon-cap", 1000) as u32;
    // DEPTH SCHEDULE. Depth 1 gives ~47x the labels per second and bootstraps the net out of
    // randomness, but at depth 1 the search is barely stronger than the raw eval, so the data
    // stops being better than the net that made it and acceptance stalls (measured: accepted
    // at gens 2-6, then nothing for 24 generations). AlphaZero's engine of improvement is that
    // SEARCH(net) > net; that only holds once the net is worth searching over. So: bootstrap
    // shallow, then deepen.
    let deepen_at = arg("--deepen-at", 1_000_000);
    let deep = arg("--deep", 2) as u32;

    // blend = 0 at iteration zero. Mixing the net's OWN root score into its target is
    // self-referential when the net is random: it trains toward what it already says and
    // teaches nothing. The blend only earns its place once the search score is better than
    // the raw outcome, which is a later measurement, not an assumption.
    //
    // THAT MEASUREMENT HAS NOW BEEN MADE, and the premise had expired (2026-09-08). Against a
    // TRAINED champion, same data, horizon 10, 10 replicates per arm
    // (examples/hyper_ab.rs --blends):
    //     blend 0.00   0.4805 [0.4722, 0.4887]   significantly WORSE than the champion
    //     blend 0.25   0.4898 [0.4799, 0.4997]   worse
    //     blend 0.50   0.5188 [0.5114, 0.5261]   BETTER, excludes 0.5
    //     blend 0.75   0.5258 [0.5123, 0.5393]   BETTER, excludes 0.5
    // +0.0453 +/- 0.0158 from 0 to 0.75, monotone. Training on the outcome ALONE makes the
    // trained champion worse; adding its own search score makes it better.
    //
    // This is the acceptance stall, and it was never the horizon. Every horizon from 3 to 1000
    // measured below 0.5 because the LABEL was wrong, not because the wrong positions were
    // selected. MASTER_PLAN's Given column reads "Objectives: game outcome; agreement with own
    // deeper search" -- half the declared objective was switched off by a comment that was
    // correct at iteration zero and never revisited.
    //
    // REFINED, and it refuted my own prediction. I expected blend = 1.0 to be degenerate
    // ("pure self-reference"). It is not:
    //     blend 0.75   0.5258 [0.5123, 0.5393]
    //     blend 0.85   0.5234 [0.5071, 0.5398]
    //     blend 0.95   0.5293 [0.5137, 0.5449]
    //     blend 1.00   0.5281 [0.5112, 0.5450]
    // A flat plateau from 0.5 to 1.0, all statistically indistinguishable. The prediction was
    // wrong because `s.root` is the SEARCH's output, not the net's: training toward it is
    // DISTILLATION of the search into the net, which is AlphaZero's actual mechanism, not a
    // fixed point of the net's own opinion.
    //
    // Consequence worth stating plainly: once the search score is in the target, the game
    // outcome contributes nothing measurable. blend = 1.0 (no outcome at all) matches the best
    // mix.
    //
    // DEFAULT 0.75 rather than 1.0, and the reason is a risk the experiment cannot see. The
    // outcome is ground truth; the search score is a bootstrap off the current net. A pure
    // bootstrap has no anchor to reality and can drift across many generations, which a
    // single-step A/B is blind to by construction. 0.75 is measured-equal to 1.0 and keeps a
    // quarter of the target anchored. That is a judgement about a failure mode this
    // measurement cannot rule out, not a claim the measurement supports.
    //
    // LIMIT: one champion, one dataset, 10 replicates (se ~0.007). The effect is ~6.6 SE, but
    // it is a single champion -- re-measure when a stronger one exists.
    let blend: f32 = a.iter().position(|x| x == "--blend")
        .and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(0.75);
    let tr = Trainer::new(0.01, blend);
    let mut rung = start_rung;
    println!("gens={gens} games/gen={games} depth={depth} epochs={epochs} gate-pairs={gate_pairs} blend={blend}");
    println!("ARCH menu {WIDTH_MENU:?}  start rung {rung} (width {})  arch-every {arch_every}",
             WIDTH_MENU[rung]);
    // ORIGIN is always the reproducible iteration-zero net, even when we resume. The control
    // measures "how far has this got from nothing", and it is only comparable across runs if
    // every run scores against the SAME opponent. Seeding it from a resumed champion would
    // silently redefine the yardstick and make every control number a fresh scale.
    let origin = Net::random(WIDTH_MENU[rung], seed);
    // --init RESUMES from a saved champion. Without it, a run killed for any reason -- a
    // rebuild, a restart to pick up a fix, a machine reboot -- throws away everything it
    // learned and starts from random again. That cost a live 2400-games run 21 generations
    // today purely to adopt a corrected acceptance rule.
    //
    // The resumed net must match the rung's width, or the ARCH menu and the origin control are
    // describing a different architecture than the one playing. Refuse rather than silently
    // reshape: a mismatched resume is the kind of error that produces plausible numbers.
    let mut champion = match init_net {
        Some(ref path) => match Net::load(path) {
            // ADOPT the saved net's rung rather than demanding it match --rung.
            //
            // The first version aborted unless the width equalled WIDTH_MENU[--rung]. That was
            // fine only while ARCH could never widen: the moment a widening step is accepted the
            // champion is width 32, and every later resume of a rung-0 run would abort on its own
            // successful progress. The saved net's width IS the current rung -- that is what
            // "resume" means -- so infer it, and abort only if the width is not on the menu at
            // all (a net this ARCH arm could not have produced).
            Ok(n) => match arch::rung_of(n.n_hidden) {
                Some(r) => {
                    if r != rung {
                        println!("RESUMED at rung {r} (width {}), overriding --rung {rung}",
                                 n.n_hidden);
                    } else {
                        println!("RESUMED champion from {path} (width {})", n.n_hidden);
                    }
                    rung = r;
                    n
                }
                None => {
                    eprintln!("ABORT: {path} is width {}, which is not on the ARCH menu {WIDTH_MENU:?}",
                              n.n_hidden);
                    std::process::exit(2);
                }
            },
            Err(e) => { eprintln!("ABORT: could not load {path}: {e}"); std::process::exit(2); }
        },
        None => origin.clone(),
    };
    let cost_nodes = {
        let mut probe = pipeline::search::Searcher::with_seed(1);
        let mut p0 = Position::startpos();
        probe.best_move_capped(&mut p0, gate_depth_cap, &origin, u64::MAX, 1);
        let full = probe.nodes.max(1);
        // Cover the tree at startpos; midgame trees are larger, so the budget still binds
        // there -- which is where a wide net should be charged for its cost per node.
        let derived = if cost_nodes_arg > 0 { cost_nodes_arg } else { full };
        let cover = derived as f64 / full as f64;
        println!("gate budget: {derived} nodes ({} at depth {gate_depth_cap} from startpos = {:.0}% coverage){}",
                 full, cover * 100.0, if cost_nodes_arg > 0 { " [--cost-nodes override]" } else { " [derived]" });
        derived
    };
    // The clock budget is fixed in TIME, measured once on the seed net, so that every later
    // width is charged its real cost against the same wall. Deriving it per-champion would let
    // a slow champion move the wall and hide its own cost.
    let seed_ns = arch::ns_per_node(&origin, depth.max(3), 5);
    let budget_ns = seed_ns * cost_nodes as f64;
    println!("clock budget: {cost_nodes} x {seed_ns:.0}ns = {:.2}ms per move (measured on the seed net)",
             budget_ns / 1e6);

    // GUARD: a budget that cannot cover the tree at the cap depth turns every budgeted gate
    // into two random movers, and a random-vs-random gate reports 0.500 with a TIGHT interval
    // -- it looks like a confident dead heat and is actually no measurement at all. That is
    // strictly worse than not running, because it produces verdicts. Refuse to start.
    {
        let mut probe = pipeline::search::Searcher::with_seed(1);
        let mut p0 = Position::startpos();
        probe.best_move_capped(&mut p0, gate_depth_cap, &origin, u64::MAX, 1);
        let full = probe.nodes.max(1);
        let cover = cost_nodes as f64 / full as f64;
        println!("gate coverage: {cost_nodes} nodes vs {full} for a full depth-{gate_depth_cap} \
                  search from startpos = {:.0}%", cover * 100.0);
        if cover < 0.5 {
            eprintln!("\nABORT: the node budget covers only {:.1}% of a full depth-{gate_depth_cap} \
                       search.\nThe capped search would not finish its first root move, both sides \
                       would play near-randomly,\nand every gate would return a confident-looking \
                       0.500 that means nothing.\nLower --gate-depth-cap or raise --cost-nodes.",
                      cover * 100.0);
            std::process::exit(2);
        }
    }

    // Append-only, and NOT derived from `--out`: a ledger that a new run silently truncates is
    // not a record. The previous ARCH run's 6 rejections were lost exactly that way, to an
    // overwritten stdout log.
    let ledger = Ledger::new(
        &a.iter().position(|x| x == "--ledger").and_then(|i| a.get(i + 1)).cloned()
            .unwrap_or_else(|| "ledger.jsonl".to_string()),
    );

    let mut rng = Rng(seed);
    let mut accepted = 0;
    // ARCH attempts already spent, so a RESUMED run continues the sweep instead of restarting it.
    //
    // `--init` restores the champion but this counter is what drives the proposal STRIDE
    // (+1,-1,+2,-2,...), so a resumed run began again at stride 1 and re-walked the rungs it had
    // already rejected. Measured: width 32 has now been proposed four times across two runs and
    // lost on the clock twice (0.372 and 0.334) -- each repeat costs a full ARCH gate, and the
    // arm cannot reach an untried rung while it is re-deriving a known answer.
    //
    // Explicit flag rather than hidden state beside the champion file: the sweep position is an
    // experimental parameter, and a resumed run that silently starts somewhere unstated is how a
    // measurement stops being reproducible.
    let mut arch_attempts = arg("--arch-attempts-done", 0);
    let mut arch_accepted = 0usize;
    // Replay buffer. An ARCH candidate starts from random weights, so it needs more than one
    // generation of data to be a fair challenger to a champion that has had many.
    let mut replay: Vec<Sample> = Vec::new();
    let mut judge_pool: Vec<Sample> = Vec::new();
    let replay_gens = 8usize;
    let mut replay_marks: std::collections::VecDeque<usize> = std::collections::VecDeque::new();

    for g in 1..=gens {
        // ---- self-play with the current champion
        let dgen_depth = if g >= deepen_at { deep } else { depth };
        let t0 = std::time::Instant::now();
        let (data, dec) = datagen::play_games(
            &champion, dgen_depth, rng.next(), games, 4, 160, threads);
        let drawn = games - dec;
        let t_gen = t0.elapsed().as_secs_f64();

        // ---- train a candidate from the champion.
        // HORIZON: only positions within `horizon` plies of the terminal, and only from decided
        // games. Measured 2026-09-07: training on ALL decided positions makes the eval WORSE
        // (sign acc 0.452 -> 0.441) while <=10 plies makes it BETTER (-> 0.543) on 5x less
        // data. Far-from-terminal labels are anti-signal while both players are near-random.
        // The horizon WIDENS with generation, because the label becomes informative further
        // back as play improves.
        let horizon = (10 + (g as u32 - 1) * 5).min(horizon_cap);
        let pool: Vec<Sample> = data.iter()
            .filter(|s| s.z != 0.0 && s.plies_to_end <= horizon)
            .map(|s| Sample { fen: s.fen.clone(), z: s.z, root: s.root, plies_to_end: s.plies_to_end })
            .collect();
        // TRUE hold-out: split BEFORE training and never train on the held part. The first
        // version evaluated McNemar on the tail of the same list it trained on, which measures
        // training-set fit and accepted 7 of 10 candidates whose champion then scored 0.500
        // against the original random net. The control caught it.
        let cut = pool.len() * 3 / 4;
        let (subset, heldout) = pool.split_at(cut);

        // Replay buffer for the ARCH arm, which trains from scratch and needs more than one
        // generation of data to be a fair challenger.
        //
        // ONLY the trained-on part goes in. The first version pushed the whole pool and then
        // took the ARCH held-out set as the last 25% of the buffer -- i.e. the most RECENT
        // generation. That trained the candidate on old positions and scored it on new ones,
        // while the champion had trained on those new ones, and the candidate lost on held-out
        // loss (1.2372 vs 0.5049) for reasons that had nothing to do with its width. Comparing
        // on data one side has seen and the other has not measures memory, not architecture.
        replay.extend(subset.iter().cloned());
        replay_marks.push_back(subset.len());
        while replay_marks.len() > replay_gens {
            let drop_n = replay_marks.pop_front().unwrap();
            replay.drain(0..drop_n.min(replay.len()));
        }
        // JUDGING RESERVOIR: every generation's held-out slice, accumulated and never given to
        // any trainer. One generation's slice is 11-89 positions after the horizon filter --
        // far under the n>=30-per-half the paired test needs -- so a per-generation set would
        // silently disable the ARCH surrogate rather than inform it.
        judge_pool.extend(heldout.iter().cloned());
        if judge_pool.len() > 6000 {
            let excess = judge_pool.len() - 6000;
            judge_pool.drain(0..excess);
        }
        let mut cand = champion.clone();
        let mut loss = 0.0;
        if steps_per_gen > 0 {
            // FIXED STEP BUDGET, drawn from the REPLAY BUFFER rather than this generation's
            // slice: the number of gradient steps stops being a side effect of how many games
            // datagen happened to play, and more data becomes a broader draw instead of a
            // longer one. `replay` already excludes every held-out position.
            let pool: &[Sample] = if replay.len() >= subset.len() { &replay } else { subset };
            loss = tr.steps(&mut cand, pool, steps_per_gen, seed ^ (g as u64) << 8);
        } else {
            for e in 0..epochs {
                loss = tr.epoch(&mut cand, subset, seed ^ (g as u64) << 8 ^ e as u64);
            }
        }

        // ---- ACCEPTANCE.
        // At iteration zero the game-gate is BLIND: two wandering nets draw 86-100% of their
        // games, so an 80-game match carries +/-0.11 and rejects everything regardless of
        // merit. During bootstrap the held-out surrogate DECIDES and the gate serves as a
        // non-regression guard; the gate takes back over once play is decisive enough to
        // resolve (tracked by the draw rate, reported every generation).
        let held: Vec<&Sample> = heldout.iter().collect();
        let (b_only, a_only) = paired_sign(&champion, &cand, &held);
        let mcnemar = if a_only + b_only > 0 {
            (a_only as f64 - b_only as f64) / ((a_only + b_only) as f64).sqrt()
        } else { 0.0 };
        // SEQUENTIAL. Same reasoning as the search track: a fixed pair count is simultaneously
        // too few for a modest real gain to clear the interval and too many for a candidate
        // that is losing every game. FITNESS 7.2 makes the count the EVIDENCE's decision.
        // gate_pairs is now a CAP, not a target, and most candidates stop far short of it.
        let (verdict, sc, llr) = gate::sprt_match_nets(
            &cand, &champion, depth, gate_pairs, seed ^ g as u64, 4, 0.0, 5.0);
        let _ = verdict;
        let _draw_rate = sc.draws as f64 / sc.games().max(1) as f64;
        // Resolution is a property of the INTERVAL, not the draw rate. Draw rate was a proxy
        // for width under the binomial; with pentanomial the same games give a ~5x tighter
        // interval, so a 90%-draw match can still decide. Test the thing directly.
        // "Can the gate decide?" is a question about the SIGN, not about absolute width.
        //
        // This read `ci95() < 0.05` and therefore almost never fired: MEASURED over
        // ledger_newdefaults.jsonl, actual ci95 at 24-64 pairs is 0.048-0.095, so it was true
        // ONCE IN TEN GENERATIONS (generation 1, at 0.048). The comment below promised the gate
        // "takes back over once play is decisive enough to resolve"; in practice the held-out
        // surrogate decided permanently, and a proxy for strength was overwriting the champion
        // on every generation where the games themselves had an opinion.
        //
        // 0.05 is an arbitrary width. What the loop actually needs to know is whether the
        // interval lies clear of 0.5 -- that is exactly "the games decided", and it is
        // reachable: a 0.573 +/- 0.069 gate resolves upward while failing ci95 < 0.05.
        let resolved_up = sc.pent_rate() - sc.ci95() > 0.5;
        let resolved_down = sc.pent_rate() + sc.ci95() < 0.5;
        let gate_can_resolve = resolved_up || resolved_down;
        // NON-REGRESSION GUARD. This must be able to REFUSE, or it is decoration.
        //
        // It used to read `pent_rate() + ci95() > 0.5`, which passes anything above
        // 0.5 - ci95. MEASURED on ledger_newdefaults.jsonl: at generation 8 that bar was 0.407,
        // and a candidate that went 15W-32D-17L -- rate 0.484, a LOSING record -- was promoted
        // over the champion on the surrogate's word (mcnemar 3.55) while the gate's own point
        // estimate said it was worse. Adding the interval to the candidate's score is the wrong
        // direction: it converts uncertainty into permission.
        //
        // The guard is not being asked "is the candidate proven worse?" -- with a wide interval
        // nothing is ever proven, so that question always answers no. It is being asked "does
        // the gate CONTRADICT the surrogate?", and a point estimate below 0.5 contradicts it.
        // Keeping the champion costs one generation; promoting a worse net costs every
        // generation after it, because the next candidate is trained against the damage.
        let no_regression = sc.pent_rate() >= 0.5;
        let better = if verdict == gate::Sprt::Accept {
            true
        } else if verdict == gate::Sprt::Reject {
            false
        } else if gate_can_resolve {
            // The games resolved the sign inside the budget. They outrank the surrogate, which
            // is only a proxy for what this match measures directly. This also REJECTS on
            // resolved_down rather than falling through, which is the half that was missing:
            // a candidate the gate had resolved as WORSE could still be promoted by mcnemar.
            resolved_up
        } else if sc.ci95() < 0.05 {
            // PRECISELY MEASURED, AND IT STRADDLES 0.5 -> not better. Reject.
            //
            // This branch is why the sign test alone is not enough, and leaving it out made the
            // loop MORE permissive, not less. A narrow interval that contains 0.5 -- 0.510 +/-
            // 0.020 -- is not ignorance, it is a precise measurement of "no meaningful
            // difference". Treating it as "undecided" hands it to the surrogate, which can then
            // promote on mcnemar alone.
            //
            // MEASURED over the three shape arms (197 generations of ledger): the sign test
            // resolves 21 of 140 generations in the 150-games arm where `ci95 < 0.05` resolves
            // 100, because near-all-draw matches give a tiny pentanomial interval. Dropping the
            // width test would have sent 79 precisely-measured null results to the surrogate.
            false
        } else {
            // Genuinely undecided: the interval is BOTH wide and straddling. Only here may the
            // surrogate speak, and only where the gate does not contradict it.
            mcnemar > 1.96 && no_regression
        };
        // LEDGER: record the decision, accepted or not, with a NAMED reason. A rejection is the
        // more reusable fact — it says do not spend this compute again — and "reject" alone is
        // not a finding.
        let net_reason = if better {
            Reason::Accepted
        } else if gate_can_resolve {
            Reason::LostOnGames
        } else if !no_regression {
            Reason::Regression
        } else {
            Reason::NoEvidence
        };
        ledger.record(&Entry {
            generation: g,
            class: "NET",
            what: format!("train {} epochs on {} samples, horizon {}, width {}",
                          epochs, subset.len(), horizon, champion.n_hidden),
            reason: net_reason,
            gates: vec![GateEvidence {
                name: "fixed-depth",
                pent: sc.pent,
                rate: sc.pent_rate(),
                ci95: sc.ci95(),
                resolved: gate_can_resolve,
            }],
            surrogate: vec![("mcnemar_z", mcnemar), ("train_loss", loss as f64), ("llr", llr)],
        });

        if better {
            champion = cand;
            accepted += 1;
            // Persist on every acceptance, not at the end: a run killed by a timeout used to
            // discard everything it had learned.
            if let Err(e) = champion.save(&out) {
                eprintln!("  WARN could not save champion to {out}: {e}");
            }
        }

        // ---- ARCH arm. A STEP ALONG THE DECLARED MENU, decided by measurement.
        //
        // This is the class FITNESS 1 lists as "architecture-menu step", and it exists because
        // MASTER_PLAN puts the menu in Given ("declared option sets, NOT choices") and width
        // itself in Learned ("width, depth, activation ... discovered or not"). Before this the
        // loop took --hidden from the command line and the author moved it after eyeballing a
        // sweep, which is exactly the choice the plan says must not be his.
        //
        // Two gates, in the order FITNESS 1 prescribes, because they answer different questions
        // and a widening step can pass one and fail the other:
        //   fixed-cost-budget (FITNESS 6) -- equal NODES. "Better per unit of search?"
        //   clock             (FITNESS 7) -- equal TIME.  "Better per unit of wall-clock?"
        // A wider net that only passes the first is the degenerate solution FITNESS 10 names
        // outright ("bigger net that wins fixed-cost-budget, loses on clock"), so BOTH are
        // required to move the rung, and both results are printed either way.
        // `held.len() >= 60` so both halves clear the n>=30 floor in paired_loss_z; below that
        // the surrogate returns 0.0 and the arm would be deciding on nothing.
        if arch_every > 0 && g % arch_every == 0 && replay.len() >= 200 && judge_pool.len() >= 120 {
            if let Some(p) = arch::propose(rung, arch_attempts) {
                arch_attempts += 1;
                // The comparison set is THIS generation's held-out slice: it is in neither the
                // champion's training history nor the replay buffer, so both arms meet it for
                // the first time. Split in two so the candidate early-stops on one half and is
                // JUDGED on the other -- early-stopping on the judging set would hand the
                // challenger a free peek that the incumbent never gets.
                // Split the reservoir by INDEX PARITY, not by a cut point: the pool is ordered
                // by generation, so a front/back cut would put early, weaker-play positions in
                // one half and late ones in the other and the two halves would not be samples
                // of the same thing. Parity interleaves them.
                let astop: Vec<&Sample> = judge_pool.iter().step_by(2).collect();
                let ajudge: Vec<&Sample> = judge_pool.iter().skip(1).step_by(2).collect();
                let (acand, _stop_loss, aeps) =
                    train_from(p.width(), Some(&champion), &replay, &astop, &tr,
                               seed ^ 0xA5 ^ g as u64, 30);
                let acand_loss = tr.loss(&acand, &ajudge);
                let champ_loss = tr.loss(&champion, &ajudge);
                let rheld_ref = ajudge;
                // FITNESS 5 filter: "must not be worse than the champion by more than 0.5%".
                // Better loss does NOT accept; it only buys the right to spend gate time.
                if acand_loss > champ_loss * 1.005 {
                    println!("      ARCH w{:>3} -> w{:>3}: held-out {:.4} vs champ {:.4} -- surrogate filter, no gate",
                             WIDTH_MENU[p.from_rung], p.width(), acand_loss, champ_loss);
                    ledger.record(&Entry {
                        generation: g,
                        class: "ARCH",
                        what: format!("width {} -> {} (rung {} -> {})",
                                      WIDTH_MENU[p.from_rung], p.width(), p.from_rung, p.to_rung),
                        reason: Reason::SurrogateFilter,
                        gates: vec![], // never reached one
                        surrogate: vec![("heldout_loss", acand_loss),
                                        ("champion_loss", champ_loss),
                                        ("epochs", aeps as f64)],
                    });
                } else {
                    let fixed = gate::match_nets_capped(
                        &acand, &champion, gate_depth_cap, cost_nodes, cost_nodes,
                        arch_pairs, seed ^ 0xB6 ^ g as u64, 4);
                    let (ca, cb) = arch::equal_time_caps(&acand, &champion, budget_ns, depth.max(3));
                    let clock = gate::match_nets_capped(
                        &acand, &champion, gate_depth_cap, ca, cb,
                        arch_pairs, seed ^ 0xC7 ^ g as u64, 4);
                    // Can these gates resolve anything? At iteration zero two wandering nets
                    // draw every game, and a match of 24 draws carries no information whatever
                    // its point estimate says. When neither gate can resolve, the decision falls
                    // to the FITNESS 5 surrogate exactly as the NET arm's does, and the step
                    // additionally has to be NON-REGRESSIVE on both gates. This is a declared
                    // bootstrap rule and it retires by itself: as soon as play is decisive
                    // enough for the interval to close, `resolves` goes true and the gates
                    // decide alone.
                    let resolves = fixed.ci95() < 0.05 && clock.ci95() < 0.05;
                    let z = tr.paired_loss_z(&champion, &acand, &rheld_ref);
                    let (fixed_win, clock_win) = if resolves {
                        (fixed.pent_rate() - fixed.ci95() > 0.5, clock.pent_rate() - clock.ci95() > 0.5)
                    } else {
                        (fixed.pent_rate() + fixed.ci95() > 0.5, clock.pent_rate() + clock.ci95() > 0.5)
                    };
                    let surrogate_ok = resolves || z > 1.96;
                    let stepped = fixed_win && clock_win && surrogate_ok;
                    // The reason has to distinguish the two asymmetric failures. Winning on
                    // equal nodes and losing on the clock is FITNESS 10's named degenerate case
                    // -- an eval too expensive for what it knows -- and is a completely
                    // different fact from the reverse, which says the win was speed rather than
                    // eval quality. Collapsing both to "reject" throws away the diagnosis.
                    let arch_reason = if stepped {
                        Reason::Accepted
                    } else if fixed_win && !clock_win {
                        Reason::LostOnClock
                    } else if !fixed_win && clock_win {
                        Reason::LostOnCost
                    } else if !surrogate_ok {
                        Reason::NoEvidence
                    } else {
                        Reason::LostOnGames
                    };
                    ledger.record(&Entry {
                        generation: g,
                        class: "ARCH",
                        what: format!("width {} -> {} (rung {} -> {})",
                                      WIDTH_MENU[p.from_rung], p.width(), p.from_rung, p.to_rung),
                        reason: arch_reason,
                        gates: vec![
                            GateEvidence { name: "fixed-cost", pent: fixed.pent,
                                           rate: fixed.pent_rate(), ci95: fixed.ci95(),
                                           resolved: fixed.ci95() < 0.05 },
                            GateEvidence { name: "clock", pent: clock.pent,
                                           rate: clock.pent_rate(), ci95: clock.ci95(),
                                           resolved: clock.ci95() < 0.05 },
                        ],
                        surrogate: vec![("paired_z", z), ("heldout_loss", acand_loss),
                                        ("champion_loss", champ_loss), ("epochs", aeps as f64),
                                        ("cand_nodes", ca as f64), ("champ_nodes", cb as f64)],
                    });
                    println!("      ARCH w{:>3} -> w{:>3} ({} ep, loss {:.4} vs {:.4}, paired z {:.2})  fixed-cost {:.3}+/-{:.3}{}  clock {:.3}+/-{:.3} [{ca} vs {cb} nodes]{}  {}=> {}",
                             WIDTH_MENU[p.from_rung], p.width(), aeps, acand_loss, champ_loss, z,
                             fixed.pent_rate(), fixed.ci95(), if fixed_win { " ok" } else { "" },
                             clock.pent_rate(), clock.ci95(), if clock_win { " ok" } else { "" },
                             if resolves { "" } else { "[gates blind, surrogate decides] " },
                             if stepped { "STEP" } else { "hold" });
                    if stepped {
                        champion = acand;
                        rung = p.to_rung;
                        accepted += 1;
                        arch_accepted += 1;
                        if let Err(e) = champion.save(&out) {
                            eprintln!("  WARN could not save champion to {out}: {e}");
                        }
                    }
                }
            }
        }

        // Periodic control against the FROZEN origin. One step of learning is not a curve:
        // the question P1 turns on is whether strength COMPOUNDS or stops after generation 1.
        // Measured against the same fixed opponent every time, so the numbers are comparable.
        // `> 0` is not decoration: `--control-every 0` reads naturally as "never run the control"
        // and instead panicked with "attempt to calculate the remainder with a divisor of zero",
        // aborting the whole run on SIGABRT. `arch_every` was already guarded this way at the
        // ARCH step; this one was not, so the two flags disagreed about what 0 meant.
        if ctrl_every > 0 && g % ctrl_every == 0 {
            // EQUAL TIME, not equal depth: once the ARCH arm can change the champion's width,
            // a depth-matched control would hand a wider champion free computation and report
            // its extra cost as strength.
            let (ca, cb) = arch::equal_time_caps(&champion, &origin, budget_ns, depth.max(3));
            let c = gate::match_nets_capped(&champion, &origin, gate_depth_cap, ca, cb,
                                            arch_pairs, seed ^ 0xC0 ^ g as u64, 4);
            println!("      control vs origin @gen {g}: {}W-{}D-{}L  rate {:.3} +/- {:.3}{}",
                c.wins, c.draws, c.losses, c.pent_rate(), c.ci95(),
                if c.rate() - c.ci95() > 0.5 { "  *" } else { "" });
        }
        println!(
            "gen {g:>3}  pos {:>6}  train {:>5} (h{:>3})  dec {:>3}/{:<3}  loss {:.4}  gate {}W-{}D-{}L {:.3}+/-{:.3}  {}  [{:.0}s]",
            data.len(), subset.len(), horizon, dec, dec + drawn, loss, sc.wins, sc.draws, sc.losses, sc.pent_rate(), sc.ci95(),
            if better { "ACCEPT" } else { "reject" }, t_gen
        );
    }

    // ---- the control that proves learning happened at all
    println!("\nARCH: started at width {} (rung {start_rung}), finished at width {} (rung {rung}); \
              {arch_accepted} of {arch_attempts} proposed steps passed both gates",
             WIDTH_MENU[start_rung], WIDTH_MENU[rung]);

    if accepted > 0 {
        let (ca, cb) = arch::equal_time_caps(&champion, &origin, budget_ns, depth.max(3));
        let sc = gate::match_nets_capped(&champion, &origin, gate_depth_cap, ca, cb,
                                         gate_pairs * 2, seed ^ 0xFFFF, 4);
        println!(
            "\nCONTROL  final champion vs the ORIGINAL random net: {}W-{}D-{}L  rate {:.3} +/- {:.3}",
            sc.wins, sc.draws, sc.losses, sc.pent_rate(), sc.ci95()
        );
        println!(
            "  => {}",
            if sc.pent_rate() - sc.ci95() > 0.5 {
                "LEARNED: beats its own random initialisation with the interval clear of 0.5"
            } else {
                "NOT PROVEN: interval includes 0.5, so no learning is demonstrated"
            }
        );
    } else {
        println!("\nno candidate was accepted; nothing to control against");
    }
}
