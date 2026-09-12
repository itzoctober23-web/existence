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
///
/// The `h.z == 0.0` skip below is a DEFENSIVE GUARD, not a fix, and I claimed otherwise.
///
/// RETRACTED 2026-09-08. I claimed this was the root cause of the loop not learning: `z` is 0 for
/// a draw, `(white > 0.0) == (z > 0.0)` demands "Black better" on every draw, and 50.3% of the
/// 276,000 self-play games today are drawn -- so half the deciding evidence looked like a coin
/// flip graded against an invented answer.
///
/// The mechanism is real and the draw rate is real. The claim is still WRONG, because the held-out
/// set never contained a draw. `pool` is built with `.filter(|s| s.z != 0.0 && ...)` and `heldout`
/// is a split of `pool`, so draws were excluded upstream before this function ever saw them. The
/// guard is a no-op. I verified the formula in isolation and never checked what data reaches it.
///
/// The real dissociation is elsewhere and is now the live hypothesis: `Trainer::loss` scores MSE
/// against `target = (1 - blend) * z + blend * root` with the SHIPPED BLEND OF 0.75, so the
/// training objective is three-quarters the net's own root search score. This function scores
/// agreement with `z` ALONE. Held-out loss falling while paired sign-agreement falls is then two
/// metrics tracking two different targets, which is not a contradiction at all -- and not
/// overfitting either.
///
/// The guard stays because a sign comparison against a zero target is meaningless if a draw ever
/// does reach here, but it buys nothing today and must not be counted as a fix.
fn paired_sign(champ: &Net, cand: &Net, held: &[&Sample]) -> (u32, u32) {
    let mut s = Vec::new();
    let (mut b_only, mut a_only) = (0u32, 0u32);
    let ok = |net: &Net, p: &Position, z: f32, s: &mut Vec<f32>| {
        let mover = net.eval(p, s) as f32;
        let white = if p.stm == Color::White { mover } else { -mover };
        (white > 0.0) == (z > 0.0)
    };
    for h in held {
        // A drawn game gives no sign to agree with.
        if h.z == 0.0 { continue; }
        let p = match Position::from_fen(&h.fen) { Ok(p) => p, Err(_) => continue };
        match (ok(champ, &p, h.z, &mut s), ok(cand, &p, h.z, &mut s)) {
            (true, false) => b_only += 1,
            (false, true) => a_only += 1,
            _ => {}
        }
    }
    (b_only, a_only)
}

/// Plies of RECORDED play per datagen game (the random opening is unrecorded and separate).
///
/// Named because it is the real ceiling on `Sample::plies_to_end`, and therefore the point at which
/// the training horizon stops filtering anything. It was an unlabelled `160` at the call site while
/// `--horizon-cap` defaulted to 1000, so nothing connected the knob to the bound it could not cross.
const MAX_PLIES: usize = 160;

/// Path for a ladder rung, and the guard that keeps a rung meaning ONE net.
///
/// `learn`'s generation counter RESTARTS AT 1 on every launch, and this loop is relaunched often --
/// five times on 2026-09-10 alone. `{out}.gen{g}.net` therefore names a point in a run, not a point
/// in the lineage, so run 7's gen100 would silently OVERWRITE run 6's gen100.
///
/// That is not a cosmetic clash. The ancestor control loads `{out}.gen{g-lag}.net` and plays the
/// champion against it; if that file had been replaced by a different run's net, the control would
/// report a strength difference between two lineages while labelling it as progress within one.
/// Exactly the failure that made a flat origin control read as a collapse today
/// (`pooled_runs_RESULT.md`) -- a key that omits the run.
///
/// `--run-tag` puts the run back in the key. Without one, the guard below REFUSES to overwrite an
/// existing rung: losing the new snapshot is recoverable, destroying an ancestor is not.
fn rung_path(out: &str, tag: &str, g: usize) -> String {
    if tag.is_empty() { format!("{out}.gen{g}.net") } else { format!("{out}.{tag}.gen{g}.net") }
}

