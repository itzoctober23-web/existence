//! The typed AST for search programs (GRAMMAR.md 1-3).
//!
//! A program is 1..4 typed functions; function 0 is `choose(Pos, Int) -> Move`. Every node
//! carries a type and the tree is well-typed by construction, so an ill-typed mutation is
//! rejected at generation time (cheap) rather than at gate time (expensive).

use std::fmt;

/// GRAMMAR.md 1. `Outcome` is symbolic on purpose — there is no Outcome-to-number coercion
/// except through `score_of`, which is a LEARNED table. That is how "draw = 0" and any
/// preference for shorter mates stay out of the Given column.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub enum Ty {
    Pos,
    Move,
    List,
    Score,
    Outcome,
    Int,
    Bool,
    Key,
    Slot,
    Unit,
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Ty::Pos => "Pos",
            Ty::Move => "Move",
            Ty::List => "List",
            Ty::Score => "Score",
            Ty::Outcome => "Outcome",
            Ty::Int => "Int",
            Ty::Bool => "Bool",
            Ty::Key => "Key",
            Ty::Slot => "Slot",
            Ty::Unit => "Unit",
        };
        write!(f, "{s}")
    }
}

/// Rules-derived move predicates. GRAMMAR 2.1 #5: rules facts only. `captured_type` plus a
/// LEARNED table is MVV-LVA waiting to be discovered; the ordering is given nowhere.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum PredId {
    IsCapture,
    GivesCheck,
    IsPromotion,
    CapturedType,
    MovingType,
    FromSquare,
    ToSquare,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    Neg,
    Sqrt,
    Log,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Rel {
    Lt,
    Le,
    Eq,
    Ge,
    Gt,
    Ne,
}

/// Slot fields (GRAMMAR 2.5 #22). The FieldId fixes the static type so the tree stays typed.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FieldId {
    Score,
    Depth,
    Flag,
    Count,
    Sum,
    Move,
}

impl FieldId {
    pub fn ty(self) -> Ty {
        match self {
            FieldId::Move => Ty::Move,
            _ => Ty::Int,
        }
    }
}

/// The 29 primitives of GRAMMAR.md 2, plus the binding forms.
#[derive(Clone, Debug)]
pub enum Node {
    // 2.1 rules access
    Moves(Box<Node>),                       // Pos -> List
    Apply(Box<Node>, Box<Node>),            // Pos x Move -> Pos
    Terminal(Box<Node>),                    // Pos -> Outcome
    Key(Box<Node>),                         // Pos -> Key
    Pred(Box<Node>, Box<Node>, PredId),     // Move x Pos x PredId -> Bool

    // 2.2 evaluation
    Eval(Box<Node>), // Pos -> Score

    // 2.3 control
    Foreach(Box<Node>, String, Box<Node>),  // List x var x body -> Unit
    Loop(Box<Node>, Box<Node>),             // Int x body -> Unit
    If(Box<Node>, Box<Node>, Option<Box<Node>>),
    Call(usize, Vec<Node>),
    Ret(Box<Node>),
    Budget,
    Let(String, Box<Node>, Box<Node>),
    Set(String, Box<Node>),

    // 2.4 arithmetic / comparison
    Arith(ArithOp, Vec<Node>),
    Cmp(Box<Node>, Box<Node>, Rel),
    Max(Box<Node>, Box<Node>),
    Min(Box<Node>, Box<Node>),
    Avg(Box<Node>, Box<Node>),
    Mix(Box<Node>, Box<Node>, Box<Node>),

    // 2.5 memory
    Probe(Box<Node>),
    Store(Box<Node>, Box<Node>),
    Field(Box<Node>, FieldId),

    // 2.6 selection
    Argmax(Box<Node>, String, Box<Node>),
    Sort(Box<Node>, String, Box<Node>),
    Sample(Box<Node>, String, Box<Node>),

    // 2.7 tables and outcomes
    TRead(usize, Vec<Node>),
    ScoreOf(Box<Node>, Box<Node>),
    Const(i8),

    // leaves
    Var(String),
    /// A literal Outcome, used only for comparison against `terminal` (no coercion exists).
    OutcomeLit(OutcomeLit),
    Nop,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum OutcomeLit {
    None,
    Win,
    Loss,
    Draw,
}

#[derive(Clone, Debug)]
pub struct Func {
    pub name: String,
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
    pub body: Node,
}

#[derive(Clone, Debug)]
pub struct Program {
    pub funcs: Vec<Func>,
    pub lineage: Lineage,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Lineage {
    Main,
    Purity,
}

impl Node {
    /// Node count as GRAMMAR 6 defines it: every primitive and variable reference counts once;
    /// a lambda counts as one node plus its body. THIS is what replaces the hand estimates —
    /// the declared prior is re-derived from these numbers, never typed by hand.
    pub fn size(&self) -> usize {
        use Node::*;
        1 + match self {
            Budget | Const(_) | Var(_) | OutcomeLit(_) | Nop => 0,
            Moves(a) | Terminal(a) | Key(a) | Eval(a) | Ret(a) | Probe(a) | Field(a, _) => a.size(),
            Apply(a, b) | Store(a, b) | Max(a, b) | Min(a, b) | Avg(a, b) | ScoreOf(a, b) => {
                a.size() + b.size()
            }
            Cmp(a, b, _) => a.size() + b.size(),
            Pred(a, b, _) => a.size() + b.size(),
            Mix(a, b, c) => a.size() + b.size() + c.size(),
            Set(_, a) => a.size(),
            Loop(a, b) => a.size() + b.size(),
            // lambda-bearing forms: +1 for the lambda itself
            Foreach(a, _, b) | Argmax(a, _, b) | Sort(a, _, b) | Sample(a, _, b) => {
                a.size() + 1 + b.size()
            }
            Let(_, a, b) => a.size() + b.size(),
            If(c, t, e) => c.size() + t.size() + e.as_ref().map_or(0, |x| x.size()),
            Call(_, args) | Arith(_, args) | TRead(_, args) => {
                args.iter().map(|a| a.size()).sum::<usize>()
            }
        }
    }

    /// Sequence of statements, rendered as nested Lets — there is no Seq primitive in the
    /// grammar and inventing one would be adding a Given row.
    ///
    /// A `Let` written as a STATEMENT must scope over the REST of the sequence, not over its
    /// own placeholder body. Getting this wrong made `best` in the alpha-beta seed invisible
    /// to the loop that accumulates it, so the encoded program silently read 0; the benchmark's
    /// eval-count equivalence check is what exposed it (1361 vs 825 leaves).
    pub fn seq(mut stmts: Vec<Node>) -> Node {
        match stmts.len() {
            0 => Node::Nop,
            1 => stmts.pop().unwrap(),
            _ => {
                let first = stmts.remove(0);
                let rest = Node::seq(stmts);
                match first {
                    // re-scope a binding over everything that follows it
                    Node::Let(name, init, body) if matches!(*body, Node::Nop) => {
                        Node::Let(name, init, Box::new(rest))
                    }
                    other => Node::Let("_".into(), Box::new(other), Box::new(rest)),
                }
            }
        }
    }
}

impl Program {
    pub fn size(&self) -> usize {
        self.funcs.iter().map(|f| f.body.size()).sum()
    }
    pub fn entry(&self) -> &Func {
        &self.funcs[0]
    }
}
