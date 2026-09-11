//! Is `Op::AddFn` ACTUALLY behaviour-preserving, and does it ACTUALLY cost more?
//!
//! `mutate::PARKED_OPS` keeps `Op::AddFn` out of the drawn set, and the whole justification is two
//! claims stated as self-evident:
//!
//!   1. "AddFn is behaviour-PRESERVING by construction -- it is a refactor";
//!   2. "every candidate it produces is its parent plus a `Call` node, which `interp::cost_of`
//!      charges 2 ... under FITNESS 3 (mates per COST) that is strictly worse".
//!
//! `crates/grammar/tests/add_fn.rs` checks that the operator adds a function, keeps scope, stays
//! well-typed and refuses to lift a `ret`. **Nothing anywhere runs a lifted program and compares
//! what it DOES to its parent.** The premise the parking decision rests on was asserted, never
//! measured, and a type checker cannot catch a semantic change -- threading a free variable to the
//! wrong binder produces a perfectly well-typed program that plays differently.
//!
//! That matters in both directions. If claim 1 is false the operator has a bug, and the day it is
//! unparked it would inject silent behaviour changes labelled "refactor". If claim 1 is true, this
//! pins it so it cannot rot, and claim 2 stops being an assumption about a constant and becomes a
//! measured cost delta.
//!
//! ON CLAIM 2, and why the number is not "2": `Interp::exec` does `self.cost += cost_of(n)` on
//! EVERY NODE VISIT (`interp/src/lib.rs:637`), so cost is RUNTIME, not static program size. A lift
//! costs 2 per EXECUTION of the call, not 2 per program. This test reports the real distribution
//! rather than restating the constant.
//!
//! ANTI-VACUITY. Three ways this test could pass while measuring nothing, each guarded below:
//!   * no candidate is ever produced        -> `compared > 0`
//!   * every run forfeits, so both sides return MOVE_NONE and equality is trivial
//!                                          -> `real_moves > 0`
//!   * the "candidate" is not actually different -> asserted per candidate (`funcs.len()` rose)

use board::chess::Position;
use board::types::MOVE_NONE;
use grammar::{mutate, reference};
use interp::Interp;
use nnue::Net;

fn tables() -> Vec<i64> {
    vec![3, 32_000, interp::uct_exploration()]
}

fn positions() -> Vec<Position> {
    vec![
        Position::startpos(),
        Position::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1").unwrap(),
        Position::from_fen("r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3")
            .unwrap(),
    ]
}

/// Keeps the test to a few seconds on a loaded box. The point is a distribution, not exhaustion.
const MAX_CANDIDATES: usize = 60;

#[test]
fn add_fn_preserves_the_move_and_raises_the_cost() {
    let net = Net::random(16, 12345);
    let pos = positions();

    let mut candidates = 0usize;
    let mut compared = 0usize;
    let mut real_moves = 0usize;
    let (mut rose, mut equal, mut fell) = (0usize, 0usize, 0usize);
    let mut worst_ratio = 1.0f64;

    'outer: for (name, prog) in reference::all() {
        for fi in 0..prog.funcs.len() {
            for k in 0..60usize {
                if candidates >= MAX_CANDIDATES {
                    break 'outer;
                }
                let mut rng = mutate::Rng::new(k as u64 * 31 + 7);
                let Some(m) = mutate::mutate_at(&prog, mutate::Op::AddFn, &mut rng, fi, k) else {
                    continue;
                };
                // POSITIVE CONTROL: a candidate that is not different makes equality vacuous.
                assert!(
                    m.funcs.len() > prog.funcs.len(),
                    "{name}: AddFn applied without raising the function count"
                );
                candidates += 1;

                for (i, p) in pos.iter().enumerate() {
                    let mut ia = Interp::new(&net, tables());
                    let a = ia.run(&prog, p, 4);
                    let mut ib = Interp::new(&net, tables());
                    let b = ib.run(&m, p, 4);

                    assert_eq!(
                        a, b,
                        "{name} fi={fi} k={k} pos={i}: AddFn CHANGED THE MOVE. It is documented as \
                         behaviour-preserving and `mutate::PARKED_OPS` parks it on that basis. \
                         Either the free-variable threading bound a name to the wrong value, or \
                         the lift captured something it should have refused -- a type checker \
                         cannot see either."
                    );
                    compared += 1;
                    if a != MOVE_NONE {
                        real_moves += 1;
                    }
                    if ib.cost > ia.cost {
                        rose += 1;
                        if ia.cost > 0 {
                            let r = ib.cost as f64 / ia.cost as f64;
                            if r > worst_ratio {
                                worst_ratio = r;
                            }
                        }
                    } else if ib.cost == ia.cost {
                        equal += 1;
                    } else {
                        fell += 1;
                    }
                }
            }
        }
    }

    println!(
        "\n  AddFn behaviour check: {candidates} candidates, {compared} program/position pairs, \
         {real_moves} of them returning a real move\n  \
         cost after the lift:  rose {rose}   unchanged {equal}   FELL {fell}\n  \
         worst cost ratio observed: {worst_ratio:.3}x the parent\n"
    );

    assert!(
        compared > 0,
        "AddFn never produced a candidate, so this test measured nothing"
    );
    // The guard that matters: if every run forfeited, both sides return MOVE_NONE and the equality
    // above is true for a reason that has nothing to do with the operator.
    assert!(
        real_moves > 0,
        "every comparison returned MOVE_NONE -- the programs all forfeited at budget 4, so \
         'identical behaviour' is trivially true and this test proves nothing. Raise the budget."
    );
    // Claim 2 says the lift cannot make a program CHEAPER. A fall would mean the call is being
    // skipped -- i.e. the lifted body is not always executed, which is a behaviour change the move
    // comparison above happened not to expose.
    assert_eq!(
        fell, 0,
        "a lifted program ran CHEAPER than its parent in {fell} comparisons. A pure refactor \
         cannot reduce runtime cost; this means the lifted body is not always being executed."
    );
}
