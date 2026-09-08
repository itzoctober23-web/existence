//! Perft as a real test, so `cargo test` gates every commit (CRATE.md 10).
//! MASTER_PLAN P0 requires the fixture; examples/xcheck.rs is the other half.

use board::{Position, perft};

const SUITE: &[(&str, &[u64])] = &[
    ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", &[20, 400, 8902, 197281]),
    ("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1", &[48, 2039, 97862]),
    ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", &[14, 191, 2812, 43238]),
    ("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1", &[6, 264, 9467]),
    ("rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8", &[44, 1486, 62379]),
    ("r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10", &[46, 2079, 89890]),
];

#[test]
fn canonical_perft_suite() {
    for (fen, expect) in SUITE {
        let mut p = Position::from_fen(fen).expect("fen parses");
        for (i, &want) in expect.iter().enumerate() {
            let d = i as u32 + 1;
            let got = perft(&mut p, d);
            assert_eq!(got, want, "perft({d}) on {fen}: got {got}, want {want}");
        }
    }
}

#[test]
fn fen_roundtrips() {
    for (fen, _) in SUITE {
        let p = Position::from_fen(fen).expect("fen parses");
        assert_eq!(&p.to_fen(), fen, "FEN did not round-trip");
    }
}

#[test]
fn make_unmake_restores_the_position() {
    // An asymmetry between make and unmake is invisible to a shallow perft but corrupts a
    // deep search. Compare the FEN before and after every legal move.
    let mut rng: u64 = 0xFEED_FACE_CAFE_BEEF;
    for (fen, _) in SUITE {
        let mut p = Position::from_fen(fen).unwrap();
        for _ in 0..40 {
            let before = p.to_fen();
            let list = p.legal_moves();
            if list.is_empty() { break; }
            for &m in list.as_slice() {
                let u = p.make_move(m);
                p.unmake_move(m, u);
                assert_eq!(p.to_fen(), before, "make/unmake changed the position on {m}");
            }
            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
            let m = list.as_slice()[(rng % list.len() as u64) as usize];
            p.make_move(m);
        }
    }
}

#[test]
fn zobrist_distinguishes_positions_and_is_stable() {
    // Two things a key must do: be a pure function of the position, and separate positions
    // that differ only in side to move, castling rights, or ep file.
    let mut seen = std::collections::HashMap::new();
    let mut rng: u64 = 0x5EED;
    for (fen, _) in SUITE {
        let mut p = Position::from_fen(fen).unwrap();
        for _ in 0..200 {
            let k = p.zobrist();
            let f = p.to_fen();
            // strip the move counters: they are not part of the key by design
            let core = f.rsplitn(3, ' ').last().unwrap().to_string();
            if let Some(prev) = seen.insert(k, core.clone()) {
                assert_eq!(prev, core, "zobrist collision between two different positions");
            }
            assert_eq!(k, p.zobrist(), "zobrist is not a pure function");
            let list = p.legal_moves();
            if list.is_empty() { break; }
            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
            p.make_move(list.as_slice()[(rng % list.len() as u64) as usize]);
        }
    }
}

/// The incremental Zobrist key must equal the from-scratch one at EVERY node.
///
/// This is the check `zobrist.rs` asked for when it said "incremental update in make/unmake is
/// the obvious next step, and the test that would guard it is `incremental == from_scratch` at
/// every node". An incremental hash that drifts is the worst kind of bug: it does not crash, it
/// silently makes two different positions collide, and FITNESS 10 lists "exploit hash collision
/// / stale slot" as a degenerate solution a program can be REWARDED for finding.
#[test]
fn incremental_zobrist_matches_from_scratch_everywhere() {
    fn walk(p: &mut board::Position, depth: u32, n: &mut usize) {
        assert_eq!(p.key, p.zobrist(), "key drifted at {}", p.to_fen());
        *n += 1;
        if depth == 0 { return; }
        let list = p.legal_moves();
        for &m in list.as_slice() {
            let u = p.make_move(m);
            walk(p, depth - 1, n);
            p.unmake_move(m, u);
            // and it must be restored EXACTLY by unmake, not merely recomputable
            assert_eq!(p.key, p.zobrist(), "key not restored by unmake of {m}");
        }
    }
    let mut n = 0;
    for fen in [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
    ] {
        let mut p = board::Position::from_fen(fen).expect("valid fen");
        walk(&mut p, 3, &mut n);
    }
    assert!(n > 20_000, "only {n} nodes checked — the walk is too shallow to mean anything");
}
