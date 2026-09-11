//! Round-trippable text form for `Program`. Writes an S-expression, reads it back to an equal tree.
//!
//! WHY THIS EXISTS. `STATE.md` records the decisive test for the acceptance-rule question as
//! "whether the veto arm's final champion beats the control's, head to head", and calls it "one match,
//! on nets that will exist for free". **That test could not actually be run.** The nets persist, but
//! the thing that differs between the two arms is the evolved PROGRAM, and `evolve.rs:927` states the
//! saved `.prog` files are `{:#?}` Debug dumps -- "readable but not parseable back". Two independent
//! checks agree: `ast.rs` carries no serde derives and `grammar/Cargo.toml` has an EMPTY
//! `[dependencies]`. So every champion this project has ever evolved was unrecoverable the moment its
//! process exited, and any cross-run or cross-arm comparison was impossible by construction.
//!
//! WHY HAND-ROLLED. The workspace has zero external dependencies -- its own interpreter, its own NNUE,
//! its own everything. Pulling in serde to fix a persistence gap would trade that property away for a
//! file format, so the format is written here instead. It is ~200 lines and needs no build changes.
//!
//! WHY A ROUND-TRIP TEST AND NOT AN EYEBALL. A writer alone is what already exists and it is exactly
//! the thing that turned out to be useless. The property that matters is `read(write(p)) == p`, so
//! `Node`/`Func`/`Program` gain `PartialEq` and the test asserts it on every reference program. A
//! format that cannot reproduce the tree is a Debug dump with extra steps.
use crate::ast::*;

// ---------------------------------------------------------------- writing

fn q(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        if c == '"' || c == '\\' { o.push('\\'); }
        o.push(c);
    }
    o.push('"');
    o
}

fn w(n: &Node, o: &mut String) {
    match n {
        Node::Budget => o.push_str("budget"),
        Node::Nop => o.push_str("nop"),
        Node::Var(s) => { o.push_str("(var "); o.push_str(&q(s)); o.push(')'); }
        Node::Const(c) => { o.push_str(&format!("(const {c})")); }
        Node::OutcomeLit(l) => { o.push_str(&format!("(outcome {l:?})")); }
        Node::Moves(a) => un("moves", a, o),
        Node::Terminal(a) => un("terminal", a, o),
        Node::Key(a) => un("key", a, o),
        Node::Eval(a) => un("eval", a, o),
        Node::Unc(a) => un("unc", a, o),
        Node::Ret(a) => un("ret", a, o),
        Node::Probe(a) => un("probe", a, o),
        Node::Apply(a, b) => bi("apply", a, b, o),
        Node::Loop(a, b) => bi("loop", a, b, o),
        Node::Max(a, b) => bi("max", a, b, o),
        Node::Min(a, b) => bi("min", a, b, o),
        Node::Avg(a, b) => bi("avg", a, b, o),
        Node::ScoreOf(a, b) => bi("scoreof", a, b, o),
        Node::Pred(a, b, p) => { o.push_str("(pred "); w(a, o); o.push(' '); w(b, o);
                                 o.push_str(&format!(" {p:?})")); }
        Node::Cmp(a, b, r) => { o.push_str("(cmp "); w(a, o); o.push(' '); w(b, o);
                                o.push_str(&format!(" {r:?})")); }
        Node::Mix(a, b, c) => { o.push_str("(mix "); w(a, o); o.push(' '); w(b, o); o.push(' ');
                                w(c, o); o.push(')'); }
        Node::Store(a, f, b) => { o.push_str(&format!("(store {f:?} ")); w(a, o); o.push(' ');
                                  w(b, o); o.push(')'); }
        Node::Field(a, f) => { o.push_str(&format!("(field {f:?} ")); w(a, o); o.push(')'); }
        Node::Foreach(a, s, b) => bind("foreach", a, s, b, o),
        Node::Argmax(a, s, b) => bind("argmax", a, s, b, o),
        Node::Sort(a, s, b) => bind("sort", a, s, b, o),
        Node::Sample(a, s, b) => bind("sample", a, s, b, o),
        Node::Let(s, a, b) => { o.push_str("(let "); o.push_str(&q(s)); o.push(' '); w(a, o);
                                o.push(' '); w(b, o); o.push(')'); }
        Node::Set(s, a) => { o.push_str("(set "); o.push_str(&q(s)); o.push(' '); w(a, o); o.push(')'); }
        Node::If(c, t, e) => {
            o.push_str("(if "); w(c, o); o.push(' '); w(t, o);
            if let Some(e) = e { o.push(' '); w(e, o); }
            o.push(')');
        }
        Node::Call(i, args) => { o.push_str(&format!("(call {i}")); for a in args { o.push(' '); w(a, o); } o.push(')'); }
        Node::TRead(i, args) => { o.push_str(&format!("(tread {i}")); for a in args { o.push(' '); w(a, o); } o.push(')'); }
        Node::Arith(op, args) => { o.push_str(&format!("(arith {op:?}")); for a in args { o.push(' '); w(a, o); } o.push(')'); }
    }
}

