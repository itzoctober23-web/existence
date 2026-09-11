//! The reference programs of GRAMMAR.md 6, written out in the grammar so their sizes are
//! MEASURED rather than estimated.
//!
//! This module is the one that matters for the thesis. GRAMMAR.md states the prior as a number
//! — "alpha-beta is N nodes and the main seed; MCTS is M and K mutations away" — and until now
//! those numbers were hand estimates flagged +/-20%. The parser makes them measurements, and
//! the declared prior is re-derived from what this module reports.

use crate::ast::*;

fn v(s: &str) -> Node {
    Node::Var(s.into())
}
fn b(n: Node) -> Box<Node> {
    Box::new(n)
}

/// Purity-lineage seed: evaluate each legal move's resulting position, play the max.
/// No lookahead, no minimax, no bounding — those are what the lineage must rediscover.
pub fn depth_one() -> Program {
    // choose(p, B) = argmax(moves(p), m -> neg(eval(apply(p, m))))
    let body = Node::Argmax(
        b(Node::Moves(b(v("p")))),
        "m".into(),
        b(Node::Arith(
            ArithOp::Neg,
            vec![Node::Eval(b(Node::Apply(b(v("p")), b(v("m")))))],
        )),
    );
    Program {
        funcs: vec![Func {
            name: "choose".into(),
            params: vec![("p".into(), Ty::Pos), ("B".into(), Ty::Int)],
            ret: Ty::Move,
            body,
        }],
        lineage: Lineage::Purity,
    }
}

/// Main-lineage seed: BARE alpha-beta. Depth and INF are TABLE READS, not constants, so even
/// the seed's search depth is a tuned value rather than a given (GRAMMAR 5.2).
pub fn bare_alpha_beta() -> Program { ab_program(false, false, false, false) }

/// RUNG 6 of the GRAMMAR 9 ladder: capture extension at the horizon (qsearch in embryo).
///
/// Single mutation of the seed: the recursive depth argument `d - 1` becomes "`d` when this
/// move is a capture, `d - 1` otherwise", so a capture does not consume depth and tactical
/// sequences are searched to their end. GRAMMAR 9 writes this as
/// "wrap-if(pred(m,p,is_capture)) around depth check".
///
/// THIS IS NOT SEEDING QSEARCH INTO THE ENGINE, and the distinction is the whole point of the
/// rig. MASTER_PLAN line 53 requires the SEED to contain "no quiescence ... all of these must be
/// DISCOVERED as program edits that beat the current program on the clock", and
/// `bare_alpha_beta()` is unchanged -- byte-identical, still 71 nodes. MASTER_PLAN line 141
/// separately requires the offline ladder to "verify alpha-beta, hash reuse, ID, and qsearch are
/// EXPRESSIBLE in the grammar and that a path of single mutations from the seed exists where
/// every step is fitter", using the test eval "for this rig only". This function is that check
/// and lives only in the reference set the ladder measures; nothing in the search or the
/// evolution loop reads it.
pub fn capture_extension() -> Program { ab_program(true, false, false, false) }

/// RUNG 7 of the GRAMMAR 9 ladder: table-driven reduction (LMR in embryo).
///
/// GRAMMAR 9 writes it as "tread(reduction, depth, index) in the recursive depth". The child
/// depth becomes `max(d - 1 - R[d, i], 0)` where `i` counts moves and R is a LEARNED table --
/// table 3, alongside D, INF and MCTS's exploration weight. Nothing states what the reduction
/// should BE: the table's contents are searched, so "reduce late moves more" stays something
/// the loop can discover rather than a rule written in by hand.
///
/// Two details are load-bearing. The floor at 0: the horizon guard tests `d == 0` with Eq, so a
/// negative depth would slip straight past it and recurse without bound. And the counter is
/// declared ONLY for this variant -- declaring it unconditionally moved the seed from 71 to 73
/// nodes, silently rewriting the declared prior that GRAMMAR 6 and every ladder distance are
/// measured against. `examples/prior` caught that.
pub fn table_reduction() -> Program { ab_program(false, true, false, false) }

/// (a) EXTEND-BY-UNCERTAINTY: spend depth where the eval says it is unreliable.
///
/// A YARDSTICK, NOT A SEED. Like `capture_extension`, it lives only in the reference set the
/// ladder measures; `bare_alpha_beta()` is untouched and nothing in the evolution loop reads it.
/// Its job is to put a NUMBER on the distance from the seed -- so if the population ever assembles
/// this shape we know how far it travelled, and if it never does we know how far it would have
/// had to.
pub fn uncertainty_extension() -> Program { ab_program(false, false, true, false) }

/// (b) MIX-BACKUP: back up a blend of max and average, weighted by the net's own uncertainty.
///
/// A YARDSTICK, NOT A SEED. `bare_alpha_beta()` is untouched. Untuned it is a pure average and
/// therefore much weaker than the seed -- which is the point of measuring it rather than shipping
/// it: it puts a number on how far this shape sits from where the loop starts.
pub fn mix_backup_program() -> Program { ab_program(false, false, false, true) }

