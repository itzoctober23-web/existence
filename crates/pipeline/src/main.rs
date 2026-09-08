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
        let mut data: Vec<Sample> = Vec::new();
        let (mut dec, mut drawn) = (0, 0);
        let t0 = std::time::Instant::now();
        for _ in 0..games {
            let r = datagen::play_game(&champion, depth, &mut rng, 4, 160, &mut data);
            match r {
                board::Outcome::Loss => dec += 1,
                _ => drawn += 1,
            }
        }
        let t_gen = t0.elapsed().as_secs_f64();

        // ---- train a candidate from the champion
        let mut cand = champion.clone();
        let mut loss = 0.0;
        for e in 0..epochs {
            loss = tr.epoch(&mut cand, &data, seed ^ (g as u64) << 8 ^ e as u64);
        }

        // ---- GATE: the only thing that decides
        let sc = gate::match_nets(&cand, &champion, depth, gate_pairs, seed ^ g as u64);
        let better = sc.rate() - sc.ci95() > 0.5;
        if better {
            champion = cand;
            accepted += 1;
        }
        println!(
            "gen {g:>3}  pos {:>6}  decisive {:>3}/{:<3}  loss {:.4}  gate {}W-{}D-{}L rate {:.3}+/-{:.3}  {}  [{:.0}s]",
            data.len(), dec, dec + drawn, loss, sc.wins, sc.draws, sc.losses, sc.rate(), sc.ci95(),
            if better { "ACCEPT" } else { "reject" }, t_gen
        );
    }

    // ---- the control that proves learning happened at all
    if accepted > 0 {
        let sc = gate::match_nets(&champion, &origin, depth, gate_pairs * 2, seed ^ 0xFFFF);
        println!(
            "\nCONTROL  final champion vs the ORIGINAL random net: {}W-{}D-{}L  rate {:.3} +/- {:.3}",
            sc.wins, sc.draws, sc.losses, sc.rate(), sc.ci95()
        );
        println!(
            "  => {}",
            if sc.rate() - sc.ci95() > 0.5 {
                "LEARNED: beats its own random initialisation with the interval clear of 0.5"
            } else {
                "NOT PROVEN: interval includes 0.5, so no learning is demonstrated"
            }
        );
    } else {
        println!("\nno candidate was accepted; nothing to control against");
    }
}
