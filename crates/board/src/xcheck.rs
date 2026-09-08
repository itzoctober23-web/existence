//! Movegen cross-check against an external engine, as a LIBRARY so the example and the test
//! run identical code.
//!
//! It used to live only in `examples/xcheck.rs`, which meant it gated nothing: `cargo test`
//! never ran it. Duplicating it into a test would have created a second copy free to drift from
//! the first — and drift between two copies of "the same" logic is the exact bug found earlier
//! today, where the pipeline's search and the shipped engine's search had silently diverged
//! while a comment asserted they were the same.
//!
//! CRATE.md 2 requires BOTH halves of movegen verification:
//!   (a) a frozen perft fixture — `tests/perft.rs`
//!   (b) random full games walked to terminal, comparing the LEGAL MOVE SET at every ply
//!
//! (b) is not redundant with (a). A fixture only probes the positions in it: on the sibling 4PC
//! project a 40-position perft fixture agreed 40/40 while 94 genuine rules divergences existed,
//! because the fixture never reached the states where they occur. Perft also compares only
//! COUNTS, so two compensating errors cancel. This compares the sets themselves.

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use crate::{Move, Outcome, Position};

pub struct Engine {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
}

impl Engine {
    pub fn new(path: &str) -> std::io::Result<Self> {
        let mut child = Command::new(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut e = Engine { child, stdin, out };
        e.send("uci");
        e.read_until("uciok");
        Ok(e)
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
    /// The engine's legal moves for a FEN, via `go perft 1` (one line per root move).
    pub fn legal(&mut self, fen: &str) -> BTreeSet<String> {
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
        let _ = writeln!(self.stdin, "quit");
        let _ = self.child.wait();
    }
}

/// Deterministic RNG so a divergence is reproducible from its seed alone.
pub struct Rng(pub u64);
impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

#[derive(Debug, Default)]
pub struct Report {
    pub games: usize,
    pub plies: usize,
    pub divergences: usize,
    /// [ongoing (hit the ply cap), checkmate, stalemate]
    pub terminals: [usize; 3],
    /// Human-readable detail for the first few disagreements.
    pub detail: Vec<String>,
}

/// Walk `games` random games, comparing the full legal-move SET at every ply.
pub fn cross_check(
    engine_path: &str,
    games: usize,
    max_plies: usize,
    seed: u64,
    mut progress: impl FnMut(&str),
) -> std::io::Result<Report> {
    let mut eng = Engine::new(engine_path)?;
    let mut rng = Rng(seed | 1);
    let mut r = Report::default();
    r.games = games;

    for g in 0..games {
        let mut pos = Position::startpos();
        for _ in 0..max_plies {
            let list = pos.legal_moves();
            let mine: BTreeSet<String> = list.as_slice().iter().map(|m| m.to_string()).collect();
            let fen = pos.to_fen();
            let theirs = eng.legal(&fen);
            r.plies += 1;

            if mine != theirs {
                r.divergences += 1;
                if r.detail.len() < 5 {
                    let only_mine: Vec<_> = mine.difference(&theirs).cloned().collect();
                    let only_theirs: Vec<_> = theirs.difference(&mine).cloned().collect();
                    r.detail.push(format!(
                        "game {g} ply {}: fen {fen}\n    mine {:>3} only-mine {:?}\n    ref  {:>3} only-ref  {:?}",
                        r.plies, mine.len(), only_mine, theirs.len(), only_theirs
                    ));
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
            Outcome::Ongoing => r.terminals[0] += 1,
            Outcome::Loss => r.terminals[1] += 1,
            Outcome::Draw => r.terminals[2] += 1,
        }
        if g > 0 && g % 25 == 0 {
            progress(&format!("  ... {g}/{games} games, {} plies, {} divergences", r.plies, r.divergences));
        }
    }
    Ok(r)
}

/// Where to find the reference engine. `XCHECK_ENGINE` overrides; otherwise PATH.
pub fn engine_path() -> String {
    std::env::var("XCHECK_ENGINE").unwrap_or_else(|_| "stockfish".into())
}