fn ab_program(cap_ext: bool, reduce: bool, unc_ext: bool, mix_backup: bool) -> Program {
    let d = Node::TRead(0, vec![]); // table "D"
    let inf = Node::TRead(1, vec![]); // table "INF"

    // choose(p,B) = argmax(moves(p), m -> neg(ab(apply(p,m), D, neg(INF), INF)))
    let choose = Node::Argmax(
        b(Node::Moves(b(v("p")))),
        "m".into(),
        b(Node::Arith(
            ArithOp::Neg,
            vec![Node::Call(
                1,
                vec![
                    Node::Apply(b(v("p")), b(v("m"))),
                    d.clone(),
                    Node::Arith(ArithOp::Neg, vec![inf.clone()]),
                    inf.clone(),
                ],
            )],
        )),
    );

    // ab(p, d, a, b):
    //   if terminal(p) != NONE: ret score_of(terminal(p), d)
    //   if d == 0: ret eval(p)
    //   let best = neg(INF)
    //   foreach m in moves(p):
    //     let vv = neg(ab(apply(p,m), d-1, neg(b), neg(a)))
    //     set best = max(best, vv); set a = max(a, vv)
    //     if a >= b: ret best
    //   ret best
    let term_guard = Node::If(
        b(Node::Cmp(
            b(Node::Terminal(b(v("p")))),
            b(Node::OutcomeLit(OutcomeLit::None)),
            Rel::Ne,
        )),
        b(Node::Ret(b(Node::ScoreOf(
            b(Node::Terminal(b(v("p")))),
            b(v("d")),
        )))),
        None,
    );
    let depth_guard = Node::If(
        b(Node::Cmp(b(v("d")), b(Node::Const(0)), Rel::Eq)),
        b(Node::Ret(b(Node::Eval(b(v("p")))))),
        None,
    );
    // `nd` = the depth handed to the child. Plain d-1 in the seed; in the capture-extension
    // rung a capture keeps the depth, so the tactical line is searched to its end. Written as
    // Let + conditional Set because the grammar's If is a STATEMENT, not a ternary expression.
    // Move counter + reduced child depth, for the reduction rung only.
    let reduce_setup: Vec<Node> = if reduce {
        vec![
            Node::Set("i".into(), b(Node::Arith(ArithOp::Add, vec![v("i"), Node::Const(1)]))),
            Node::Let("nd".into(),
                b(Node::Max(
                    b(Node::Arith(ArithOp::Sub, vec![
                        Node::Arith(ArithOp::Sub, vec![v("d"), Node::Const(1)]),
                        Node::TRead(3, vec![v("d"), v("i")]),
                    ])),
                    b(Node::Const(0)),
                )),
                b(Node::Nop)),
        ]
    } else { vec![] };
    // (a) EXTEND-BY-UNCERTAINTY -- a yardstick, not a seed.
    //
    // Identical in SHAPE to the capture extension above with the CONDITION swapped: instead of
    // "this move is a capture", the guard is "the net says it is unsure here". The plan writes it
    // as `wrap-if(cmp(unc, tread(T)))` around an extension, which is literally what this is.
    //
    // THE THRESHOLD IS A LEARNED TABLE, never a constant. Nothing here states how unsure is unsure
    // enough -- table T's contents are searched, so "extend where the eval is unreliable" stays
    // something the loop discovers rather than a rule written in by hand. Same discipline
    // table_reduction uses for its reduction amounts.
    //
    // AT THE HORIZON ONLY (`d == 1`), for the reason the capture extension records directly above:
    // applying an extension at every depth made captures free throughout the tree and blew cost up
    // 73x. An uncertainty extension is if anything more dangerous there, because `unc` is non-zero
    // over far more positions than `is_capture` is true.
    //
    // INERT UNTIL BOTH HALVES EXIST, which is worth stating. `TRead` falls back to
    // `tables.get(i).unwrap_or(&0)`, so with no table supplied the threshold is 0; and `unc` reads
    // 0 on every net whose head is untrained. The guard is then `0 > 0` = false and this program is
    // behaviourally the seed. It cannot quietly do something before there is anything to do.
    let unc_setup: Vec<Node> = if unc_ext {
        vec![
            Node::Let("nd".into(),
                b(Node::Arith(ArithOp::Sub, vec![v("d"), Node::Const(1)])), b(Node::Nop)),
            Node::If(
                b(Node::Cmp(b(v("d")), b(Node::Const(1)), Rel::Eq)),
                b(Node::If(
                    b(Node::Cmp(
                        b(Node::Unc(b(v("p")))),
                        b(Node::TRead(4, vec![])),      // table "T": the uncertainty threshold
                        Rel::Gt,
                    )),
                    b(Node::Set("nd".into(), b(v("d")))),
                    None,
                )),
                None,
            ),
        ]
    } else { vec![] };
    let nd_setup: Vec<Node> = if cap_ext {
        vec![
            Node::Let("nd".into(),
                b(Node::Arith(ArithOp::Sub, vec![v("d"), Node::Const(1)])), b(Node::Nop)),
            // AT THE HORIZON ONLY. GRAMMAR 9 rung 6 says "capture extension AT HORIZON", and this
            // applied the extension inside the move loop at EVERY depth, so captures were free
            // throughout the tree -- not quiescence, but full-width search with captures
            // unbounded.
            //
            // That was invisible for as long as `pred` was a stub returning false: the branch
            // never fired, so nobody could see it was in the wrong place. The moment the predicate
            // was implemented it showed up as a 73x cost blowup that truncated 2 of 3 searches
            // even at a raised cost cap (24,440,705 evals against the seed's 441,471).
            //
            // Guarding on `d == 1` means the extension fires only when the child would otherwise
            // hit the horizon. A capture there keeps depth 1, so a capture CHAIN extends until the
            // captures run out -- which is quiescence, and is what the rung is supposed to be.
            Node::If(
                b(Node::Cmp(b(v("d")), b(Node::Const(1)), Rel::Eq)),
                b(Node::If(
                    b(Node::Pred(b(v("m")), b(v("p")), PredId::IsCapture)),
                    b(Node::Set("nd".into(), b(v("d")))),
                    None,
                )),
                None,
            ),
        ]
    } else { vec![] };
    let mut loop_stmts = nd_setup;
    loop_stmts.extend(unc_setup);
    loop_stmts.extend(reduce_setup);
    loop_stmts.extend(vec![
        Node::Let(
            "vv".into(),
            b(Node::Arith(
                ArithOp::Neg,
                vec![Node::Call(
                    1,
                    vec![
                        Node::Apply(b(v("p")), b(v("m"))),
                        if cap_ext || reduce || unc_ext { v("nd") } else {
                            Node::Arith(ArithOp::Sub, vec![v("d"), Node::Const(1)])
                        },
                        Node::Arith(ArithOp::Neg, vec![v("b")]),
                        Node::Arith(ArithOp::Neg, vec![v("a")]),
                    ],
                )],
            )),
            b(Node::Nop),
        ),
        // (b) MIX-BACKUP -- the value backup becomes a weighted blend of max and average.
        //
        // `mix(a, b, w)` is `(a*w + b*(16-w))/16`, so w = 16 is pure max (exactly the seed) and
        // w = 0 is pure average. The weight comes from a LEARNED table indexed by the net's own
        // uncertainty, which is the `mix(max, avg, tread(W, unc_bucket))` the plan asks for: "back
        // up something richer than a max of noisy numbers", with HOW MUCH richer left to be
        // discovered rather than written in.
        //
        // THE BUCKETING IS THE TABLE'S, not a hand-written quantiser. `unc` returns Score units, and
        // an indexed `tread` clamps each index with `v.min(dim-1)`, so the table's own dimension
        // defines the buckets. Writing `unc / K` here would invent the quantisation policy, which is
        // exactly the kind of choice that belongs to the search.
        //
        // ONLY THE VALUE IS MIXED. The alpha update on the next line stays a MAX deliberately:
        // alpha is a BOUND, and averaging it would cut branches the bound no longer justifies --
        // alpha-beta's soundness rests on alpha being a true lower bound, not an estimate.
        //
        // WITH NO TABLE SUPPLIED THIS IS A PURE AVERAGE, not the seed. `tread` falls back to
        // `tables.get(i).unwrap_or(&0)` and w = 0 means all-average, so an untuned mix-backup is a
        // genuinely different and much weaker program. That is a true property of it and the
        // reason it is a YARDSTICK: the ladder measures what this shape costs, it is not a
        // candidate anyone ships.
        Node::Set("best".into(), if mix_backup {
            b(Node::Mix(
                b(Node::Max(b(v("best")), b(v("vv")))),
                b(Node::Avg(b(v("best")), b(v("vv")))),
                b(Node::TRead(5, vec![Node::Unc(b(v("p")))])),   // table "W", indexed by unc
            ))
        } else {
            b(Node::Max(b(v("best")), b(v("vv"))))
        }),
        Node::Set("a".into(), b(Node::Max(b(v("a")), b(v("vv"))))),
        Node::If(
            b(Node::Cmp(b(v("a")), b(v("b")), Rel::Ge)),
            b(Node::Ret(b(v("best")))),
            None,
        ),
    ]);
    let loop_body = Node::seq(loop_stmts);
    let mut body_stmts = vec![
        term_guard,
        depth_guard,
        Node::Let(
            "best".into(),
            b(Node::Arith(ArithOp::Neg, vec![inf])),
            b(Node::Nop),
        ),
    ];
    if reduce {
        body_stmts.push(Node::Let("i".into(), b(Node::Const(0)), b(Node::Nop)));
    }
    body_stmts.push(Node::Foreach(b(Node::Moves(b(v("p")))), "m".into(), b(loop_body)));
    body_stmts.push(Node::Ret(b(v("best"))));
    let ab_body = Node::seq(body_stmts);

    Program {
        funcs: vec![
            Func {
                name: "choose".into(),
                params: vec![("p".into(), Ty::Pos), ("B".into(), Ty::Int)],
                ret: Ty::Move,
                body: choose,
            },
            Func {
                name: "ab".into(),
                params: vec![
                    ("p".into(), Ty::Pos),
                    ("d".into(), Ty::Int),
                    ("a".into(), Ty::Score),
                    ("b".into(), Ty::Score),
                ],
                ret: Ty::Score,
                body: ab_body,
            },
        ],
        lineage: Lineage::Main,
    }
}

