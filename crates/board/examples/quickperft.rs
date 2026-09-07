use board::{Position, perft};

fn main() {
    let cases: &[(&str, &[u64])] = &[
        ("startpos", &[20, 400, 8902, 197281, 4865609]),
        ("kiwipete", &[48, 2039, 97862, 4085603]),
        ("position3", &[14, 191, 2812, 43238, 674624]),
        ("position4", &[6, 264, 9467, 422333]),
        ("position5", &[44, 1486, 62379, 2103487]),
        ("position6", &[46, 2079, 89890, 3894594]),
    ];
    let fens = [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
    ];
    let mut fail = 0;
    for (i, (name, expect)) in cases.iter().enumerate() {
        let mut p = Position::from_fen(fens[i]).expect("fen");
        for (d0, &want) in expect.iter().enumerate() {
            let d = d0 as u32 + 1;
            let got = perft(&mut p, d);
            let ok = got == want;
            if !ok { fail += 1; }
            println!("  {:<10} d{d}  got {got:>9}  want {want:>9}  {}", name, if ok {"ok"} else {"MISMATCH"});
        }
    }
    println!("\n  {}", if fail == 0 { "ALL PERFT PASS".to_string() } else { format!("{fail} MISMATCHES") });
}
