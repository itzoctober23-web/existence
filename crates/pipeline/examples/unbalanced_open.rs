//! MASTER_PLAN remedy #2, measured: does an UNBALANCED opening make the gate's games decisive?
//!
//! THE PROBLEM. On the real gate population the games are 80.6% draws with ZERO wins in 36, because
//! both sides evaluate with `Net::random(32, ...)` -- noise -- so neither can convert, and the gate
//! starts them from a BALANCED position (`gate.rs`: 4 random plies from startpos, MASTER_PLAN opening
//! source #1). Nothing to convert, and no ability to convert it.
//!
//! WHY NOT THE RECIPE VERBATIM. Remedy #2 mines positions "where own search eval sits in a band". With
//! a random net that eval is noise. The rules-derived substitute used here is a PIECE COUNT gap: how
//! many pieces each side has is a fact about the position, where piece VALUES would be chess knowledge
//! MASTER_PLAN requires to be discovered rather than supplied.
//!
//! BOTH ARMS ARE A/A -- the SAME program against itself. So the true rate is 0.500 by construction in
//! both, and any difference is purely in how often the games RESOLVE. That isolates decisiveness from
//! strength, which is the only thing being claimed.
//!
//! THE CONTROL IS THE POINT. Arm 1 reproduces the gate's own opening policy and must land near the
//! independently measured 67% A/A draw rate (W-D-L 2-8-2). If it does not, this harness is not the
//! gate and arm 2 means nothing.
use board::Position;
use grammar::reference;
use nnue::Net;
use pipeline::gate;

fn gap(p: &Position) -> i32 {
    let (mut w, mut b) = (0i32, 0i32);
    for sq in 0..64u8 {
        if let Some((c, _)) = p.piece_at(sq) {
            if c == board::Color::White { w += 1 } else { b += 1 }
        }
    }
    (w - b).abs()
}

fn walk(rng: &mut grammar::mutate::Rng, plies: usize) -> Position {
    let mut pos = Position::startpos();
    for _ in 0..plies {
        let l = pos.legal_moves();
        if l.is_empty() { break }
        let m = l.as_slice()[(rng.next() % l.as_slice().len() as u64) as usize];
        let _ = pos.make_move(m);
    }
    pos
}

fn main() {
    let pairs: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(6);
    let depth: i64 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(3);
    let net = Net::random(32, 20260907);
    let p = reference::bare_alpha_beta();
    let t = vec![depth, 32_000, interp::uct_exploration()];
    let mut rng = grammar::mutate::Rng::new(0xB0A7D);

    // Build the two books. Balanced = the gate's own policy. Unbalanced = rejection-sampled.
    let balanced: Vec<Position> = (0..pairs).map(|_| walk(&mut rng, 4)).collect();
    let mut unbalanced: Vec<Position> = Vec::new();
    let mut walks = 0usize;
    while unbalanced.len() < pairs && walks < 200_000 {
        walks += 1;
        let c = walk(&mut rng, 12);
        if gap(&c) >= 2 { unbalanced.push(c) }
    }
    println!("unbalanced-opening probe — {pairs} pairs, depth {depth}, budget 16, A/A both arms");
    println!("  book generation: {} walks for {} unbalanced openings ({:.1}% hit rate)",
             walks, unbalanced.len(), 100.0 * unbalanced.len() as f64 / walks.max(1) as f64);
    println!("  walks need no search, so this cost is negligible next to playing the games\n");
    println!("  {:<32} {:>9} {:>8} {:>26}", "arm", "W-D-L", "draws", "pent");

    for (label, book) in [("CONTROL balanced (gate today)", &balanced),
                          ("TREATMENT material gap >= 2", &unbalanced)] {
        if book.is_empty() { println!("  {label:<32}  (no openings generated)"); continue }
        let sc = gate::match_progs_from(&p, &p, &net, t.clone(), 16, pairs, 0xB0A7D, book, u64::MAX);
        let g = (sc.wins + sc.draws + sc.losses).max(1);
        println!("  {label:<32} {:>3}-{}-{} {:>7.1}% {:>26}",
                 sc.wins, sc.draws, sc.losses, 100.0 * sc.draws as f64 / g as f64,
                 format!("{:?}", sc.pent));
    }
    println!("\n  Control must land near 67% draws (the independently measured A/A rate at these");
    println!("  settings). A large drop in the treatment is remedy #2 working; no drop means the");
    println!("  balanced start was not what was preventing resolution.");
}
