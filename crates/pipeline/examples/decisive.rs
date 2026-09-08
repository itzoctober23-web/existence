//! WHY DO THE GAMES DRAW? The binding constraint on the whole loop, measured.
//!
//! The ARCH run reached generations where self-play produced `dec 0/80` -- zero decisive games
//! out of eighty -- and every gate in that run came back 0.500 +/- 0.06, i.e. carrying no
//! information at all. A loop whose gate cannot resolve is running on its surrogate, and the
//! surrogate is a proxy nobody has shown tracks strength.
//!
//! "Draw" is three different failures wearing one label:
//!   STALEMATE  - real chess, nothing to fix
//!   FIFTY-MOVE - both sides shuffling; the eval has no opinion that survives 50 moves
//!   PLY CAP    - the harness gave up at max_plies; the game was still going
//! They call for completely different responses, so the first job is to tell them apart.
//!
//! Then the lever: FITNESS 7.3 permits random-ply openings until the self-generated book
//! exists. Deeper random walks create material imbalance, and imbalance is what makes a game
//! decisive. This sweeps the opening depth and reports the decisive rate, so the choice is
//! measured rather than picked.

use nnue::Net;
use pipeline::datagen::{self, GameEnd, Rng, Sample};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let net_path = args.iter().position(|a| a == "--net").and_then(|i| args.get(i + 1));
    // Measure the ACTUAL champion where one exists. A random net draws for a different reason
    // (no opinion at all) than a trained one (an opinion too weak to convert), and it is the
    // trained case the loop is stuck in.
    let net = match net_path {
        Some(p) => match Net::load(p) {
            Ok(n) => { println!("net: {p}  hidden {}", n.n_hidden); n }
            Err(e) => { println!("could not load {p} ({e}); using random 256"); Net::random(256, 7) }
        },
        None => { println!("no --net given; using random 256"); Net::random(256, 7) }
    };
    let games: usize = args.iter().position(|a| a == "--games")
        .and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(60);
    let depth: u32 = args.iter().position(|a| a == "--depth")
        .and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(2);
    let max_plies: usize = args.iter().position(|a| a == "--max-plies")
        .and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok()).unwrap_or(160);

    println!("games/arm {games}  depth {depth}  ply cap {max_plies}\n");
    println!("{:>5}  {:>8}  {:>6} {:>6} {:>6} {:>6}  {:>9}  {:>7}",
             "open", "decisive", "mate", "stale", "50mv", "cap", "labels/g", "sec");

    for open in [4usize, 8, 12, 16, 20, 24, 32] {
        let mut rng = Rng(0xD1CE ^ open as u64);
        let mut data: Vec<Sample> = Vec::new();
        let (mut mate, mut stale, mut fifty, mut cap) = (0, 0, 0, 0);
        let t0 = std::time::Instant::now();
        for _ in 0..games {
            let before = data.len();
            let (_, how) = datagen::play_game_ext(&net, depth, &mut rng, open, max_plies, &mut data);
            let _ = before;
            match how {
                GameEnd::Mate => mate += 1,
                GameEnd::Stalemate => stale += 1,
                GameEnd::FiftyMove => fifty += 1,
                GameEnd::PlyCap => cap += 1,
            }
        }
        let secs = t0.elapsed().as_secs_f64();
        println!("{:>5}  {:>7.1}%  {:>6} {:>6} {:>6} {:>6}  {:>9.1}  {:>7.1}",
                 open, 100.0 * mate as f64 / games as f64, mate, stale, fifty, cap,
                 data.len() as f64 / games as f64, secs);
    }

    println!("\nDecisive games are the only ones that carry a label the trainer can use \
              (main.rs filters on `s.z != 0.0`), so the decisive rate IS the label rate.");
}
