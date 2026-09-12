//! What kind of positions does a NET steer its own self-play into?
//!
//! `budget_harm_is_emergent_RESULT.md` eliminated every single-pass explanation for Candidate A: the
//! label channel is null, the position channel is null, per-move quality does not resolve, and
//! reallocation is structurally impossible. The harm only appears when a net trained on budget data
//! generates the next generation's data, 2000 times. Three mechanisms could do that, and the first is
//! DRIFT: the trajectory distribution moves somewhere the evaluation is worse, and the net chases it.
//!
//! This measures the distribution directly. Every net plays at **fixed depth 3**, so the search is
//! identical across arms and any shift belongs to the net. Comparing
//!
//! ```text
//!   cand_start.net     the frozen champion both arms began from
//!   candA2_fixed.net   2000 generations of fixed-depth training
//!   candB2_budget.net  2000 generations of node-budget training
//! ```
//!
//! answers: does the budget-trained net steer into systematically different positions than the
//! depth-trained one? If the width distributions separate, drift is live. If they coincide, drift is
//! not the mechanism and the remaining candidates are variance amplification and decisiveness
//! feedback.
//!
//! Openings are drawn from ONE rng stream seeded identically for every net, so all arms start from
//! the same set of positions and only their own play differs.
//!
//! Usage: traj_profile [net] [games] [seed]

use board::Position;
use nnue::Net;
use pipeline::datagen::Rng;
use pipeline::search::Searcher;

fn arg<T: std::str::FromStr>(i: usize, d: T) -> T {
    std::env::args().nth(i).and_then(|v| v.parse().ok()).unwrap_or(d)
}

const NB: usize = 5;
const EDGES: [usize; 4] = [12, 20, 28, 36];
const NAMES: [&str; NB] = ["<=12", "13-20", "21-28", "29-36", ">36"];
fn bucket(w: usize) -> usize { EDGES.iter().position(|&e| w <= e).unwrap_or(NB - 1) }

fn main() {
    let net_path = std::env::args().nth(1).unwrap_or_else(|| "cand_start.net".into());
    let games: usize = arg(2, 60);
    let seed: u64 = arg(3, 20260912);

    let net = match Net::load(&net_path) {
        Ok(n) => n,
        Err(e) => { eprintln!("cannot load {net_path}: {e}"); std::process::exit(1); }
    };

    let mut rng = Rng(seed);
    let mut b_n = [0usize; NB];
    let mut widths: Vec<f64> = Vec::new();
    let (mut decisive, mut plies) = (0usize, 0usize);

    for _ in 0..games {
        let mut pos = Position::startpos();
        let mut s = Searcher::with_seed(rng.next());
        for _ in 0..6 {
            let l = pos.legal_moves();
            if l.is_empty() { break; }
            let m = l.as_slice()[rng.below(l.len())];
            pos.make_move(m);
        }
        for ply in 0..160 {
            let l = pos.legal_moves();
            if l.is_empty() { decisive += 1; break; }
            if pos.halfmove >= 100 { break; }
            let (mv, _sc) = s.best_move(&mut pos, 3, &net);
            if mv == board::types::MOVE_NONE { break; }
            b_n[bucket(l.len())] += 1;
            widths.push(l.len() as f64);
            plies += 1;
            let m = if ply < 6 && rng.next() % 4 == 0 { l.as_slice()[rng.below(l.len())] } else { mv };
            pos.make_move(m);
        }
    }

    let n = widths.len() as f64;
    let mean = widths.iter().sum::<f64>() / n;
    let var = widths.iter().map(|w| (w - mean).powi(2)).sum::<f64>() / (n - 1.0);
    println!("  net {net_path}  ({games} games, fixed depth 3, seed {seed})");
    println!("  positions {}  mean width {:.2} +/- {:.2} (95% CI on the mean)  sd {:.2}",
             widths.len(), mean, 1.96 * (var / n).sqrt(), var.sqrt());
    println!("  decisive {}/{} = {:.1}%   plies/game {:.1}",
             decisive, games, 100.0 * decisive as f64 / games as f64, plies as f64 / games as f64);
    print!("  width mix ");
    for i in 0..NB {
        print!("{} {:.1}%  ", NAMES[i], 100.0 * b_n[i] as f64 / n);
    }
    println!();
}
