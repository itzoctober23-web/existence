//! The only place a search program runs.
//!
//! STAGE 1 IS A TREE-WALKER, DELIBERATELY. CRATE.md 4 specifies a register bytecode, and that
//! is still the target. A tree-walking interpreter over the AST is strictly slower than
//! bytecode, so the NPS ratio it produces is a LOWER BOUND on what the design can achieve. It
//! is built first because it is small enough to be obviously correct, and because a lower
//! bound is already decision-relevant: if the tree-walker is close to the 50% acceptance line
//! (GRAMMAR 8), bytecode clears it comfortably; if it is orders of magnitude below, the
//! problem is the evaluation model rather than the encoding, and that is worth knowing before
//! writing a compiler.
//!
//! Cost accounting is per-primitive and IS the budget (GRAMMAR 8): `mates-per-cost` and every
//! fixed-budget check in FITNESS are denominated in these units, which is what keeps them
//! paradigm-neutral and ungameable by a program that simply avoids calling `eval`.

use std::collections::HashMap;

use board::types::{MOVE_NONE, Move, Outcome};
use board::Position;
use grammar::ast::*;
use nnue::Net;

#[derive(Clone, Debug)]
pub enum Value {
    Pos(Position),
    Mv(Move),
    List(Vec<Move>),
    Num(i64),
    Out(Outcome),
    Bool(bool),
    Key(u64),
    Unit,
}

impl Value {
    fn num(&self) -> i64 {
        match self {
            Value::Num(n) => *n,
            Value::Bool(b) => *b as i64,
            _ => 0,
        }
    }
    fn pos(&self) -> &Position {
        match self {
            Value::Pos(p) => p,
            _ => panic!("type error: expected Pos"),
        }
    }
    fn mv(&self) -> Move {
        match self {
            Value::Mv(m) => *m,
            _ => MOVE_NONE,
        }
    }
}

/// Early exit carrying a `ret` value, so the window cutoff costs no extra machinery.
enum Flow {
    Normal(Value),
    Ret(Value),
}

pub struct Interp<'a> {
    pub net: &'a Net,
    pub cost: u64,
    /// Learned integer tables. Index 0 = D (depth), 1 = INF, 2.. = whatever a program reads.
    pub tables: Vec<i64>,
    hash: HashMap<u64, i64>,
    scratch: Vec<f32>,
    budget: i64,
}

type Env = Vec<(String, Value)>;

impl<'a> Interp<'a> {
    pub fn new(net: &'a Net, tables: Vec<i64>) -> Self {
        Interp {
            net,
            cost: 0,
            tables,
            hash: HashMap::new(),
            scratch: Vec::new(),
            budget: i64::MAX,
        }
    }

    pub fn run(&mut self, prog: &Program, pos: &Position, budget: i64) -> Move {
        self.cost = 0;
        self.budget = budget;
        self.hash.clear();
        let f = prog.entry();
        let mut env: Env = vec![
            (f.params[0].0.clone(), Value::Pos(pos.clone())),
            (f.params[1].0.clone(), Value::Num(budget)),
        ];
        match self.exec(&f.body, prog, &mut env) {
            Flow::Ret(v) | Flow::Normal(v) => v.mv(),
        }
    }

