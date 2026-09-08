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
pub fn bare_alpha_beta() -> Program { ab_program(false, false) }

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
pub fn capture_extension() -> Program { ab_program(true, false) }

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
pub fn table_reduction() -> Program { ab_program(false, true) }

fn ab_program(cap_ext: bool, reduce: bool) -> Program {
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
    let nd_setup: Vec<Node> = if cap_ext {
        vec![
            Node::Let("nd".into(),
                b(Node::Arith(ArithOp::Sub, vec![v("d"), Node::Const(1)])), b(Node::Nop)),
            Node::If(
                b(Node::Pred(b(v("m")), b(v("p")), PredId::IsCapture)),
                b(Node::Set("nd".into(), b(v("d")))),
                None,
            ),
        ]
    } else { vec![] };
    let mut loop_stmts = nd_setup;
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
                        if cap_ext || reduce { v("nd") } else {
                            Node::Arith(ArithOp::Sub, vec![v("d"), Node::Const(1)])
                        },
                        Node::Arith(ArithOp::Neg, vec![v("b")]),
                        Node::Arith(ArithOp::Neg, vec![v("a")]),
                    ],
                )],
            )),
            b(Node::Nop),
        ),
        Node::Set("best".into(), b(Node::Max(b(v("best")), b(v("vv"))))),
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
    let body = wrap_tail(p.funcs[1].body.clone(), &move |tail| match tail {
        Node::Ret(e) => Node::Let(
            "r".into(),
            e,
            b(Node::seq(vec![store.clone(), Node::Ret(b(v("r")))])),
        ),
        other => other,
    });

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
pub fn uct_mcts() -> Program {
    let c = Node::TRead(2, vec![]); // exploration weight, learned

    let visits = |node: Node| Node::Field(b(Node::Probe(b(Node::Key(b(node))))), FieldId::Count);
    let sum = |node: Node| Node::Field(b(Node::Probe(b(Node::Key(b(node))))), FieldId::Sum);
    let child = || Node::Apply(b(v("p")), b(v("m")));

    // Q + C * sqrt(log(N_parent) / N_child)
    let q = Node::Avg(b(sum(child())), b(visits(child())));
    let u = Node::Arith(
        ArithOp::Sqrt,
        vec![Node::Arith(
            ArithOp::Div,
            vec![
                Node::Arith(ArithOp::Log, vec![visits(v("p"))]),
                visits(child()),
            ],
        )],
    );
    let select = Node::Argmax(
        b(Node::Moves(b(v("p")))),
        "m".into(),
        b(Node::Mix(b(q), b(u), b(c))),
    );

    // simulate(p) -> Score
    let sim_body = Node::seq(vec![
        Node::If(
            b(Node::Cmp(
                b(Node::Terminal(b(v("p")))),
                b(Node::OutcomeLit(OutcomeLit::None)),
                Rel::Ne,
            )),
            b(Node::Ret(b(Node::ScoreOf(
                b(Node::Terminal(b(v("p")))),
                b(Node::Const(0)),
            )))),
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
        ("capture extension (rung 6)", capture_extension()),
        ("table reduction (rung 7)", table_reduction()),
        ("proof-number search", proof_number()),
    ]
}
