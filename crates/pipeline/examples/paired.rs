//! Does training improve the eval on held-out positions? PAIRED test.
//!
//! Comparing two correlations throws the pairing away: the same positions are scored before
//! and after, and those scores are highly correlated, so the difference has far less variance
//! than either correlation does. The earlier +/-0.118 interval was conservative to the point
//! of being useless. Here every held-out decided position contributes one paired difference,
//! and the interval comes from the differences themselves.
//!
//! Two statistics, both paired:
//!   squared error   (pred - target)^2 before vs after
//!   sign agreement  does the eval have the same sign as the result? (McNemar)
use board::{Color, Position};
use nnue::Net;
use pipeline::datagen::{self, Rng, Sample};
use pipeline::trainer::Trainer;

/// White-POV prediction in [-1,1] — the frame the net's raw output actually lives in.
fn pred_white(net: &Net, p: &Position, s: &mut Vec<f32>) -> f32 {
    let mover = net.eval(p, s) as f32 / net.scale;
    let white = if p.stm == Color::White { mover } else { -mover };
    white.tanh()
}

fn main() {
    let games: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(400);
    let epochs: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(6);
    let gen_net = Net::random(128, 20260907);
    let mut rng = Rng(23);

    let mut train: Vec<Sample> = Vec::new();
    for _ in 0..games { datagen::play_game(&gen_net, 2, &mut rng, 6, 160, &mut train); }
    let mut held: Vec<Sample> = Vec::new();
    for _ in 0..(games / 3) { datagen::play_game(&gen_net, 2, &mut rng, 6, 160, &mut held); }
    let held: Vec<&Sample> = held.iter().filter(|s| s.z != 0.0).collect();
    println!("  train {} pos ({} decided)   held-out decided {}",
        train.len(), train.iter().filter(|s| s.z != 0.0).count(), held.len());

    // Single variable: how far from the end may a training position be?
    let cutoffs = [400usize, 60, 30, 10];
    println!("\n  {:>10} {:>9} {:>22} {:>16}", "max plies", "train n", "paired sq-err improve", "sign acc b->a");
    for cut in cutoffs {
        let subset: Vec<Sample> = train.iter()
            .filter(|s| s.z != 0.0 && (s.plies_to_end as usize) <= cut)
            .map(|s| Sample { fen: s.fen.clone(), z: s.z, root: s.root, plies_to_end: s.plies_to_end })
            .collect();
        if subset.len() < 50 { continue; }
        let mut net = Net::random(32, 777);
        let mut s2 = Vec::new();
        let snap = |net: &Net, s: &mut Vec<f32>| -> Vec<(f32, f32)> {
            held.iter().filter_map(|h| {
                let p = Position::from_fen(&h.fen).ok()?;
                Some((pred_white(net, &p, s), h.z))
            }).collect()
        };
        let b = snap(&net, &mut s2);
        let tr2 = Trainer::new(0.02, 0.0);
        for e in 1..=epochs { tr2.epoch(&mut net, &subset, 31 + e as u64); }
        let a = snap(&net, &mut s2);
        let d: Vec<f64> = b.iter().zip(&a).map(|((pb, z), (pa, _))| {
            let eb = (*pb - *z) as f64; let ea = (*pa - *z) as f64; eb * eb - ea * ea
        }).collect();
        let n2 = d.len() as f64;
        let m2 = d.iter().sum::<f64>() / n2;
        let sd2 = (d.iter().map(|x| (x - m2) * (x - m2)).sum::<f64>() / (n2 - 1.0)).sqrt();
        let sok = |p: f32, z: f32| (p > 0.0) == (z > 0.0);
        let acc2 = |v: &Vec<(f32, f32)>| v.iter().filter(|(p, z)| sok(*p, *z)).count() as f64 / n2;
        println!("  {:>10} {:>9} {:>+13.5} +/-{:.5} {:>8.3}->{:.3}{}",
            if cut >= 400 { "all".to_string() } else { cut.to_string() },
            subset.len(), m2, 1.96 * sd2 / n2.sqrt(), acc2(&b), acc2(&a),
            if m2 - 1.96 * sd2 / n2.sqrt() > 0.0 { "  *" } else { "" });
    }
    println!("  * = paired improvement clear of its interval\n");

    let mut net = Net::random(32, 777);
    let mut s = Vec::new();
    let snapshot = |net: &Net, s: &mut Vec<f32>| -> Vec<(f32, f32)> {
        held.iter().filter_map(|h| {
            let p = Position::from_fen(&h.fen).ok()?;
            Some((pred_white(net, &p, s), h.z))
        }).collect()
    };

    let before = snapshot(&net, &mut s);
    let tr = Trainer::new(0.02, 0.0);
    for e in 1..=epochs { tr.epoch(&mut net, &train, 31 + e as u64); }
    let after = snapshot(&net, &mut s);

    // paired squared-error difference
    let d: Vec<f64> = before.iter().zip(&after)
        .map(|((pb, z), (pa, _))| {
            let eb = (*pb - *z) as f64; let ea = (*pa - *z) as f64;
            eb * eb - ea * ea            // positive = after is better
        }).collect();
    let n = d.len() as f64;
    let m = d.iter().sum::<f64>() / n;
    let sd = (d.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / (n - 1.0)).sqrt();
    let sem = sd / n.sqrt();

    // paired sign agreement (McNemar-style)
    let sign_ok = |p: f32, z: f32| (p > 0.0) == (z > 0.0);
    let (mut b_only, mut a_only) = (0u32, 0u32);
    for ((pb, z), (pa, _)) in before.iter().zip(&after) {
        match (sign_ok(*pb, *z), sign_ok(*pa, *z)) {
            (true, false) => b_only += 1,
            (false, true) => a_only += 1,
            _ => {}
        }
    }
    let acc = |v: &Vec<(f32, f32)>| v.iter().filter(|(p, z)| sign_ok(*p, *z)).count() as f64 / n;
    let mcn = if b_only + a_only > 0 {
        (a_only as f64 - b_only as f64).abs() / ((a_only + b_only) as f64).sqrt()
    } else { 0.0 };

    println!("\n  PAIRED squared-error improvement : {m:+.5} +/- {:.5} (95%)", 1.96 * sem);
    println!("  sign accuracy   before {:.3}  ->  after {:.3}", acc(&before), acc(&after));
    println!("  McNemar         after-only {a_only}, before-only {b_only}, z = {mcn:.2}");
    println!("\n  => {}", if m - 1.96 * sem > 0.0 {
        "LEARNED: paired error improvement is clear of its interval"
    } else if m + 1.96 * sem < 0.0 {
        "WORSE: training degraded the eval, clear of the interval"
    } else {
        "NOT DEMONSTRATED: paired difference straddles zero"
    });
}
