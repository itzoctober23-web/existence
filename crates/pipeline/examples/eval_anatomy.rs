//! How much of the trained eval is just MATERIAL? The last untested ceiling suspect is the features.
//!
//! WHY THIS IS THE REMAINING SUSPECT. Everything else named has been measured and closed or
//! reversed: capacity (w64 does not beat w16 head-to-head, and loses at equal time), the draw
//! filter, the horizon schedule, the acceptance gate (batching produces real KEEPs), and the
//! "training signal collapses" hypothesis (the search-minus-eval gap GROWS with strength, it does
//! not vanish). The datagen-depth lever turned out to be search parity.
//!
//! What has never been tested is the INPUT ENCODING. The net sees 782 features: 768 piece-square
//! bits plus side-to-move, castling and en-passant. There is no mobility, no king safety, no pawn
//! structure, no attack information. A net over piece-square inputs can learn material and a
//! piece-square table and interactions between those, and NOTHING that requires reading the board
//! relationally -- at any width. That is consistent with w64 failing to beat w16: extra capacity
//! over an impoverished encoding buys nothing.
//!
//! THE MEASUREMENT. For each net, regress its eval on plain material balance across a shared
//! position set and report R^2 plus the residual spread.
//!   * R^2 near 1 with a tiny residual => the net IS a material counter with a thin positional
//!     veneer, and the encoding is the ceiling. More generations cannot add what the inputs cannot
//!     express.
//!   * R^2 high but the residual LARGE and growing with training => the net has learned material
//!     early and is putting real signal into the positional remainder, so the encoding is not yet
//!     the binding limit and the ceiling is elsewhere.
//!
//! The residual is the informative half, which is why it is reported in the eval's own units AND as
//! a fraction of total spread. R^2 alone would hide it: material dominates the variance in random
//! positions, so ANY sane eval scores a high R^2 and the number would look damning for every net,
//! including ones with real positional knowledge.
//!
//! CONTROL BUILT IN: the random origin net is included. It has no training at all, so its R^2
//! against material is whatever an untrained net gives -- if a trained net's R^2 is not clearly
//! higher, then this instrument cannot see learning and its readings mean nothing.
use board::{Color, Position};
use nnue::Net;
use pipeline::datagen::Rng;

/// Centipawn-ish values. The exact numbers do not matter much: this is a regression, so a constant
/// rescaling of material changes the slope and leaves R^2 alone.
const VAL: [f64; 6] = [100.0, 320.0, 330.0, 500.0, 900.0, 0.0]; // P N B R Q K

fn material_white(p: &Position) -> f64 {
    // piece_at over 64 squares rather than the bitboard accessors: `board::bb` is not public from
    // outside the crate, and a diagnostic does not need the fast path.
    let mut m = 0.0;
    for sq in 0u8..64 {
        if let Some((c, k)) = p.piece_at(sq) {
            let sign = if c == Color::White { 1.0 } else { -1.0 };
            m += sign * VAL[k.idx()];
        }
    }
    m
}

fn main() {
    let mut a = std::env::args().skip(1);
    let n_pos: usize = a.next().and_then(|s| s.parse().ok()).unwrap_or(4000);
    let seed: u64 = a.next().and_then(|s| s.parse().ok()).unwrap_or(20260907);

    let nets: Vec<(String, Net)> = vec![
        ("origin(random)".into(), Net::random(16, 20260907)),
        ("bn_000 blend0".into(), Net::load("bn_000.net").unwrap()),
        ("bn_075 20gen".into(), Net::load("bn_075.net").unwrap()),
        ("bh_100 blend1".into(), Net::load("bh_100.net").unwrap()),
        ("champion_long".into(), Net::load("champion_long.net").unwrap()),
    ];

    // One shared position set: differences must be the NET, never the sample.
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
    let mat: Vec<f64> = ps.iter().map(material_white).collect();

    println!("eval_anatomy: {} positions, seed {seed}", ps.len());
    println!("  regressing each net's WHITE-POV eval on plain material balance\n");
    println!("  {:<16} {:>8} {:>12} {:>12} {:>10}", "net", "R^2", "resid sd", "total sd", "resid/tot");

    for (name, net) in &nets {
        let mut scratch = Vec::new();
        let ev: Vec<f64> = ps.iter().map(|p| {
            let mover = net.eval(p, &mut scratch) as f64;
            // eval() returns MOVER-relative; flip to white POV so it is comparable with material.
            if p.stm == Color::White { mover } else { -mover }
        }).collect();

        let n = ev.len() as f64;
        let (mx, my) = (mat.iter().sum::<f64>() / n, ev.iter().sum::<f64>() / n);
        let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
        for (x, y) in mat.iter().zip(ev.iter()) {
            sxy += (x - mx) * (y - my);
            sxx += (x - mx) * (x - mx);
            syy += (y - my) * (y - my);
        }
        let r2 = if sxx > 0.0 && syy > 0.0 { (sxy * sxy) / (sxx * syy) } else { f64::NAN };
        let total_sd = (syy / (n - 1.0)).sqrt();
        let resid_sd = total_sd * (1.0 - r2).max(0.0).sqrt();
        println!("  {:<16} {:>8.4} {:>12.1} {:>12.1} {:>10.3}",
                 name, r2, resid_sd, total_sd, if total_sd > 0.0 { resid_sd / total_sd } else { f64::NAN });
    }

    println!("\n  === reading ===");
    println!("  Compare the TRAINED rows against origin(random). If R^2 barely moves, this");
    println!("  instrument cannot see training and nothing below it should be believed.");
    println!("  R^2 -> 1 with a collapsing residual means the net is a material counter and the");
    println!("  782-feature encoding is the ceiling: no number of generations adds knowledge the");
    println!("  inputs cannot represent. A large, growing residual means the opposite.");
}