/// FAITHFUL alpha-beta + transposition table. The first milestone the search track is expected
/// to discover (GRAMMAR 9 step 4).
///
/// THE PREVIOUS VERSION IN THIS FILE WAS NOT A TRANSPOSITION TABLE AT ALL. It was:
///
/// ```text
/// if probe(p).depth >= d:  ret probe(p).score
/// ...body...
/// store(key p, Score, best)
/// ```
///
/// and it stored ONLY `Score`. `Depth` was never written, so every slot in the table had
/// `depth = 0`, and an EMPTY slot returns `Slot::default()` which is also `depth = 0`. So the
/// guard read `0 >= d`, which is TRUE at every leaf — and the program returned `score` from an
/// empty slot, i.e. the constant 0, without ever calling `eval`.
///
/// MEASURED (crates/interp/examples/tt_pressure.rs), 60 random positions:
///
/// ```text
/// depth 2   EVALS hash 0   bare   516,829     cost 0.285x
/// depth 3   EVALS hash 0   bare 7,206,527     cost 0.218x
/// depth 4   EVALS hash 0   bare 66,932,291    cost 0.072x
/// ```
/// Zero evaluations at every depth. The "14x cheaper than alpha-beta" was a search that had
/// stopped searching. It agreed with bare alpha-beta on 1 of 60 positions at depth 4.
///
/// This is precisely the failure GRAMMAR 6 records as its central lesson — "a sketch is a lower
/// bound, never a datum" — sitting inside the table that claims "no sketches remain", labelled
/// `faithful`. It is also the degenerate solution FITNESS 10 lists first ("prune everything /
/// return eval"), except worse: it returns a CONSTANT and never evaluates. It was not produced
/// by evolution; it was in the reference set used to calibrate the prior.
///
/// The faithful version needs the two things a real TT cannot omit:
///   1. A VALIDITY marker. `flag != 0` distinguishes a stored entry from an empty slot; the
///      old code had no way to, which is the whole bug.
///   2. BOUND TYPES. An alpha-beta score is only a bound outside the window it was searched
///      with, so a cutoff is legal only when the bound permits: EXACT always, LOWER when it
///      still fails high, UPPER when it still fails low. Returning a stored score
///      unconditionally is unsound even with a correct depth check.
/// flags: 0 = empty, 1 = EXACT, 2 = LOWER (fail-high), 3 = UPPER (fail-low).
///
/// Declared deviation, stated rather than hidden: the beta-cutoff path returns from inside the
/// move loop and so does not store. That is sound (it stores strictly less), and it is what the
/// seed's control flow allows without restructuring the loop.
pub fn ab_hash() -> Program {
    ab_hash_parts(true, true)
}

