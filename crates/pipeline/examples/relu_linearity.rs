//! Is the trained net effectively LINEAR in its own features? The sharp form of the encoding question.
//!
//! WHY THIS AND NOT THE MATERIAL REGRESSION. `eval_anatomy` showed champion_long is only R^2 0.588
//! against plain material, so it is not a material counter -- which kills the STRONG form of "the
//! 782-feature encoding is the ceiling". It does not kill the real form. A piece-square table is
//! just as expressible in this encoding as material is, and the 41% residual could be entirely PST.
//! Regressing out material and finding structure left over does not show that structure is
//! RELATIONAL.
//!
//! THE DECISIVE PROPERTY. `eval` is
//!     score = sum_h relu(acc_h) * w2_h + b2
//! and `acc_h` is a plain SUM of input rows, so it is linear in the features. The ONLY nonlinearity
//! in the whole network is the relu gate. Therefore:
//!
//!   * a hidden unit that is ALWAYS ACTIVE across positions contributes exactly linearly;
//!   * a unit that is ALWAYS INACTIVE contributes nothing at all;
//!   * only units that SWITCH make the net more than a linear function of its inputs.
//!
//! If every unit is always-on or always-off, the net computes a linear function of 768 piece-square
//! bits -- a piece-square table with material folded in -- and the hidden layer is decoration. That
//! would be the encoding ceiling stated exactly, and it would explain why w64 does not beat w16:
//! more units that never switch add no expressive power.
//!
//! This reads the pre-relu activations straight out of `eval`'s scratch buffer, which holds
//! `b1 + sum of active rows` after the call. No new API and no reimplementation of the forward
//! pass -- reimplementing it would risk measuring my own copy rather than the shipped one.
//!
//! CONTROL: the untrained origin is included. If training does not change the switching structure
//! at all, this instrument is not sensitive to training and its readings mean nothing.
use board::Position;
use nnue::Net;
use pipeline::datagen::Rng;
use std::collections::HashSet;

fn main() {
    let mut a = std::env::args().skip(1);
    let n_pos: usize = a.next().and_then(|s| s.parse().ok()).unwrap_or(3000);
    let seed: u64 = a.next().and_then(|s| s.parse().ok()).unwrap_or(20260907);

    let nets: Vec<(String, Net)> = vec![
        ("origin(random)".into(), Net::random(16, 20260907)),
        ("bn_075 20gen".into(), Net::load("bn_075.net").unwrap()),
        ("bh_100 blend1".into(), Net::load("bh_100.net").unwrap()),
        ("champion_long".into(), Net::load("champion_long.net").unwrap()),
        ("wd_r2 (w64)".into(), Net::load("wd_r2.net").unwrap()),
    ];

    let mut rng = Rng(seed | 1);
    let mut ps: Vec<Position> = Vec::with_capacity(n_pos);
    while ps.len() < n_pos {
        let mut p = Position::startpos();
        let plies = 4 + rng.below(50);
        let mut ok = true;
        for _ in 0..plies {
            let l = p.legal_moves();
            if l.is_empty() { ok = false; break; }
            p.make_move(l.as_slice()[rng.below(l.len())]);
        }
        if ok && !p.legal_moves().is_empty() { ps.push(p); }
    }

    println!("relu_linearity: {} positions, seed {seed}", ps.len());
    println!("  the relu gate is the ONLY nonlinearity; a unit that never switches is linear\n");
    println!("  {:<16} {:>4} {:>8} {:>9} {:>9} {:>10}",
             "net", "w", "always+", "always0", "SWITCH", "patterns");

    for (name, net) in &nets {
        let h = net.n_hidden;
        let mut on = vec![0usize; h];
        let mut pats: HashSet<u64> = HashSet::new();
        let mut scratch = Vec::new();
        for p in &ps {
            let _ = net.eval(p, &mut scratch); // leaves pre-relu accumulators in scratch
            let mut mask = 0u64;
            for j in 0..h {
                if scratch[j] > 0.0 {
                    on[j] += 1;
                    if j < 64 { mask |= 1 << j; }
                }
            }
            pats.insert(mask);
        }
        let n = ps.len();
        let always_on = on.iter().filter(|&&c| c == n).count();
        let always_off = on.iter().filter(|&&c| c == 0).count();
        let switching = h - always_on - always_off;
        println!("  {:<16} {:>4} {:>8} {:>9} {:>9} {:>10}",
                 name, h, always_on, always_off, switching, pats.len());
    }

    println!("\n  === reading ===");
    println!("  SWITCH = 0 and patterns = 1 => the net is exactly linear over these positions: a");
    println!("  piece-square table, and the hidden layer is decoration. More width cannot help,");
    println!("  which is what w64 failing to beat w16 would look like from the inside.");
    println!("  A large SWITCH count means real nonlinearity and the encoding is not the binding");
    println!("  limit -- in which case this hypothesis dies like the last one and the ceiling is");
    println!("  somewhere still unnamed.");
    println!("  Compare every trained row against origin(random) FIRST: if training does not move");
    println!("  the switching structure, the instrument is blind and no row below it means anything.");
}
