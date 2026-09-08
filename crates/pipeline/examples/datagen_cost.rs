//! Where does datagen time go? Labels per generation is the binding constraint, and labels
//! come from DECIDED games near the terminal. Two levers: games per second, and the fraction
//! of games that are decisive. Measure both before optimising either.
use nnue::Net;
use pipeline::datagen::{self, Rng, Sample};
use std::time::Instant;

fn main() {
    let net = Net::random(32, 20260907);
    for depth in [1u32, 2, 3] {
        let mut rng = Rng(5);
        let mut data: Vec<Sample> = Vec::new();
        let games = 60;
        let t0 = Instant::now();
        let mut dec = 0;
        for _ in 0..games {
            if datagen::play_game(&net, depth, &mut rng, 6, 160, &mut data) == board::Outcome::Loss {
                dec += 1;
            }
        }
        let el = t0.elapsed().as_secs_f64();
        let near = data.iter().filter(|s| s.z != 0.0 && s.plies_to_end <= 30).count();
        println!(
            "  depth {depth}  {:>5.1} games/s  decisive {:>2}/{games} ({:>3.0}%)  usable labels/s {:>6.1}",
            games as f64 / el, dec, 100.0 * dec as f64 / games as f64, near as f64 / el
        );
    }
    println!("\n  usable = decided AND within 30 plies of the terminal");
}
