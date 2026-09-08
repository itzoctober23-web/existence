//! Capacity is not the explanatory variable: 4, 16 and 128 hidden all collapse to the same
//! near-zero held-out correlation. The remaining suspect is the LABEL DISTRIBUTION -- ~85% of
//! training targets are exactly 0, so predicting the mean is the cheapest solution available.
//!
//! Single variable: train on ALL positions vs train on DECIDED positions only. Same net, same
//! seeds, same held-out set, same epochs.
use nnue::Net;
use pipeline::datagen::{self, Rng, Sample};
use pipeline::trainer::Trainer;
use board::{Color, Position};

fn corr(net: &Net, data: &[Sample]) -> f64 {
    let mut s = Vec::new();
    let (mut xs, mut ys) = (Vec::new(), Vec::new());
    for d in data {
        if d.z == 0.0 { continue; }
        let p = match Position::from_fen(&d.fen) { Ok(p) => p, Err(_) => continue };
        xs.push(net.eval(&p, &mut s) as f64);
        ys.push(if p.stm == Color::White { d.z } else { -d.z } as f64);
    }
    let n = xs.len() as f64;
    if n < 10.0 { return 0.0; }
    let (mx, my) = (xs.iter().sum::<f64>() / n, ys.iter().sum::<f64>() / n);
    let (mut num, mut dx, mut dy) = (0.0, 0.0, 0.0);
    for i in 0..xs.len() {
        let (a, b) = (xs[i] - mx, ys[i] - my);
        num += a * b; dx += a * a; dy += b * b;
    }
    if dx <= 0.0 || dy <= 0.0 { return 0.0; }
    num / (dx.sqrt() * dy.sqrt())
}

fn main() {
    let games: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(300);
    let gen_net = Net::random(128, 20260907);
    let mut rng = Rng(11);
    let mut all: Vec<Sample> = Vec::new();
    for _ in 0..games { datagen::play_game(&gen_net, 2, &mut rng, 6, 160, &mut all); }
    let mut held: Vec<Sample> = Vec::new();
    for _ in 0..(games / 3) { datagen::play_game(&gen_net, 2, &mut rng, 6, 160, &mut held); }

    let decided: Vec<Sample> = all.iter()
        .filter(|s| s.z != 0.0)
        .map(|s| Sample { fen: s.fen.clone(), z: s.z, root: s.root, plies_to_end: s.plies_to_end })
        .collect();
    println!("  all {} pos, decided-only {} pos, held-out decided {}",
        all.len(), decided.len(), held.iter().filter(|s| s.z != 0.0).count());

    let tr = Trainer::new(0.02, 0.0);
    const SEEDS: usize = 6;
    for (name, data) in [("ALL positions (~85% zeros)", &all), ("DECIDED only", &decided)] {
        let (mut b, mut a) = (Vec::new(), Vec::new());
        for s in 0..SEEDS {
            let mut net = Net::random(32, 1000 + s as u64 * 7919);
            b.push(corr(&net, &held));
            for e in 1..=6 { tr.epoch(&mut net, data, 99 + e as u64); }
            a.push(corr(&net, &held));
        }
        let stat = |v: &Vec<f64>| {
            let n = v.len() as f64;
            let m = v.iter().sum::<f64>() / n;
            let sd = (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (n - 1.0)).sqrt();
            (m, sd / n.sqrt())
        };
        let (bm, bs) = stat(&b);
        let (am, asem) = stat(&a);
        let sig = (am - bm) > 1.96 * (bs * bs + asem * asem).sqrt();
        println!("  {name:<28} before {bm:>+7.4}+/-{bs:.4}   after {am:>+7.4}+/-{asem:.4}{}",
            if sig { "   IMPROVED" } else { "" });
    }
}
