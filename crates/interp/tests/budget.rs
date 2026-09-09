//! GRAMMAR 8: "Hard runtime ceilings: recursion depth 128; total cost units per `choose` =
//! budget." Only the recursion ceiling was enforced. The cost ceiling was stored, exposed to
//! programs via the `budget` primitive, and never once compared against the accumulated cost.
//!
//! An evolved program with an unbounded loop therefore ran forever. The first real search-track
//! run burned a full core for FOUR HOURS on a single candidate and emitted nothing — and from
//! outside, a wedged evolution loop is indistinguishable from a slow one. FITNESS 10 lists
//! "infinite loop / budget abuse" as a degenerate solution the ceilings exist to catch, so an
//! unenforced ceiling is a hole a search over programs will eventually find on its own.

use board::Position;
use grammar::ast::*;
use grammar::{reference, Lineage};
use interp::Interp;
use nnue::Net;

fn b(n: Node) -> Box<Node> { Box::new(n) }

/// A program that loops far past any sane budget. This is the shape a mutation can produce by
/// accident: `loop` over a table-read count with a body that does real work.
fn runaway() -> grammar::Program {
    let body = Node::seq(vec![
        Node::Let("acc".into(), b(Node::Const(0)), b(Node::Nop)),
        // 1<<20 iterations is the interpreter's own clamp on Loop; each does an eval, which
        // costs 1365 units, so this wants ~1.4 BILLION cost units.
        Node::Loop(
            b(Node::Const(8)),
            b(Node::Loop(
                b(Node::Const(8)),
                b(Node::Loop(
                    b(Node::Const(8)),
                    b(Node::Set("acc".into(),
                        b(Node::Arith(ArithOp::Add, vec![v_acc(), Node::Eval(b(v_p()))])))),
                )),
            )),
        ),
        Node::Ret(b(Node::Argmax(
            b(Node::Moves(b(v_p()))), "m".into(), b(Node::Const(0)),
        ))),
    ]);
    grammar::Program {
        funcs: vec![Func {
            name: "choose".into(),
            params: vec![("p".into(), Ty::Pos), ("B".into(), Ty::Int)],
            ret: Ty::Move,
            body,
        }],
        lineage: Lineage::Main,
    }
}
fn v_acc() -> Node { Node::Var("acc".into()) }
fn v_p() -> Node { Node::Var("p".into()) }

#[test]
fn a_runaway_program_is_stopped_by_its_budget() {
    let net = Net::random(32, 1);
    let pos = Position::startpos();
    let mut it = Interp::new(&net, vec![2, 32_000, interp::UCT_EXPLORATION]);
    let budget = 50_000u64;
    it.cost_cap = budget;
    let mv = it.run(&runaway(), &pos, 1 << 20);
    assert!(it.over_budget, "the runaway program was not flagged over budget");
    assert!(it.cost < budget + 5000,
            "cost {} ran well past the budget {budget}: the ceiling is not binding", it.cost);
    assert_eq!(mv, board::types::MOVE_NONE,
               "an over-budget program must forfeit, not return a move it never justified");
}

#[test]
fn the_seed_finishes_inside_a_reasonable_budget() {
    // The ceiling must not be so tight that it rejects the seed itself — that would make every
    // candidate look like a runaway and the search would accept nothing.
    let net = Net::random(32, 1);
    let pos = Position::startpos();
    let mut it = Interp::new(&net, vec![2, 32_000, interp::UCT_EXPLORATION]);
    it.cost_cap = 2_000_000_000;
    let mv = it.run(&reference::bare_alpha_beta(), &pos, 1 << 20);
    assert!(!it.over_budget, "the seed ran out of budget at 50M units (cost {})", it.cost);
    assert!(pos.legal_moves().as_slice().contains(&mv), "seed returned an illegal move");
}
