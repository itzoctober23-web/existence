//! Movegen cross-check against an external engine. CRATE.md 2 requires BOTH halves:
//!
//!   (a) a frozen perft fixture  -- see examples/quickperft.rs
//!   (b) random full games walked to terminal, comparing the LEGAL MOVE SET at every ply
//!
//! (b) is not redundant with (a). A fixture only probes the positions in it: on the sibling
//! 4PC project a 40-position perft fixture agreed 40/40 while 94 genuine rules divergences
//! existed, because the fixture never reached the states where they occur. Perft also compares
//! only COUNTS -- two compensating errors cancel. This compares the sets themselves.
//!
//! Usage: cargo run --release --example xcheck -- [games] [max_plies]

use board::{Move, Outcome, Position};
use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Engine {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
}

impl Engine {
    fn new(path: &str) -> Self {
        let mut child = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn engine");
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut e = Engine { child, stdin, out };
        e.send("uci");
        e.read_until("uciok");
        e
    }
    fn send(&mut self, s: &str) {
        writeln!(self.stdin, "{s}").expect("write engine");
        self.stdin.flush().unwrap();
    }
    fn read_until(&mut self, marker: &str) -> Vec<String> {
        let mut lines = Vec::new();
        let mut buf = String::new();
        loop {
            buf.clear();
            if self.out.read_line(&mut buf).expect("read engine") == 0 {
                panic!("engine closed its output");
            }
            let l = buf.trim_end().to_string();
            let done = l.starts_with(marker);
            lines.push(l);
            if done {
                return lines;
            }
        }
    }
    /// The engine's legal moves for a FEN, via `go perft 1` (which prints one line per root move).
    fn legal(&mut self, fen: &str) -> BTreeSet<String> {
        self.send(&format!("position fen {fen}"));
        self.send("go perft 1");
        let lines = self.read_until("Nodes searched");
        self.send("isready");
        self.read_until("readyok");
        lines
            .iter()
            .filter_map(|l| l.split_once(':'))
            .filter(|(m, _)| {
                let b = m.as_bytes();
                (4..=5).contains(&b.len()) && b[0].is_ascii_lowercase() && b[1].is_ascii_digit()
            })
            .map(|(m, _)| m.trim().to_string())
            .collect()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.send_quit();
    }
}
impl Engine {
    fn send_quit(&mut self) -> std::io::Result<()> {
        writeln!(self.stdin, "quit")?;
        let _ = self.child.wait();
        Ok(())
    }
}

/// Deterministic RNG so a divergence is reproducible from its seed alone.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let games: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(200);
    let max_plies: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(300);
    let path = std::env::var("XCHECK_ENGINE").unwrap_or_else(|_| "stockfish".into());

    let mut eng = Engine::new(&path);
    let mut rng = Rng(0x2026_09_07_1234_5678);
    let (mut plies, mut diverged, mut terminal) = (0usize, 0usize, [0usize; 3]);

    for g in 0..games {
        let mut pos = Position::startpos();
        for _ in 0..max_plies {
            let list = pos.legal_moves();
            let mine: BTreeSet<String> = list.as_slice().iter().map(|m| m.to_string()).collect();
            let fen = pos.to_fen();
            let theirs = eng.legal(&fen);
            plies += 1;

            if mine != theirs {
                diverged += 1;
                if diverged <= 5 {
                    let only_mine: Vec<_> = mine.difference(&theirs).cloned().collect();
                    let only_theirs: Vec<_> = theirs.difference(&mine).cloned().collect();
                    println!("DIVERGENCE game {g} ply {plies}");
                    println!("  fen         {fen}");
                    println!("  mine {:>3}    only-mine   {:?}", mine.len(), only_mine);
                    println!("  ref  {:>3}    only-ref    {:?}", theirs.len(), only_theirs);
                }
                break;
            }

            if list.is_empty() {
                break;
            }
            let m: Move = list.as_slice()[rng.below(list.len())];
            pos.make_move(m);
        }
        match pos.outcome() {
            Outcome::Ongoing => terminal[0] += 1,
            Outcome::Loss => terminal[1] += 1,
            Outcome::Draw => terminal[2] += 1,
        }
        if g > 0 && g % 25 == 0 {
            println!("  ... {g}/{games} games, {plies} plies, {diverged} divergences");
        }
    }

    println!("\n{games} games, {plies} plies compared, {diverged} divergences");
    println!(
        "terminals reached: ongoing(ply-cap) {}, checkmate {}, stalemate {}",
        terminal[0], terminal[1], terminal[2]
    );
    if diverged == 0 {
        println!("PASS — the move generators agree on every position visited.");
    } else {
        println!("FAIL — {diverged} disagreement(s).");
        std::process::exit(1);
    }
}
