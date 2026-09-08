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

use std::rc::Rc;

use board::types::{MOVE_NONE, Move, Outcome};
use board::Position;
use grammar::ast::*;
use nnue::Net;

#[derive(Clone, Debug)]
pub enum Value {
    /// Rc, not a bare Position: a variable reference clones its Value, and the seed program
    /// names `p` four times per node. A bare Position made every mention a deep copy.
    Pos(Rc<Position>),
    Mv(Move),
    List(Rc<Vec<Move>>),
    Num(i64),
    Out(Outcome),
    Bool(bool),
    Key(u64),
    Slot(Slot),
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
            Value::Pos(p) => p.as_ref(),
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

/// A hash record. GRAMMAR 1 lists the fields; a program uses the ones it reads and writes.
/// Storing a single scalar per key (the first cut) made MCTS's `count` and `sum` collide, so
/// a faithful UCT would have computed garbage while still measuring the right SIZE.
#[derive(Copy, Clone, Debug, Default)]
pub struct Slot {
    pub score: i64,
    pub depth: i64,
    pub flag: i64,
    pub count: i64,
    pub sum: i64,
    pub mv: u32,
}

/// A BOUNDED transposition table: fixed slot count, direct-indexed, always-replace.
///
/// The first cut was a `HashMap<u64, Slot>`, and that is a SKETCH of a transposition table in
/// exactly the sense GRAMMAR 6 warns about. It never evicts, so:
///   - it grows without limit during a search (millions of entries at depth 5), which is the
///     unbounded-resource failure GRAMMAR 8's ceilings exist to prevent; and
///   - it hands an evolved program an IDEALISED INFINITE TABLE that no real engine has. Hash
///     reuse is item 1 on the expected discovery order, so the very first thing evolution is
///     predicted to find would have had its gain measured against a table that cannot exist.
/// GRAMMAR 6's own lesson: a sketch is a lower bound, and a lower bound can invert the sign of
/// the claim being made.
///
/// Replacement policy is ALWAYS-REPLACE, deliberately the most naive one available. MASTER_PLAN
/// lists "hash replacement" among the table-driven decisions SPSA tunes, so the default here
/// has to be the policy with the least opinion in it, not a hand-tuned depth-preferred scheme
/// that would be an undeclared search heuristic smuggled into the Given column.
///
/// Slots are validated by full key, and a mismatch counts as a collision rather than returning
/// another position's data — FITNESS 10 lists "exploit hash collision / stale slot" as a
/// degenerate solution, and it is only checkable if collisions are counted.
pub const HASH_SLOTS: usize = 1 << 16;

struct Tt {
    keys: Vec<u64>,
    /// Generation stamp, so clearing the table between searches is O(1) instead of 64k writes,
    /// and so a slot holding the legitimate key 0 is distinguishable from an empty one.
    stamp: Vec<u32>,
    slots: Vec<Slot>,
    cur: u32,
    pub collisions: u64,
}

impl Tt {
    fn new() -> Self {
        Tt {
            keys: vec![0; HASH_SLOTS],
            stamp: vec![0; HASH_SLOTS],
            slots: vec![Slot::default(); HASH_SLOTS],
            cur: 1,
            collisions: 0,
        }
    }
    #[inline]
    fn idx(key: u64) -> usize {
        // Multiplicative index: the low bits of a Zobrist key are not better mixed than the
        // high ones, and `key % n` on a power of two would use the low bits only.
        (((key as u128 * HASH_SLOTS as u128) >> 64) as usize).min(HASH_SLOTS - 1)
    }
    fn clear(&mut self) {
        self.cur = self.cur.wrapping_add(1);
        if self.cur == 0 {
            // Wrapped: stale stamps could alias, so genuinely reset once every 4 billion runs.
            self.stamp.iter_mut().for_each(|g| *g = 0);
            self.cur = 1;
        }
        self.collisions = 0;
    }
    fn probe(&mut self, key: u64) -> Slot {
        let i = Self::idx(key);
        if self.stamp[i] == self.cur && self.keys[i] == key {
            self.slots[i]
        } else {
            if self.stamp[i] == self.cur { self.collisions += 1; }
            Slot::default()
        }
    }
    fn entry(&mut self, key: u64) -> &mut Slot {
        let i = Self::idx(key);
        if self.stamp[i] != self.cur || self.keys[i] != key {
            self.keys[i] = key;
            self.stamp[i] = self.cur;
            self.slots[i] = Slot::default();
        }
        &mut self.slots[i]
    }
}


/// PER-PRIMITIVE COST, from `configs/cost.toml`, MEASURED by examples/cost_calibrate.rs.
///
/// GRAMMAR 8 specifies "a declared per-primitive cost (cycles estimate) table. The interpreter
/// accumulates it at runtime; that running sum IS the budget unit." CRATE 4 names the file.
/// Neither existed: this charged a flat 1 per node, so a full NNUE forward pass cost exactly
/// what `const 3` cost.
///
/// THE FLAT MODEL WAS NOT NEUTRAL. It is a thumb on the scale against any program that spends
/// cheap work to avoid expensive work -- which is exactly a transposition table's trade. On
/// this machine at width 32:
///
/// ```text
/// arith/cmp/const/var     1
/// key (zobrist)          97
/// terminal              703
/// apply                 788
/// eval                 1365      <- 293 ns
/// moves                2232
/// ```
///
/// So skipping ONE eval is worth ~1365 units against a probe costing ~110. Priced flat, that
/// same trade reads as a loss, which is precisely the result GRAMMAR 9's ladder reported for
/// hash reuse. Those numbers must be re-derived under this table.
///
/// Values are relative to one integer op, because the budget only needs RATIOS and ratios
/// survive a change of CPU far better than absolute nanoseconds do.
fn cost_of(n: &Node) -> u64 {
    match n {
        Node::Eval(_) => 1365,
        Node::Moves(_) => 2232,
        Node::Apply(..) => 788,
        Node::Terminal(_) => 703,
        Node::Key(_) => 97,
        Node::Pred(..) => 40,
        Node::Probe(_) => 12,
        Node::Store(..) => 12,
        Node::Argmax(..) | Node::Sort(..) | Node::Sample(..) => 4,
        Node::Field(..) => 2,
        Node::ScoreOf(..) => 2,
        Node::TRead(..) => 2,
        // Control flow and arithmetic: a few ops each. The CHILDREN they evaluate are charged
        // on their own visits, so this is only the node's own overhead.
        _ => 2,
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
    /// Leaf evaluations. The benchmark compares this against the reference's count to prove
    /// both arms searched the SAME tree before believing any speed ratio.
    pub evals: u64,
    /// Learned integer tables. Index 0 = D (depth), 1 = INF, 2.. = whatever a program reads.
    pub tables: Vec<i64>,
    hash: Tt,
    scratch: Vec<f32>,
    budget: i64,
    /// Current call depth, against the hard ceiling of GRAMMAR 8. Without it an evolved
    /// program that recurses unboundedly takes down the whole process -- FITNESS 10 lists
    /// "infinite loop / stack blow" as a degenerate solution the ceilings are supposed to
    /// catch, and a faithful proof-number search hit it on the very first run.
    depth: u32,
}

/// GRAMMAR 8: "Hard runtime ceilings: recursion depth 128."
pub const MAX_CALL_DEPTH: u32 = 128;

type Env<'p> = Vec<(&'p str, Value)>;

impl<'a> Interp<'a> {
    pub fn new(net: &'a Net, tables: Vec<i64>) -> Self {
        Interp {
            net,
            cost: 0,
            evals: 0,
            tables,
            hash: Tt::new(),
            scratch: Vec::new(),
            budget: i64::MAX,
            depth: 0,
        }
    }