fn un(t: &str, a: &Node, o: &mut String) { o.push('('); o.push_str(t); o.push(' '); w(a, o); o.push(')'); }
fn bi(t: &str, a: &Node, b: &Node, o: &mut String) {
    o.push('('); o.push_str(t); o.push(' '); w(a, o); o.push(' '); w(b, o); o.push(')');
}
fn bind(t: &str, a: &Node, s: &str, b: &Node, o: &mut String) {
    o.push('('); o.push_str(t); o.push(' '); w(a, o); o.push(' '); o.push_str(&q(s)); o.push(' ');
    w(b, o); o.push(')');
}

/// Serialise a program to text that `from_str` reads back to an equal tree.
pub fn to_string(p: &Program) -> String {
    let mut o = format!("(program {:?}", p.lineage);
    for f in &p.funcs {
        o.push_str("\n  (func ");
        o.push_str(&q(&f.name));
        o.push_str(" (");
        for (i, (n, t)) in f.params.iter().enumerate() {
            if i > 0 { o.push(' '); }
            o.push('('); o.push_str(&q(n)); o.push(' '); o.push_str(&format!("{t}")); o.push(')');
        }
        o.push_str(&format!(") {} ", f.ret));
        w(&f.body, &mut o);
        o.push(')');
    }
    o.push_str(")\n");
    o
}

// ---------------------------------------------------------------- reading

fn lex(s: &str) -> Result<Vec<String>, String> {
    let mut t = Vec::new();
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() { i += 1; }
        else if c == '(' || c == ')' { t.push(c.to_string()); i += 1; }
        else if c == '"' {
            let mut v = String::new();
            i += 1;
            loop {
                if i >= cs.len() { return Err("unterminated string".into()); }
                if cs[i] == '\\' { i += 1; if i >= cs.len() { return Err("bad escape".into()); } v.push(cs[i]); i += 1; }
                else if cs[i] == '"' { i += 1; break; }
                else { v.push(cs[i]); i += 1; }
            }
            t.push(format!("\u{1}{v}"));   // \u{1} marks "this was a quoted string"
        } else {
            let mut v = String::new();
            while i < cs.len() && !cs[i].is_whitespace() && cs[i] != '(' && cs[i] != ')' { v.push(cs[i]); i += 1; }
            t.push(v);
        }
    }
    Ok(t)
}

struct P { t: Vec<String>, i: usize }

