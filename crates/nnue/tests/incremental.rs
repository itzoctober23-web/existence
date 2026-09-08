//! refresh() == update() must hold for EVERY legal move, or the incremental path silently
//! diverges from the truth and every measurement taken afterwards is worthless. This is the
//! same class of guard as the sparse-vs-dense test.
use board::Position;
use nnue::{Acc, Net};

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
