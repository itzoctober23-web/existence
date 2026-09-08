//! Play the SHIPPED BINARY against itself over UCI, one side loading a learned net and the
//! other loading none.
//!
//! Every control until now compared two in-memory Net objects. That is a closed circuit: the
//! loop could be perfectly correct, the trainer could be perfectly correct, the numbers could
//! all replicate -- and the engine that actually ships could still be at iteration zero,
//! because nothing was ever written to disk and the binary had no load path. It was.
//!
//! This measures the artifact: spawn ./target/release/engine twice, give each a different
//! EXISTENCE_NET, and make them play. If the champion cannot beat a random net THROUGH THE
//! BINARY, the pipeline has produced nothing regardless of what its internal controls say.
use board::{Color, Outcome, Position};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Uci { child: Child, inp: ChildStdin, out: BufReader<ChildStdout> }

impl Uci {
    fn new(exe: &str, net: Option<&str>) -> Self {
        let mut c = Command::new(exe);
        c.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
        match net {
            Some(p) => { c.env("EXISTENCE_NET", p); }
            None => { c.env("EXISTENCE_NET", "/nonexistent-so-random.net"); }
        }
        let mut child = c.spawn().expect("spawn engine");
        let inp = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut u = Uci { child, inp, out };
        u.send("uci"); u.wait_for("uciok");
        u
    }
    fn send(&mut self, s: &str) { writeln!(self.inp, "{s}").unwrap(); self.inp.flush().unwrap(); }
    fn wait_for(&mut self, m: &str) -> String {
        let mut b = String::new();
        loop {
            b.clear();
            if self.out.read_line(&mut b).unwrap() == 0 { return String::new(); }
            if b.starts_with(m) { return b.trim().to_string(); }
        }
    }
    fn bestmove(&mut self, moves: &[String]) -> String {
        let mut cmd = String::from("position startpos");
        if !moves.is_empty() { cmd.push_str(" moves "); cmd.push_str(&moves.join(" ")); }
        self.send(&cmd);
        self.send("go");
        let line = self.wait_for("bestmove");
        line.split_whitespace().nth(1).unwrap_or("0000").to_string()
    }
}
impl Drop for Uci { fn drop(&mut self) { let _ = self.send("quit"); let _ = self.child.wait(); } }

fn main() {
    let net = std::env::args().nth(1).unwrap_or_else(|| "champion.net".into());
    let games: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(40);
    let exe = "./target/release/engine";

    // Random opening plies, and each opening played from BOTH sides. Without this, two
    // deterministic engines replay the SAME game every time -- the first version returned
    // 0W-30D-0L because it was one drawn game repeated 30 times, not evidence of parity.
    let mut rng: u64 = 0x0EA7_BEEF;
    let (mut w, mut d, mut l) = (0u32, 0u32, 0u32);
    for g in 0..games {
        let champ_is_white = g % 2 == 0;
        // same opening for the pair, so colour-swapped games are comparable
        if g % 2 == 0 {
            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
        }
        let mut a = Uci::new(exe, Some(&net));   // champion
        let mut b = Uci::new(exe, None);         // random init
        let mut pos = Position::startpos();
        let mut moves: Vec<String> = Vec::new();
        let mut r = rng;
        for _ in 0..6 {
            let l = pos.legal_moves();
            if l.is_empty() { break; }
            r ^= r << 13; r ^= r >> 7; r ^= r << 17;
            let m = l.as_slice()[(r % l.len() as u64) as usize];
            moves.push(m.to_string());
            pos.make_move(m);
        }
        let mut result = None;
        for _ in 0..160 {
            if pos.legal_moves().is_empty() {
                result = Some(match pos.outcome() {
                    Outcome::Loss => Some(pos.stm == Color::Black), // white won?
                    _ => None,
                });
                break;
            }
            if pos.halfmove >= 100 { result = Some(None); break; }
            let white_to_move = pos.stm == Color::White;
            let mv = if white_to_move == champ_is_white { a.bestmove(&moves) } else { b.bestmove(&moves) };
            let found = pos.legal_moves().as_slice().iter().copied().find(|m| m.to_string() == mv);
            match found {
                Some(m) => { pos.make_move(m); moves.push(mv); }
                None => { result = Some(None); break; }
            }
        }
        match result.flatten() {
            Some(white_won) => if white_won == champ_is_white { w += 1 } else { l += 1 },
            None => d += 1,
        }
    }
    let n = (w + d + l).max(1) as f64;
    let p = (w as f64 + 0.5 * d as f64) / n;
    let ci = 1.96 * (p * (1.0 - p) / n).sqrt();
    println!("  BINARY GATE  champion({net}) vs random-init, through ./target/release/engine");
    println!("  {w}W-{d}D-{l}L over {} games   rate {p:.3} +/- {ci:.3}", n as u32);
    println!("  => {}", if p - ci > 0.5 {
        "the SHIPPED BINARY is stronger with the learned net"
    } else {
        "NOT DEMONSTRATED through the binary"
    });
}
