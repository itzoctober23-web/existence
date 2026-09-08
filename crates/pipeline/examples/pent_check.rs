//! Does pentanomial actually tighten the interval on the SAME games? FITNESS 7.3 claims
//! pairing cancels opening bias and reduces variance at any draw rate. Verify rather than
//! assume: same match, same seed, both statistics.
use nnue::Net;
use pipeline::gate;

fn main() {
    let a = Net::random(32, 111);
    let b = Net::random(32, 222);
    for pairs in [40usize, 80] {
        let sc = gate::match_nets(&a, &b, 2, pairs, 0xABC);
        let n = sc.games().max(1) as f64;
        let p = sc.rate();
        let binom = 1.96 * (p * (1.0 - p) / n).sqrt();
        println!("  {pairs} pairs ({} games)  {}W-{}D-{}L  pent {:?}",
            sc.games(), sc.wins, sc.draws, sc.losses, sc.pent);
        println!("      trinomial  rate {:.4}  +/- {:.4}", p, binom);
        println!("      pentanomial rate {:.4}  +/- {:.4}   ({:.2}x tighter)",
            sc.pent_rate(), sc.ci95(), binom / sc.ci95().max(1e-9));
    }
}
