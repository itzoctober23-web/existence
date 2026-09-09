//! RUNG 7 IS INERT: its reduction table is never populated, so `tread` reads 0 and the rung plays
//! exactly as the seed while costing more.
//!
//! WHAT GRAMMAR 9 ASKS FOR. Rung 7 is "table-driven reduction (LMR in embryo)": the child depth
//! becomes `max(d - 1 - R[d, i], 0)` where `i` counts moves and **R is a LEARNED table -- table 3,
//! alongside D, INF and MCTS's exploration weight** (`reference.rs:63-69`). The contents are meant to
//! be searched, so that "reduce late moves more" is something the loop DISCOVERS rather than a rule
//! written in by hand. That design is right, and it is not what is implemented.
//!
//! WHAT IS ACTUALLY IMPLEMENTED. `Interp` carries two table stores. `tables_nd: Vec<NdTable>` holds
//! genuinely indexed tables and is checked first; `tables: Vec<i64>` holds scalars. Repo-wide,
//! `tables_nd` appears exactly four times: the field declaration, an initialisation to `Vec::new()`
//! (`lib.rs:519`), a comment, and the read. **Nothing ever populates it.** So `TRead(3, [d, i])`
//! misses `tables_nd` and falls through to `tables.get(3)` -- and every caller in the repo passes a
//! THREE-element vector (`vec![depth, 32_000, uct_exploration()]`), so index 3 is absent and the read
//! yields `0`.
//!
//! WHY THAT IS WORSE THAN AN UNFINISHED FEATURE. It does not fail; it returns zero. With `R = 0` the
//! reduced depth `max(d - 1 - 0, 0)` is just `d - 1`, so rung 7 makes exactly the seed's moves --
//! while carrying the extra move-counter nodes the rung declares. Measured on the GRAMMAR 9 ladder it
//! would therefore read as **strictly worse than the seed**, and the natural conclusion, "table-driven
//! reduction does not help", would be a measurement of an unpopulated table rather than of the idea.
//! STATE.md already lists "Rung 7 needs its table contents DECLARED before it can be measured" under
//! *Not started*; this pins down exactly what is missing and locks it so it cannot regress silently.
//!
//! WHEN THE TABLE IS DECLARED, `plays_identically_to_the_seed` MUST START FAILING. That is the point:
//! it is a canary, not a guarantee. A rung whose behaviour is indistinguishable from the seed is
//! inert by definition.
use board::Position;
use grammar::reference;
use interp::Interp;
use nnue::Net;

const DEPTH: i64 = 3;

fn tables() -> Vec<i64> {
    // Exactly what every caller in the repo passes: three entries, so index 3 does not exist.
    vec![DEPTH, 32_000, interp::uct_exploration()]
}

#[test]
fn table_three_reads_zero_because_nothing_populates_it() {
    let net = Net::random(32, 20260909);
    let it = Interp::new(&net, tables());
    assert!(it.tables_nd.is_empty(),
            "tables_nd is populated now -- rung 7 may be live; update this test's premise");
    assert_eq!(tables().get(3), None,
               "a 4th scalar table exists now, so TRead(3) no longer falls through to 0");
}

#[test]
fn plays_identically_to_the_seed() {
    let net = Net::random(32, 20260909);
    let ab = reference::bare_alpha_beta();
    let tr = reference::table_reduction();

    let mut seed: u64 = 0x7A19_2026;
    let mut rnd = || { seed ^= seed << 13; seed ^= seed >> 7; seed ^= seed << 17; seed };

    let (mut compared, mut agreed) = (0usize, 0usize);
    // 8 walks x 16 plies = 128 positions: enough to clear the >100 floor below, while keeping
    // this at ~60s rather than the 136s the first version cost on every `cargo test`.
    for _ in 0..8 {
        let mut pos = Position::startpos();
        for _ in 0..16 {
            let list = pos.legal_moves();
            if list.is_empty() { break; }
            let a = Interp::new(&net, tables()).run(&ab, &pos, DEPTH);
            let b = Interp::new(&net, tables()).run(&tr, &pos, DEPTH);
            compared += 1;
            if a == b { agreed += 1; }
            let m = list.as_slice()[(rnd() % list.len() as u64) as usize];
            let _u = pos.make_move(m);
        }
    }
    println!("rung 7 vs seed: {agreed}/{compared} positions chose the SAME move");
    assert!(compared > 100, "only {compared} positions compared -- the walk is too short to conclude");
    assert_eq!(agreed, compared,
               "rung 7 diverged from the seed on {} of {} positions. If the reduction table has been \
                DECLARED, that is the intended outcome and this test should be retired.",
               compared - agreed, compared);
}