/// THE TWO HALVES OF THE HASH-REUSE RUNG, as MEASUREMENT INSTRUMENTS. Not search targets.
///
/// GRAMMAR 9 requires that "a path of single mutations from the seed exists where every step is
/// fitter". Hash reuse is the ONLY rung ever measured as fitter than the seed (0.98x its cost at
/// D=3, GRAMMAR.md:473), so it is the one place that premise can be TESTED rather than assumed --
/// and `evolve` accepts on `rate > best_rate`, STRICTLY greater, so a merely-equal step cannot be
/// taken and a worse one cannot be taken back.
///
/// A transposition table is two edits that only pay off TOGETHER: a store nothing reads is pure
/// overhead, and a probe of a table nothing wrote can never hit. If both halves measure WORSE
/// than the seed, the rung sits at the bottom of a VALLEY and no mutation operator makes it
/// reachable by a strict hill climb -- which redirects the search track away from "add operators
/// that can build Probe/Key/Field/Store" and toward the search itself.
///
/// Deliberately NOT added to `all()`. That function feeds the prior's node counts and the
/// reachability test's constructible-kind set, and quietly widening either from a probe would
/// corrupt two published numbers in order to answer a third question.
pub fn ab_probe_only() -> Program {
    ab_hash_parts(true, false)
}

/// See [`ab_probe_only`]. Stores every node's result; never reads one back.
pub fn ab_store_only() -> Program {
    ab_hash_parts(false, true)
}

fn ab_hash_parts(probe_on: bool, store_on: bool) -> Program {
    let mut p = bare_alpha_beta();
    let slot = || Node::Probe(b(Node::Key(b(v("p")))));
    let f = |id: FieldId| Node::Field(b(slot()), id);

    // if flag != 0 { if depth >= d { <bound-checked cutoffs> } }
    // Nested ifs rather than a conjunction: the grammar has no `and`, and adding one for this
    // would change the primitive count that GRAMMAR 6's prior is measured in.
    let exact = Node::If(
        b(Node::Cmp(b(f(FieldId::Flag)), b(Node::Const(1)), Rel::Eq)),
        b(Node::Ret(b(f(FieldId::Score)))),
        None,
    );
    let lower = Node::If(
        b(Node::Cmp(b(f(FieldId::Flag)), b(Node::Const(2)), Rel::Eq)),
        b(Node::If(
            b(Node::Cmp(b(f(FieldId::Score)), b(v("b")), Rel::Ge)),
            b(Node::Ret(b(f(FieldId::Score)))),
            None,
        )),
        None,
    );
    let upper = Node::If(
        b(Node::Cmp(b(f(FieldId::Flag)), b(Node::Const(3)), Rel::Eq)),
        b(Node::If(
            b(Node::Cmp(b(f(FieldId::Score)), b(v("a")), Rel::Le)),
            b(Node::Ret(b(f(FieldId::Score)))),
            None,
        )),
        None,
    );
    let probe = Node::If(
        b(Node::Cmp(b(f(FieldId::Flag)), b(Node::Const(0)), Rel::Ne)),
        b(Node::If(
            b(Node::Cmp(b(f(FieldId::Depth)), b(v("d")), Rel::Ge)),
            b(Node::seq(vec![exact, lower, upper])),
            None,
        )),
        None,
    );

    // Store score, depth, and the bound type. `a0` captures the ORIGINAL alpha, because the
    // loop mutates `a` and the bound type is defined against the window the node was entered
    // with, not the one it ended with.
    let store = Node::seq(vec![
        Node::Store(b(Node::Key(b(v("p")))), FieldId::Score, b(v("r"))),
        Node::Store(b(Node::Key(b(v("p")))), FieldId::Depth, b(v("d"))),
        Node::Store(b(Node::Key(b(v("p")))), FieldId::Flag, b(Node::Const(1))),
        Node::If(
            b(Node::Cmp(b(v("r")), b(v("a0")), Rel::Le)),
            b(Node::Store(b(Node::Key(b(v("p")))), FieldId::Flag, b(Node::Const(3)))),
            None,
        ),
        Node::If(
            b(Node::Cmp(b(v("r")), b(v("b")), Rel::Ge)),
            b(Node::Store(b(Node::Key(b(v("p")))), FieldId::Flag, b(Node::Const(2)))),
            None,
        ),
    ]);

    // THE STORE CANNOT SIMPLY BE APPENDED AFTER THE BODY. `Node::seq` desugars to nested
    // `Let("_", stmt, rest)`, and the body's tail is `ret best`, so a Ret unwinds straight past
    // anything sequenced after it. `seq[probe, body, store]` therefore never reached `store` --
    // the table was never written, every slot kept `flag = 0` / `depth = 0`, and that is the
    // root of BOTH the original bug and of the "sound but saves nothing" first repair (evals
    // identical to bare alpha-beta at every depth, cost 1.265x for probe machinery that could
    // never hit).
    //
    // So rewrite the TAIL RETURN instead: bind its value once, store, then return the binding.
    // Binding matters -- duplicating the expression would re-run `eval` and break the
    // eval-count equivalence the benchmark depends on.
    fn wrap_tail(n: Node, f: &dyn Fn(Node) -> Node) -> Node {
        match n {
            Node::Let(name, init, body) => Node::Let(name, init, b(wrap_tail(*body, f))),
            other => f(other),
        }
    }
    let body = if store_on {
        wrap_tail(p.funcs[1].body.clone(), &move |tail| match tail {
            Node::Ret(e) => Node::Let(
                "r".into(),
                e,
                b(Node::seq(vec![store.clone(), Node::Ret(b(v("r")))])),
            ),
            other => other,
        })
    } else {
        p.funcs[1].body.clone()
    };
    let probe = if probe_on { probe } else { Node::Nop };

    p.funcs[1].body = Node::Let("a0".into(), b(v("a")), b(Node::seq(vec![probe, body])));
    p
}


