//! A/B a TRAINING hyperparameter without the loop's path dependence.
//!
//! WHY NOT JUST RUN THE LOOP TWICE. Because the loop is path-dependent: whether a run happens
//! to accept a candidate at generation 3 changes every generation after it, so two arms sharing
//! a seed diverge the moment their gates disagree. MEASURED consequence — epochs-3 scored 0.641
//! and 0.792 on two seeds at IDENTICAL settings, a 0.151 spread on the control metric. That is
//! larger than most effects worth testing, and it means resolving a 0.03 effect through the
//! loop needs ~(0.15/0.03)^2 = 25 runs per arm.
//!
//! FITNESS 1 classes a training hyperparameter as HYPER, "judged by the NET it produces". That
//! is the whole design: fix ONE dataset from ONE champion, train N candidates per arm differing
//! only in the hyperparameter and the training seed, and gate each against that same champion.
//! No compounding, no path dependence, and the replicates are independent — so the interval
//! shrinks as sqrt(n) instead of being swamped by trajectory divergence.
//!
//! What it cannot tell you: whether an effect COMPOUNDS over generations. That needs the loop.
//! This answers "does this setting produce a better net from the same data", which is the
//! question that was actually being asked.

use nnue::Net;
use pipeline::datagen::{self, Rng, Sample};
use pipeline::gate;
use pipeline::trainer::Trainer;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let get = |k: &str, d: usize| -> usize {
        a.iter().position(|x| x == k).and_then(|i| a.get(i + 1))
            .and_then(|v| v.parse().ok()).unwrap_or(d)
    };
    let games = get("--games", 6000);
    let reps = get("--reps", 8);
    let epochs = get("--epochs", 3);
    let steps = get("--steps", 20000);
    let pairs = get("--pairs", 64);
    let depth = get("--depth", 2) as u32;
    let seed = get("--seed", 20260907) as u64;
    let width = get("--width", 16);
    // Which hyperparameter to compare. "epochs" contrasts training-step budgets on one filtered
    // dataset; "horizon" contrasts the DATA FILTER itself on one raw generation.
    let compare = a.iter().position(|x| x == "--compare").and_then(|i| a.get(i + 1))
        .cloned().unwrap_or_else(|| "epochs".into());
    let h_a = get("--horizon-a", 40) as u32;
    let h_b = get("--horizon-b", 1000) as u32;

    // A TRAINED champion changes the answer, not just the numbers: the horizon schedule widens
    // with generation BECAUSE labels far from the terminal become informative as play improves.
    // Measuring the optimum against a random net measures it at generation zero only.
    let champion = match a.iter().position(|x| x == "--net").and_then(|i| a.get(i + 1)) {
        Some(p) => match Net::load(p) {
            Ok(n) => { println!("champion: {p} (trained, width {})", n.n_hidden); n }
            Err(e) => { eprintln!("could not load {p}: {e}"); std::process::exit(2); }
        },
        None => { println!("champion: random width {width}"); Net::random(width, seed) }
    };
    println!("one shared dataset from {games} games\n");

    // ONE dataset, shared by every replicate of both arms. This is the variance the loop
    // could not control: same positions, same labels, same split.
    let mut rng = Rng(seed);
    let (data, dec) = datagen::play_games(&champion, depth, rng.next(), games, 4, 160, 4);
    // The RAW decided positions. The horizon filter is applied per-arm below when that is the
    // thing under test, so both arms see the same generated games.
    let raw: Vec<Sample> = data.into_iter().filter(|s| s.z != 0.0).collect();
    println!("{dec}/{games} decisive, {} decided positions before any horizon filter\n", raw.len());




    // BLEND sweep. The loop hardcodes blend = 0 with the stated reason that mixing the net's
    // own root score into its target is self-referential WHEN THE NET IS RANDOM. That premise
    // expires the moment the net is trained, and MASTER_PLAN lists "agreement with own deeper
    // search" as a Given objective — so whether it helps now is a measurement, not a doctrine.
    let blends: Vec<f32> = a.iter().position(|x| x == "--blends")
        .and_then(|i| a.get(i + 1))
        .map(|v| v.split(',').filter_map(|t| t.trim().parse().ok()).collect())
        .unwrap_or_default();
    let sweep: Vec<u32> = a.iter().position(|x| x == "--horizons")
        .and_then(|i| a.get(i + 1))
        .map(|v| v.split(',').filter_map(|t| t.trim().parse().ok()).collect())
        .unwrap_or_default();
    // DATAGEN DEPTH sweep. The loop's own comment says AlphaZero's engine of improvement is
    // that SEARCH(net) > net -- and --deepen-at defaults to 1,000,000, so datagen has ALWAYS
    // run at depth 2 and the deepening never happened. If the labels are not better than the
    // net that produced them there is nothing to bootstrap from, which is exactly what "no
    // horizon setting produces a gain" looks like from the outside.
    //
    // Same champion, same game count, same horizon; only the SEARCH DEPTH used to generate the
    // labels differs. Deeper costs more per game, so this measures label QUALITY at equal
    // games rather than at equal wall-clock -- the honest question is whether better labels
    // help at all before asking what they cost.
    let dg_depths: Vec<u32> = a.iter().position(|x| x == "--datagen-depths")
        .and_then(|i| a.get(i + 1))
        .map(|v| v.split(',').filter_map(|t| t.trim().parse().ok()).collect())
        .unwrap_or_default();
    if !dg_depths.is_empty() {
        let horizon = get("--horizon", 10) as u32;
        let dg_blend: f32 = a.iter().position(|x| x == "--blend")
            .and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(0.0);
        println!("datagen-depth sweep at horizon {horizon}, blend {dg_blend}, {games} games each\n");
        for d in &dg_depths {
            let mut r2 = Rng(seed ^ 0xDEE9);
            let (data, dec) = datagen::play_games(&champion, *d, r2.next(), games, 4, 160, 4);
            let owned: Vec<Sample> = data.into_iter()
                .filter(|s| s.z != 0.0 && s.plies_to_end <= horizon).collect();
            let cut = owned.len() * 3 / 4;
            let (train, _) = owned.split_at(cut);
            // Use the CONFIGURED blend, not 0. At blend = 0 the stored root score is unused,
            // so datagen depth can only change which games are played and never the label --
            // which is exactly what the first run of this sweep measured: depth 2 and depth 3
            // both returned 0.4703 despite depth 3 producing 63% decisive games against 28%.
            // The two knobs are coupled: deeper search is worth more precisely when the target
            // includes the search score.
            let tr = Trainer::new(0.01, dg_blend);
            let mut rates = Vec::new();
            for r in 0..reps {
                let tseed = seed ^ ((r as u64 + 1) << 32) ^ *d as u64;
                let mut cand = champion.clone();
                for e in 0..epochs { tr.epoch(&mut cand, train, tseed ^ e as u64); }
                let sc = gate::match_nets(&cand, &champion, depth, pairs, tseed ^ 0xA17E);
                rates.push(sc.pent_rate());
            }
            let n = rates.len() as f64;
            let mean = rates.iter().sum::<f64>() / n;
            let var = rates.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0).max(1.0);
            let se = (var / n).sqrt();
            println!("dg-depth {d}  {dec}/{games} decisive  {:>6} samples  MEAN {mean:.4}  \
                      se {se:.4}  95% [{:.4}, {:.4}]",
                     train.len(), mean - 1.96 * se, mean + 1.96 * se);
        }
        return;
    }

    if !blends.is_empty() {
        let horizon = get("--horizon", 10) as u32;
        let owned: Vec<Sample> = raw.iter().filter(|s| s.plies_to_end <= horizon).cloned().collect();
        let cut = owned.len() * 3 / 4;
        let (train, _) = owned.split_at(cut);
        println!("blend sweep at horizon {horizon}, {} samples\n", train.len());
        for b in &blends {
            let tr = Trainer::new(0.01, *b);
            let mut rates = Vec::new();
            for r in 0..reps {
                let tseed = seed ^ ((r as u64 + 1) << 32) ^ (*b * 1000.0) as u64;
                let mut cand = champion.clone();
                for e in 0..epochs { tr.epoch(&mut cand, train, tseed ^ e as u64); }
                let sc = gate::match_nets(&cand, &champion, depth, pairs, tseed ^ 0xA17E);
                rates.push(sc.pent_rate());
            }
            let n = rates.len() as f64;
            let mean = rates.iter().sum::<f64>() / n;
            let var = rates.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0).max(1.0);
            let se = (var / n).sqrt();
            println!("blend {b:<5}  MEAN {mean:.4}  sd {:.4}  se {se:.4}  95% [{:.4}, {:.4}]",
                     var.sqrt(), mean - 1.96 * se, mean + 1.96 * se);
        }
        return;
    }

    let tr = Trainer::new(0.01, 0.0);
    let arms: Vec<(String, u32, bool)> = if !sweep.is_empty() {
        sweep.iter().map(|h| (format!("horizon {h}"), *h, true)).collect()
    } else if compare == "horizon" {
        vec![(format!("horizon {h_a}"), h_a, true), (format!("horizon {h_b}"), h_b, true)]
    } else {
        vec![("epochs".into(), 40, true), ("steps".into(), 40, false)]
    };

    for (name, horizon, use_epochs) in arms {
        let pool: Vec<&Sample> = raw.iter().filter(|s| s.plies_to_end <= horizon).collect();
        let owned: Vec<Sample> = pool.into_iter().cloned().collect();
        let cut = owned.len() * 3 / 4;
        let (train, _held) = owned.split_at(cut);
        let mut rates = Vec::new();
        for r in 0..reps {
            let tseed = seed ^ ((r as u64 + 1) << 32) ^ name.len() as u64;
            let mut cand = champion.clone();
            if use_epochs {
                for e in 0..epochs { tr.epoch(&mut cand, train, tseed ^ e as u64); }
            } else {
                tr.steps(&mut cand, train, steps, tseed);
            }
            let sc = gate::match_nets(&cand, &champion, depth, pairs, tseed ^ 0xA17E);
            rates.push(sc.pent_rate());
        }
        let n = rates.len() as f64;
        let mean = rates.iter().sum::<f64>() / n;
        let var = rates.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0).max(1.0);
        let se = (var / n).sqrt();
        println!("{:<14} {:>7} samples  MEAN {mean:.4}  sd {:.4}  se {se:.4}  95% [{:.4}, {:.4}]",
                 name, train.len(), var.sqrt(), mean - 1.96 * se, mean + 1.96 * se);
    }

    println!("Both arms trained on the SAME data from the SAME champion, so a difference here \
              is the hyperparameter and not the trajectory.");
}
