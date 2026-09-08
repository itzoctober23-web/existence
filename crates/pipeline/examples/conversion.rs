//! DOES IT CONVERT? Measure how often a won position turns into a win.
//!
//! WHY. One game, watched move by move, showed the champion win +18 material (queen and both
//! rooks) and then shuffle a piece between d8 and f6 for thirteen straight moves without ever
//! trying to mate. That is a single trace: suggestive, not a fact, and exactly the kind of thing
//! that is easy to over-read. The aggregate numbers hid it completely -- 0.838 against the
//! origin and ~49% decisive both look healthy while this is happening.
//!
//! So this counts it. For each game: the champion's PEAK material lead, and how the game ended.
//! The question is narrow and answerable -- of the games where the champion got clearly ahead,
//! what fraction actually finished?
//!
//! Material is an OUTSIDE yardstick. No piece value appears in the eval, the training target or
//! the search, so it cannot be what the engine is optimising; it is only how a human reads
//! "clearly winning".
use board::{Color, Position};
use nnue::Net;
use pipeline::search::Searcher;

fn material(p: &Position) -> i32 {
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
    let games: usize = get("--games").and_then(|v| v.parse().ok()).unwrap_or(60);
    let cap: usize = get("--plies").and_then(|v| v.parse().ok()).unwrap_or(160);
    let lead: i32 = get("--lead").and_then(|v| v.parse().ok()).unwrap_or(5);
    // THE CONTROL. The gate never starts from the real starting position: it plays
    // `startpos + open_plies random moves`. So "0 mates from startpos" only means something if
    // the SAME harness finds mates when given the gate's own randomised openings. Without this
    // flag the result is unfalsifiable -- it could equally be a bug in this file.
    let open_plies: usize = get("--open-plies").and_then(|v| v.parse().ok()).unwrap_or(0);

    let champ = match Net::load(&champ_path) {
        Ok(n) => n,
        Err(e) => { eprintln!("could not load {champ_path}: {e}"); std::process::exit(2); }
    };
    let origin = Net::random(champ.n_hidden, 20260907);

    let (mut got_ahead, mut converted, mut ahead_unfinished, mut lost_after_ahead) = (0, 0, 0, 0);
    let (mut mates, mut stalemates, mut plycaps) = (0, 0, 0);
    for g in 0..games {
        let mut pos = Position::startpos();
        let mut s = Searcher::with_seed(g as u64 * 7919 + 1);
        {
            let mut r = pipeline::datagen::Rng((g as u64 + 1) * 0x9E37_79B9);
            for _ in 0..open_plies {
                let l = pos.legal_moves();
                if l.is_empty() { break; }
                let m = l.as_slice()[(r.next() % l.len() as u64) as usize];
                pos.make_move(m);
            }
        }
        let mut peak = 0i32;
        let mut end = "plycap";
        for ply in 0..cap {
            if pos.legal_moves().is_empty() {
                // No legal move: mate if the side to move is in check, else stalemate.
                end = if pos.in_check(pos.stm) { "mate" } else { "stalemate" };
                break;
            }
            let net = if pos.stm == Color::White { &champ } else { &origin };
            let (mv, _) = s.best_move_capped(&mut pos, depth, net, u64::MAX, ply as u64 + 1);
            pos.make_move(mv);
            peak = peak.max(material(&pos));
        }
        match end {
            "mate" => mates += 1,
            "stalemate" => stalemates += 1,
            _ => plycaps += 1,
        }
        if peak >= lead {
            got_ahead += 1;
            // A mate with the champion ahead: the side to move (the loser) is mated. Since the
            // champion is white and moves first, a mate on black's turn is the champion's win.
            if end == "mate" && pos.stm != Color::White { converted += 1; }
            else if end == "mate" { lost_after_ahead += 1; }
            else { ahead_unfinished += 1; }
        }
    }
    println!("champion {champ_path} vs frozen origin, {games} games, depth {depth}, ply cap {cap}");
    println!("start: {}\n", if open_plies == 0 { "the REAL starting position".to_string() }
                             else { format!("startpos + {open_plies} random plies (what the gate uses)") });
    println!("  games where the champion reached >= +{lead} material : {got_ahead}/{games}");
    if got_ahead > 0 {
        println!("    of those, CONVERTED to mate                      : {converted} ({:.0}%)",
                 100.0 * converted as f64 / got_ahead as f64);
        println!("    of those, ran out the ply cap still ahead        : {ahead_unfinished} ({:.0}%)",
                 100.0 * ahead_unfinished as f64 / got_ahead as f64);
        println!("    of those, went on to be MATED                   : {lost_after_ahead}");
    }
    println!("\n  endings overall: mate {mates}, stalemate {stalemates}, ply cap {plycaps}");
    println!("\n  A high 'ran out the ply cap still ahead' is the conversion failure: the engine");
    println!("  wins material and then cannot finish. Material is an outside yardstick only --");
    println!("  no piece value exists in the eval, the target, or the search.");
}