/// ITERATIVE DEEPENING, with and without the transposition table.
///
/// GRAMMAR 9's ladder was corrected on 2026-09-07: hash reuse measured 1.6 mates/Mcost against
/// the seed's 1.9 — a LOSS, not the 3.2x gain the broken encoding had shown. The explanation
/// offered there was that a transposition table needs REPEATED searches of the same positions
/// to pay for itself, and iterative deepening is what creates them. That explanation makes a
/// sharp, falsifiable prediction, which these two programs exist to test:
///
///   ID alone      should be a LOSS  — it re-searches every shallower depth from scratch and,
///                                     without ordering or a table, gains nothing for it.
///   ID + TT       should be a GAIN  — each iteration re-visits positions the previous one
///                                     stored, which is the traffic the table was missing.
///
/// If BOTH are losses the reordering is wrong. If ID+TT is the only gain, then the first
/// "discovery" on the ladder requires TWO simultaneous mutations rather than one, which is a
/// real problem for the plan's feasibility and belongs in the declared prior — GRAMMAR 9
/// requires each rung to be 1-3 mutations from the last.
///
/// `choose` becomes: search depth 1, then loop D-1 times searching depths 2..D, keeping the
/// last completed result. `Loop` exposes no counter, so the depth is an explicit variable.
fn with_iterative_deepening(mut p: Program) -> Program {
    let d = Node::TRead(0, vec![]);
    let inf = Node::TRead(1, vec![]);
    let search_at = |depth: Node| {
        Node::Argmax(
            b(Node::Moves(b(v("p")))),
            "m".into(),
            b(Node::Arith(
                ArithOp::Neg,
                vec![Node::Call(
                    1,
                    vec![
                        Node::Apply(b(v("p")), b(v("m"))),
                        depth,
                        Node::Arith(ArithOp::Neg, vec![inf.clone()]),
                        inf.clone(),
                    ],
                )],
            )),
        )
    };
    p.funcs[0].body = Node::seq(vec![
        Node::Let("dd".into(), b(Node::Const(1)), b(Node::Nop)),
        Node::Let("bm".into(), b(search_at(v("dd"))), b(Node::Nop)),
        Node::Loop(
            b(Node::Arith(ArithOp::Sub, vec![d, Node::Const(1)])),
            b(Node::seq(vec![
                Node::Set("dd".into(), b(Node::Arith(ArithOp::Add, vec![v("dd"), Node::Const(1)]))),
                Node::Set("bm".into(), b(search_at(v("dd")))),
            ])),
        ),
        Node::Ret(b(v("bm"))),
    ]);
    p
}

/// Bare alpha-beta driven by iterative deepening. No table.
pub fn ab_id() -> Program {
    with_iterative_deepening(bare_alpha_beta())
}

/// Alpha-beta + transposition table, driven by iterative deepening. The combination the
/// corrected ladder predicts is the real first rung.
pub fn ab_hash_id() -> Program {
    with_iterative_deepening(ab_hash())
}

/// FAITHFUL UCT: descend by the selection rule to a leaf, expand it, evaluate, and
/// backpropagate the value up the visited path. The earlier version in this file was a sketch
/// (no descent, no backprop) and therefore a LOWER BOUND on MCTS's length, which is why
/// GRAMMAR 6 recorded the skew direction as UNRESOLVED. This one has all four phases, so its
/// count is comparable with the alpha-beta seed's.
///
/// Structure, in the grammar's terms:
///   choose(p,B) = loop B times { simulate(p) }; argmax(moves(p), visits)
///   simulate(p) -> Score:
///     if terminal(p) != NONE: ret score_of(terminal(p), 0)
///     if visits(p) == 0: store(key p, 1); ret eval(p)              -- expand + evaluate
///     m   = argmax(moves(p), UCT)                                   -- select
///     v   = neg(simulate(apply(p,m)))                               -- recurse
///     store(key p, visits+1); store(sum p, sum+v); ret v            -- backpropagate
/// The declared UCT reference. **Sum selection since 2026-09-09** — see `uct_mcts_mix` for the
/// encoding this replaced and the measurements that justified the swap.
///
/// Changing this changes GRAMMAR 6's declared prior (132 -> 131 nodes), which the docs require to be
/// "a deliberate, recorded act, not a silent fix". It is recorded in GRAMMAR 6 and STATE.md, the old
/// encoding is kept and still counted, and the evidence is: the blend form tops out at 20/23 forced
/// mates at ANY weight while this reaches 23/23, and at matched cost (~1.0x bare alpha-beta) on the
/// 25-position mate set it scores 17/25 against the blend form's 11/25.
pub fn uct_mcts() -> Program { uct_program(true) }

/// The ORIGINAL blend-selection encoding, kept because GRAMMAR 6's recorded 132-node count and every
/// ladder distance measured against it must still refer to a program that exists.
///
/// It selects on `Mix(q, u, c) = (q*c + u*(16-c))/16`, so table slot 2 is BOTH the exploration scale
/// inside the sqrt and the blend weight. `u`'s coefficient is `(16 - c)`: zero at 16, negative above.
///
/// **It must be run at an explicit weight near 8, never at `uct_exploration()`**, which is now 600
/// for the sum encoding. At 600 this form is pathological — 1604 recursion-ceiling hits against the
/// sum form's 1, because a negative coefficient makes selection prefer already-visited children and
/// each descent runs deeper until it hits `MAX_CALL_DEPTH`.
pub fn uct_mcts_mix() -> Program { uct_program(false) }

/// SUM-SELECTION UCT: `argmax(q + u)` instead of `argmax(Mix(q, u, c))`.
///
/// WHY THIS EXISTS AS A SECOND ENCODING RATHER THAN AN EDIT TO `uct_mcts`. GRAMMAR 6 declares the
/// prior as a NODE COUNT, so changing the declared program changes the declared prior; the docs
/// require that to be "a deliberate, recorded act, not a silent fix". Both encodings therefore live
/// here and both are counted, so the prior can be restated with the evidence attached.
///
/// THE DEFECT IT REMOVES, measured not argued. `uct_mcts` reads table slot 2 TWICE: once to scale
/// the exploration term inside the sqrt, and once as the weight of `Mix(q, u, c)`, which
/// interp/src/lib.rs:696 computes as `(q*c + u*(16-c))/16`. Those two uses FIGHT -- raising the
/// slot enlarges `u` inside the sqrt while shrinking `u`'s blend coefficient `(16 - c)` toward
/// zero, and past zero. Swept at budget 256 over the 23 mate-in-one positions (fenced as `text`,
/// because a 4-space indented block in a doc comment is a rustdoc DOCTEST and this table is not Rust):
///
/// ```text
/// c =      1   coefficient  +15   20/23
/// c =      2                +14   18/23
/// c =      4                +12   18/23
/// c =      8                 +8   15/23
/// c =     12                 +4   14/23
/// c =     16   coefficient    0   14/23   <- u cannot matter at all: pure greed
/// c =     24                 -8   12/23
/// c =     64                -48    9/23
/// c = 360000           -359984    0/23
/// ```
///
/// Monotone, crossing the greedy baseline exactly where the coefficient reaches zero. This
/// CORRECTS the explanation on record: the documented K = 360_000 result of 0 mates was read as
/// "exploration swamps exploitation so the search never exploits", but the coefficient on `u` is
/// negative there -- the program is PENALISED for exploring. The term is inverted, not dominant.
/// Both endpoints of that bracket failed for the same reason, which is why the interior never
/// contained an answer.
///
/// Real UCT adds an exploration bonus to the value; it does not convex-blend the two. `arith`
/// already declares `add` (GRAMMAR 2.4), so no primitive is introduced -- the existing ones are
/// composed the way the algorithm actually specifies, leaving slot 2 as a pure exploration
/// constant that can be swept without simultaneously changing the blend.
pub fn uct_mcts_sum() -> Program { uct_program(true) }   // alias of the declared form

