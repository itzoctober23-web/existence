//! ARCH class: the declared menu, the node budget, and the equal-time charge.
//!
//! The point of these tests is that the WIDTH CHOICE is now a measurement. Two things have to
//! hold for that to be true rather than decorative: the search must actually stop at its budget
//! (otherwise "equal cost" is a label on nothing), and a wider net must actually be charged for
//! being slower (otherwise every widening step passes and the menu walk is a foregone conclusion
//! I dressed up as a search).

use board::Position;
use nnue::Net;
use pipeline::arch::{self, WIDTH_MENU};
use pipeline::search::Searcher;

#[test]
fn menu_is_a_declared_increasing_option_set() {
    assert!(WIDTH_MENU.windows(2).all(|w| w[0] < w[1]), "menu must be ordered");
    for (i, &w) in WIDTH_MENU.iter().enumerate() {
        assert_eq!(arch::rung_of(w), Some(i));
    }
    assert_eq!(arch::rung_of(100), None, "off-menu widths are not rungs");
}

#[test]
fn steps_stay_on_the_menu() {
    assert_eq!(arch::step(0, -1), None, "cannot go below the bottom rung");
    assert_eq!(arch::step(WIDTH_MENU.len() - 1, 1), None, "cannot go above the top rung");
    assert_eq!(arch::step(0, 1), Some(1));
    assert_eq!(arch::step(2, -1), Some(1));
}

#[test]
fn proposals_alternate_direction() {
    // From a middle rung the arm must be able to propose NARROWING, not only widening.
    // "Bigger is better" is the hypothesis under test; an arm that can only widen is not
    // testing it, it is assuming it.
    let up = arch::propose(2, 0).unwrap();
    let down = arch::propose(2, 1).unwrap();
    assert_eq!(up.dir, 1);
    assert_eq!(down.dir, -1);
    assert_eq!(up.to_rung, 3);
    assert_eq!(down.to_rung, 1);
    // At the bottom rung there is nowhere to narrow to, so both attempts widen.
    assert_eq!(arch::propose(0, 1).unwrap().dir, 1);
}

#[test]
fn the_node_budget_actually_binds() {
    let net = Net::random(32, 7);
    let mut s = Searcher::new();

    let mut p = Position::startpos();
    s.best_move_capped(&mut p, 4, &net, u64::MAX, 1);
    let uncapped = s.nodes;
    assert!(uncapped > 5_000, "depth 4 from startpos should be well over the cap, got {uncapped}");

    let cap = 1_000;
    let mut p = Position::startpos();
    let (m, _) = s.best_move_capped(&mut p, 4, &net, cap, 1);
    assert!(s.aborted, "a 1k budget at depth 4 must run out");
    assert!(s.nodes <= cap, "search ran {} nodes past a cap of {cap}", s.nodes);
    assert!(m != board::types::MOVE_NONE, "must still return a legal move when the budget ends");
    assert!(p.legal_moves().as_slice().contains(&m), "returned move must be legal");
}

#[test]
fn capped_search_is_deterministic() {
    // The budget is in NODES, not milliseconds, precisely so a gate replays identically from
    // its seed (FITNESS 10, determinism check). A wall-clock deadline would not.
    let net = Net::random(32, 11);
    let mut a = Searcher::new();
    let mut b = Searcher::new();
    let (mut p1, mut p2) = (Position::startpos(), Position::startpos());
    let (m1, s1) = a.best_move_capped(&mut p1, 4, &net, 900, 12345);
    let (m2, s2) = b.best_move_capped(&mut p2, 4, &net, 900, 12345);
    assert_eq!(m1, m2);
    assert_eq!(s1, s2);
    assert_eq!(a.nodes, b.nodes);
}

#[test]
fn a_wider_net_is_charged_for_being_slower() {
    // THE mechanism. If this fails, the ARCH arm is a rubber stamp: equal-time budgets would
    // hand a 512-wide net as many nodes as a 16-wide one and the menu walk would always go up.
    let small = Net::random(WIDTH_MENU[0], 3);
    let large = Net::random(*WIDTH_MENU.last().unwrap(), 3);
    let budget_ns = arch::ns_per_node(&small, 3, 5) * 4_000.0;
    let (cap_small, cap_large) = arch::equal_time_caps(&small, &large, budget_ns, 3);
    assert!(
        cap_small > cap_large,
        "width {} got {cap_small} nodes and width {} got {cap_large}: the wide net was not charged",
        small.n_hidden, large.n_hidden
    );
    // 32x the hidden width should cost meaningfully more per node. Deliberately loose -- this
    // asserts the SIGN and rough magnitude of the charge, not a benchmark number that would go
    // red whenever the box is busy.
    assert!(
        cap_small as f64 > cap_large as f64 * 1.5,
        "expected the 32x wider net to get well under 2/3 the nodes; {cap_small} vs {cap_large}"
    );
}