impl P {
    fn peek(&self) -> Result<&str, String> {
        self.t.get(self.i).map(|s| s.as_str()).ok_or_else(|| "unexpected end".to_string())
    }
    fn next(&mut self) -> Result<String, String> {
        let v = self.peek()?.to_string(); self.i += 1; Ok(v)
    }
    fn open(&mut self) -> Result<(), String> {
        if self.next()? == "(" { Ok(()) } else { Err("expected (".into()) }
    }
    fn close(&mut self) -> Result<(), String> {
        if self.next()? == ")" { Ok(()) } else { Err("expected )".into()) }
    }
    fn str(&mut self) -> Result<String, String> {
        let v = self.next()?;
        v.strip_prefix('\u{1}').map(|s| s.to_string()).ok_or_else(|| format!("expected string, got {v}"))
    }
    fn atom(&mut self) -> Result<String, String> {
        let v = self.next()?;
        if v.starts_with('\u{1}') { Err("expected atom".into()) } else { Ok(v) }
    }
    fn boxed(&mut self) -> Result<Box<Node>, String> { Ok(Box::new(self.node()?)) }

    fn rest(&mut self) -> Result<Vec<Node>, String> {
        let mut v = Vec::new();
        while self.peek()? != ")" { v.push(self.node()?); }
        Ok(v)
    }

    fn node(&mut self) -> Result<Node, String> {
        let h = self.peek()?.to_string();
        if h == "budget" { self.i += 1; return Ok(Node::Budget); }
        if h == "nop" { self.i += 1; return Ok(Node::Nop); }
        self.open()?;
        let tag = self.atom()?;
        let n = match tag.as_str() {
            "var" => Node::Var(self.str()?),
            "const" => Node::Const(self.atom()?.parse::<i8>().map_err(|e| e.to_string())?),
            "outcome" => Node::OutcomeLit(outcome(&self.atom()?)?),
            "moves" => Node::Moves(self.boxed()?),
            "terminal" => Node::Terminal(self.boxed()?),
            "key" => Node::Key(self.boxed()?),
            "eval" => Node::Eval(self.boxed()?),
            "unc" => Node::Unc(self.boxed()?),
            "ret" => Node::Ret(self.boxed()?),
            "probe" => Node::Probe(self.boxed()?),
            "apply" => Node::Apply(self.boxed()?, self.boxed()?),
            "loop" => Node::Loop(self.boxed()?, self.boxed()?),
            "max" => Node::Max(self.boxed()?, self.boxed()?),
            "min" => Node::Min(self.boxed()?, self.boxed()?),
            "avg" => Node::Avg(self.boxed()?, self.boxed()?),
            "scoreof" => Node::ScoreOf(self.boxed()?, self.boxed()?),
            "pred" => { let a = self.boxed()?; let b = self.boxed()?; Node::Pred(a, b, pred(&self.atom()?)?) }
            "cmp" => { let a = self.boxed()?; let b = self.boxed()?; Node::Cmp(a, b, rel(&self.atom()?)?) }
            "mix" => Node::Mix(self.boxed()?, self.boxed()?, self.boxed()?),
            "store" => { let f = field(&self.atom()?)?; let a = self.boxed()?; let b = self.boxed()?; Node::Store(a, f, b) }
            "field" => { let f = field(&self.atom()?)?; Node::Field(self.boxed()?, f) }
            "foreach" => { let a = self.boxed()?; let s = self.str()?; Node::Foreach(a, s, self.boxed()?) }
            "argmax" => { let a = self.boxed()?; let s = self.str()?; Node::Argmax(a, s, self.boxed()?) }
            "sort" => { let a = self.boxed()?; let s = self.str()?; Node::Sort(a, s, self.boxed()?) }
            "sample" => { let a = self.boxed()?; let s = self.str()?; Node::Sample(a, s, self.boxed()?) }
            "let" => { let s = self.str()?; Node::Let(s, self.boxed()?, self.boxed()?) }
            "set" => { let s = self.str()?; Node::Set(s, self.boxed()?) }
            "if" => {
                let c = self.boxed()?; let t = self.boxed()?;
                let e = if self.peek()? == ")" { None } else { Some(self.boxed()?) };
                Node::If(c, t, e)
            }
            "call" => { let i = self.atom()?.parse::<usize>().map_err(|e| e.to_string())?; Node::Call(i, self.rest()?) }
            "tread" => { let i = self.atom()?.parse::<usize>().map_err(|e| e.to_string())?; Node::TRead(i, self.rest()?) }
            "arith" => { let o = arith(&self.atom()?)?; Node::Arith(o, self.rest()?) }
            other => return Err(format!("unknown node tag `{other}`")),
        };
        self.close()?;
        Ok(n)
    }
}

