//! Before changing the loop, measure WHY it is not learning.
use nnue::Net;
use pipeline::datagen::{self, Rng, Sample};

fn main() {
    let depth: u32 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(2);
    let games: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(30);
    let net = Net::random(128, 20260907);
    let mut rng = Rng(1);
    let mut data: Vec<Sample> = Vec::new();
    let (mut dec, mut tot) = (0, 0);
    for _ in 0..games {
        let r = datagen::play_game(&net, depth, &mut rng, 4, 160, &mut data);
        tot += 1;
        if r == board::Outcome::Loss { dec += 1; }
    }
    let nonzero = data.iter().filter(|s| s.z != 0.0).count();
    let mar: f64 = data.iter().map(|s| (s.root as f64).abs()).sum::<f64>() / data.len().max(1) as f64;
    println!("  depth {depth}, {games} games");
    println!("  decisive games   : {dec}/{tot} ({:.0}%)", 100.0 * dec as f64 / tot as f64);
    println!("  positions        : {}", data.len());
    println!("  NONZERO labels   : {nonzero} ({:.1}%)  <- the learnable fraction",
        100.0 * nonzero as f64 / data.len().max(1) as f64);
    println!("  mean |root|      : {mar:.0}");
}
