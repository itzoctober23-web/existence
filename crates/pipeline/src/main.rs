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

/// Train a fresh net of `width` on `data`, stopping when held-out loss stops improving.
///
/// An ARCH candidate cannot inherit the champion's weights — it is a different shape — so it
/// starts from random init and has to catch up on the replay buffer. How long that takes is a
/// property of the width, so it is MEASURED (early stopping on the held-out split) rather than
/// fixed at a number that would quietly advantage whichever width the constant happened to suit.
fn train_fresh(
    width: usize, data: &[Sample], held: &[&Sample], tr: &Trainer, seed: u64, max_epochs: usize,
) -> (Net, f64, usize) {
    let mut net = Net::random(width, seed);
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
    // FITNESS 6 declares the fixed cost budget as "the cost of ~20k seed-program evaluations on
    // the seed net". 20k at P1 scale makes a single gate take minutes, so the budget is smaller
    // here and RECORDED rather than silently different; the ratio, not the absolute, is what
    // makes the clock gate mean anything.
    let cost_nodes = arg("--cost-nodes", 4000) as u64;
    // Depth for budgeted gates is a CEILING, not the thing that stops the search — the node
    // budget is. A depth-limited gate cannot charge a wide net for being slow, which is the
    // whole point of the ARCH clock gate (FITNESS 10: "bigger net that wins fixed-cost-budget,
    // loses on clock").
    let gate_depth_cap = arg("--gate-depth-cap", 6) as u32;
    let seed = arg("--seed", 20260907) as u64;
    let ctrl_every = arg("--control-every", 10);
    let out = a.iter().position(|x| x == "--out").and_then(|i| a.get(i + 1)).cloned()
        .unwrap_or_else(|| "champion.net".to_string());
    // Cap on the widening horizon. Measured 2026-09-07: labels far from the terminal are
    // ANTI-signal while play is weak (sign acc 0.452 -> 0.441 when training on all decided
    // positions). An unbounded schedule reaches 205 plies by gen 40, i.e. no filter at all,
    // which would reintroduce exactly that. This tests whether the plateau is self-inflicted.
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
    let blend: f32 = a.iter().position(|x| x == "--blend")
        .and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let tr = Trainer::new(0.01, blend);
    let mut rung = start_rung;
    println!("gens={gens} games/gen={games} depth={depth} epochs={epochs} gate-pairs={gate_pairs} blend={blend}");
    println!("ARCH menu {WIDTH_MENU:?}  start rung {rung} (width {})  arch-every {arch_every}  cost-budget {cost_nodes} nodes",
             WIDTH_MENU[rung]);
    let mut champion = Net::random(WIDTH_MENU[rung], seed);
    let origin = champion.clone();
    // The clock budget is fixed in TIME, measured once on the seed net, so that every later
    // width is charged its real cost against the same wall. Deriving it per-champion would let
    // a slow champion move the wall and hide its own cost.
    let seed_ns = arch::ns_per_node(&origin, depth.max(3), 5);
    let budget_ns = seed_ns * cost_nodes as f64;
    println!("clock budget: {cost_nodes} x {seed_ns:.0}ns = {:.2}ms per move (measured on the seed net)",
             budget_ns / 1e6);

    // Append-only, and NOT derived from `--out`: a ledger that a new run silently truncates is
    // not a record. The previous ARCH run's 6 rejections were lost exactly that way, to an
    // overwritten stdout log.
    let ledger = Ledger::new(
        &a.iter().position(|x| x == "--ledger").and_then(|i| a.get(i + 1)).cloned()
            .unwrap_or_else(|| "ledger.jsonl".to_string()),
    );

    let mut rng = Rng(seed);
    let mut accepted = 0;
    let mut arch_attempts = 0usize;
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
        let mut data: Vec<Sample> = Vec::new();
        let (mut dec, mut drawn) = (0, 0);
        let t0 = std::time::Instant::now();
        for _ in 0..games {
            let r = datagen::play_game(&champion, dgen_depth, &mut rng, 4, 160, &mut data);
            match r {
                board::Outcome::Loss => dec += 1,
                _ => drawn += 1,
            }
        }
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
        for e in 0..epochs {
            loss = tr.epoch(&mut cand, subset, seed ^ (g as u64) << 8 ^ e as u64);
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
        let sc = gate::match_nets(&cand, &champion, depth, gate_pairs, seed ^ g as u64);
        let _draw_rate = sc.draws as f64 / sc.games().max(1) as f64;
        // Resolution is a property of the INTERVAL, not the draw rate. Draw rate was a proxy
        // for width under the binomial; with pentanomial the same games give a ~5x tighter
        // interval, so a 90%-draw match can still decide. Test the thing directly.
        let gate_can_resolve = sc.ci95() < 0.05;
        let no_regression = sc.pent_rate() + sc.ci95() > 0.5;
        let better = if gate_can_resolve {
            sc.rate() - sc.ci95() > 0.5
        } else {
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
            surrogate: vec![("mcnemar_z", mcnemar), ("train_loss", loss as f64)],
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
                    train_fresh(p.width(), &replay, &astop, &tr, seed ^ 0xA5 ^ g as u64, 30);
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
                        gate_pairs, seed ^ 0xB6 ^ g as u64, 4);
                    let (ca, cb) = arch::equal_time_caps(&acand, &champion, budget_ns, depth.max(3));
                    let clock = gate::match_nets_capped(
                        &acand, &champion, gate_depth_cap, ca, cb,
                        gate_pairs, seed ^ 0xC7 ^ g as u64, 4);
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
        if g % ctrl_every == 0 {
            // EQUAL TIME, not equal depth: once the ARCH arm can change the champion's width,
            // a depth-matched control would hand a wider champion free computation and report
            // its extra cost as strength.
            let (ca, cb) = arch::equal_time_caps(&champion, &origin, budget_ns, depth.max(3));
            let c = gate::match_nets_capped(&champion, &origin, gate_depth_cap, ca, cb,
                                            gate_pairs, seed ^ 0xC0 ^ g as u64, 4);
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