fn outcome(s: &str) -> Result<OutcomeLit, String> {
    Ok(match s { "None" => OutcomeLit::None, "Win" => OutcomeLit::Win, "Loss" => OutcomeLit::Loss,
                 "Draw" => OutcomeLit::Draw, _ => return Err(format!("bad outcome `{s}`")) })
}
fn pred(s: &str) -> Result<PredId, String> {
    Ok(match s { "IsCapture" => PredId::IsCapture, "GivesCheck" => PredId::GivesCheck,
                 "IsPromotion" => PredId::IsPromotion, "CapturedType" => PredId::CapturedType,
                 "MovingType" => PredId::MovingType, "FromSquare" => PredId::FromSquare,
                 "ToSquare" => PredId::ToSquare, _ => return Err(format!("bad pred `{s}`")) })
}
fn rel(s: &str) -> Result<Rel, String> {
    Ok(match s { "Lt" => Rel::Lt, "Le" => Rel::Le, "Eq" => Rel::Eq, "Ge" => Rel::Ge, "Gt" => Rel::Gt,
                 "Ne" => Rel::Ne, _ => return Err(format!("bad rel `{s}`")) })
}
fn field(s: &str) -> Result<FieldId, String> {
    Ok(match s { "Score" => FieldId::Score, "Depth" => FieldId::Depth, "Flag" => FieldId::Flag,
                 "Count" => FieldId::Count, "Sum" => FieldId::Sum, "Move" => FieldId::Move,
                 _ => return Err(format!("bad field `{s}`")) })
}
fn arith(s: &str) -> Result<ArithOp, String> {
    Ok(match s { "Add" => ArithOp::Add, "Sub" => ArithOp::Sub, "Mul" => ArithOp::Mul,
                 "Div" => ArithOp::Div, "Neg" => ArithOp::Neg, "Sqrt" => ArithOp::Sqrt,
                 "Log" => ArithOp::Log, _ => return Err(format!("bad arith `{s}`")) })
}
fn ty(s: &str) -> Result<Ty, String> {
    Ok(match s { "Pos" => Ty::Pos, "Move" => Ty::Move, "List" => Ty::List, "Score" => Ty::Score,
                 "Outcome" => Ty::Outcome, "Int" => Ty::Int, "Bool" => Ty::Bool, "Key" => Ty::Key,
                 "Slot" => Ty::Slot, "Unit" => Ty::Unit, _ => return Err(format!("bad ty `{s}`")) })
}

/// Parse a program written by [`to_string`]. Returns a message rather than panicking, because this is
/// also the entry point for files a human may have edited.
pub fn from_str(s: &str) -> Result<Program, String> {
    let mut p = P { t: lex(s)?, i: 0 };
    p.open()?;
    if p.atom()? != "program" { return Err("expected `program`".into()); }
    let lineage = match p.atom()?.as_str() {
        "Main" => Lineage::Main, "Purity" => Lineage::Purity,
        o => return Err(format!("bad lineage `{o}`")),
    };
    let mut funcs = Vec::new();
    while p.peek()? != ")" {
        p.open()?;
        if p.atom()? != "func" { return Err("expected `func`".into()); }
        let name = p.str()?;
        p.open()?;
        let mut params = Vec::new();
        while p.peek()? != ")" {
            p.open()?;
            let pn = p.str()?;
            let pt = ty(&p.atom()?)?;
            p.close()?;
            params.push((pn, pt));
        }
        p.close()?;
        let ret = ty(&p.atom()?)?;
        let body = p.node()?;
        p.close()?;
        funcs.push(Func { name, params, ret, body });
    }
    p.close()?;
    Ok(Program { funcs, lineage })
}
