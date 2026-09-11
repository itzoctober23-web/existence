//! DIAGNOSTIC for the AddFn behaviour failure — is it the operator, or is it my harness?
//!
//! `add_fn_is_behaviour_preserving.rs` reports that AddFn changes the move on the bare alpha-beta
//! seed at fi=1 (the `ab` function), k=40, startpos. Before that is written down as "the operator
//! has a bug", the harness has to be cleared. Two ways the TEST could be wrong:
//!
//!   1. `Interp::run` is not deterministic (a `Sample` node, or state carried across runs), in
//!      which case parent-vs-parent would differ too and the comparison is meaningless;
//!   2. the cost cap truncates the more expensive program, so the difference is the CAP biting
//!      rather than a semantic change. (`cost_cap` is 2e9 and budget is 4, so this is already
//!      unlikely -- it is checked rather than assumed.)
//!
//! Prints rather than asserts, except for the determinism control, which is the one thing that
//! must hold for any of the rest to mean anything.

use board::chess::Position;
use board::types::MOVE_NONE;
use grammar::{mutate, reference};
use interp::Interp;
use nnue::Net;

fn tables() -> Vec<i64> {
    vec![3, 32_000, interp::uct_exploration()]
}

#[test]
fn diagnose_the_add_fn_behaviour_change() {
    let net = Net::random(16, 12345);
    let p = Position::startpos();

    let (name, prog) = reference::all()
        .into_iter()
        .find(|(n, _)| n.contains("alpha-beta") || n.contains("main seed"))
        .expect("could not find the bare alpha-beta seed in reference::all()");
    println!("\n  program: {name}   funcs={}", prog.funcs.len());
    for (i, f) in prog.funcs.iter().enumerate() {
        println!("    func[{i}] {} params={:?} ret={:?}", f.name, f.params, f.ret);
    }

    // ---- CONTROL 1: is the interpreter deterministic at all? -----------------------------------
    let mut c1 = Interp::new(&net, tables());
    let m1 = c1.run(&prog, &p, 4);
    let mut c2 = Interp::new(&net, tables());
    let m2 = c2.run(&prog, &p, 4);
    println!(
        "\n  DETERMINISM CONTROL: parent twice -> {:?} / {:?}   cost {} / {}",
        m1, m2, c1.cost, c2.cost
    );
    assert_eq!(
        m1, m2,
        "the SAME program returned two different moves. The interpreter is not deterministic, so \
         the behaviour comparison in add_fn_is_behaviour_preserving.rs cannot mean anything and \
         that test must be rewritten before its failure is interpreted."
    );
    assert_eq!(c1.cost, c2.cost, "the same program cost two different amounts");

    // ---- The failing candidate ------------------------------------------------------------------
    let fi = 1usize;
    let k = 40usize;
    let mut rng = mutate::Rng::new(k as u64 * 31 + 7);
    let Some(m) = mutate::mutate_at(&prog, mutate::Op::AddFn, &mut rng, fi, k) else {
        println!("  candidate no longer applies at fi={fi} k={k}");
        return;
    };

    println!("\n  candidate: funcs={} (parent {})", m.funcs.len(), prog.funcs.len());
    if let Some(lifted) = m.funcs.last() {
        println!(
            "    lifted fn: {} params={:?} ret={:?} body_size={}",
            lifted.name,
            lifted.params,
            lifted.ret,
            lifted.body.size()
        );
    }

    // ---- THE MECHANISM, checked rather than asserted -------------------------------------------
    // `Node::Call` builds a FRESH env from the params, runs the body against it, and drops it --
    // only the return value escapes. So a `Set` to a free variable inside a lifted body writes to
    // the callee's local copy and the update is lost. If the lifted body contains a `Set`, that is
    // the whole explanation for a Unit-returning lift changing behaviour.
    // Checked on the Debug rendering rather than by matching variants: the walk has to cover every
    // node kind to be sound, and getting that list wrong would silently answer "no Set" for a body
    // that has one -- the exact false-negative this diagnostic exists to avoid. The body is printed
    // in full directly below, so the claim is verifiable by eye and does not rest on this check.
    if let Some(lifted) = m.funcs.last() {
        let dbg = format!("{:?}", lifted.body);
        println!(
            "    lifted body mentions Set( ? {}   (assignment to a threaded param)",
            dbg.contains("Set(")
        );
        println!("    lifted body = {}", &dbg[..dbg.len().min(400)]);
    }

    let mut ia = Interp::new(&net, tables());
    let a = ia.run(&prog, &p, 4);
    let mut ib = Interp::new(&net, tables());
    let b = ib.run(&m, &p, 4);

    println!(
        "\n  parent    move={:?}  cost={}  over_budget_forfeit={}",
        a,
        ia.cost,
        a == MOVE_NONE
    );
    println!(
        "  candidate move={:?}  cost={}  over_budget_forfeit={}",
        b,
        ib.cost,
        b == MOVE_NONE
    );
    println!(
        "  cost cap = {} (a truncation explanation needs cost to reach this)",
        ia.cost_cap
    );
    println!("  SAME MOVE? {}\n", a == b);
}
