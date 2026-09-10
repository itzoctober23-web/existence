//! The accumulator must not change a score DATAGEN or the GATE would record.
//!
//! WHY THIS IS SEPARATE from the engine's test. `engine/src/search.rs` has its own check against an
//! independent from-scratch alpha-beta, but that is a DIFFERENT search. `pipeline::search::Searcher`
//! is the one `datagen.rs` and `gate.rs` actually run, and on 2026-09-10 its default changed: the
//! `n_hidden >= 64` gate was removed, so every width now takes the incremental path where width 32
//! previously took a full refresh. Every label written and every verdict returned since then comes
//! from a path that was not previously exercised at that width.
//!
//! The bench showed identical NODE COUNTS across paths, which is strong (a differing score changes a
//! cutoff and therefore the tree) but indirect. This asserts the scores themselves.
//!
//! Three paths must agree, not two: full refresh, the original O(active) diff, and the O(changed)
//! XOR diff that is now the default.
use board::Position;
use nnue::Net;
use pipeline::search::Searcher;

#[test]
fn all_three_eval_paths_return_identical_scores_and_trees() {
    let fens = [
        None,
        Some("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1"),
        Some("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1"),
        Some("r3k2r/1P6/8/8/8/8/6p1/R3K2R w KQkq - 0 1"),
        Some("8/8/8/3pP3/8/8/8/4K2k w - d6 0 1"),
    ];
    // Widths either side of the retired >= 64 gate: 32 is the default and the one that changed.
    let mut checked = 0usize;
    for hidden in [8usize, 32, 128] {
        let net = Net::random(hidden, 0xC0FFEE);
        for (i, f) in fens.into_iter().enumerate() {
            let pos = match f { None => Position::startpos(), Some(x) => Position::from_fen(x).unwrap() };
            for depth in 1..=3u32 {
                let run = |inc: bool, xor: bool| {
                    // Same seed each time: the shuffle must be identical or the trees differ for a
                    // reason that has nothing to do with the eval path.
                    let mut s = Searcher::with_seed(0xABCD ^ (i as u64) << 8 ^ depth as u64);
                    s.set_paths(inc, xor);
                    let (mv, sc) = s.best_move(&mut pos.clone(), depth, &net);
                    (mv, sc, s.nodes)
                };
                let refresh = run(false, false);
                let old_diff = run(true, false);
                let xor_diff = run(true, true);

                assert_eq!(refresh, old_diff,
                    "hidden {hidden} fen {i} depth {depth}: refresh {refresh:?} vs OLD delta {old_diff:?}");
                assert_eq!(refresh, xor_diff,
                    "hidden {hidden} fen {i} depth {depth}: refresh {refresh:?} vs XOR delta {xor_diff:?}");
                checked += 1;
            }
        }
    }
    assert!(checked >= 45, "only {checked} probes ran");
    println!("refresh == old-delta == XOR-delta on {checked} (width, position, depth) probes");
}
