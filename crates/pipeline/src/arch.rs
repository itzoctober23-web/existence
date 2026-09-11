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

/// Alternate the direction proposed so the arm cannot only ever widen, and GROW THE STRIDE with
/// each attempt: +1, -1, +2, -2, +3, ... `attempt` counts ARCH proposals made so far. Widening is
/// tried first only because the champion starts at the bottom rung, where narrowing does not exist.
///
/// WHY THE STRIDE GROWS. At a fixed stride of 1 the menu has a COST CLIFF at its first rung that
/// the arm can never cross. Measured in search.rs:68-74 -- node counts are identical on both
/// paths at every width, so the ratios are real:
///
/// | width | refresh | incremental | |
/// |---|---|---|---|
/// | 32 | 1718969 | 1560497 | 0.91x LOSS |
/// | 128 | 889406 | 1014240 | 1.14x win |
/// | 512 | 240701 | 363458 | 1.51x win |
///
/// ⚠ **SUPERSEDED 2026-09-10 -- THE CLIFF THIS DESCRIBES IS GONE.** The table predates `cd3e924`
/// (02:01), which replaced the accumulator's diff with `FeatSnap::delta`, a per-plane bitboard XOR
/// that is O(features CHANGED) (typically 2-6) rather than O(features ACTIVE). That commit edited
/// this crate's `search.rs` and relabelled the copy of the table THERE as "the ORIGINAL finding",
/// but did not touch this copy -- so the stale figure survived in the file that reasons about the
/// menu, and `width_clock_RESULT.md` later cited it.
///
/// Re-measured 22:3x on `search_bench` depth 4, node counts IDENTICAL between arms
/// (144321 = 144321 at w32; 77146 = 77146 at w128), best-of-5:
///
/// | width | refresh | incremental | |
/// |---|---|---|---|
/// | 32 | 1718107 | 2255016 | **1.31x WIN** (was 0.91x LOSS) |
/// | 128 | 940805 | 1264689 | 1.34x win |
///
/// The refresh arm reproduces the old number to **0.05%** (1718107 against 1718969) -- that is what
/// establishes the harness is unchanged and the incremental path genuinely got faster, rather than
/// the measurement having drifted.
///
/// **What still holds:** a wider net is still slower per SECOND (w128 incremental is 0.56x w32
/// incremental), so the clock gate still rejects widening and every ARCH verdict in
/// `width_clock_RESULT.md` stands. What is no longer true is the REASON -- there is no cost cliff
/// at 32 specifically, and the accumulator does not "start paying at 128". It pays at every width.
/// The stride growth below is still wanted, but for menu REACH, not to clear a cliff.
///
/// The historical reading, kept because the stride logic was built on it: the incremental
/// accumulator did not pay until ~128, so width 32 carried a wider net's cost
/// with none of its saving. That is precisely what the first widening ever to reach a game gate
/// measured: `ARCH w 16 -> w 32 (loss 0.0746 vs 0.0847, paired z 4.21) fixed-cost 0.525 ok,
/// clock 0.372 [5962 vs 6985 nodes] => hold` -- better per NODE, much worse per SECOND, rejected
/// on the clock exactly as FITNESS 10 intends. That rejection is correct. The defect is that with
/// stride 1 the ONLY widening reachable from rung 0 is that rung, so the arm re-proposes a known
/// cost cliff forever and never sees 128, where the same measurements say the cost flips.
///
/// This does NOT hardcode "128 is good" -- that would hand the search its answer, which is the
/// thing this project refuses to do. It widens the arm's REACH so the menu stays searchable past
/// a locally-unprofitable rung; the fixed-cost and clock gates still decide every step, on games.
pub fn propose(rung: usize, attempt: usize) -> Option<ArchProposal> {
    let stride = (attempt / 2 + 1) as i32;
    let order = if attempt % 2 == 0 { [stride, -stride] } else { [-stride, stride] };
    for dir in order {
        if let Some(to) = step(rung, dir) {
            return Some(ArchProposal { from_rung: rung, to_rung: to, dir });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The arm must be able to REACH a rung past a locally-unprofitable one.
    ///
    /// With a fixed stride of 1 the only widening from rung 0 is width 32, where the
    /// incremental accumulator is a measured 0.91x LOSS -- so the clock gate rejects it (it did:
    /// 0.372 +/- 0.053), the arm re-proposes the same cliff forever, and width 128 (1.14x win)
    /// is unreachable by construction. This asserts the menu stays searchable past it.
    #[test]
    fn proposals_reach_past_the_cost_cliff() {
        let widths: Vec<usize> = (0..8)
            .filter_map(|a| propose(0, a).map(|p| p.width()))
            .collect();
        assert!(widths.contains(&128),
                "the arm never reaches width 128, where the accumulator starts paying: {widths:?}");
        assert!(widths.contains(&32), "it must still try the adjacent rung first: {widths:?}");
        assert_eq!(widths[0], 32, "the FIRST proposal must stay the cheap adjacent step");
    }

    /// It must still be able to NARROW. Capacity search that can only grow is not a search, and
    /// EXPERIMENTS.md records that moving capacity DOWN was never even tried on this loop.
    #[test]
    fn proposals_go_both_ways_from_the_middle() {
        let mid = rung_of(64).expect("64 is on the menu");
        let dirs: Vec<i32> = (0..4).filter_map(|a| propose(mid, a).map(|p| p.dir)).collect();
        assert!(dirs.iter().any(|&d| d > 0), "never widens from the middle: {dirs:?}");
        assert!(dirs.iter().any(|&d| d < 0), "never narrows from the middle: {dirs:?}");
    }

    /// A proposal must never fall off the menu.
    #[test]
    fn proposals_stay_in_bounds() {
        for rung in 0..WIDTH_MENU.len() {
            for attempt in 0..12 {
                if let Some(p) = propose(rung, attempt) {
                    assert!(p.to_rung < WIDTH_MENU.len(), "rung {} off the menu", p.to_rung);
                }
            }
        }
    }
}
