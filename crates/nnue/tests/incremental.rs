//! refresh() == update() must hold for EVERY legal move, or the incremental path silently
//! diverges from the truth and every measurement taken afterwards is worthless. This is the
//! same class of guard as the sparse-vs-dense test.
use board::Position;
use nnue::{Acc, FeatSnap, Net};

#[test]
fn incremental_matches_full_refresh() {
    let net = Net::random(32, 0xC0FFEE);
    let mut rng: u64 = 0xDEAD_BEEF_1234;
    let mut checked = 0usize;

    for _ in 0..30 {
        let mut pos = Position::startpos();
        for _ in 0..60 {
            let list = pos.legal_moves();
            if list.is_empty() { break; }

            // For every legal move: apply it, rebuild from scratch, and compare against an
            // accumulator updated by the feature delta.
            for &m in list.as_slice() {
                let mut before = Vec::new();
                Net::active(&pos, &mut before);
                let u = pos.make_move(m);
                let mut after = Vec::new();
                Net::active(&pos, &mut after);

                let on: Vec<u16> = after.iter().copied().filter(|f| !before.contains(f)).collect();
                let off: Vec<u16> = before.iter().copied().filter(|f| !after.contains(f)).collect();

                let mut inc = Acc::new(&net);
                inc.refresh(&net, &{ let mut p = pos.clone(); p.unmake_move(m, u); p });
                inc.update(&net, &on, &off);

                let mut full = Acc::new(&net);
                full.refresh(&net, &pos);

                for h in 0..net.n_hidden {
                    assert!(
                        (inc.vals[h] - full.vals[h]).abs() < 1e-3,
                        "hidden {h} diverged after {m}: incremental {} vs refresh {}",
                        inc.vals[h], full.vals[h]
                    );
                }
                checked += 1;
                pos.unmake_move(m, u);
            }

            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
            let l = pos.legal_moves();
            pos.make_move(l.as_slice()[(rng % l.len() as u64) as usize]);
        }
    }
    assert!(checked > 5000, "only {checked} move-deltas checked");
    println!("incremental == refresh over {checked} move-deltas");
}

/// The SAME guarantee as above, but for the delta the engine actually uses.
///
/// The test above derives its on/off sets by enumerating both active sets and diffing them with
/// `contains()`. That proves `Acc::update` is right; it says nothing about `FeatSnap::delta`, which
/// is what `search.rs` calls, and which re-derives the same sets from bitboard XORs. A bug there --
/// a missed castled rook, a promotion kind, an en-passant pawn that is not on the destination
/// square -- would be invisible to the test above and would silently corrupt every eval.
///
/// Random walks from startpos are NOT sufficient coverage on their own: they almost never promote,
/// and reach en passant and castling rarely. The FEN suite forces exactly those cases.
#[test]
fn feat_snap_delta_matches_full_refresh() {
    let net = Net::random(32, 0xC0FFEE);

    // Positions chosen for the rules that are easy to get wrong, not for typicality.
    let suite = [
        ("startpos", None),
        // White pawn on the 7th: promotion, including capture-promotion onto a rook.
        ("promo", Some("r3k2r/1P6/8/8/8/8/6p1/R3K2R w KQkq - 0 1")),
        // Both sides castling available, rooks and kings on their original squares.
        ("castle", Some("r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1")),
        // En-passant square live, so the ep feature is on and a capture removes a pawn
        // that is NOT on the destination square.
        ("ep", Some("8/8/8/3pP3/8/8/8/4K2k w - d6 0 1")),
        ("ep_black", Some("4k3/8/8/8/3Pp3/8/8/4K3 b - d3 0 1")),
    ];

    let mut total = 0usize;
    let mut promos = 0usize;
    for (si, (name, fen)) in suite.into_iter().enumerate() {
      // Several independent walks per entry. One walk down one random line samples the move types
      // reachable from that position exactly once; the coverage guard at the end wants more than that.
      for restart in 0..4u64 {
        let mut pos = match fen {
            None => Position::startpos(),
            Some(f) => Position::from_fen(f).unwrap_or_else(|e| panic!("{name}: bad fen: {e}")),
        };
        let mut rng: u64 = 0x5EED_1234_9ABC ^ (si as u64).wrapping_mul(0x9E37_79B9) ^ (restart << 32) ^ 1;
        let walk = if fen.is_none() { 40 } else { 8 };

        for _ in 0..walk {
            let list = pos.legal_moves();
            if list.is_empty() { break; }

            for &m in list.as_slice() {
                let before = FeatSnap::of(&pos);
                let mut base = Acc::new(&net);
                base.refresh(&net, &pos);

                let u = pos.make_move(m);
                let after = FeatSnap::of(&pos);

                let (mut on, mut off) = (Vec::new(), Vec::new());
                before.delta(&after, &mut on, &mut off);
                if on.len() + off.len() > 6 { promos += 1; }

                let mut inc = base.clone();
                inc.update(&net, &on, &off);

                let mut full = Acc::new(&net);
                full.refresh(&net, &pos);

                for h in 0..net.n_hidden {
                    assert!(
                        (inc.vals[h] - full.vals[h]).abs() < 1e-3,
                        "{name}: hidden {h} diverged after {m}: delta {} vs refresh {} \
                         (on={on:?} off={off:?})",
                        inc.vals[h], full.vals[h]
                    );
                }
                total += 1;
                pos.unmake_move(m, u);
            }

            rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
            let l = pos.legal_moves();
            if l.is_empty() { break; }
            pos.make_move(l.as_slice()[(rng % l.len() as u64) as usize]);
        }
      }
    }
    assert!(total > 2000, "only {total} deltas checked -- suite did not run");
    println!("FeatSnap::delta == refresh over {total} move-deltas ({promos} with >6 feature changes)");
}
