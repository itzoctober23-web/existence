//! ARCH — the architecture candidate class (FITNESS 1).
//!
//! > | ARCH | architecture-menu step | no | fixed-cost-budget gate, then STC, LTC, VLTC once
//! > declared | held-out loss surrogate |
//!
//! MASTER_PLAN's Given table declares the MENU and nothing more:
//!
//! > | Menus (activations, optimizers, net-size steps) the searches choose from | Declared
//! > option sets, **not choices** |
//!
//! and the Learned section says of the net: *"Width, depth, activation, output bucketing,
//! king-relativity, symmetry: **discovered or not**."*
//!
//! Until now the loop took `--hidden 128` and the author moved it by hand after eyeballing a
//! sweep. That is the choice living on the wrong side of the line: the menu is declared, the
//! STEP ALONG IT has to be measured. This module holds the menu and proposes steps; nothing
//! here decides whether a step is good — `main.rs` sends it through the same gate as any other
//! candidate.

use nnue::Net;

use crate::search::Searcher;
use board::Position;

/// The declared option set. Net-size steps, geometric so each rung is a real change rather
/// than a rounding difference. This list is Given; the POSITION on it is not.
///
/// The bottom rung is deliberately tiny. Iteration zero should assume as little structure as
/// possible, and starting at 128 would be assuming the answer to the very question the ARCH
/// arm exists to ask.
pub const WIDTH_MENU: [usize; 6] = [16, 32, 64, 128, 256, 512];

/// Which rung a width sits on, if any.
pub fn rung_of(width: usize) -> Option<usize> {
    WIDTH_MENU.iter().position(|&w| w == width)
}

/// A step along the menu. `+1` widens, `-1` narrows; both are proposed, because "bigger is
/// better" is a hypothesis and the arm has to be able to walk back down.
pub fn step(rung: usize, dir: i32) -> Option<usize> {
    let next = rung as i32 + dir;
    if next < 0 || next as usize >= WIDTH_MENU.len() { None } else { Some(next as usize) }
}

/// Nanoseconds of search time per node for this net, measured rather than modelled.
///
/// A wider net costs more per evaluated leaf, and that cost is the entire reason the menu is
/// not simply "pick the largest rung". Measuring it is what lets the gate hand both arms the
/// same amount of WORK instead of the same depth.
///
/// Estimator is the MINIMUM over repeats: search time is a positive quantity contaminated by
/// scheduler noise in one direction only, so the min is the closest thing to the true cost
/// and the mean is biased upward by however busy the box happened to be.
pub fn ns_per_node(net: &Net, depth: u32, repeats: usize) -> f64 {
    let mut s = Searcher::new();
    let mut best = f64::INFINITY;
    // Warm-up: first call allocates the accumulator stack and touches the weight matrix.
    let mut warm = Position::startpos();
    s.best_move_capped(&mut warm, depth, net, u64::MAX, 1);
    for i in 0..repeats.max(1) {
        let mut pos = Position::startpos();
        let t = std::time::Instant::now();
        s.best_move_capped(&mut pos, depth, net, u64::MAX, i as u64 + 1);
        let el = t.elapsed().as_nanos() as f64;
        if s.nodes > 0 {
            let per = el / s.nodes as f64;
            if per < best { best = per; }
        }
    }
    if best.is_finite() { best } else { 0.0 }
}

/// Node budgets that give two nets the SAME wall-clock time per move.
///
/// This is the difference between FITNESS 6 and FITNESS 7, made concrete. The fixed-cost-budget
/// gate asks "is this net better per unit of search?" — equal nodes. The clock asks "is it
/// better per unit of TIME?" — and a net twice as expensive per node gets half the nodes. The
/// degenerate solution FITNESS 10 names ("bigger net that wins fixed-cost-budget, loses on
/// clock") is only catchable if something actually charges for the width.
///
/// Budgets are in NODES, so a gate run stays deterministic and replayable from its seed; the
/// clock enters through the measured per-node cost, once, instead of through a wall-clock
/// deadline inside the game.
pub fn equal_time_caps(a: &Net, b: &Net, budget_ns: f64, probe_depth: u32) -> (u64, u64) {
    let ca = ns_per_node(a, probe_depth, 3).max(1e-9);
    let cb = ns_per_node(b, probe_depth, 3).max(1e-9);
    let na = (budget_ns / ca).max(1.0) as u64;
    let nb = (budget_ns / cb).max(1.0) as u64;
    (na, nb)
}

/// What the ARCH arm proposes this generation: a target rung, or None when the menu has no
/// untried neighbour left in that direction.
#[derive(Debug, Clone, Copy)]
pub struct ArchProposal {
    pub from_rung: usize,
    pub to_rung: usize,
    pub dir: i32,
}

impl ArchProposal {
    pub fn width(&self) -> usize {
        WIDTH_MENU[self.to_rung]
    }
}

/// Alternate the direction proposed so the arm cannot only ever widen. `attempt` counts ARCH
/// proposals made so far. Widening is tried first only because the champion starts at the
/// bottom rung, where narrowing does not exist.
pub fn propose(rung: usize, attempt: usize) -> Option<ArchProposal> {
    let order = if attempt % 2 == 0 { [1, -1] } else { [-1, 1] };
    for dir in order {
        if let Some(to) = step(rung, dir) {
            return Some(ArchProposal { from_rung: rung, to_rung: to, dir });
        }
    }
    None
}
