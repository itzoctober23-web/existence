//! UCI front end. Iteration zero: random-init net, bare alpha-beta, fixed depth.
//!
//! It plays legal chess and finds mates inside its horizon because terminal conditions are
//! rules; everything else it does is noise, and it is supposed to be. MASTER_PLAN says so
//! plainly, and the explanation layer will say so too when it exists.

mod search;

use board::types::MOVE_NONE;
use board::Position;
use nnue::Net;
use search::Search;
use std::io::{BufRead, Write};

const NAME: &str = "Existence";
const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    let mut pos = Position::startpos();
    // Iteration zero: the net has no knowledge in it. The seed only makes runs reproducible.
    // Load a learned net if one is present, else fall back to iteration zero. The engine used
    // to ALWAYS call Net::random(), so every training run was discarded and the shipped binary
    // stayed at iteration-zero strength no matter what the loop had learned.
    let net_path = std::env::var("EXISTENCE_NET").unwrap_or_else(|_| "champion.net".to_string());
    let (net, loaded) = match Net::load(&net_path) {
        Ok(n) => (n, true),
        Err(_) => (Net::random(256, 0xE1_57_E0_1C), false),
    };
    if loaded {
        eprintln!("info string loaded net {net_path} ({} hidden)", net.n_hidden);
    } else {
        eprintln!("info string no net at {net_path}; using random init (iteration zero)");
    }
    let mut search = Search::new(net, 0xC0FFEE);
    let mut depth: u32 = 4;

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let mut it = line.split_whitespace();
        match it.next() {
            Some("uci") => {
                println!("id name {NAME} {VERSION}");
                println!("id author ExistenceIsPain");
                println!("option name Depth type spin default 4 min 1 max 8");
                println!("uciok");
            }
            Some("isready") => println!("readyok"),
            Some("ucinewgame") => pos = Position::startpos(),
            Some("setoption") => {
                let toks: Vec<&str> = line.split_whitespace().collect();
                if let Some(i) = toks.iter().position(|t| *t == "value") {
                    if toks.get(i.wrapping_sub(1)) == Some(&"Depth")
                        || toks.contains(&"Depth")
                    {
                        if let Some(v) = toks.get(i + 1).and_then(|s| s.parse().ok()) {
                            depth = v;
                        }
                    }
                }
            }
            Some("position") => {
                let rest: Vec<&str> = it.collect();
                pos = parse_position(&rest).unwrap_or_else(Position::startpos);
            }
            Some("go") => {
                let (m, score) = search.best_move(&mut pos, depth);
                println!(
                    "info depth {depth} score cp {score} nodes {} pv {}",
                    search.nodes,
                    if m == MOVE_NONE { "0000".into() } else { m.to_string() }
                );
                println!(
                    "bestmove {}",
                    if m == MOVE_NONE { "0000".into() } else { m.to_string() }
                );
            }
            Some("d") => println!("{}", pos.to_fen()),
            Some("quit") => break,
            _ => {}
        }
        let _ = out.flush();
    }
}

fn parse_position(toks: &[&str]) -> Option<Position> {
    let (mut pos, i) = match toks.first()? {
        &"startpos" => (Position::startpos(), 1usize),
        &"fen" => {
            let end = toks.iter().position(|t| *t == "moves").unwrap_or(toks.len());
            (Position::from_fen(&toks[1..end].join(" ")).ok()?, end)
        }
        _ => return None,
    };
    if toks.get(i) == Some(&"moves") {
        for ms in &toks[i + 1..] {
            let legal = pos.legal_moves();
            let found = legal.as_slice().iter().find(|m| m.to_string() == *ms).copied();
            match found {
                Some(m) => {
                    pos.make_move(m);
                }
                None => break,
            }
        }
    }
    Some(pos)
}
