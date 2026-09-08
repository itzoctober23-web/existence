//! Play ONE game, champion vs the frozen origin, and print the moves.
//!
//! WHY. Every number in EXPERIMENTS.md is aggregate -- rates, intervals, decisive counts. None of
//! them answer "is it playing chess or shuffling pieces", and that is a fair question to ask of a
//! net that started as noise. Aggregates can look healthy while the play is nonsense: a 0.838
//! score against a RANDOM opponent is exactly what a merely-not-random player would also score.
//!
//! So this prints the actual game and, for each position, whether the champion's chosen move was
//! a capture and what the material balance is afterwards. Material is not the objective -- nothing
//! in this project tells the net that a queen beats a pawn -- which is what makes it a fair
//! external check: if piece values were never given and material still climbs, the net worked
//! something out.
use board::{Color, Position};
use nnue::Net;
use pipeline::search::Searcher;

fn material(p: &Position) -> i32 {
    // Conventional values, used ONLY as an outside yardstick for the reader. The engine has
    // never seen these numbers; they are not in the eval, the training target, or the search.
    const V: [i32; 6] = [1, 3, 3, 5, 9, 0];
    let mut s = 0;
    for (ci, sign) in [(Color::White.idx(), 1i32), (Color::Black.idx(), -1)] {
        for k in board::types::PieceKind::ALL {
            s += sign * V[k.idx()] * (p.pieces[ci][k.idx()].count_ones() as i32);
        }
    }
    s
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let get = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let champ_path = get("--champion").unwrap_or_else(|| "champion_long.net".into());
    let depth: u32 = get("--depth").and_then(|v| v.parse().ok()).unwrap_or(3);
    let plies: usize = get("--plies").and_then(|v| v.parse().ok()).unwrap_or(60);
    let seed: u64 = get("--seed").and_then(|v| v.parse().ok()).unwrap_or(7);

    let champ = match Net::load(&champ_path) {
        Ok(n) => n,
        Err(e) => { eprintln!("could not load {champ_path}: {e}"); std::process::exit(2); }
    };
    let origin = Net::random(champ.n_hidden, 20260907);
    println!("CHAMPION (white) vs FROZEN ORIGIN (black, random init), depth {depth}\n");

    let mut pos = Position::startpos();
    let mut s = Searcher::with_seed(seed);
    let mut caps = 0;
    for ply in 0..plies {
        if pos.legal_moves().is_empty() { println!("\n  game over at ply {ply}"); break; }
        let champ_to_move = pos.stm == Color::White;
        let net = if champ_to_move { &champ } else { &origin };
        let (mv, _score) = s.best_move_capped(&mut pos, depth, net, u64::MAX, ply as u64 + 1);
        let was_capture = pos.piece_at(mv.to().0).is_some();
        pos.make_move(mv);
        if champ_to_move && was_capture { caps += 1; }
        if champ_to_move {
            println!("  {:>3}. {mv}{}   material {:+}",
                     ply / 2 + 1, if was_capture { " x" } else { "  " }, material(&pos));
        }
    }
    println!("\n  champion captures: {caps}");
    println!("  final material (white = champion): {:+}", material(&pos));
    println!("\n  Material is an OUTSIDE yardstick only. No piece value appears in the eval, the");
    println!("  training target, or the search -- so a rising balance is something the net found,");
    println!("  not something it was told.");
}