/// SCHEMAS 8 `where_it_mattered`: the position where this candidate most changed the evaluation.
///
/// Cheap by construction -- it reuses the training subset already in memory and caps the scan, so a
/// per-generation ledger field costs a bounded eval pass rather than a new search. It is the honest
/// answer to "where did this actually matter", which a rate and a p-value cannot give.
fn top_disagreement(a: &Net, b: &Net, samples: &[Sample], cap: usize) -> Option<String> {
    let mut best: Option<(i32, String)> = None;
    let mut scratch: Vec<f32> = Vec::new();
    for sm in samples.iter().take(cap) {
        let Ok(pos) = Position::from_fen(&sm.fen) else { continue };
        let d = (a.eval(&pos, &mut scratch) - b.eval(&pos, &mut scratch)).abs();
        if best.as_ref().map_or(true, |(bd, _)| d > *bd) { best = Some((d, sm.fen.clone())); }
    }
    best.map(|(_, f)| f)
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let arg = |k: &str, d: usize| -> usize {
        a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(d)
    };
    let gens = arg("--gens", 5);
    let games = arg("--games", 60);
    // DEFAULT 1, NOT 2 (changed 2026-09-10). MASTER_PLAN:617 concludes "Until then depth 1 is the
    // correct datagen setting" and this default said 2, so the code contradicted the project's own
    // stated conclusion for every run that did not pass --depth explicitly.
    //
    // The evidence is a THROUGHPUT argument, and it is worth stating precisely because the
    // head-to-head is not a win: depth2x2_RESULT measures d1 vs d2 at equal generations as a
    // precise NULL (0.512 +/- 0.014). What separates them is label yield -- d1 produces 72.8
    // decisive games per 300 against d2's 31.2, and costs less per generation -- so at equal wall
    // clock d1 gets more generations AND more usable labels. MASTER_PLAN measures the same thing
    // from the other side: ~47x fewer usable labels/second at depth 2.
    //
    // depth2x2_RESULT also ranks d2 LAST of the four arms it tested, and had been asserting that
    // the shipped depth was already 1 -- it was not, which is how a default nobody chose survived.
    let depth = arg("--depth", 1) as u32;
    // DATAGEN AT A NODE BUDGET instead of a fixed depth. 0 = off, which reproduces every prior run.
    //
    // "Thousands of nodes per move" is the unit real from-zero pipelines generate in, and it is
    // position-independent in a way a depth is not: depth 3 costs ~10,300 nodes on a midgame
    // position and ~2,350 from the start position, so a fixed depth spends wildly different amounts
    // of search on different positions while a node budget does not.
    //
    // The DEPTH is still derived from the budget rather than raised out of the way, for the reason
    // measured in `speed_cannot_pay_RESULT.md`: without iterative deepening, a cap that fires inside
    // the first root child leaves NO completed move and the score is -INF. `datagen.rs` guards that
    // case, but a guard that fires on most moves would silently turn a deep run into a depth-1 run.
    // SCHEMAS 8 `earned.e1_at_acceptance`. Only meaningful when a bound is in force; the
    // fixed-depth gate in this binary has none, so it is None unless the SPRT bounds are set.
    let gate_e1: Option<f64> = std::env::var("EXISTENCE_GATE_ELO1").ok().and_then(|v| v.parse().ok());
    // Default for decision sites with no position set in hand; shadowed where one exists.
    let top_fen: Option<String> = None;
    let dg_nodes = arg("--datagen-nodes", 0) as u64;
    let depth = if dg_nodes > 0 {
        pipeline::datagen::NODE_CAP.store(dg_nodes, std::sync::atomic::Ordering::Relaxed);
        // Measured single-position costs, midgame (the expensive case, so this never overspends):
        // d2 352, d3 10,309, d4 72,977, d5 1,234,802, d6 12,696,968.
        let d = [(3u32, 10_309u64), (4, 72_977), (5, 1_234_802), (6, 12_696_968)]
            .iter().filter(|(_, n)| *n <= dg_nodes).map(|(d, _)| *d).max().unwrap_or(2);
        eprintln!("  datagen node budget {dg_nodes}/move -> depth {d} (cap is the safety net)");
        d
    } else { depth };
    // TRUE node budget (structural_next_PREREG.md Candidate A). Separate flag from
    // `--datagen-nodes`, which selects a fixed depth and then truncates -- see datagen::BUDGET for
    // why that one could not be repurposed. Default 0 leaves every existing run byte-identical.
    let dg_budget = arg("--datagen-budget", 0) as u64;
    if dg_budget > 0 {
        let md = arg("--datagen-budget-max-depth", 8) as u64;
        pipeline::datagen::BUDGET.store(dg_budget, std::sync::atomic::Ordering::Relaxed);
        pipeline::datagen::BUDGET_MAX_DEPTH.store(md, std::sync::atomic::Ordering::Relaxed);
        // Cell C: budget LABELS on the fixed-depth arm's POSITIONS. Decomposes the two channels
        // that arm B changes together; see datagen::BUDGET_LABELS_ONLY.
        let lo = arg("--datagen-budget-labels-only", 0) as u64;
        pipeline::datagen::BUDGET_LABELS_ONLY.store(lo, std::sync::atomic::Ordering::Relaxed);
        eprintln!(
            "  datagen TRUE node budget {dg_budget}/move, iterative deepening, max depth {md}{} \
             (measured depth-3 mean is 5269/move -- datagen_node_census_RESULT.md)",
            if lo != 0 { "  [LABELS ONLY: moves come from fixed depth, cell C]" } else { "" }
        );
    }
    let epochs = arg("--epochs", 3);
    // GATE RESOLUTION. 40 -> 224, because the gate could not see the improvements it exists to
    // detect. MEASURED over 230 generations of ledger: median ci95 0.079, so it only resolves a
    // candidate better than 0.579 -- **+56 Elo**. Self-play gains do not arrive in +56 Elo steps,
    // so the loop was rejecting real small improvements as noise and accepting noise that
    // happened to look large (every accept observed sits at 0.52-0.66, exactly a random
    // fluctuation at that interval).
    //
    // The cost is trivial and that is why it went unnoticed: the gate is 2.7% of a generation
    // (64 games against 2400 for datagen). 224 pairs resolves ~+21 Elo for 17% overhead.
    //
    // The ARCH arm already learned this in miniature -- 32 -> 160 -> 224 for the same reason --
    // and the per-generation gate was simply never revisited. SPRT stops early when the evidence
    // is clear, so this is a CAP: a decisive candidate still costs far fewer than 224 pairs.
    let gate_pairs = arg("--gate-pairs", 224);
    // GATE MATCH DEPTH. Defaults to the DATAGEN depth, which is what it has always silently been,
    // so this flag changes nothing until it is set.
    //
    // WHY IT EXISTS. The batch gate scored with `match_nets(..., depth, ...)` -- the same `depth`
    // the datagen uses. So the loop learns from depth-2 labels AND selects on depth-2 matches,
    // while this project judges strength at depth 4 (gate_depth_cap, and the gate budget is sized
    // as "100% coverage of a full depth-4 search"). It optimises the game it measures.
    //
    // MEASURED, both of today's candidates: b2_5 beats champion_long by +0.029 at depth 2
    // (resolved) and +0.002 at depth 4 (unresolved); bh_100 scores 0.864 against the origin at
    // depth 2 and 0.832 at depth 4. Gains in the measured game, not in the judged one.
    //
    // Aligning the GATE costs gate_pairs matches at a deeper depth; aligning the DATAGEN costs
    // ~30 min per generation (pd_d4 was killed over exactly that). Same alignment, far cheaper.
    let gate_match_depth = arg("--gate-match-depth", depth as usize) as u32;
    // ANCHOR GATE. A candidate must also not REGRESS against a fixed opponent.
    //
    // MEASURED 2026-09-08, and this is the root cause of the loop not learning rather than a
    // refinement: ep_1 beats champion_long head to head at 0.545 +/- 0.018 (excludes 0.5) while
    // scoring 0.834 against the frozen origin where champion_long scores 0.864 (a 0.030 gap
    // against a 0.018 floor). Both directions resolved. The accept rule is "does the candidate
    // beat the current champion", and that quantity demonstrably moves OPPOSITE to strength
    // against a fixed opponent, so the loop can run perfectly and still walk downhill.
    //
    // Across eight runs the mean gate rate was >= 0.5 in ALL EIGHT -- candidates really do beat
    // their parents -- and not one final net was above the 0.864 they started from, three
    // resolved worse. Intransitivity is a known property of a self-play objective; the standard
    // response is to score against a FIXED reference or a pool rather than only the champion.
    //
    // 0 = off, preserving today's behaviour exactly so this can be A/B'd rather than assumed.
    let anchor_pairs = arg("--anchor-pairs", 0);
    // K generations of unconditional training per gate. 1 = today's per-generation gate, exactly.
    // See the long note at the gate call for why 1 cannot work: the gate's resolution floor is
    // coarser than the per-generation signal by 2.7-6.4x, measured.
    let gate_every = arg("--gate-every", 1);
    // Node budget per move for the NET gate. 0 = fixed depth (today's behaviour); >0 = the
    // fixed-cost-budget gate of FITNESS 6, which the loop has never actually used even though
    // match_nets_capped was implemented for it and ARCH already calls it.
    let gate_nodes = arg("--gate-nodes", 0) as u64;
    // --rollback: revert the champion to the best control checkpoint when the periodic origin
    // control resolves BELOW it. Off by default so it is A/B-able rather than silently changing
    // what every prior run measured.
    let rollback = std::env::args().any(|a| a == "--rollback");
    // The surrogate may override the games only if asked for explicitly. See the acceptance
    // chain: measured at corr -0.095 against 239 paired gate results, it is not a decision rule.
    let surrogate_fallback = std::env::args().any(|a| a == "--surrogate-fallback");
    // Keep drawn games in the training pool. See the filter for the measurement that motivates it.
    let include_draws = std::env::args().any(|a| a == "--include-draws");
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
    // CONTROL PAIRS, separate from arch_pairs. The origin control is THE metric for whether the
    // loop is working at all, and it was sharing the ARCH gate's pair count -- which gave it a
    // detection floor of ~70 Elo between successive readings (ci95 0.038, so two readings differ
    // only if the gap exceeds 1.41 x 0.038 = 0.054).
    //
    // That is why "the plateau" was overstated. Seven readings across two runs -- 0.780, 0.805,
    // 0.838, 0.853, 0.831, 0.823, 0.856 -- have EVERY pairwise gap below 0.054, so they are
    // consistent with no change AND with the champion gaining up to ~70 Elo unseen. "Flat" was
    // the wrong word for a metric that cannot see anything smaller.
    //
    // The control is a ONE-OFF match every `ctrl_every` generations, not per-generation work, so
    // resolution here is cheap:
    //    224 pairs -> ~70 Elo floor, 0.7% overhead   (the old shared value)
    //   1000 pairs -> ~33 Elo floor, 3.3% overhead   (this default)
    //   2900 pairs -> ~19 Elo floor, 9.7% overhead
    // 1000 is the point where the control can finally see an effect the size the loop plausibly
    // produces per 25 generations, without the cost becoming a real tax on datagen.
    let ctrl_pairs = arg("--control-pairs", 1000);
    // HOW OFTEN TO SNAPSHOT A LADDER RUNG, decoupled from the control.
    //
    // Rungs used to be saved ONLY inside the control block, so the cadence of the cheap thing was
    // welded to the cadence of the expensive one. With `--control-every 400` that is one ancestor
    // per 400 generations -- and at ~1800 gens/hour this run reached gen 685 with exactly TWO
    // rungs on disk.
    //
    // The two have opposite cost profiles and opposite reliability, which is why welding them is
    // wrong in both directions:
    //   * The control costs `ctrl_pairs` games and is the instrument this project has MEASURED
    //     reversing sign, at 0.861 and 0.967 (`instrument_saturation_RESULT.md`). It is run rarely
    //     because it is expensive, and read cautiously because it saturates.
    //   * A rung costs ONE 50KB file write and is the input to the instrument that still works
    //     above the saturation band: head-to-head against a recent ancestor.
    //
    // So the reliable instrument was rationed at the price of the unreliable one. Measured today
    // on two rungs ~49 minutes apart: the later net wins by +0.043 at depth 1 and +0.139 at
    // depth 4, both resolved at 448 pairs, same files and seed with only the depth differing.
    // (An earlier note here said +0.039 at depth 4. That was a stale unmatched run and it is
    // wrong -- the margin TRIPLES at the judged depth, it does not stay flat.)
    //
    // Two things follow. The "decline" the control reported over this window (0.871 -> 0.819) was
    // an artefact of pooling two runs' logs, see pooled_runs_RESULT.md. And the depth-1 gate sees
    // roughly a third of the improvement it is selecting on, which makes raising the gate depth a
    // live lead rather than a settled non-issue.
    //
    // That comparison was only possible because two rungs happened to exist; at gen 500 or 600
    // there was nothing to compare against and the question would have been unanswerable.
    //
    // 0 = off, preserving the previous behaviour exactly for any caller that does not ask.
    let rung_every = arg("--rung-every", 0);
    // Keep every Nth REJECTED candidate paired with the champion it lost to, so the depth-1 gate's
    // false-reject rate can be measured at depth 4 instead of argued about. See the use site.
    let rej_every = arg("--save-rejects-every", 0);
    // ACCEPTS, the complementary half. reject_holdout_RESULT.md measured what the gate THREW AWAY
    // (34 candidates, 0.4982 [0.4847,0.5117] -- exactly champion strength) and explicitly recorded
    // that it says nothing about what the gate KEEPS. That is the half which decides whether the
    // loop is doing anything: main.rs's own rollback note measures the 224-pair gate as resolving
    // only ~21.5 Elo while self-play steps are far smaller, so accepts could be largely noise --
    // which would produce a random walk, and the ancestor control's first readings (0.464, 0.498
    // over 400-generation windows) are what a random walk looks like.
    let acc_every = arg("--save-accepts-every", 0);
    let rej_dir = a.iter().position(|x| x == "--save-rejects")
        .and_then(|i| a.get(i + 1)).cloned().unwrap_or_default();
    // ANCESTOR CONTROL: play the champion against its own rung from `anc_lag` generations back,
    // every `anc_every`. 0 = off. See the use site for why a moving opponent is the point.
    // anc_lag MUST be a multiple of the rung cadence or the rung will not exist; the use site warns
    // rather than failing, so a misconfiguration is visible instead of silently skipped.
    // Distinguishes this run's rungs from a previous run's. See rung_path().
    let run_tag = a.iter().position(|x| x == "--run-tag")
        .and_then(|i| a.get(i + 1)).cloned().unwrap_or_default();
    // Rungs written BY THIS RUN. The ancestor control refuses any other, see its use site.
    let mut my_rungs: std::collections::HashSet<String> = std::collections::HashSet::new();
    let anc_every = arg("--ancestor-every", 0);
    let anc_lag = arg("--ancestor-lag", 400);
    let anc_pairs = arg("--ancestor-pairs", 200);
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
    // DEFAULT 1000 -> MAX_PLIES. The FILTERING is identical; the LOGGED NUMBER changes.
    //
    // Precisely: past gen 31 both caps admit every position, so no training set changes and no
    // decision changes. What differs is the horizon recorded in the log and ledger -- 160 rather
    // than a number climbing to 855. That is the point: the old value described a filter width
    // that had not existed since gen 31.
    //
    // The horizon filter keeps positions with `plies_to_end <= horizon`, and datagen plays at most
    // MAX_PLIES plies per game, so `plies_to_end` cannot exceed MAX_PLIES - 1. A cap of 1000 could
    // therefore never bind: it looked like a limit and was dead code, the same shape as `histCap`
    // at 2000 in the sibling 4PC project (a clamp above the largest value being clamped).
    //
    // THE SCHEDULE ITSELF SATURATES, and that is the finding this default was hiding.
    // `horizon = (10 + (g-1)*5).min(horizon_cap)` reaches MAX_PLIES at **generation 31**, after
    // which the filter admits EVERY position of every game and the "widening schedule" is no
    // longer a schedule. In the 170-generation ledger_long4 run, 140 of 170 generations (82%) ran
    // with the filter inert, while the schedule kept incrementing to 855 -- five times a ceiling
    // it had passed 139 generations earlier.
    //
    // NOT CLAIMED: that this harmed anything. Accept rate either side of gen 31 is 0.267 +/- 0.158
    // vs 0.229 +/- 0.070, a difference of -0.038 +/- 0.173 -- NOT resolved -- and MASTER_PLAN is
    // explicit that "acceptance rate is not a proxy for strength" anyway. What is established is
    // structural: MASTER_PLAN calls this schedule a hyperparameter that should be LEARNED and
    // widen "with strength", and as written it is declared, linear, and saturated by gen 31.
    // Passing --horizon-cap below MAX_PLIES still works and is how the 2x2 arms were run (45).
    let horizon_cap = arg("--horizon-cap", MAX_PLIES) as u32;
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
    // SHIPPED 2026-09-11: 0.75 -> 0.85.
    //
    // Two full-length seeds, 2000 generations per arm, exactly matched, shared start from champion
    // 044e754f57ba, only --blend differing, binding verified from each arm's own log header:
    //
    //     seed 20260917   0.541 +/- 0.032   lower bound 0.509
    //     seed 20260918   0.606 +/- 0.026   lower bound 0.580
    //     POOLED          0.574 +/- 0.021   lower bound 0.553
    //
    // The margin above 0.5 is 0.074 against a measured BETWEEN-SEED sd of 0.047. Seed 1 alone gave
    // 0.041, SMALLER than that spread, and was refused for exactly that reason -- the same standard
    // that refused lr 0.0001 on a 0.011 gap. Two seeds move it clear.
    //
    // Against the normal rule, blend_085 vs the shared start (which IS the champion, verified
    // unmoved at swap time) scores 0.583 +/- 0.027, lower bound 0.556 >= 0.500.
    //
    // The prior evidence was FIVE seeds at TWENTY generations (blend_RESULT.md), non-monotonic --
    // 0.75 -> 0.85 gains, 0.85 -> 1.00 gives it back -- so this is not "more is better". What those
    // lacked was full length, and the lr sweep is the standing proof that 20-generation and
    // 2000-generation answers differ in magnitude.
    let blend: f32 = a.iter().position(|x| x == "--blend")
        .and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(0.85);
    // LEARNING RATE, now a flag. It was the literal 0.01 here -- constant, no decay, no momentum,
    // no schedule, and not reachable from the command line, so it is the one generator knob this
    // project has never varied. Every other one is measured: label depth (spent at 3), blend (see
    // the correction below), epochs (under-fitting refuted), horizon (cap obsolete), games per
    // generation (4x changes nothing, `games_per_gen_RESULT.md`).
    //
    // CORRECTION 2026-09-11: "blend flat across 0.75-1.00" is a BLIND-METRIC NULL and is wrong.
    // That reading comes from the table at ~line 491 below, measured candidate-vs-champion.
    // `blend_RESULT.md` then showed that instrument COMPRESSES this very contrast ~3x (+0.036
    // against +0.112 on the SAME two arms) and concluded: "a NULL measured on the blind metric is
    // worthless. Compression manufactures nulls." On the working metric the response is
    // NON-MONOTONIC -- 0.75 -> 0.85 gains (0.460 +/- 0.022, clear of 0.5), 0.85 -> 1.00 gives it
    // back (0.567 +/- 0.022) -- replicated on five training seeds. blend 0.85 is recorded there as
    // "the best-evidenced open candidate in the tree". It is NOT shipped: every one of those arms
    // is a TWENTY-generation run, and `blend_sweep.sh` is the full-length matched pair that would
    // authorise a default change.
    //
    // WHY IT IS THE LIVE SUSPECT. `nontransitive_walk_RESULT.md` dates the plateau: prod2 gained
    // +28.6 Elo between generations 2,162 and 4,818 and then **-0.7 Elo over the next 1,985**, with
    // its 5-generation steps sitting at 0.4869 [0.4364, 0.5374] -- indistinguishable from a coin
    // flip. A net that moves every generation but goes nowhere is what a CONSTANT step size
    // produces: it keeps kicking the weights around a basin instead of settling into it.
    //
    // Default 0.01 so this is byte-identical to every measurement taken so far.
    let lr: f32 = a.iter().position(|x| x == "--lr")
        .and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(0.01);
    // LEARNING-RATE DECAY. Per-generation multiplier: lr_g = max(lr * decay^g, lr_min).
    //
    // DEFAULT 1.0 IS AN EXACT NO-OP, deliberately, so every measurement taken before this flag
    // existed stays byte-identical -- the same discipline `--lr` itself was added under.
    //
    // WHY A SCHEDULE AND NOT A FOURTH FIXED RATE. Measured 2026-09-11: lr 0.002 beat 0.01
    // decisively from a net plateaued at 0.01 (0.692 vs 0.499 against the shared start), but from a
    // net ALREADY trained at 0.002 it stops paying -- prod3 read 0.478 against the champion after
    // 7,515 generations and an independent run read 0.458. Dropping again to 0.0005 passed at
    // 0.625. Each drop buys a burst that then saturates, which is the signature of a SCHEDULE
    // rather than of one correct constant.
    let lr_decay: f32 = a.iter().position(|x| x == "--lr-decay")
        .and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(1.0);
    let lr_min: f32 = a.iter().position(|x| x == "--lr-min")
        .and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let mut tr = Trainer::new(lr, blend);
    let mut rung = start_rung;
    // anchor-pairs is printed with the rest of the settings, and that is load-bearing rather than
    // cosmetic. chain_anchor.sh verified the flag existed by grepping the BINARY for the string --
    // which returns 0 for any arg()-only literal, because rustc does not store them as contiguous
    // greppable text. (Control: --horizon-cap also greps 0 and demonstrably works; --gate-pairs
    // greps 1 only because it appears in THIS format string.) That false negative aborted the
    // anchor A/B. A setting that cannot be observed in the program's own output cannot be verified
    // by anything except reading the source.
    println!("lr={lr} lr-decay={lr_decay} lr-min={lr_min} gens={gens} games/gen={games} depth={depth} gate-match-depth={gate_match_depth} epochs={epochs} gate-pairs={gate_pairs} gate-nodes={gate_nodes} gate-every={gate_every} include-draws={include_draws} anchor-pairs={anchor_pairs} rollback={rollback} blend={blend}");
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
    // The SAME construction examples/control.rs uses, so the in-loop anchor and the end-of-run
    // origin score are the same opponent. A different seed here would silently make the accept
    // decision and the final measurement disagree about what "the origin" is.
    let anchor = Net::random(champion.n_hidden, 20260907);
    let mut champ_anchor: Option<f64> = None;   // measured lazily, only if the anchor gate is on
    // Best (origin-control rate, champion, generation) seen. The rollback target.
    let mut best_ctrl: Option<(f64, Net, usize)> = None;
    // The net a batch started from, and what --gate-every rolls back to when a batch fails.
    // SEEDED FROM THE STARTING CHAMPION, not lazily at the first gate. This was
    //     let base = batch_base.get_or_insert_with(|| champion.clone()).clone();
    // inside the gate block, so `base` was first set to the champion AFTER the first K generations
    // of training. Two consequences, both measured:
    //   * the first gate compared a net against ITSELF -- first-gate increments were -0.002, -0.009
    //     and -0.009 across three runs, noise by construction;
    //   * the first K generations were NEVER GATED, so any damage they did was permanent. ga_d4 ran
    //     20 generations with ZERO KEEPs and still finished at 0.813 against champion_long's 0.861:
    //     its first 5 generations cost 0.048 and there was no baseline to roll back to.
    // Seeding it here makes the first gate a real comparison and puts generations 1..K under the
    // same rollback protection as every later block.
    let mut batch_base: Option<Net> = Some(champion.clone());
    /// The batch base's score against the FIXED origin, cached: constant for a whole batch.
    let mut batch_base_anchor: Option<(f64, f64)> = None;
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
    // REPLAY WINDOW, in generations. Was a bare `8` with no derivation and no measurement.
    //
    // He asked whether throwing data away is worth it: 22.6M positions have been generated and
    // the loop trains on the newest ~250k. The argument FOR a window is real -- positions from
    // 100 generations ago came from a much weaker champion, and training on them teaches the net
    // to reproduce play it has outgrown; AlphaZero and Leela both use one. But 8 was never
    // tested here, and today four separate constants turned out wrong for exactly that reason:
    // `ci95 < 0.05` could never fire (measured 0.050-0.053), `arch-pairs 160` was derived to hit
    // that threshold EXACTLY and missed by 0.001, the 30-epoch training budget was tuned for
    // width 16 and silently blocked widths 128/256, and the +/-1 proposal stride could not reach
    // past one rung. A constant that has never been varied is not a measurement.
    //
    // A flag, so it can be swept against the same frozen origin with everything else held fixed.
    let replay_gens = arg("--replay-gens", 8);
    let mut replay_marks: std::collections::VecDeque<usize> = std::collections::VecDeque::new();

    for g in 1..=gens {
        // Applied BEFORE this generation trains. At decay 1.0 (the default) `powi` returns exactly
        // 1.0 and this assigns `lr` unchanged, so a no-decay run is bit-identical to one compiled
        // before the flag existed.
        if lr_decay != 1.0 || lr_min != 0.0 {
            tr.lr = (lr * lr_decay.powi(g as i32)).max(lr_min);
            // Observable, but WITHOUT touching the `gen` line's format -- adding a field there would
            // change the output of every run that uses no decay at all, and the whole point of the
            // 1.0 default is that such runs stay byte-identical.
            if g == 1 || g % 200 == 0 {
                println!("  lr now {:.6} at gen {g}", tr.lr);
            }
        }
        // ---- self-play with the current champion
        let dgen_depth = if g >= deepen_at { deep } else { depth };
        let t0 = std::time::Instant::now();
        let (data, dec) = datagen::play_games(
            &champion, dgen_depth, rng.next(), games, 4, MAX_PLIES, threads);
        let drawn = games - dec;
        let t_gen = t0.elapsed().as_secs_f64();

        // ---- train a candidate from the champion.
        // HORIZON: only positions within `horizon` plies of the terminal, and only from decided
        // games. Measured 2026-09-07: training on ALL decided positions makes the eval WORSE
        // (sign acc 0.452 -> 0.441) while <=10 plies makes it BETTER (-> 0.543) on 5x less
        // data. Far-from-terminal labels are anti-signal while both players are near-random.
        // The horizon WIDENS with generation, because the label becomes informative further
        // back as play improves.
        // A RESUMED RUN IS NOT AT ITERATION ZERO, so it must not re-enter bootstrap mode.
        //
        // The ramp is CORRECT IN SHAPE and both ends are measured. At iteration zero, narrow wins:
        // training on all decided positions gives sign accuracy 0.441 against 0.543 for <=10 plies,
        // because with near-random play the result barely depends on a position 40 plies back. Past
        // bootstrap, WIDE wins: horizon_RESULT.md records capped-at-10 scoring 0.774 +/- 0.025
        // against the origin where uncapped scores 0.838 +/- 0.023 -- +0.064 +/- 0.034, resolved,
        // about +72 Elo. So narrow-then-wide is right, and `10 + (g-1)*5` delivers exactly that.
        //
        // What it gets wrong is the VARIABLE. It ramps on the GENERATION COUNTER, which resets on
        // every resume, so a run started with --init from a strong champion spends its first 30
        // generations at the BOOTSTRAP horizon -- the configuration measured as worse for a champion
        // that is not random. Observed on this very loop 2026-09-10: three runs, all starting at
        // h10, two of them resumed from a champion scoring 0.798 against the origin.
        //
        // MASTER_PLAN asks for a horizon that widens "with strength rather than being fixed", and a
        // resume is the one moment when strength is KNOWN without measuring it: the champion was
        // inherited, not initialised. So skip the ramp entirely when resuming.
        let horizon = if init_net.is_some() {
            horizon_cap
        } else {
            (10 + (g as u32 - 1) * 5).min(horizon_cap)
        };
        let pool: Vec<Sample> = data.iter()
            // --include-draws KEEPS z == 0 samples. Default OFF, so every result measured so far
            // stays comparable and this is A/B-able rather than silently swapped in.
            //
            // WHY IT IS WORTH ASKING. Measured over 750 generations from 49 runs on disk: the
            // decisive-game rate averages 0.360, so roughly 64% of every self-play batch is drawn
            // and discarded here. Volume is not the issue -- 35,754 training samples per
            // generation against a 12,528-weight net is ample. The DISTRIBUTION might be: the
            // value head only ever sees positions from games that ended decisively, so it is
            // never taught what a drawn position looks like, while most positions are drawn.
            // Excluding the majority class outright is unusual; AlphaZero-style loops train draws
            // at target 0, which this target already supports since z = 0 is well defined.
            //
            // THE COUNTER-ARGUMENT, stated because it may well win: the eval feeds alpha-beta,
            // which needs a RANKING, not calibrated draw probabilities. Separating win-ish from
            // loss-ish may be all the search requires, and the draws may be exactly the
            // uninformative middle the filter was put here to remove.
            //
            // NOT A CLAIM THAT THE FILTER IS WRONG. The horizon half of this same filter is
            // backed by a real measurement (training on ALL decided positions moved sign accuracy
            // 0.452 -> 0.441, while <=10 plies moved it to 0.543). The draw half has never been
            // measured separately, and that is the whole gap this flag exists to close.
            .filter(|s| (include_draws || s.z != 0.0) && s.plies_to_end <= horizon)
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
        // --gate-nodes N > 0 switches the NET gate from fixed DEPTH to the fixed COST BUDGET
        // that FITNESS 6 specifies. Default 0 keeps today's behaviour so the two are A/B-able
        // rather than silently swapped -- every result measured today used the depth gate, and
        // changing it by default would make them incomparable without saying so.
        // ---- BATCH GATING (--gate-every K). THE PER-GENERATION GATE IS OFF WHEN K > 1.
        //
        // WHY, quantitatively. The pair standard deviation is 0.2362 (MEASURED, 15,008 real
        // pentanomial pairs), so a K-pair gate resolves no better than 1.96*0.2362/sqrt(K). The
        // model is validated against this harness's OWN logged bars: it predicts +/-0.0732 at 40
        // pairs and an_0.log printed +/-0.073, 0.076, 0.069, 0.074.
        //
        // The signal it is being asked to filter is far smaller than that:
        //     pooled over 239 generations, 7 runs   rate 0.5024  -> needs 37,209 pairs to resolve
        //     epochs-2 arm, replicated on 2 seeds   rate 0.5114  -> needs  1,649 pairs to resolve
        // A 40-pair gate is 6.4x too coarse to see the best per-generation edge ever measured
        // here; a 224-pair gate is still 2.7x too coarse. So it rejects EVERYTHING -- 13
        // generations, 0 accepts, every one "reject" -- and when it was made permissive instead
        // (the surrogate fallback) it accepted noise and drove the champion 0.864 -> 0.826. Both
        // failure modes are the same cause: a filter whose resolution is coarser than its signal.
        //
        // A gate cannot be made to work per-generation by tuning it. The fix is to stop asking it
        // a question it cannot answer: train for K generations unconditionally, then gate the
        // ACCUMULATED change against the net the batch started from, and roll back if it lost.
        // If per-generation edges accumulate at all, K=10 needs ~16 pairs to resolve.
        //
        // PRE-REGISTERED, and this is a real experiment rather than a fix I am confident in:
        //   * If the accumulated champion RESOLVES above its batch base, the per-generation gains
        //     were real and merely unmeasurable one at a time. The plateau was a MEASUREMENT
        //     failure and this is the repair.
        //   * If it does NOT resolve after K generations of unconditional training, then the
        //     gains are not real and not cumulative -- the pooled 0.5024 is the honest number and
        //     the loop's problem is the TRAINING SIGNAL, not the gate. That would refute the
        //     reasoning above, and it is the more likely outcome given 0.5024.
        // Either answer is decisive, which is why it is worth the box time.
        //
        // Default K=1 preserves today's behaviour exactly, so every result measured so far stays
        // comparable and this is A/B-able rather than silently swapped in.
        let batch_mode = gate_every > 1;
        let (verdict, sc, llr) = if batch_mode {
            // No match is played. Score::default() is an HONEST empty record -- zero games -- and
            // the Accept short-circuits `better` before any of its fields are read.
            (gate::Sprt::Accept, gate::Score::default(), 0.0)
        } else if gate_nodes > 0 {
            gate::sprt_match_nets_capped(&cand, &champion, depth, gate_nodes, gate_nodes,
                                         gate_pairs, seed ^ g as u64, 4, 0.0, 5.0)
        } else {
            gate::sprt_match_nets(&cand, &champion, depth, gate_pairs, seed ^ g as u64, 4, 0.0, 5.0)
        };
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
        } else if surrogate_fallback {
            // Genuinely undecided: the interval is BOTH wide and straddling. Only here may the
            // surrogate speak, and only where the gate does not contradict it.
            //
            // OFF BY DEFAULT NOW, because the surrogate has been MEASURED against the games and
            // carries no usable information about strength. Every generation records both a
            // mcnemar_z and a 224-pair gate result on the SAME candidate; over 239 such pairs
            // from seven runs the correlation is -0.095, 95% CI [-0.220, +0.032], six of seven
            // runs negative. A decision rule needs a strong POSITIVE correlation with strength;
            // this one cannot be distinguished from zero and leans the wrong way.
            //
            // This branch is where the damage happened, and the numbers line up exactly. Accepts
            // cluster in the runs where this branch is REACHABLE:
            //     gate 40  (median ci95 0.072)  7 accepts     scored 0.826 vs the frozen origin
            //     gate 32  (median ci95 0.081) 11 accepts
            //     gate 32  (median ci95 0.083)  8 accepts
            //     gate 224 (median ci95 0.031)  1 accept      scored 0.859
            //     gate 224 (median ci95 0.031)  1 accept
            //     gate 224 (median ci95 0.030)  0 accepts
            // 26 of 28 accepts across every run today came from gates coarse enough to reach
            // here. At 224 pairs `ci95 < 0.05` fires first and rejects, which is why the sharper
            // gate scored higher: it did not just measure better, it took the decision AWAY from
            // this branch. The gate A/B was really a test of this line.
            //
            // NOT DELETED, because the bootstrap argument above is real: from random nets the
            // match is near-all-draws and the gate genuinely cannot resolve, so something has to
            // decide or the loop never starts. --surrogate-fallback restores it for that case.
            // What is no longer allowed is reaching it by accident with a coarse gate.
            mcnemar > 1.96 && no_regression
        } else {
            false
        };

        // ---- ANCHOR GATE: a promotion must not REGRESS against a fixed opponent.
        //
        // Applied AFTER the champion match, and only to candidates that already passed it, so it
        // can only ever veto. It cannot promote anything the champion gate rejected.
        //
        // The rule is "not resolved WORSE", not "better". Demanding an improvement against the
        // anchor every generation would reject genuine small gains the anchor match cannot see:
        // at 224 pairs its ci95 is ~0.031, and real steps are far smaller than that. Demanding
        // merely that the candidate is not MEASURABLY worse blocks the failure actually observed
        // -- ep_1 was 0.030 below its champion against the origin, which a 0.031 interval resolves
        // -- while staying silent where the anchor has no opinion.
        let (better, anchor_veto) = if better && anchor_pairs > 0 && !batch_mode {
            let ca = *champ_anchor.get_or_insert_with(|| {
                gate::match_nets(&champion, &anchor, depth as u32, anchor_pairs, seed ^ 0xA1C).pent_rate()
            });
            let cs = gate::match_nets(&cand, &anchor, depth as u32, anchor_pairs, seed ^ 0xA1C ^ g as u64);
            // NOTE, and this corrects a comment that used to sit here claiming "same seed family for
            // both sides so they meet the anchor on the same openings". THEY DO NOT. `match_nets`
            // derives every opening from `Rng(seed | 1)` (gate.rs:100), so `seed ^ 0xA1C` and
            // `seed ^ 0xA1C ^ g` walk two entirely different opening sets. Sharing a prefix is not
            // sharing a seed. The comparison is UNPAIRED, and its variance is the sum of two
            // independent sampling variances rather than the variance of a paired difference.
            let veto = cs.pent_rate() + cs.ci95() < ca;
            (!veto, veto)
        } else {
            (better, false)
        };
        // LEDGER: record the decision, accepted or not, with a NAMED reason. A rejection is the
        // more reusable fact — it says do not spend this compute again — and "reject" alone is
        // not a finding.
        let net_reason = if better {
            Reason::Accepted
        } else if anchor_veto {
            Reason::AnchorRegression
        } else if gate_can_resolve {
            Reason::LostOnGames
        } else if !no_regression {
            Reason::Regression
        } else {
            Reason::NoEvidence
        };
        // Shadowed here because this is the one decision site with a position set in hand.
        let top_fen = top_disagreement(&cand, &champion, subset, 512).or_else(|| top_fen.clone());
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
                // BELT AND BRACES. `Score::ci95()` now returns 1.0 for a zero-game record, so
                // `gate_can_resolve` is already false here. This flag is the one COUNTED out of
                // the ledger months later, though, and it should be impossible to set from a match
                // that never happened even if the interval logic moves again.
                resolved: sc.games() > 0 && gate_can_resolve,
            }],
            // `pool` is recorded so post-hoc analysis can correlate HOW MUCH HISTORY the
            // trainer saw with what the gate decided. It goes to the log line already, but the
            // log is not queryable -- every quantitative claim made about this loop today came
            // from the ledger, and the replay-window question could not be asked of it at all.
            // TRAINING loss, on the trained-on subset. I renamed this to "heldout_loss" earlier
            // today and that was WRONG: the held-out one is `train_from`'s best_loss
            // (tr.loss(&net, held)), and train_from is called ONLY by the ARCH arm. The NET arm
            // trains with `for e in 0..epochs { loss = tr.epoch(&mut cand, subset, ..) }`, and
            // tr.epoch accumulates over the data it is training on. Second time today I asserted
            // which code path produced a value without following it; the name stays literal now.
            surrogate: vec![("mcnemar_z", mcnemar), ("train_loss", loss as f64), ("llr", llr),
                            ("pool", replay.len() as f64), ("train_n", subset.len() as f64)],
            e1: gate_e1,
            top_disagreement_fen: top_fen.clone(),
        });

        // SAVE A SAMPLE OF REJECTED CANDIDATES, so the gate's error rate can be MEASURED.
        //
        // Measured today on two rungs: the later net beats the earlier by +0.043 at depth 1 and
        // +0.139 at depth 4 (448 pairs, same files and seed, only depth differing). The gate runs
        // at the datagen depth -- 1 by default -- so it sees roughly a THIRD of the improvement it
        // is selecting on. The obvious consequence is that it rejects candidates which are real
        // gains at the depth strength is judged at.
        //
        // That is an inference, and this project's rule is to build the discriminator rather than
        // the paragraph. The discriminator needs the rejected nets, and the loop currently drops
        // them on the floor -- `cand` is overwritten next generation and is gone. So: keep every
        // Nth reject, paired with the champion it lost to AT THAT MOMENT, which is the only valid
        // opponent for it (the champion moves, so a later one would be a different question).
        //
        // Then offline: play each pair at depth 4. The fraction of rejects that WIN there is the
        // gate's false-reject rate, in units of real strength. If it is small the depth-1 gate is
        // vindicated cheaply; if it is large, that is the cost of the 200x saving, quantified.
        //
        // Sampled every Nth rather than all, because a w16 net is 50KB and a run does ~1800
        // generations/hour -- keeping every reject would write ~85MB/hour to fill a disk with
        // near-duplicates. 0 = off, so no existing caller changes behaviour.
        if !better && rej_every > 0 && !rej_dir.is_empty() && g % rej_every == 0 {
            // TAGGED for the same reason the rungs are: the generation counter restarts every
            // launch, so an untagged rej_g100 from run 7 overwrites run 6's, and reject_audit would
            // then pool candidates from different lineages into one "gate error rate".
            let stem = if run_tag.is_empty() { format!("rej_g{g}") } else { format!("rej_{run_tag}_g{g}") };
            let c = format!("{rej_dir}/{stem}_cand.net");
            let h = format!("{rej_dir}/{stem}_champ.net");
            if let Err(e) = std::fs::create_dir_all(&rej_dir) {
                eprintln!("      WARNING: could not create {rej_dir}: {e}");
            } else if std::path::Path::new(&c).exists() {
                eprintln!("      WARNING: reject sample {c} already exists -- NOT overwriting. \
                           Pass --run-tag to keep this run's samples distinct.");
            } else if let Err(e) = cand.save(&c).and_then(|_| champion.save(&h)) {
                eprintln!("      WARNING: could not save reject pair at gen {g}: {e}");
            } else {
                println!("      reject sample saved: {c}");
            }
        }

        // SAVE A SAMPLE OF ACCEPTED CANDIDATES, paired with the champion they BEAT. Same pairing
        // rule as the reject sampler and for the same reason: the champion moves on every accept,
        // so measuring a saved candidate against a later champion answers a different question than
        // the one the gate decided. Saved BEFORE `champion = cand`, which is the only point where
        // both nets still exist.
        if better && acc_every > 0 && !rej_dir.is_empty() && g % acc_every == 0 {
            let stem = if run_tag.is_empty() { format!("acc_g{g}") } else { format!("acc_{run_tag}_g{g}") };
            let c = format!("{rej_dir}/{stem}_cand.net");
            let h = format!("{rej_dir}/{stem}_champ.net");
            if let Err(e) = std::fs::create_dir_all(&rej_dir) {
                eprintln!("      WARNING: could not create {rej_dir}: {e}");
            } else if std::path::Path::new(&c).exists() {
                eprintln!("      WARNING: accept sample {c} already exists -- NOT overwriting.");
            } else if let Err(e) = cand.save(&c).and_then(|_| champion.save(&h)) {
                eprintln!("      WARNING: could not save accept pair at gen {g}: {e}");
            } else {
                println!("      accept sample saved: {c}");
            }
        }

        if better {
            // The new champion's anchor score is re-measured lazily on the next generation that
            // needs it; clearing it is what prevents the OLD champion's score being compared
            // against a NEW champion's candidates.
            if anchor_pairs > 0 { champ_anchor = None; }
            champion = cand;
            accepted += 1;
            // Persist on every acceptance, not at the end: a run killed by a timeout used to
            // discard everything it had learned.
            if let Err(e) = champion.save(&out) {
                eprintln!("  WARN could not save champion to {out}: {e}");
            }
        }

        // ---- THE BATCH GATE. Runs every K generations on the ACCUMULATED champion.
        //
        // This is the comparison the per-generation gate could not afford: `champion` has now
        // absorbed K unconditional promotions, so the edge being measured is K generations of
        // change rather than one, against the net the batch started from. Same gate, same pair
        // count, a signal K times larger.
        //
        // ROLLBACK IS THE POINT, not a safety extra. Without the per-generation gate nothing
        // stops a batch from wandering downhill, and 0.864 -> 0.826 is what that looks like when
        // it happens. Requiring the batch to RESOLVE upward (interval clear of 0.5) rather than
        // merely score above it keeps the same standard the per-generation gate used, so a batch
        // that is indistinguishable from its base is discarded rather than kept on a coin flip.
        if batch_mode && g % gate_every == 0 {
            let base = batch_base.get_or_insert_with(|| champion.clone()).clone();
            // ---- INCREMENT OVER A FIXED ANCHOR, not a match against the parent.
            //
            // This compared champion vs BASE, which is a parent-relative comparison -- and
            // parent-relative comparison is the thing measured as uninformative here. Two similar
            // nets at depth 2 draw nearly everything, so that match returns 0.500 +/- 0.007 on a
            // pair the ORIGIN separates easily, and non-transitivity was demonstrated directly (a
            // net can beat its parent while being weaker against a third opponent).
            //
            // proxies_RESULT.md: all three cheap signals are now measured uninformative -- the
            // held-out surrogate (r = -0.095 against 239 gate results), the training loss
            // (r = +0.379, CI [-0.249, +0.783], n=12), and the parent gate. The ONLY thing that has
            // resolved anything is games against a FIXED opponent: they refuted capacity
            // (0.179 +/- 0.021) and the draw filter (+0.054 +/- 0.034).
            //
            // So measure both nets against the ORIGIN and compare the INCREMENTS. Same principle
            // the 4PC ICC protocol arrived at independently: judge a change by its increment over
            // a fixed baseline, never by a head-to-head against the thing it came from.
            //
            // THE BASE'S ANCHOR SCORE IS CACHED. It is constant for the whole batch, so measuring
            // it once per batch rather than once per gate halves the games. It is recomputed
            // whenever the base moves, which is the only time it can change.
            // PAIRED MODE (EXISTENCE_PAIRED_BATCH=1). MEASURED AND NOT WORTH TAKING -- stays OFF.
            //
            // examples/pairing_match.rs, 10 replicates at 112 pairs, equal games both ways:
            //     unpaired  mean +0.0297  sd 0.0164
            //     paired    mean +0.0308  sd 0.0155     variance ratio 1.12x
            // The two means agree, so the harness is unbiased; pairing simply does not help. At
            // n=10 the F(9,9) interval on that ratio spans about [0.28, 4.5] and does not clear 1.
            //
            // WHY, mechanically: match_nets_open walks only 4 random plies before the nets take
            // over (open_plies = 4), so almost all openings are near-balanced and opening
            // difficulty contributes little of the variance. The variance is in the GAMES. There is
            // nothing for pairing to cancel. Deeper openings would give pairing more to work with
            // and add spread of their own; that is a different experiment, not a fix to this one.
            //
            // The flag is kept, defaulted off, so the negative is reproducible rather than folklore.
            // Do not re-enable it expecting resolution: the 0.862-vs-0.833 spread on a single net
            // that motivated this is ~2 sigma of ordinary sampling noise at 224 pairs, not an
            // opening artefact.
            //
            // The unpaired default gives champion and base DIFFERENT opening sets, because
            // match_nets builds every opening from Rng(seed | 1) and the two seeds differ by ^g.
            // Directly observed: after the g10 KEEP the base BECAME that champion, and the same net
            // scored 0.862 +/- 0.020 as champion and 0.833 +/- 0.022 as base -- a 0.029 gap on one
            // net, from opening luck alone. Increments of +0.033 and +0.021 are being judged
            // against a baseline carrying that much noise.
            //
            // Paired mode re-measures the base on the SAME openings as the champion, so
            // opening difficulty is common to both sides and cancels in the difference. It costs
            // the cached-base saving -- two matches per batch instead of one -- which is exactly
            // why it is a flag and not a default: it must beat the cache at EQUAL TOTAL GAMES, not
            // merely have lower variance per batch. pairing_ab.sh measures that.
            let paired = std::env::var("EXISTENCE_PAIRED_BATCH").is_ok();
            let bseed = if paired { seed ^ 0xA9C0 ^ g as u64 } else { seed ^ 0xA9C0 };
            let cs = gate::match_nets(&champion, &origin, gate_match_depth, gate_pairs,
                                      seed ^ 0xA9C0 ^ g as u64);
            let (br, bc) = if paired {
                // Re-measured every batch on the champion's own openings; never cached, since a
                // cached score is by definition from a different opening set.
                let m = gate::match_nets(&base, &origin, gate_match_depth, gate_pairs, bseed);
                (m.pent_rate(), m.ci95())
            } else {
                *batch_base_anchor.get_or_insert_with(|| {
                    let m = gate::match_nets(&base, &origin, gate_match_depth, gate_pairs, seed ^ 0xA9C0);
                    (m.pent_rate(), m.ci95())
                })
            };
            // EXISTENCE_DIRECT_BATCH=1: decide on a DIRECT champion-vs-base match instead of the
            // difference of two vs-origin scores.
            //
            // WHY. Both modes above score each net against the frozen random ORIGIN, and that scale
            // SATURATES. Measured over 13 batch decisions on 2026-09-10, champ-vs-origin sat at
            // 0.948-0.981, mean 0.963 -- and `instrument_saturation_RESULT.md`, dated the same day
            // as the file that proposed batch gating, records this metric REVERSING SIGN at 0.861
            // and 0.967. Every decision fell inside that band; the one KEEP was at 0.981, the most
            // saturated reading in the set, clearing its interval by 0.004.
            //
            // Three defects, all removed by asking the question directly:
            //   * SATURATION -- five generations moved the metric 0.960 -> 0.970. Under a ceiling of
            //     1.0 a real gain has nowhere to go. A champion-vs-base match is centred at 0.5.
            //   * TWO measurements where one will do -- subtracting independent scores gives a
            //     combined ci95 of ~0.018 against increments of +0.003 to +0.021.
            //   * PAIRING -- EXISTENCE_PAIRED_BATCH exists because opening luck alone moved one net
            //     0.029. A single head-to-head is paired by construction: same openings both sides.
            // It also costs ONE match rather than the paired mode's two.
            //
            // The rule is the project's standard `rate - ci95 >= 0.5`, the same bar the
            // per-generation gate and auto_promote use. The FLOOR is untouched and still exceeds one
            // generation's edge; what batching contributes is K generations of edge against an
            // unchanged interval -- acceptance_floor_RESULT.md's original argument, now made on a
            // scale that can express it.
            //
            // Default OFF, so every result measured so far stays comparable and this is A/B-able
            // rather than silently swapped in -- the convention the two flags above already set.
            let direct = std::env::var("EXISTENCE_DIRECT_BATCH").is_ok();
            let up = if direct {
                let m = gate::match_nets(&champion, &base, gate_match_depth, gate_pairs,
                                         seed ^ 0xD1EC ^ g as u64);
                let keep = m.pent_rate() - m.ci95() >= 0.5;
                println!("      batch gate g{g} (last {gate_every} gens) DIRECT: champ-vs-base \
{:.3}+/-{:.3} -> {}", m.pent_rate(), m.ci95(), if keep { "KEEP" } else { "ROLL BACK" });
                keep
            } else {
                // Two-sample: the increment must clear the combined interval, not merely be
                // positive. Requiring only `cs > br` would promote on noise every other batch.
                let diff = cs.pent_rate() - br;
                let se = ((cs.ci95() / 1.96).powi(2) + (bc / 1.96).powi(2)).sqrt();
                let u = diff - 1.96 * se > 0.0;
                println!("      batch gate g{g} (last {gate_every} gens): champ-vs-origin \
{:.3}+/-{:.3} base {:.3}+/-{:.3}  increment {:+.3}+/-{:.3} -> {}",
                         cs.pent_rate(), cs.ci95(), br, bc, diff, 1.96 * se,
                         if u { "KEEP" } else { "ROLL BACK" });
                u
            };
            if up {
                batch_base = Some(champion.clone());
                batch_base_anchor = None; // base moved, so its anchor score must be re-measured
            } else {
                // champ_anchor is deliberately NOT cleared here. Its only read site is
                // guarded by `!batch_mode`, so in batch mode the cache is never consulted and
                // clearing it would be dead code. Stating that, because the enumeration above
                // flags this line and the next reader deserves to know it was checked.
                champion = base;
                if let Err(e) = champion.save(&out) {
                    eprintln!("  WARN could not save champion to {out}: {e}");
                }
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
                        e1: gate_e1,
                        top_disagreement_fen: top_fen.clone(),
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
                        e1: gate_e1,
                        top_disagreement_fen: top_fen.clone(),
                    });
                    println!("      ARCH w{:>3} -> w{:>3} ({} ep, loss {:.4} vs {:.4}, paired z {:.2})  fixed-cost {:.3}+/-{:.3}{}  clock {:.3}+/-{:.3} [{ca} vs {cb} nodes]{}  {}=> {}",
                             WIDTH_MENU[p.from_rung], p.width(), aeps, acand_loss, champ_loss, z,
                             fixed.pent_rate(), fixed.ci95(), if fixed_win { " ok" } else { "" },
                             clock.pent_rate(), clock.ci95(), if clock_win { " ok" } else { "" },
                             if resolves { "" } else { "[gates blind, surrogate decides] " },
                             if stepped { "STEP" } else { "hold" });
                    if stepped {
                        // Same invalidation the accept path at line 849 does. Found by
                        // enumerating every `champion = ` site and checking each for a nearby
                        // `champ_anchor = None`: this one had none, so with --anchor-pairs > 0 an
                        // ARCH step would leave the OLD champion's cached anchor score in place and
                        // judge the NEW champion's candidates against it. Same bug class as the
                        // batch_base defect -- a lazy cache whose subject changed underneath it.
                        // Latent rather than live: anchor-pairs defaults to 0, arch-every to 5.
                        if anchor_pairs > 0 { champ_anchor = None; }
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
        // LADDER RUNG ON ITS OWN CADENCE. Skipped when the control also fires this generation,
        // so the control keeps writing the rung it annotates with an origin score and no rung is
        // ever written twice. Deliberately NOT gated on `accepted`: a rung's job is to mark where
        // the lineage was at generation g, and "unchanged since the last rung" is a fact worth
        // being able to demonstrate rather than infer.
        if rung_every > 0 && g % rung_every == 0 && !(ctrl_every > 0 && g % ctrl_every == 0)
            && !out.is_empty() && out != "/dev/null" {
            let rung = rung_path(&out, &run_tag, g);
            if std::path::Path::new(&rung).exists() {
                eprintln!("      WARNING: ladder rung {rung} already exists -- NOT overwriting. \
                           The generation counter restarts each launch; pass --run-tag to keep \
                           this run's rungs distinct.");
            } else if let Err(e) = champion.save(&rung) {
                eprintln!("      WARNING: could not write ladder rung {rung}: {e}");
            } else {
                my_rungs.insert(rung.clone());
                println!("      ladder rung saved: {rung}");
            }
        }

        // ANCESTOR CONTROL, on its OWN cadence -- deliberately NOT inside the origin-control block.
        //
        // I wrote it nested inside that block first, which made it fire only when the origin
        // control fired. That is exactly the coupling I removed from the ladder rungs earlier the
        // same day, reintroduced one screen lower: a cheap independent measurement welded to the
        // cadence of an expensive one. It was invisible in review -- the code reads correctly, the
        // guard is right, and with the default --control-every it would even have produced output
        // on the usual schedule. The smoke test caught it because it ran --control-every 0, and the
        // ancestor line never appeared and no warning did either.
        if anc_every > 0 && g % anc_every == 0 && g > anc_lag
            && !out.is_empty() && out != "/dev/null" {
            let past = rung_path(&out, &run_tag, g - anc_lag);
            // PROVENANCE, not just existence. The no-clobber guard preserves a rung left by an
            // EARLIER run that used the same --run-tag -- which is the right call for the file, but
            // it means `past` can exist and belong to a different lineage. Loading it anyway would
            // print "ancestor control @gen G vs gen G-lag" while actually comparing across runs:
            // the exact mislabel this whole feature exists to prevent, reproduced by its own safety
            // guard. Observed in the smoke test, where a second pass under the same tag happily
            // measured itself against the first pass's net.
            if !my_rungs.contains(&past) {
                eprintln!("      WARNING: ancestor control skipped -- {past} was not written by \
                           THIS run, so comparing against it would cross lineages. Use a fresh \
                           --run-tag.");
            } else {
            match Net::load(&past) {
                Ok(old) => {
                    let (pa, pb) = arch::equal_time_caps(&champion, &old, budget_ns, depth.max(3));
                    let ac = gate::match_nets_capped(&champion, &old, gate_depth_cap, pa, pb,
                                                     anc_pairs, seed ^ 0xA9CE ^ g as u64, 4);
                    println!("      ancestor control @gen {g} vs gen {}: {}W-{}D-{}L  rate {:.3} +/- {:.3}  [{pa} vs {pb} nodes]{}",
                        g - anc_lag, ac.wins, ac.draws, ac.losses, ac.pent_rate(), ac.ci95(),
                        if ac.rate() - ac.ci95() > 0.5 { "  *" } else { "" });
                }
                // Not fatal and not silent: the rung is missing whenever anc_lag is not a multiple
                // of the rung cadence, which is a configuration mistake worth seeing rather than a
                // reason to stop training.
                Err(e) => eprintln!("      WARNING: ancestor control skipped, cannot load {past}: {e}"),
            }
            }
        }

        if ctrl_every > 0 && g % ctrl_every == 0 {
            // EQUAL TIME, not equal depth: once the ARCH arm can change the champion's width,
            // a depth-matched control would hand a wider champion free computation and report
            // its extra cost as strength.
            let (ca, cb) = arch::equal_time_caps(&champion, &origin, budget_ns, depth.max(3));
            let c = gate::match_nets_capped(&champion, &origin, gate_depth_cap, ca, cb,
                                            ctrl_pairs, seed ^ 0xC0 ^ g as u64, 4);
            // PRINT THE CAPS, not just the rate. These caps are DERIVED PER READING from a
            // wall-clock probe (`equal_time_caps` -> `ns_per_node`), so the operating point is
            // an output of the run, not a constant of it -- and until now it was never recorded.
            // That made successive readings unfalsifiable against each other: the control reported
            // 0.873 -> 0.871 -> 0.847 -> 0.819 across gens 100-400 and there was no way to ask
            // whether those four numbers were even played at the same node budget.
            //
            // `cap_stability.rs` then measured the caps as stable to 1.9% with the nets held
            // fixed, which REFUTES the reading that a moving budget produced that decline -- the
            // hypothesis was mine and its threshold was declared before the run. So this line is
            // not a fix for a live bug. It exists because the check cost one number and could not
            // be performed at all from the log, and the decline is still unexplained: whatever
            // does explain it, the next person should not have to rebuild the binary to rule this
            // out a second time.
            println!("      control vs origin @gen {g}: {}W-{}D-{}L  rate {:.3} +/- {:.3}  [{ca} vs {cb} nodes]{}",
                c.wins, c.draws, c.losses, c.pent_rate(), c.ci95(),
                if c.rate() - c.ci95() > 0.5 { "  *" } else { "" });

            // KEEP A LADDER RUNG. `--out` is a single file that every accept OVERWRITES, so the
            // run's history is destroyed as it is made and the only way to compare the champion
            // with its own past is to have copied a file by hand at the right moment.
            //
            // That matters now rather than in principle. The origin control is a SATURATING
            // measurement -- `instrument_saturation_RESULT.md` records it REVERSING SIGN twice, at
            // 0.861 and 0.967 -- and this loop's champion reached 0.873 on 2026-09-10, i.e. inside
            // the band where the instrument has already been wrong. Above that, "stronger" has to
            // be settled head-to-head against a RECENT ANCESTOR, which is only possible if the
            // ancestor still exists.
            //
            // Snapshotting HERE, at the control, is deliberate: every rung then carries a measured
            // origin score at the moment it was saved, so the ladder is a series of points with
            // both an absolute reading (while it still means something) and a playable net (after
            // it stops). `netmatch` takes two of these directly.
            //
            // Cost is one 50KB file per control -- every 100 generations as currently configured.
            if !out.is_empty() && out != "/dev/null" {
                let rung = rung_path(&out, &run_tag, g);
                if std::path::Path::new(&rung).exists() {
                    eprintln!("      WARNING: ladder rung {rung} already exists -- NOT overwriting. \
                               Pass --run-tag to keep this run's rungs distinct.");
                } else if let Err(e) = champion.save(&rung) {
                    eprintln!("      WARNING: could not write ladder rung {rung}: {e}");
                } else {
                    my_rungs.insert(rung.clone());
                    println!("      ladder rung saved: {rung}");
                }
            }

            // ANCESTOR CONTROL -- a progress reading that does NOT saturate.
            //
            // The origin control has stopped working on this champion, and the log now says so
            // outright: gen 400 and gen 800 both read 0.882 +/- 0.022. That is not a plateau, it is
            // the ceiling. `instrument_saturation_RESULT.md` records this instrument REVERSING SIGN
            // twice near the top of its scale -- blend 0.75-vs-1.00 at 0.861, and w16-vs-w64 at
            // 0.967 where it called w64 "far better" while head-to-head said w16. Against a FIXED
            // weak opponent, every net strong enough to win ~90% of the time looks identical,
            // because the remaining games are decided by the opponent's blunders rather than by the
            // difference under test.
            //
            // The fix was already written down in netmatch's header -- "above that, stronger has to
            // be settled head-to-head against a RECENT ANCESTOR" -- and was not implementable,
            // because `--out` was one file every accept overwrote and no ancestor survived. With
            // --rung-every they do, so the instrument can finally exist.
            //
            // A MOVING opponent cannot saturate: it improves at the same rate as the champion, so
            // the measured gap stays in the range where a match can resolve it. What it measures is
            // also the thing actually wanted -- "did the last L generations buy anything" -- rather
            // than "is it still far better than random", which was answered long ago.
            //
            // Deliberately NOT replacing the origin control. The origin is the only ABSOLUTE
            // reference in the project, comparable across runs and rebuilds; the ancestor is
            // relative and its opponent differs at every reading, so a rise cannot be summed into a
            // total. They answer different questions and both are cheap enough to keep.
            //
            // Equal-time caps for the same reason the origin control uses them, and gate_depth_cap
            // rather than the datagen depth because this project judges strength at depth 4.
            // CHECKPOINT AND ROLLBACK. Until now this control measured the lineage and then
            // ignored the answer -- it printed and did not even reach the ledger.
            //
            // WHY THIS IS THE RIGHT PLACE TO ACT, measured 2026-09-08 from 15,008 real gate
            // pairs (pair-score sd 0.2362): the per-generation gate at 224 pairs resolves only
            // ~21.5 Elo, and self-play steps are far smaller, so a 5-Elo improvement is invisible
            // to it BY CONSTRUCTION. Detecting individual steps at this budget is not achievable
            // -- 5 Elo needs ~29,400 pairs, about 22 hours per accept decision against a ~25s
            // generation. But DRIFT ACCUMULATES: many small wrong accepts compound into a gap
            // this control CAN see (1000 pairs resolves ~10 Elo), which is exactly what happened
            // in three of eight runs today -- they finished resolved WORSE than they started
            // (0.826, 0.834, 0.843 against 0.864) and nothing noticed.
            //
            // So: stop trying to catch each bad step, and catch the accumulated damage instead.
            // Roll back only when the drop is RESOLVED (rate + ci95 below the best seen), never
            // on a point estimate -- an unresolved dip is noise and reverting on it would throw
            // away real progress the control cannot see.
            let r = c.pent_rate();
            match &best_ctrl {
                Some((best_rate, best_net, best_gen)) if rollback && r + c.ci95() < *best_rate => {
                    println!("      ROLLBACK: {r:.3} +/- {:.3} is resolved below the gen-{best_gen} \
                              checkpoint {best_rate:.3}; restoring it", c.ci95());
                    champion = best_net.clone();
                    if anchor_pairs > 0 { champ_anchor = None; }
                }
                Some((best_rate, _, _)) if r <= *best_rate => {}
                _ => {
                    if best_ctrl.is_some() { println!("      checkpoint updated: {r:.3}"); }
                    best_ctrl = Some((r, champion.clone(), g));
                }
            }
        }
        println!(
            // `pool` is the size of what the TRAINER actually saw. Without it the log shows
            // positions GENERATED and this generation's slice, and the replay buffer -- the
            // thing --replay-gens varies -- is invisible. A smoke test comparing windows 1 and
            // 999 printed identical lines for exactly that reason, which is indistinguishable
            // from the flag not working. Show the quantity under test.
            "gen {g:>3}  pos {:>6}  train {:>5} pool {:>6} (h{:>3})  dec {:>3}/{:<3}  loss {:.4}  gate {}W-{}D-{}L {:.3}+/-{:.3}  {}  [{:.0}s]",
            data.len(), subset.len(), replay.len(), horizon, dec, dec + drawn, loss, sc.wins, sc.draws, sc.losses, sc.pent_rate(), sc.ci95(),
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