    fn lookup(env: &Env, name: &str) -> Value {
        env.iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.clone())
            .unwrap_or(Value::Unit)
    }

    fn assign(env: &mut Env, name: &str, v: Value) {
        if let Some(slot) = env.iter_mut().rev().find(|(n, _)| n == name) {
            slot.1 = v;
        } else {
            env.push((name.to_string(), v));
        }
    }

    fn exec(&mut self, n: &Node, prog: &Program, env: &mut Env) -> Flow {
        self.cost += 1;
        macro_rules! val {
            ($e:expr) => {
                match self.exec($e, prog, env) {
                    Flow::Ret(v) => return Flow::Ret(v),
                    Flow::Normal(v) => v,
                }
            };
        }
        let v = match n {
            Node::Const(c) => Value::Num(*c as i64),
            Node::Var(name) => Self::lookup(env, name),
            Node::Budget => Value::Num(self.budget),
            Node::Nop => Value::Unit,
            Node::OutcomeLit(o) => Value::Out(match o {
                OutcomeLit::None => Outcome::Ongoing,
                OutcomeLit::Loss => Outcome::Loss,
                _ => Outcome::Draw,
            }),

            Node::Moves(p) => {
                let p = val!(p);
                let l = p.pos().legal_moves();
                Value::List(l.as_slice().to_vec())
            }
            Node::Apply(p, m) => {
                let pv = val!(p);
                let mv = val!(m).mv();
                let mut np = pv.pos().clone();
                np.make_move(mv);
                Value::Pos(np)
            }
            Node::Terminal(p) => {
                let p = val!(p);
                Value::Out(p.pos().outcome())
            }
            Node::Key(p) => {
                let p = val!(p);
                // Cheap positional key; the real Zobrist lands with the TT work.
                let f = p.pos().to_fen();
                let mut h = 1469598103934665603u64;
                for b in f.bytes() {
                    h ^= b as u64;
                    h = h.wrapping_mul(1099511628211);
                }
                Value::Key(h)
            }
            Node::Eval(p) => {
                let p = val!(p);
                Value::Num(self.net.eval(p.pos(), &mut self.scratch) as i64)
            }

            Node::Arith(op, args) => {
                let a = val!(&args[0]).num();
                match op {
                    ArithOp::Neg => Value::Num(-a),
                    ArithOp::Sqrt => Value::Num((a.max(0) as f64).sqrt() as i64),
                    ArithOp::Log => Value::Num((a.max(1) as f64).ln() as i64),
                    _ => {
                        let b = val!(&args[1]).num();
                        Value::Num(match op {
                            ArithOp::Add => a + b,
                            ArithOp::Sub => a - b,
                            ArithOp::Mul => a.saturating_mul(b),
                            ArithOp::Div => {
                                if b == 0 {
                                    0
                                } else {
                                    a / b
                                }
                            }
                            _ => 0,
                        })
                    }
                }
            }
            Node::Cmp(a, b, rel) => {
                let (av, bv) = (val!(a), val!(b));
                let r = match (&av, &bv) {
                    (Value::Out(x), Value::Out(y)) => match rel {
                        Rel::Eq => x == y,
                        _ => x != y,
                    },
                    _ => {
                        let (x, y) = (av.num(), bv.num());
                        match rel {
                            Rel::Lt => x < y,
                            Rel::Le => x <= y,
                            Rel::Eq => x == y,
                            Rel::Ge => x >= y,
                            Rel::Gt => x > y,
                            Rel::Ne => x != y,
                        }
                    }
                };
                Value::Bool(r)
            }
            Node::Max(a, b) => Value::Num(val!(a).num().max(val!(b).num())),
            Node::Min(a, b) => Value::Num(val!(a).num().min(val!(b).num())),
            Node::Avg(a, b) => {
                let (s, c) = (val!(a).num(), val!(b).num());
                Value::Num(if c == 0 { 0 } else { s / c })
            }
            Node::Mix(a, b, w) => {
                let (x, y, k) = (val!(a).num(), val!(b).num(), val!(w).num());
                Value::Num((x * k + y * (16 - k)) / 16)
            }

            Node::TRead(i, _) => Value::Num(*self.tables.get(*i).unwrap_or(&0)),
            Node::ScoreOf(o, d) => {
                let ov = val!(o);
                let dv = val!(d).num();
                Value::Num(match ov {
                    Value::Out(Outcome::Loss) => -30_000 + (64 - dv),
                    Value::Out(Outcome::Draw) => 0,
                    _ => 0,
                })
            }

            Node::Probe(k) => {
                let key = match val!(k) {
                    Value::Key(x) => x,
                    _ => 0,
                };
                Value::Num(*self.hash.get(&key).unwrap_or(&i64::MIN))
            }
            Node::Store(k, v) => {
                let key = match val!(k) {
                    Value::Key(x) => x,
                    _ => 0,
                };
                let val = val!(v).num();
                self.hash.insert(key, val);
                Value::Unit
            }
            Node::Field(s, _) => val!(s),

            Node::Let(name, init, body) => {
                let v = val!(init);
                env.push((name.clone(), v));
                let r = self.exec(body, prog, env);
                env.pop();
                return r;
            }
            Node::Set(name, e) => {
                let v = val!(e);
                Self::assign(env, name, v);
                Value::Unit
            }
            Node::If(c, t, e) => {
                let cond = val!(c).num() != 0;
                if cond {
                    return self.exec(t, prog, env);
                } else if let Some(e) = e {
                    return self.exec(e, prog, env);
                }
                Value::Unit
            }
            Node::Ret(e) => return Flow::Ret(val!(e)),
            Node::Foreach(list, var, body) => {
                let items = match val!(list) {
                    Value::List(l) => l,
                    _ => vec![],
                };
                for m in items {
                    env.push((var.clone(), Value::Mv(m)));
                    let r = self.exec(body, prog, env);
                    env.pop();
                    if let Flow::Ret(v) = r {
                        return Flow::Ret(v);
                    }
                }
                Value::Unit
            }
            Node::Loop(count, body) => {
                let k = val!(count).num().clamp(0, 1 << 20);
                for _ in 0..k {
                    if let Flow::Ret(v) = self.exec(body, prog, env) {
                        return Flow::Ret(v);
                    }
                }
                Value::Unit
            }
            Node::Argmax(list, var, key) | Node::Sort(list, var, key) | Node::Sample(list, var, key) => {
                let items = match val!(list) {
                    Value::List(l) => l,
                    _ => vec![],
                };
                let mut best = MOVE_NONE;
                let mut best_k = i64::MIN;
                for m in items {
                    env.push((var.clone(), Value::Mv(m)));
                    let r = self.exec(key, prog, env);
                    env.pop();
                    let k = match r {
                        Flow::Ret(v) => return Flow::Ret(v),
                        Flow::Normal(v) => v.num(),
                    };
                    if k > best_k {
                        best_k = k;
                        best = m;
                    }
                }
                Value::Mv(best)
            }
            Node::Call(idx, args) => {
                let f = &prog.funcs[*idx];
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(val!(a));
                }
                let mut inner: Env = f
                    .params
                    .iter()
                    .zip(vals)
                    .map(|((n, _), v)| (n.clone(), v))
                    .collect();
                match self.exec(&f.body, prog, &mut inner) {
                    Flow::Ret(v) | Flow::Normal(v) => v,
                }
            }
            Node::Pred(..) => Value::Bool(false),
        };
        Flow::Normal(v)
    }
}