/// Shared body. `sum_selection` picks `argmax(q + u)` over `argmax(Mix(q, u, c))`.
fn uct_program(sum_selection: bool) -> Program {
    let c = Node::TRead(2, vec![]); // exploration weight, learned

    let visits = |node: Node| Node::Field(b(Node::Probe(b(Node::Key(b(node))))), FieldId::Count);
    let sum = |node: Node| Node::Field(b(Node::Probe(b(Node::Key(b(node))))), FieldId::Sum);
    let child = || Node::Apply(b(v("p")), b(v("m")));

    // Q + C * sqrt(log(N_parent) / N_child)
    //
    // NEGATED, and it was not. A node's `sum` is accumulated in ITS OWN mover's perspective --
    // backprop stores `val = Neg(Call(simulate, child))`, i.e. the child's return flipped into
    // the parent's frame. So reading a CHILD's sum from the parent gives the child's frame and
    // must be flipped again. Without the Neg the sign is inverted and the parent prefers the
    // moves that are WORST for it.
    //
    // This is why the mating move could never be found. A child that is checkmate stores
    // ScoreOf(Loss, 0) = -29936 from its own perspective (it is lost, which is exactly what the
    // parent wants). Un-negated, that reads as q = -29936: the single most REPELLENT move on the
    // board. MEASURED at 0/23 forced mates across budgets 64, 256 and 1024.
    let q = Node::Arith(
        ArithOp::Neg,
        vec![Node::Avg(b(sum(child())), b(visits(child())))],
    );
    // FIRST-PLAY URGENCY. The denominator is visits(child) + 1, not visits(child).
    //
    // The interpreter is integer arithmetic and `Div` by zero returns 0, so an UNVISITED child
    // computed u = Sqrt(Div(Log(N), 0)) = 0, and q = Avg(sum, 0) = 0 by the same guard. An
    // unexplored child therefore scored the LOWEST possible value when UCT requires it to score
    // the HIGHEST -- so the search locked onto the first child it expanded and never looked at a
    // sibling again. MEASURED before the fix: 0/23 forced mates at budgets 64, 256 AND 1024, only
    // 2 distinct moves returned across 23 different positions, and the ladder sweep independently
    // reported 0 mates out of 120.
    //
    // Same root cause family as the PN defects fixed in this file today: an unvisited slot is
    // Slot::default(), all zeros, and nothing distinguished "no data" from a real value. There it
    // made unexplored nodes look PROVEN (zero = best); here it makes them look WORTHLESS (zero =
    // worst). Both directions, same missing distinction.
    //
    // With +1: an unvisited child gets u = Sqrt(Log(N)) > 0, above any heavily-visited sibling
    // whose integer Div collapses to 0, so unexplored moves are tried before the tree deepens --
    // which is what UCT's infinite FPU does in the real algorithm.
    // SCALE INSIDE THE DIVISION, not outside it. Fixed 2026-09-08.
    //
    // The previous form was Sqrt(Div(Log(N), n+1)) and it made the exploration term NUMERICALLY
    // INERT. Measured off the interpreter's arithmetic: `Log` is ln truncated to an integer, so
    // Log(256)=5, Log(1024)=6, Log(4096)=8; `Div` is integer division, so once a child had more
    // than Log(N) visits the quotient was 0 and u was 0 PERMANENTLY for that child; and `Sqrt`
    // truncates, so with Div <= 8 the term could only ever be 0, 1 or 2.
    //
    // Two is nothing. `q` is a SCORE spanning about +/-30000 (mate is +/-29936), so an exploration
    // bonus capped at 2 breaks ties near zero and does nothing else. That is why UCT scored 12/25
    // and why mates FELL as the budget rose -- 12 at 256, then 10 at 1024 and 10 at 4096: with more
    // playouts every child passes the threshold, u is 0 everywhere, and the search degenerates to
    // greed over averages from a weak eval.
    //
    // Multiplying AFTER the Sqrt cannot fix it: {0,1,2} scaled by anything is still three values.
    // The resolution has to be created before the truncation, so the numerator is scaled first and
    // the division then has room to produce a real gradient. With table 2 at its declared value the
    // term decays smoothly with visits instead of falling off a cliff at Log(N).
    //
    // The scale is a TABLE READ, not a constant: GRAMMAR 2.7 keeps magnitudes in learned tables so
    // they stay out of the Given column, and Section 2.4 already declares that `sqrt` and `log`
    // exist ONLY so this term is expressible. `mul` is likewise already in the grammar -- nothing
    // new is added here, the existing primitives are simply composed in the order that survives
    // integer truncation.
    let u = Node::Arith(
        ArithOp::Sqrt,
        vec![Node::Arith(
            ArithOp::Div,
            vec![
                Node::Arith(
                    ArithOp::Mul,
                    vec![Node::Arith(ArithOp::Log, vec![visits(v("p"))]), c.clone()],
                ),
                Node::Arith(ArithOp::Add, vec![visits(child()), Node::Const(1)]),
            ],
        )],
    );
    let score = if sum_selection {
        // Faithful UCT: value PLUS exploration bonus. Slot 2 now scales only `u`, inside the sqrt.
        Node::Arith(ArithOp::Add, vec![q, u])
    } else {
        // The declared encoding, kept byte-identical so GRAMMAR 6's recorded node count still
        // refers to a program that exists.
        Node::Mix(b(q), b(u), b(c))
    };
    let select = Node::Argmax(b(Node::Moves(b(v("p")))), "m".into(), b(score));

    // simulate(p) -> Score
    let sim_body = Node::seq(vec![
        // TERMINAL. It must RECORD the visit, not just return a value.
        //
        // This branch used to `Ret` immediately with no Store, so a terminal child accumulated no
        // visits and no sum. `choose` selects by Argmax(moves, m, visits(child)) -- visit count --
        // so a MATING child sat permanently at visits 0 and could never be chosen however good it
        // was. Measured before the fix: 0/23 forced mates at budgets 64, 256 AND 1024, and only 4
        // distinct moves returned across 23 different positions. The ladder sweep independently
        // reported 0 mates out of 120.
        //
        // Same shape as the PN terminal bug fixed in this file today, and the same shape as the
        // hash-reuse program that never wrote its table: the branch that reaches the ANSWER is the
        // one that forgets to record it.
        Node::If(
            b(Node::Cmp(
                b(Node::Terminal(b(v("p")))),
                b(Node::OutcomeLit(OutcomeLit::None)),
                Rel::Ne,
            )),
            b(Node::seq(vec![
                Node::Store(
                    b(Node::Key(b(v("p")))),
                    FieldId::Count,
                    b(Node::Arith(ArithOp::Add, vec![visits(v("p")), Node::Const(1)])),
                ),
                Node::Store(
                    b(Node::Key(b(v("p")))),
                    FieldId::Sum,
                    b(Node::Arith(
                        ArithOp::Add,
                        vec![
                            sum(v("p")),
                            Node::ScoreOf(b(Node::Terminal(b(v("p")))), b(Node::Const(0))),
                        ],
                    )),
                ),
                Node::Ret(b(Node::ScoreOf(
                    b(Node::Terminal(b(v("p")))),
                    b(Node::Const(0)),
                ))),
            ])),
            None,
        ),
        // unvisited leaf: expand, evaluate, return
        Node::If(
            b(Node::Cmp(b(visits(v("p"))), b(Node::Const(0)), Rel::Le)),
            b(Node::seq(vec![
                Node::Store(b(Node::Key(b(v("p")))), FieldId::Count, b(Node::Const(1))),
                Node::Ret(b(Node::Eval(b(v("p"))))),
            ])),
            None,
        ),
        Node::Let("m".into(), b(select), b(Node::Nop)),
        Node::Let(
            "val".into(),
            b(Node::Arith(
                ArithOp::Neg,
                vec![Node::Call(1, vec![child()])],
            )),
            b(Node::Nop),
        ),
        // backpropagate: bump this node's visit count and running sum
        Node::Store(
            b(Node::Key(b(v("p")))),
            FieldId::Count,
            b(Node::Arith(ArithOp::Add, vec![visits(v("p")), Node::Const(1)])),
        ),
        Node::Store(
            b(Node::Key(b(v("p")))),
            FieldId::Sum,
            b(Node::Arith(ArithOp::Add, vec![sum(v("p")), v("val")])),
        ),
        Node::Ret(b(v("val"))),
    ]);

    let choose = Node::seq(vec![
        Node::Loop(b(Node::Budget), b(Node::Call(1, vec![v("p")]))),
        Node::Ret(b(Node::Argmax(
            b(Node::Moves(b(v("p")))),
            "m".into(),
            b(visits(child())),
        ))),
    ]);

    Program {
        funcs: vec![
            Func {
                name: "choose".into(),
                params: vec![("p".into(), Ty::Pos), ("B".into(), Ty::Int)],
                ret: Ty::Move,
                body: choose,
            },
            Func {
                name: "simulate".into(),
                params: vec![("p".into(), Ty::Pos)],
                ret: Ty::Score,
                body: sim_body,
            },
        ],
        lineage: Lineage::Main,
    }
}

