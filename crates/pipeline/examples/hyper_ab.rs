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

    let champion = Net::random(width, seed);
    println!("champion: random width {width}; one shared dataset from {games} games\n");

    // ONE dataset, shared by every replicate of both arms. This is the variance the loop
    // could not control: same positions, same labels, same split.
    let mut rng = Rng(seed);
    let (data, dec) = datagen::play_games(&champion, depth, rng.next(), games, 4, 160, 4);
    // The RAW decided positions. The horizon filter is applied per-arm below when that is the
    // thing under test, so both arms see the same generated games.
    let raw: Vec<Sample> = data.into_iter().filter(|s| s.z != 0.0).collect();
    println!("{dec}/{games} decisive, {} decided positions before any horizon filter\n", raw.len());

    let tr = Trainer::new(0.01, 0.0);


    let sweep: Vec<u32> = a.iter().position(|x| x == "--horizons")
        .and_then(|i| a.get(i + 1))
        .map(|v| v.split(',').filter_map(|t| t.trim().parse().ok()).collect())
        .unwrap_or_default();
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
