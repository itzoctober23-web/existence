use board::{Position, divide};
fn main() {
    let mut p = Position::startpos();
    for (m, n) in divide(&mut p, 2) {
        if n != 20 { println!("  {m} -> {n}   (expected 20)"); }
    }
    // look at one bad line in detail
    let mut p = Position::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
    let bad: Vec<_> = divide(&mut p, 2).into_iter().filter(|(_, n)| *n != 20).collect();
    if let Some((m, _)) = bad.first() {
        let u = p.make_move(*m);
        println!("\n  after {m}: fen {}", p.to_fen());
        let l = p.legal_moves();
        println!("  black has {} moves:", l.len());
        let mut v: Vec<String> = l.as_slice().iter().map(|x| x.to_string()).collect();
        v.sort();
        println!("  {}", v.join(" "));
        p.unmake_move(*m, u);
    }
}