/// FAITHFUL PROOF-NUMBER SEARCH. The earlier version here was a sketch (a visit-count loop
/// with no proof/disproof numbers and no back-up), so its node count was a LOWER BOUND and
/// GRAMMAR 6 could not state a PN distance. This one has the actual algorithm:
///
///   proof(n)     = 0        if n is a proven WIN
///                = infinity if n is a proven LOSS
///                = min over children of proof   (OR node: one winning child suffices)
///   disproof(n)  = sum over children of disproof (OR node: all must fail)
///
/// Slot fields carry them: `score` = proof number, `count` = disproof number. PN is
/// terminal-driven and calls `eval` nowhere, which is exactly why FITNESS 3 denominates
/// mates-per-COST rather than mates-per-evaluation -- an eval-free prover would otherwise
/// divide by zero and dominate the metric.
pub fn proof_number() -> Program {
    let inf = Node::TRead(1, vec![]);
    let child = || Node::Apply(b(v("p")), b(v("m")));

    fn slot(n: Node, f: FieldId) -> Node {
        Node::Field(b(Node::Probe(b(Node::Key(b(n))))), f)
    }
    // An unvisited slot is Slot::default(), all zeros, and proof==0 means PROVEN WIN. So a bare
    // read made every unexplored child look already proven. Flag is the validity marker: with
    // flag in {0,1}, effective = stored + (1 - flag), so unvisited reads 1 and visited reads the
    // stored value, with no conditional.
    fn with_init(n: Node, f: FieldId) -> Node {
        Node::Arith(
            ArithOp::Add,
            vec![
                slot(n.clone(), f),
                Node::Arith(ArithOp::Sub, vec![Node::Const(1), slot(n, FieldId::Flag)]),
            ],
        )
    }
    let proof = |n: Node| with_init(n, FieldId::Score);
    let disproof = |n: Node| with_init(n, FieldId::Count);
    let mark_seen = || Node::Store(b(Node::Key(b(v("p")))), FieldId::Flag, b(Node::Const(1)));

    let body = Node::seq(vec![
        // TERMINAL. The mover is checkmated or the game is drawn, so the mover cannot WIN:
        // pn = infinity, dn = 0. The previous encoding stored ScoreOf(Terminal(p), 0) here, which
        // is -29936 -- a mate SCORE where a proof NUMBER belongs, and a negative proof number is
        // always the minimum, so it also corrupted the selection.
        Node::If(
            b(Node::Cmp(
                b(Node::Terminal(b(v("p")))),
                b(Node::OutcomeLit(OutcomeLit::None)),
                Rel::Ne,
            )),
            b(Node::seq(vec![
                Node::Store(b(Node::Key(b(v("p")))), FieldId::Score, b(inf.clone())),
                Node::Store(b(Node::Key(b(v("p")))), FieldId::Count, b(Node::Const(0))),
                mark_seen(),
                Node::Ret(b(inf.clone())),
            ])),
            None,
        ),
        // EXPANSION. Real PN descends to the most-proving LEAF, expands it one ply and backs up;
        // it never recurses to a terminal. Without this the recursion ran to the ceiling on EVERY
        // iteration (measured 3840 hits over 60 positions at budget 64 = 60 x 64), and at the
        // ceiling `Call` returns 0 -- which for proof numbers means PROVEN WIN.
        //
        // dn is set to the CHILD COUNT, not 1. An expanded node's dn = sum over children of pn,
        // and every child is unvisited at pn=1, so dn = number of children. That is what makes an
        // expanded node unattractive and forces the search to fan out sideways; setting it to 1
        // leaves every sibling tied forever and the descent never leaves the first child.
        Node::If(
            b(Node::Cmp(b(slot(v("p"), FieldId::Flag)), b(Node::Const(0)), Rel::Eq)),
            b(Node::Let(
                "n".into(),
                b(Node::Const(0)),
                b(Node::seq(vec![
                    Node::Foreach(
                        b(Node::Moves(b(v("p")))),
                        "m".into(),
                        b(Node::Set("n".into(), b(Node::Arith(ArithOp::Add, vec![v("n"), Node::Const(1)])))),
                    ),
                    Node::Store(b(Node::Key(b(v("p")))), FieldId::Score, b(Node::Const(1))),
                    Node::Store(b(Node::Key(b(v("p")))), FieldId::Count, b(v("n"))),
                    mark_seen(),
                    Node::Ret(b(Node::Const(1))),
                ])),
            )),
            None,
        ),
        // SELECT the most-proving child. NEGAMAX form: numbers are from each node's own mover's
        // perspective, so the child's DISPROOF is what this node's proof is built from.
        //     pn(n) = min over children of dn(child)
        //     dn(n) = sum over children of pn(child)
        // The previous encoding took min/sum of the child's PROOF for both, i.e. it treated every
        // node as an OR node with no AND/OR alternation. Without alternation an expanded child's
        // proof stays 1 forever, the descent never moves off child_0, and the mating child is
        // never visited -- exactly the "returns the first legal move 23/23" signature measured.
        Node::Let(
            "m".into(),
            b(Node::Argmax(
                b(Node::Moves(b(v("p")))),
                "m".into(),
                b(Node::Arith(ArithOp::Neg, vec![disproof(child())])),
            )),
            b(Node::Nop),
        ),
        Node::Let("sub".into(), b(Node::Call(1, vec![child()])), b(Node::Nop)),
        // BACK UP over ALL children, not incrementally against this node's stale value. The
        // previous version accumulated `disproof(p) + disproof(child)` on every visit, which
        // double-counts a child each time it is revisited.
        Node::Let(
            "pmin".into(),
            b(inf.clone()),
            b(Node::Let(
                "dsum".into(),
                b(Node::Const(0)),
                b(Node::seq(vec![
                    Node::Foreach(
                        b(Node::Moves(b(v("p")))),
                        "m".into(),
                        b(Node::seq(vec![
                            Node::Set("pmin".into(), b(Node::Min(b(v("pmin")), b(disproof(child()))))),
                            Node::Set("dsum".into(), b(Node::Arith(ArithOp::Add, vec![v("dsum"), proof(child())]))),
                        ])),
                    ),
                    Node::Store(b(Node::Key(b(v("p")))), FieldId::Score, b(v("pmin"))),
                    Node::Store(b(Node::Key(b(v("p")))), FieldId::Count, b(v("dsum"))),
                    mark_seen(),
                    Node::Ret(b(v("pmin"))),
                ])),
            )),
        ),
    ]);

    // choose: run the prover to the budget, then play the child that is easiest to DISPROVE for
    // the opponent -- smallest dn(child), the same quantity the descent selects on.
    let choose = Node::seq(vec![
        Node::Loop(b(Node::Budget), b(Node::Call(1, vec![v("p")]))),
        Node::Ret(b(Node::Argmax(
            b(Node::Moves(b(v("p")))),
            "m".into(),
            b(Node::Arith(ArithOp::Sub, vec![inf.clone(), disproof(child())])),
        ))),
    ]);

    Program {
        funcs: vec![
            Func {
                name: "choose".into(),
                params: vec![("p".into(), Ty::Pos), ("B".into(), Ty::Int)],
                ret: Ty::Move,
                body: choose,
            },
            Func {
                name: "prove".into(),
                params: vec![("p".into(), Ty::Pos)],
                ret: Ty::Score,
                body,
            },
        ],
        lineage: Lineage::Main,
    }
}


pub fn all() -> Vec<(&'static str, Program)> {
    vec![
        ("depth-one (purity seed)", depth_one()),
        ("bare alpha-beta (main seed)", bare_alpha_beta()),
        ("alpha-beta + hash reuse", ab_hash()),
        ("alpha-beta + iterative deepening", ab_id()),
        ("alpha-beta + hash + ID", ab_hash_id()),
        ("UCT-style MCTS", uct_mcts()),
        ("UCT-style MCTS (blend selection, historical)", uct_mcts_mix()),
        ("capture extension (rung 6)", capture_extension()),
        ("extend-by-uncertainty (yardstick a)", uncertainty_extension()),
        ("mix-backup (yardstick b)", mix_backup_program()),
        ("table reduction (rung 7)", table_reduction()),
        ("proof-number search", proof_number()),
    ]
}
