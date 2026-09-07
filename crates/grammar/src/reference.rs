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
pub fn bare_alpha_beta() -> Program {
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
    let loop_body = Node::seq(vec![
        Node::Let(
            "vv".into(),
            b(Node::Arith(
                ArithOp::Neg,
                vec![Node::Call(
                    1,
                    vec![
                        Node::Apply(b(v("p")), b(v("m"))),
                        Node::Arith(ArithOp::Sub, vec![v("d"), Node::Const(1)]),
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
    let ab_body = Node::seq(vec![
        term_guard,
        depth_guard,
        Node::Let(
            "best".into(),
            b(Node::Arith(ArithOp::Neg, vec![inf])),
            b(Node::Nop),
        ),
        Node::Foreach(b(Node::Moves(b(v("p")))), "m".into(), b(loop_body)),
        Node::Ret(b(v("best"))),
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

/// Alpha-beta plus hash reuse: probe before searching, store after. The first milestone the
/// search track is expected to discover (GRAMMAR 9 step 4).
pub fn ab_hash() -> Program {
    let mut p = bare_alpha_beta();
    let probe = Node::If(
        b(Node::Cmp(
            b(Node::Field(
                b(Node::Probe(b(Node::Key(b(v("p")))))),
                FieldId::Depth,
            )),
            b(v("d")),
            Rel::Ge,
        )),
        b(Node::Ret(b(Node::Field(
            b(Node::Probe(b(Node::Key(b(v("p")))))),
            FieldId::Score,
        )))),
        None,
    );
    let store = Node::Store(b(Node::Key(b(v("p")))), b(v("best")));
    if let Node::Let(_, _, _) = &p.funcs[1].body {}
    p.funcs[1].body = Node::seq(vec![probe, p.funcs[1].body.clone(), store]);
    p
}

/// UCT-style MCTS with the net's value and no policy head. Present so that "the engine stayed
/// in the alpha-beta basin" is a finding rather than an artefact of MCTS being inexpressible;
/// `sqrt`, `log`, `avg`, `sample` and the count/sum slot fields exist for exactly this.
pub fn uct_mcts() -> Program {
    let c = Node::TRead(2, vec![]); // exploration constant, learned
    // select(p) = argmax(moves(p), m -> mix(avg(sum,count), sqrt(log(N)/count), C))
    let child_key = Node::Key(b(Node::Apply(b(v("p")), b(v("m")))));
    let q = Node::Avg(
        b(Node::Field(b(Node::Probe(b(child_key.clone()))), FieldId::Sum)),
        b(Node::Field(b(Node::Probe(b(child_key.clone()))), FieldId::Count)),
    );
    let u = Node::Arith(
        ArithOp::Sqrt,
        vec![Node::Arith(
            ArithOp::Div,
            vec![
                Node::Arith(
                    ArithOp::Log,
                    vec![Node::Field(b(Node::Probe(b(Node::Key(b(v("p")))))), FieldId::Count)],
                ),
                Node::Field(b(Node::Probe(b(child_key))), FieldId::Count),
            ],
        )],
    );
    let select = Node::Argmax(
        b(Node::Moves(b(v("p")))),
        "m".into(),
        b(Node::Mix(b(q), b(u), b(c))),
    );
    // choose runs `budget` simulations then picks the most-visited child.
    let sim = Node::seq(vec![
        Node::Let("m".into(), b(select.clone()), b(Node::Nop)),
        Node::Let(
            "leaf".into(),
            b(Node::Apply(b(v("p")), b(v("m")))),
            b(Node::Nop),
        ),
        Node::Store(b(Node::Key(b(v("leaf")))), b(Node::Eval(b(v("leaf"))))),
    ]);
    let body = Node::seq(vec![
        Node::Loop(b(Node::Budget), b(sim)),
        Node::Ret(b(Node::Argmax(
            b(Node::Moves(b(v("p")))),
            "m".into(),
            b(Node::Field(
                b(Node::Probe(b(Node::Key(b(Node::Apply(b(v("p")), b(v("m")))))))),
                FieldId::Count,
            )),
        ))),
    ]);
    Program {
        funcs: vec![Func {
            name: "choose".into(),
            params: vec![("p".into(), Ty::Pos), ("B".into(), Ty::Int)],
            ret: Ty::Move,
            body,
        }],
        lineage: Lineage::Main,
    }
}

/// Proof-number-ish search: terminal-driven, no eval on the proving path. Included because
/// GRAMMAR 6 lists it and because FITNESS 3 uses mates-per-COST partly to keep an eval-free
/// prover from dominating a mates-per-eval metric.
pub fn proof_number() -> Program {
    let body = Node::seq(vec![
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
        Node::Loop(
            b(Node::Budget),
            b(Node::Let(
                "m".into(),
                b(Node::Argmax(
                    b(Node::Moves(b(v("p")))),
                    "m".into(),
                    b(Node::Arith(
                        ArithOp::Neg,
                        vec![Node::Field(
                            b(Node::Probe(b(Node::Key(b(Node::Apply(b(v("p")), b(v("m")))))))),
                            FieldId::Count,
                        )],
                    )),
                )),
                b(Node::Store(
                    b(Node::Key(b(Node::Apply(b(v("p")), b(v("m")))))),
                    b(Node::Const(1)),
                )),
            )),
        ),
        Node::Ret(b(Node::Argmax(
            b(Node::Moves(b(v("p")))),
            "m".into(),
            b(Node::Field(
                b(Node::Probe(b(Node::Key(b(Node::Apply(b(v("p")), b(v("m")))))))),
                FieldId::Count,
            )),
        ))),
    ]);
    Program {
        funcs: vec![Func {
            name: "choose".into(),
            params: vec![("p".into(), Ty::Pos), ("B".into(), Ty::Int)],
            ret: Ty::Move,
            body,
        }],
        lineage: Lineage::Main,
    }
}

pub fn all() -> Vec<(&'static str, Program)> {
    vec![
        ("depth-one (purity seed)", depth_one()),
        ("bare alpha-beta (main seed)", bare_alpha_beta()),
        ("alpha-beta + hash reuse", ab_hash()),
        ("UCT-style MCTS", uct_mcts()),
        ("proof-number search", proof_number()),
    ]
}
