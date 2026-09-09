//! Why does the loop learn from random and not from a trained champion? Measure the DISTILLATION GAP.
//!
//! THE MECHANISM UNDER TEST. The training target is `(1 - blend) * z + blend * root`, and the
//! shipped blend is 0.75 (1.00 measures at least as good), so the target is mostly the SEARCH's
//! score at the root. The net currently predicts its own static eval. So the quantity the training
//! step is actually asked to close is
//!
//!     gap = | tanh(root/scale) - tanh(eval/scale) |
//!
//! in exactly the frame the trainer works in. That is not a proxy for the learning signal, it IS
//! the learning signal: with blend = 1 the target is `root` and the current output is `eval`.
//!
//! AlphaZero works because search is much stronger than the raw net, so distilling search into the
//! net moves it. If a depth-2 search stops improving on a good net's own eval, the target collapses
//! onto what the net already says, the gradient goes to zero, and training does nothing -- no
//! matter what the gate does. That would explain, with one mechanism:
//!   * why 20 generations from `--rung 0` reach 0.847 but 20 from champion_long move nothing;
//!   * why datagen DEPTH is the only ceiling candidate still standing (+0.025);
//!   * why blend 1.00 is fine -- pure distillation is harmless while a gap exists.
//!
//! PRE-REGISTERED READING:
//!   * GAP SHRINKS WITH NET STRENGTH => the plateau is a vanishing training signal, and the lever is
//!     anything that widens the gap (more datagen depth being the obvious one, already measured
//!     positive).
//!   * GAP FLAT ACROSS NETS => the signal is still there and the plateau is elsewhere; this
//!     hypothesis is dead and should be recorded as such rather than quietly dropped.
//!   * GAP GROWS WITH SEARCH DEPTH is expected and is NOT itself the finding -- deeper search
//!     differing more from a static eval is nearly tautological. The finding is the interaction:
//!     whether depth restores the gap MORE for the strong net than the weak one.
//!
//! Positions come from random walks of varying length so the distribution spans openings through
//! late middlegame. Mate scores are excluded: tanh saturates on them and a handful of forced mates
//! would dominate a mean absolute difference while saying nothing about the eval's slope.
use board::Position;
use nnue::Net;
use pipeline::datagen::Rng;
use pipeline::search::Searcher;

fn main() {
    let mut a = std::env::args().skip(1);
    let n_pos: usize = a.next().and_then(|s| s.parse().ok()).unwrap_or(300);
    let seed: u64 = a.next().and_then(|s| s.parse().ok()).unwrap_or(20260907);
    // Depths are an ARGUMENT because the first run showed d3 >> d4 > d2 on every net, which looks
    // like the classic odd-even effect rather than anything about depth. Testing that needs a
    // second odd/even pair (5 vs 6), and hardcoding [2,3,4] would have made the parity claim
    // untestable without editing the instrument mid-investigation.
    let depths: Vec<u32> = a.next()
        .map(|s| s.split(',').filter_map(|x| x.parse().ok()).collect())
        .unwrap_or_else(|| vec![2, 3, 4]);

    // Weakest -> strongest, so a monotone trend is visible by eye in the printed order.
    let named: Vec<(String, Net)> = vec![
        ("origin(random)".to_string(), Net::random(16, 20260907)),
        ("bn_000 (blend0)".to_string(), Net::load("bn_000.net").unwrap()),
        ("bn_075 (20 gen)".to_string(), Net::load("bn_075.net").unwrap()),
        ("champion_long ".to_string(), Net::load("champion_long.net").unwrap()),
    ];

    // One shared position set for every net and depth, so differences are the NET, not the sample.
    let mut rng = Rng(seed | 1);
    let mut positions: Vec<Position> = Vec::with_capacity(n_pos);
    while positions.len() < n_pos {
        let mut p = Position::startpos();
        let plies = 4 + rng.below(36);
        let mut ok = true;
        for _ in 0..plies {
            let l = p.legal_moves();
            if l.is_empty() { ok = false; break; }
            p.make_move(l.as_slice()[rng.below(l.len())]);
        }
        if ok && !p.legal_moves().is_empty() { positions.push(p); }
    }
    println!("distill_gap: {} positions, seed {seed}", positions.len());
    println!("  gap = |tanh(root/scale) - tanh(eval/scale)|, the trainer's own frame\n");
    let hdr: Vec<String> = depths.iter().map(|d| format!("d{d}")).collect();
    println!("  {:<16}{}", "net", hdr.iter().map(|h| format!("{h:>9}")).collect::<String>());

    let mut rows: Vec<(String, Vec<f64>)> = Vec::new();
    for (name, net) in &named {
        let mut row = Vec::new();
        for &depth in &depths {
            let mut scratch = Vec::new();
            let (mut sum, mut cnt) = (0.0f64, 0usize);
            for p in &positions {
                let mut q = p.clone();
                let (_, sc) = Searcher::new().best_move(&mut q, depth, net);
                if sc.abs() > 20_000 { continue; } // mate: tanh saturates, excluded by design
                let ev = net.eval(p, &mut scratch);
                if ev.abs() > 20_000 { continue; }
                let r = (sc as f32 / net.scale).tanh() as f64;
                let e = (ev as f32 / net.scale).tanh() as f64;
                sum += (r - e).abs();
                cnt += 1;
            }
            row.push(if cnt > 0 { sum / cnt as f64 } else { f64::NAN });
        }
        println!("  {:<16}{}", name, row.iter().map(|v| format!("{v:>9.4}")).collect::<String>());
        rows.push((name.clone(), row));
    }

    println!("\n  === reading ===");
    let first = &rows[0].1;
    let last = &rows[rows.len() - 1].1;
    for (i, d) in hdr.iter().enumerate() {
        let ratio = if last[i] > 0.0 { first[i] / last[i] } else { f64::NAN };
        println!("  {d}: random {:.4} -> champion {:.4}   shrink {:.2}x", first[i], last[i], ratio);
    }
    println!("\n  A gap that SHRINKS as nets get stronger means the training signal is vanishing:");
    println!("  the target converges onto what the net already says, and no gate can rescue that.");
    println!("  Compare champion's d2 against its d4: if depth restores the gap, datagen depth is");
    println!("  not a tuning knob but the mechanism that keeps the loop alive.");
}
