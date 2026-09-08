//! Does training move the eval toward the OUTCOME on held-out positions?
//!
//! The game-gate cannot answer this at iteration zero: two wandering nets draw 86-100% of
//! their games, so a 60-game match has a +/-0.13 interval and resolves nothing. This is the
//! held-out surrogate FITNESS 5 specifies, and it is far more sensitive: correlation between
//! the net's eval and the eventual result, on data it never trained on.
//!
//! Correlation ~0 before and clearly positive after = the net learned something real.
use nnue::Net;
use pipeline::datagen::{self, Rng, Sample};
use pipeline::trainer::Trainer;
use board::{Color, Position};

fn corr(net: &Net, data: &[Sample]) -> f64 {
    let mut s = Vec::new();
    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    for d in data {
        if d.z == 0.0 { continue; } // only decided positions carry a direction
        let p = match Position::from_fen(&d.fen) { Ok(p) => p, Err(_) => continue };
        let e = net.eval(&p, &mut s) as f64;
        let mover_z = if p.stm == Color::White { d.z } else { -d.z } as f64;
        xs.push(e);
        ys.push(mover_z);
    }
    let n = xs.len() as f64;
    if n < 10.0 { return 0.0; }
    let (mx, my) = (xs.iter().sum::<f64>() / n, ys.iter().sum::<f64>() / n);
    let mut num = 0.0; let (mut dx, mut dy) = (0.0, 0.0);
    for i in 0..xs.len() {
        let a = xs[i] - mx; let b = ys[i] - my;
        num += a * b; dx += a * a; dy += b * b;
    }
    if dx <= 0.0 || dy <= 0.0 { return 0.0; }
    num / (dx.sqrt() * dy.sqrt())
}

fn main() {
    let games: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(120);
    let epochs: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(12);
    let net0 = Net::random(128, 20260907);
    let mut rng = Rng(7);

    let mut train: Vec<Sample> = Vec::new();
    for _ in 0..games { datagen::play_game(&net0, 2, &mut rng, 6, 160, &mut train); }
    let mut held: Vec<Sample> = Vec::new();
    for _ in 0..(games / 3).max(10) { datagen::play_game(&net0, 2, &mut rng, 6, 160, &mut held); }

    let dec_tr = train.iter().filter(|s| s.z != 0.0).count();
    let dec_he = held.iter().filter(|s| s.z != 0.0).count();
    println!("  train {} pos ({} decided)   held-out {} pos ({} decided)",
        train.len(), dec_tr, held.len(), dec_he);

    let mut net = net0.clone();
    let tr = Trainer::new(0.02, 0.0);
    println!("  epoch  train-loss   held-out corr(eval, outcome)");
    let c0 = corr(&net, &held);
    println!("  {:>5}  {:>10}   {:+.4}   <- before any training", 0, "-", c0);
    for e in 1..=epochs {
        let l = tr.epoch(&mut net, &train, 1234 + e as u64);
        if e % 2 == 0 || e == 1 {
            println!("  {:>5}  {:>10.5}   {:+.4}", e, l, corr(&net, &held));
        }
    }
    // The verdict must compare AFTER against BEFORE, with an interval. An absolute threshold
    // is a tautology: the random net already scores ~+0.20 here, because its eval is a fixed
    // function and losing sides tend to have fewer pieces, so any fixed function correlates a
    // little. Fisher z gives the interval on a correlation.
    let c1 = corr(&net, &held);
    let n = dec_he as f64;
    let z = |r: f64| 0.5 * ((1.0 + r) / (1.0 - r)).ln();
    let se = if n > 4.0 { (1.0 / (n - 3.0)).sqrt() } else { 1.0 };
    let dz = z(c1) - z(c0);
    let dz_ci = 1.96 * se * 2f64.sqrt();      // difference of two z's on the same held-out set
    println!("\n  before {c0:+.4}   after {c1:+.4}   delta-z {dz:+.4} +/- {dz_ci:.4}  (n={dec_he} decided)");
    println!("  => {}", if dz - dz_ci > 0.0 {
        "LEARNING: the improvement is clear of its own interval"
    } else {
        "NOT DEMONSTRATED: the change is inside the noise band. Either it is not learning, or the held-out set is too small to tell."
    });
}
