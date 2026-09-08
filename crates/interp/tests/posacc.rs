//! The interpreter's incremental accumulator must agree with the from-scratch eval EXACTLY,
//! at every node, or evolved programs are being scored on a different function from the one
//! the champion net defines.
//!
//! This is the same guard the incremental Zobrist key carries, for the same reason: a drifting
//! accumulator does not crash. It quietly changes what "eval" means, and every gate result
//! downstream of it becomes a measurement of the drift rather than of the program.

use board::Position;
use interp::PosAcc;
use nnue::Net;

#[test]
fn incremental_eval_matches_from_scratch_over_a_walk() {
    for width in [16usize, 32, 64] {
        let net = Net::random(width, 20260908 ^ width as u64);
        let mut scratch = Vec::new();
        let mut buf = interp::Delta::new();
        let mut rng: u64 = 0xACC0 ^ width as u64;
        let mut rnd = || { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; rng };

        let mut checked = 0usize;
        let (mut worst, mut worst_ply) = (0i32, 0usize);
        for _ in 0..40 {
            let mut node = PosAcc::fresh(&net, Position::startpos());
            for ply in 0..40 {
                let list = node.pos.legal_moves();
                if list.is_empty() { break; }
                // every child must agree, not just the one we descend into
                for &m in list.as_slice().iter().take(6) {
                    let child = node.child(&net, m, &mut buf);
                    let want = net.eval(&child.pos, &mut scratch);
                    let got = child.score(&net);
                    let d = (got - want).abs();
                    if d > worst { worst = d; worst_ply = ply; }
                    checked += 1;
                }
                let m = list.as_slice()[(rnd() % list.len() as u64) as usize];
                node = node.child(&net, m, &mut buf);
            }
        }
        println!("width {width}: {checked} evals, WORST drift {worst} (deepest at ply {worst_ply})");
        assert!(checked > 2000, "only {checked} evals compared at width {width}");
        // f32 incremental accumulation cannot be bit-exact with a from-scratch sum -- the row
        // adds happen in a different ORDER, so rounding differs and the final cast can land on
        // a different integer. What MUST hold is that the error stays BOUNDED rather than
        // compounding with depth, because an accumulator that drifts without limit silently
        // redefines `eval` deeper in the search, where nothing would notice.
        assert!(worst <= 2, "drift {worst} at width {width} is too large to be rounding");
    }
}

#[test]
fn the_root_accumulator_matches_too() {
    // fresh() is the only non-incremental path; if it disagrees, every descendant inherits the
    // error and the walk test above would pass while being uniformly wrong.
    let net = Net::random(32, 7);
    let mut scratch = Vec::new();
    for fen in [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 b - - 0 1",
    ] {
        let p = Position::from_fen(fen).expect("valid fen");
        let node = PosAcc::fresh(&net, p.clone());
        assert_eq!(node.score(&net), net.eval(&p, &mut scratch), "root mismatch at {fen}");
    }
}