    /// Slot collisions in the last run: probes that hit an occupied slot holding a DIFFERENT
    /// position's key. FITNESS 10 lists "exploit hash collision / stale slot" as a degenerate
    /// solution, and a rate nobody measures is a rate nobody can bound.
    pub fn hash_collisions(&self) -> u64 {
        self.hash.collisions
    }

    pub fn run<'p>(&mut self, prog: &'p Program, pos: &Position, budget: i64) -> Move {
        self.cost = 0;
        self.evals = 0;
        self.budget = budget;
        self.depth = 0;
        self.hash.clear();
        let f = prog.entry();
        let mut env: Env<'p> = vec![
            (f.params[0].0.as_str(), Value::Pos(Rc::new(pos.clone()))),
            (f.params[1].0.as_str(), Value::Num(budget)),
        ];
        match self.exec(&f.body, prog, &mut env) {
            Flow::Ret(v) | Flow::Normal(v) => v.mv(),
        }
    }

    fn lookup(env: &Env<'_>, name: &str) -> Value {
        env.iter()
            .rev()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v.clone())
            .unwrap_or(Value::Unit)
    }

    fn assign<'p>(env: &mut Env<'p>, name: &'p str, v: Value) {
        if let Some(slot) = env.iter_mut().rev().find(|(n, _)| *n == name) {
            slot.1 = v;
        } else {
            env.push((name, v));
        }
    }

    fn exec<'p>(&mut self, n: &'p Node, prog: &'p Program, env: &mut Env<'p>) -> Flow {
        self.cost += cost_of(n);
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
                Value::List(Rc::new(l.as_slice().to_vec()))
            }
            Node::Apply(p, m) => {
                let pv = val!(p);
                let mv = val!(m).mv();
                let mut np = pv.pos().clone();
                np.make_move(mv);
                Value::Pos(Rc::new(np))
            }
            Node::Terminal(p) => {
                let p = val!(p);
                Value::Out(p.pos().outcome())
            }
            Node::Key(p) => {
                let p = val!(p);
                Value::Key(p.pos().zobrist())
            }
            Node::Eval(p) => {
                self.evals += 1;
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
                Value::Slot(self.hash.probe(key))
            }
            Node::Store(k, field, v) => {
                let key = match val!(k) {
                    Value::Key(x) => x,
                    _ => 0,
                };
                let val = val!(v).num();
                let e = self.hash.entry(key);
                match field {
                    FieldId::Score => e.score = val,
                    FieldId::Depth => e.depth = val,
                    FieldId::Flag => e.flag = val,
                    FieldId::Count => e.count = val,
                    FieldId::Sum => e.sum = val,
                    FieldId::Move => e.mv = val as u32,
                }
                Value::Unit
            }
            Node::Field(s, field) => {
                let sv = val!(s);
                match (sv, field) {
                    (Value::Slot(sl), FieldId::Score) => Value::Num(sl.score),
                    (Value::Slot(sl), FieldId::Depth) => Value::Num(sl.depth),
                    (Value::Slot(sl), FieldId::Flag) => Value::Num(sl.flag),
                    (Value::Slot(sl), FieldId::Count) => Value::Num(sl.count),
                    (Value::Slot(sl), FieldId::Sum) => Value::Num(sl.sum),
                    (Value::Slot(sl), FieldId::Move) => Value::Mv(Move(sl.mv)),
                    (other, _) => other,
                }
            }

            Node::Let(name, init, body) => {
                let v = val!(init);
                env.push((name.as_str(), v));
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
                    _ => Rc::new(vec![]),
                };
                for &m in items.iter() {
                    env.push((var.as_str(), Value::Mv(m)));
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
                    _ => Rc::new(vec![]),
                };
                let mut best = MOVE_NONE;
                let mut best_k = i64::MIN;
                for &m in items.iter() {
                    env.push((var.as_str(), Value::Mv(m)));
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
                if self.depth >= MAX_CALL_DEPTH {
                    // Ceiling reached: unwind with a neutral value rather than crashing.
                    return Flow::Ret(Value::Num(0));
                }
                let f = &prog.funcs[*idx];
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(val!(a));
                }
                let mut inner: Env<'p> = f
                    .params
                    .iter()
                    .zip(vals)
                    .map(|((n, _), v)| (n.as_str(), v))
                    .collect();
                self.depth += 1;
                let r = self.exec(&f.body, prog, &mut inner);
                self.depth -= 1;
                match r {
                    Flow::Ret(v) | Flow::Normal(v) => v,
                }
            }
            Node::Pred(..) => Value::Bool(false),
        };
        Flow::Normal(v)
    }
}
