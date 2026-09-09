//! Does `match_progs_sprt` actually reach a verdict? The path is new and, until an arm hits a gate
//! call, entirely unexercised -- a bug in it would surface only after hours of evolve time.
//!
//! Two checks, both of which must pass or the sequential gate is not a gate:
//!   1. A/A (a program against ITSELF) must NOT accept. The true difference is zero, so an accept
//!      here means the stopping rule fires on noise.
//!   2. A/B against a WEAKER program must reach a verdict, and reach it EARLY -- the whole claim of
//!      going sequential is that an obvious dud costs few pairs, not the cap.
use grammar::reference;
use nnue::Net;
use pipeline::gate;
use board::Position;

fn main() {
    let pairs: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(60);
    let depth: i64 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(3);
    let net = Net::random(32, 20260907);
    let t = vec![depth, 32_000, interp::uct_exploration()];
    let seed_p = reference::bare_alpha_beta();
    let weak = reference::depth_one();          // 9 nodes vs 71: unambiguously worse
    let mut rng = grammar::mutate::Rng::new(0x5A17);
    let openings: Vec<Position> = (0..pairs).map(|_| {
        let mut o = Position::startpos();
        for _ in 0..4 {
            let l = o.legal_moves();
            if l.is_empty() { break }
            let m = l.as_slice()[(rng.next() as usize) % l.as_slice().len()];
            let _ = o.make_move(m);
        }
        o
    }).collect();

    println!("sprt smoke — bounds [0,10], LLR stop +/-2.944, cap {pairs} pairs, depth {depth}");
    for (label, a, b) in [("A/A  seed vs ITSELF (must NOT accept)", &seed_p, &seed_p),
                          ("A/B  seed vs depth_one (must decide)", &seed_p, &weak)] {
        let (v, sc, llr) = gate::match_progs_sprt(a, b, &net, t.clone(), 16, 0x5A17,
                                                  &openings, u64::MAX, 0.0, 10.0, pairs);
        let n: u32 = sc.pent.iter().sum();
        println!("  {label:<38} {:?}  llr {llr:+.2}  after {n} pairs  W-D-L {}-{}-{}",
                 v, sc.wins, sc.draws, sc.losses);
    }
    println!("\n  An A/A that ACCEPTS means the stopping rule fires on noise.");
    println!("  An A/B that runs to the cap means the gate cannot resolve even an obvious dud.");
}
