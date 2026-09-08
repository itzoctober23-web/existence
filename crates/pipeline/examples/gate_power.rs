//! The gate must be able to SEE a difference before it can decide anything. With ~95% draws a
//! 60-game match has a +/-0.13 interval, which no realistic improvement clears. Measure the
//! draw rate as a function of how far the random opening walks: deeper random openings produce
//! material imbalance, and an imbalanced start is convertible, which is what makes a result.
use nnue::Net;
use pipeline::datagen::Rng;
use pipeline::gate;

fn main() {
    let a = Net::random(128, 1);
    let b = Net::random(128, 2); // a DIFFERENT random net: any measured edge is noise
    for open in [4usize, 8, 12, 16, 20] {
        let sc = gate::match_nets_open(&a, &b, 2, 25, 0xBEEF, open);
        println!(
            "  opening {open:>2} plies -> {}W-{}D-{}L  draws {:>3.0}%  rate {:.3} +/- {:.3}",
            sc.wins, sc.draws, sc.losses,
            100.0 * sc.draws as f64 / sc.games().max(1) as f64,
            sc.rate(), sc.ci95()
        );
    }
    println!("\n  a decisive gate is the precondition for any acceptance decision");
}
