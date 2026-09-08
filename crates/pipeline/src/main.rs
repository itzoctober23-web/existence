//! The learning loop.
//!
//!   datagen (self-play with the champion)  ->  train a candidate  ->  GATE it vs the champion
//!   accept only if it wins the match; otherwise keep the champion and try again.
//!
//! The training loss is NOT evidence. Only the gate decides (MASTER_PLAN: SPRT decides,
//! surrogate proposes). Loss falling while strength does nothing is the exact failure this
//! project has already seen elsewhere.

use nnue::Net;
use pipeline::datagen::{self, Rng, Sample};
use pipeline::gate;
use pipeline::trainer::Trainer;
use board::{Color, Position};

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
    let hidden = arg("--hidden", 128);
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
    println!("gens={gens} games/gen={games} depth={depth} epochs={epochs} gate-pairs={gate_pairs} hidden={hidden} blend={blend}");
    let mut champion = Net::random(hidden, seed);
    let origin = champion.clone();

    let mut rng = Rng(seed);
    let mut accepted = 0;

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
        let draw_rate = sc.draws as f64 / sc.games().max(1) as f64;
        let gate_can_resolve = draw_rate < 0.60;
        let no_regression = sc.pent_rate() + sc.ci95() > 0.5;
        let better = if gate_can_resolve {
            sc.rate() - sc.ci95() > 0.5
        } else {
            mcnemar > 1.96 && no_regression
        };
        if better {
            champion = cand;
            accepted += 1;
            // Persist on every acceptance, not at the end: a run killed by a timeout used to
            // discard everything it had learned.
            if let Err(e) = champion.save(&out) {
                eprintln!("  WARN could not save champion to {out}: {e}");
            }
        }
        // Periodic control against the FROZEN origin. One step of learning is not a curve:
        // the question P1 turns on is whether strength COMPOUNDS or stops after generation 1.
        // Measured against the same fixed opponent every time, so the numbers are comparable.
        if g % ctrl_every == 0 {
            let c = gate::match_nets(&champion, &origin, depth, gate_pairs, seed ^ 0xC0 ^ g as u64);
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
    if accepted > 0 {
        let sc = gate::match_nets(&champion, &origin, depth, gate_pairs * 2, seed ^ 0xFFFF);
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
