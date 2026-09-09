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


/// A learned table with real index features, for `tread`. `dims` gives the extent of each index and
/// `data` is row-major. GRAMMAR 2.7: contents are tuned, never written by programs.
#[derive(Clone, Debug, Default)]
pub struct NdTable {
    pub dims: Vec<usize>,
    pub data: Vec<i64>,
}

/// A position together with its NNUE accumulator (the hidden-layer sums).
///
/// `eval` was a from-scratch gather at every leaf: ~38 active feature rows summed into the
/// hidden layer, every time, even when the caller had just made a single move. The hand-written
/// searcher has used an incremental accumulator for a while; the INTERPRETER, where every
/// evolved program actually runs, did not — so the search track paid full price per eval and
/// that is what makes depth-2 game gating unaffordable (GRAMMAR 8).
///
/// A move changes at most a handful of features, so `apply` updates the parent's accumulator by
/// the feature DELTA and `eval` becomes just the output layer.
/// Reusable scratch for the feature diff: the two active sets plus a membership bitset.
#[derive(Default)]
pub struct Delta {
    pub before: Vec<u16>,
    pub after: Vec<u16>,
    pub mark: Vec<u64>,
}

impl Delta {
    pub fn new() -> Self {
        Delta { before: Vec::new(), after: Vec::new(), mark: vec![0u64; (nnue::N_INPUTS + 63) / 64] }
    }
}

#[derive(Debug)]
pub struct PosAcc {
    pub pos: Position,
    pub acc: Vec<f32>,
}

impl PosAcc {
    /// Below this width the accumulator COSTS more than it saves, measured on this machine:
    ///
    /// | width | eval+apply from-scratch | incremental | |
    /// |---|---|---|---|
    /// | 16 | 399 ns | 441 ns | WORSE |
    /// | 64 | 665 ns | 582 ns | 12% better |
    /// | 256 | 2290 ns | 1456 ns | 36% better |
    ///
    /// The interpreter's values are immutable, so `apply` CLONES a position and an accumulator
    /// rather than mutating in place the way make/unmake does. That fixed clone cost is paid
    /// per node regardless of width, while the eval saving scales with width — so there is a
    /// crossover, and below it the carry is pure overhead.
    ///
    /// `pipeline::search::Searcher` had already found the same crossover for the hand-written
    /// search ("Crossover is near 64") and switches on it. Two independent measurements, same
    /// answer. The champion in play is width 16, so enabling this unconditionally would have
    /// made the net actually being used SLOWER.
    pub const INCREMENTAL_MIN_WIDTH: usize = 64;

    /// Full rebuild. Used once at the root; everything below it is incremental.
    pub fn fresh(net: &Net, pos: Position) -> Self {
        if net.n_hidden < Self::INCREMENTAL_MIN_WIDTH {
            return PosAcc { pos, acc: Vec::new() };
        }
        let mut acc = net.b1.clone();
        let mut idx = Vec::with_capacity(40);
        Net::active(&pos, &mut idx);
        let h = net.n_hidden;
        for f in idx {
            let row = &net.w1[f as usize * h..(f as usize + 1) * h];
            for (a, w) in acc.iter_mut().zip(row) { *a += *w; }
        }
        PosAcc { pos, acc }
    }

    /// Child after `mv`, with the accumulator carried forward by the feature delta.
    ///
    /// The set difference uses a BITSET, not `Vec::contains`. The first version used contains()
    /// and measured a NET LOSS: eval fell 311.6ns -> 34.3ns but apply rose 199ns -> 751ns, and
    /// alpha-beta does roughly one apply per eval, so the node got more expensive overall. The
    /// active sets are ~38 entries, so contains() inside a loop over the other set is 38x38
    /// comparisons per move. `pipeline::search::Searcher` had already hit this and its comment
    /// says so outright -- "O(n) diff via a bitset instead of the O(n^2) Vec::contains the first
    /// version used". Two implementations of the same idea, and the second repeated the mistake
    /// the first had written down.
    pub fn child(&self, net: &Net, mv: Move, scratch: &mut Delta) -> Self {
        if self.acc.is_empty() {
            let mut pos = self.pos.clone();
            pos.make_move(mv);
            return PosAcc { pos, acc: Vec::new() };
        }
        let Delta { before, after, mark } = scratch;
        Net::active(&self.pos, before);
        let mut pos = self.pos.clone();
        pos.make_move(mv);
        Net::active(&pos, after);

        let h = net.n_hidden;
        let mut acc = self.acc.clone();

        for w in mark.iter_mut() { *w = 0; }
        for &f in before.iter() { mark[f as usize >> 6] |= 1u64 << (f & 63); }
        for &f in after.iter() {
            if mark[f as usize >> 6] & (1u64 << (f & 63)) == 0 {
                let row = &net.w1[f as usize * h..(f as usize + 1) * h];
                for (a, w) in acc.iter_mut().zip(row) { *a += *w; }
            }
        }
        for w in mark.iter_mut() { *w = 0; }
        for &f in after.iter() { mark[f as usize >> 6] |= 1u64 << (f & 63); }
        for &f in before.iter() {
            if mark[f as usize >> 6] & (1u64 << (f & 63)) == 0 {
                let row = &net.w1[f as usize * h..(f as usize + 1) * h];
                for (a, w) in acc.iter_mut().zip(row) { *a -= *w; }
            }
        }
        PosAcc { pos, acc }
    }

    /// Mover-relative score from the accumulator. Must equal `Net::eval` exactly.
    ///
    /// Convenience wrapper that allocates its own scratch. Fine for tests and one-off callers;
    /// the interpreter's hot path uses `score_with` and passes a buffer it already owns.
    pub fn score(&self, net: &Net) -> i32 {
        self.score_with(net, &mut Vec::new())
    }

    /// As `score`, but borrows the caller's scratch buffer.
    ///
    /// The narrow path used to call `net.eval(&self.pos, &mut Vec::new())`, justified by "this
    /// path is only taken below width 64 where the gather is small". The gather being small is
    /// true and beside the point: a malloc/free pair is a FIXED cost per call and does not shrink
    /// with the feature count. Since the champion is width 16, that fallback IS the hot path --
    /// the excuse and the hot path were the same line.
    ///
    /// MEASURED at width 16 over 64 random-walk positions, best-of-7, both arms asserted to
    /// return identical scores (examples/alloc_probe.rs):
    ///     fresh Vec::new() per call  228.5 ns
    ///     reused scratch buffer      214.5 ns   -> 13.9 ns, 6.1% of eval
    /// That is 6% of an EVAL, not of a node -- `apply` clones a position on top of this -- so it
    /// is a small win, quoted as one. `Interp` already carried a `scratch` field for exactly this
    /// purpose, marked #[allow(dead_code)] because nothing could reach it: the buffer was kept
    /// and the parameter to receive it was never added, and the allow silenced the warning that
    /// would have said so.
    pub fn score_with(&self, net: &Net, scratch: &mut Vec<f32>) -> i32 {
        if self.acc.is_empty() {
            return net.eval(&self.pos, scratch);
        }
        let mut out = 0.0f32;
        for h in 0..net.n_hidden {
            let a = self.acc[h];
            if a > 0.0 { out += a * net.w2[h]; }
        }
        let white = (out + net.b2) * net.scale;
        let v = if self.pos.stm == board::types::Color::White { white } else { -white };
        v.clamp(-30_000.0, 30_000.0) as i32
    }
}

#[derive(Clone, Debug)]
pub enum Value {
    /// Rc, not a bare Position: a variable reference clones its Value, and the seed program
    /// names `p` four times per node. A bare Position made every mention a deep copy.
    Pos(Rc<PosAcc>),
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
    // TOTALITY. `num` returns 0 and `mv` returns MOVE_NONE for a value of the wrong shape; these
    // two PANICKED, and that asymmetry killed the search track twice.
    //
    // WHY A PANIC IS THE WRONG ANSWER HERE. The type checker admits a program, so the interpreter
    // must be able to RUN it. It cannot: an unbound variable types as Ty::Unit (typecheck.rs:59)
    // and can reach a site expecting a Pos, which crossover found within one generation of going
    // live. And the workspace builds with `panic = "abort"` (Cargo.toml), so catch_unwind CANNOT
    // rescue it -- the process dies whatever the caller does. Any fix that lives outside the
    // interpreter is therefore dead code, which is what my first two attempts at this were.
    //
    // Returning None makes the callers total, and every caller below degrades to the neutral
    // answer for its primitive -- no legal moves, no outcome, zero score. A program that asks for
    // a position where there is none computes nothing useful and SCORES badly, which is the
    // correct outcome for it and the whole point of a surrogate.
    fn pos(&self) -> Option<&Position> {
        match self {
            Value::Pos(p) => Some(&p.pos),
            _ => None,
        }
    }
    fn posacc(&self) -> Option<&PosAcc> {
        match self {
            Value::Pos(p) => Some(p.as_ref()),
            _ => None,
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
        // Incremental output layer at width >= 64 (from-scratch below it -- measured
        // crossover). Was 1365 when every eval was a from-scratch gather.
        Node::Eval(_) => 165,
        Node::Moves(_) => 2232,
        // Now also carries the NNUE accumulator forward, so it costs more than a bare
        // make_move and eval costs far less. The pair is what matters, not either alone.
        Node::Apply(..) => 1959,
        Node::Terminal(_) => 703,
        // Was 97 (a from-scratch zobrist: ~32 XORs plus bitboard iteration). Position now
        // maintains the key incrementally through make/unmake, verified equal to the
        // from-scratch value at every node of a perft walk, so reading it is a field load.
        Node::Key(_) => 1,
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
    /// Illegal/none moves handed to `apply` by a program. Diagnostic, not an error.
    pub illegal_applies: u64,
    /// Times the recursion ceiling was hit and a call unwound with the substitute value 0.
    ///
    /// That substitute is documented as "a neutral value", and for most programs it is. For
    /// proof-number search it is not neutral at all: 0 is the value meaning PROVEN WIN, so a
    /// ceiling hit does not unwind quietly, it asserts that the line is won and the back-up
    /// carries that all the way to the root. The reference PN program returned the first legal
    /// move on 23/23 mate-in-one positions and this is why -- invisible, because a ceiling hit
    /// looked exactly like a completed search. Counting it makes the failure observable.
    pub ceiling_hits: u64,
    /// Learned integer tables. Index 0 = D (depth), 1 = INF, 2.. = whatever a program reads.
    pub tables: Vec<i64>,
    /// INDEXED tables, checked before `tables`. Empty by default, so behaviour is unchanged unless
    /// a caller supplies one. See the TRead arm for why this exists.
    pub tables_nd: Vec<NdTable>,
    hash: Tt,
    /// Scratch for the narrow-width fallback in `PosAcc::score_with`, so the hot eval path
    /// allocates nothing per node. This field existed already but nothing could reach it --
    /// `score` took no buffer -- so it sat behind an #[allow(dead_code)] that hid the gap.
    scratch: Vec<f32>,
    /// Reused active-feature buffers for the incremental accumulator, so `apply` allocates
    /// nothing per node.
    featbuf: Delta,
    budget: i64,
    /// Set when a program exceeded its cost budget. GRAMMAR 8 requires "total cost units per
    /// `choose` = budget" as a HARD ceiling, alongside the recursion ceiling. Only the
    /// recursion one existed, so an evolved program with an unbounded loop ran forever: the
    /// first real search-track run burned a full core for FOUR HOURS on one candidate with no
    /// output. FITNESS 10 lists exactly this ("infinite loop / budget abuse") as a degenerate
    /// solution the ceilings are supposed to catch.
    pub over_budget: bool,
    /// SAFETY ceiling on accumulated cost, separate from the program-visible `budget`.
    ///
    /// These are different quantities and conflating them was a mistake. `budget` is the value
    /// the program RECEIVES as its second parameter and reads via the `budget` primitive —
    /// UCT uses it as a simulation count, alpha-beta ignores it. The ceiling is the harness
    /// refusing to let any program run forever. Enforcing the former as the latter made
    /// `run(prog, pos, 24)` mean "stop after 24 cost units", which is less than a single eval
    /// (1365) and made every reference program forfeit instantly.
    pub cost_cap: u64,
    /// Current call depth, against the hard ceiling of GRAMMAR 8. Without it an evolved
    /// program that recurses unboundedly takes down the whole process -- FITNESS 10 lists
    /// "infinite loop / stack blow" as a degenerate solution the ceilings are supposed to
    /// catch, and a faithful proof-number search hit it on the very first run.
    depth: u32,
}

/// GRAMMAR 8: "Hard runtime ceilings: recursion depth 128."
pub const MAX_CALL_DEPTH: u32 = 128;

/// UCT exploration weight -- table slot 2, read by `uct_mcts` as `TRead(2)`.
///
/// DERIVED FROM THE INTERPRETER'S ARITHMETIC, not tuned. The term is
/// `u = Sqrt(Div(Mul(Log(visits(parent)), K), visits(child) + 1))`, which is
/// `sqrt(K) * sqrt(ln N / (n+1))`, so **sqrt(K) IS the UCT exploration constant C**.
///
/// Two requirements fix it:
///  1. `Div` truncates, so `u` collapses to 0 once `n + 1 > Log(N) * K` -- permanently, for that
///     child. At the previous K = 8 that cliff sits at 40 visits (budget 256, Log = 5) through 64
///     visits (budget 4096, Log = 8), all far BELOW the reachable visit counts, so exploration
///     switched off partway through every run. That is measurable and was measured: mates rise to
///     13 at budget 1024 and then FALL to 12 and 11 at 2048 and 4096. More playouts made UCT worse.
///  2. `u` must be commensurable with `q`, which is in eval units. The net's declared scale is 600,
///     so C = 600 -- one eval unit -- gives K = 360_000. Standard UCT uses C ~ 1.4 against values
///     in [0,1], i.e. exploration EXCEEDS the value range early and decays; this reproduces that
///     shape in eval units.
///
/// At K = 360_000 the collapse point moves to ~1.8M visits, unreachable at any budget here, and the
/// term decays smoothly: 1341, 948, 404, 209, 133, 42, 21 at 0/1/10/40/100/1000/4000 visits.
///
/// It is a CONSTANT AND NOT A LITERAL because the value was previously written out at ~20 call
/// sites as `8` and at four more in `reference_audit.rs` as `1`. The audit that certified these
/// reference programs therefore ran UCT with a different exploration weight than the loop does.
/// DEFAULT IS THE DECLARED 8, because both values I derived to replace it measured WORSE:
/// K = 2000 gives 2 mates at budget 64 and K = 360_000 gives 0, against 10 for K = 8, and cost
/// rises 50x for the same playout count. The saturation analysis below is still arithmetically
/// true; it simply is not what limits mate-finding at 16-4096 playouts against a branching factor
/// near 30, where the search has to exploit rather than explore.
// RAISED 8 -> 600 on 2026-09-09, when the SUM encoding became the lineage seed.
//
// Safe to change globally because slot 2 is read in exactly one place -- reference.rs:536, inside
// the shared UCT body. Alpha-beta reads slots 0 and 1, table_reduction reads 3. Nothing else can be
// affected by this number.
//
// 600 is the net's declared eval scale, i.e. C = one eval unit, and it is the value at which the
// sum encoding first solves 23/23 mate-in-one (holding at 4096 and 360000). The comment below
// records that derivation as "REFUTED by measurement"; it was refuted only through the Mix
// encoding, whose blend coefficient (16 - c) is hugely NEGATIVE at that value, so the test inverted
// the exploration term instead of enlarging it. Measured directly: at K=600 the Mix form takes 1604
// recursion-ceiling hits and the sum form takes 1.
//
// THE MIX FORM MUST NOT BE RUN AT THIS VALUE. It is pathological there -- descents deepen until
// they hit MAX_CALL_DEPTH and unwind without reaching a leaf. Anything measuring the Mix encoding
// passes its own weight explicitly rather than reading this constant.
pub const UCT_EXPLORATION: i64 = 600;

/// Env-overridable accessor, so the weight can be SWEPT instead of guessed again.
///
/// 360_000 was derived as "C = one eval unit" and is REFUTED by measurement: it gives 0 mates at
/// budgets 16 and 64 at 3.6x alpha-beta's cost, because exploration then dominates so completely
/// that the search never exploits. The derivation was right in form -- sqrt(K) is the UCT constant,
/// and the collapse-to-zero cliff at K = 8 is real -- and wrong in magnitude: `u` needs to be
/// commensurable with the DIFFERENCES between sibling q values, not with q's absolute range.
///
/// Both endpoints are now known to fail, which brackets the answer rather than settling it:
///   K = 8        cliff at 40-64 visits, exploration switches off mid-run, mates peak then FALL
///   K = 360_000  no cliff, but exploration swamps exploitation and mates go to 0
/// `EXISTENCE_UCT_K` exists so the interior is measured.
pub fn uct_exploration() -> i64 {
    std::env::var("EXISTENCE_UCT_K").ok().and_then(|s| s.parse().ok()).unwrap_or(UCT_EXPLORATION)
}

type Env<'p> = Vec<(&'p str, Value)>;

impl<'a> Interp<'a> {
    pub fn new(net: &'a Net, tables: Vec<i64>) -> Self {
        Interp {
            net,
            cost: 0,
            evals: 0,
            illegal_applies: 0,
            ceiling_hits: 0,
            tables,
            tables_nd: Vec::new(),
            hash: Tt::new(),
            scratch: Vec::new(),
            featbuf: Delta::new(),
            budget: i64::MAX,
            over_budget: false,
            // Generous but FINITE. Large enough that no honest program notices, small enough
            // that a runaway dies in seconds rather than hours.
            cost_cap: 2_000_000_000,
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
        self.illegal_applies = 0;
        self.ceiling_hits = 0;
        self.budget = budget;
        self.over_budget = false;
        self.depth = 0;
        self.hash.clear();
        let f = prog.entry();
        let mut env: Env<'p> = vec![
            (f.params[0].0.as_str(), Value::Pos(Rc::new(PosAcc::fresh(self.net, pos.clone())))),
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
        if self.cost >= self.cost_cap {
            // Stop the whole program, not just this node. Returning Ret unwinds every frame,
            // and `run` reports MOVE_NONE, which callers already treat as a forfeit.
            self.over_budget = true;
            return Flow::Ret(Value::Unit);
        }
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
                // No position => no moves. Foreach over an empty list is a no-op.
                let l = match p.pos() { Some(x) => x.legal_moves(), None => Default::default() };
                Value::List(Rc::new(l.as_slice().to_vec()))
            }
            Node::Apply(p, m) => {
                let pv = val!(p);
                let mv = val!(m).mv();
                // APPLYING AN ILLEGAL MOVE IS A NO-OP, NOT A PANIC.
                //
                // `Position::make_move` expects a legal move and panics on an empty from-square.
                // That is the right contract for the engine, where every move comes from the
                // generator -- and the wrong one for the INTERPRETER, where the move is whatever
                // an evolved program computed.
                //
                // FOUND BY CROSSOVER, on its first run: grafting `field(probe(key(p)), Move)` into
                // an Apply site reads the Move field of an EMPTY hash slot, which is MOVE_NONE,
                // and the whole search track died with "make_move: empty from-square". The bug is
                // not the graft. A program that computes nonsense must SCORE badly, never take the
                // process down -- otherwise the fitness function is undefined on exactly the
                // candidates the search is there to explore, and the operator that finds them
                // looks broken instead of the interpreter.
                //
                // Returning the position unchanged makes it score badly by itself: a program that
                // never advances the position finds no mates and burns cost.
                if mv == board::types::MOVE_NONE
                    || pv.posacc().is_none_or(|a| !a.pos.legal_moves().as_slice().contains(&mv))
                {
                    self.illegal_applies += 1;
                    return Flow::Normal(pv);
                }
                let mut sc = std::mem::take(&mut self.featbuf);
                let child = match pv.posacc() {
                    Some(a) => a.child(self.net, mv, &mut sc),
                    None => { self.featbuf = sc; return Flow::Normal(pv); }
                };
                self.featbuf = sc;
                Value::Pos(Rc::new(child))
            }
            Node::Terminal(p) => {
                let p = val!(p);
                // No position => no terminal verdict. NONE is the grammar's 'not terminal'.
                Value::Out(p.pos().map(|x| x.outcome()).unwrap_or(board::Outcome::Ongoing))
            }
            Node::Key(p) => {
                let p = val!(p);
                Value::Key(p.pos().map(|x| x.key).unwrap_or(0))
            }
            Node::Eval(p) => {
                self.evals += 1;
                let p = val!(p);
                // Output layer only: the hidden sums were carried forward by `apply`.
                let net = self.net;
                Value::Num(match p.posacc() {
                    Some(a) => a.score_with(net, &mut self.scratch) as i64,
                    None => 0,
                })
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

            // INDEXED TABLE READ. GRAMMAR primitive #26 declares `tread` as "read a learned
            // integer table BY INDEX FEATURES"; this discarded its arguments entirely
            // (`TRead(i, _)`), so a table could only ever return one scalar.
            //
            // Measured consequence: table_reduction (ladder rung 7) reads TRead(3, [d, i]) to
            // reduce by depth and move index, and had a BYTE-IDENTICAL eval count to the seed
            // (441,471 both) -- a no-op costing 0.7%. Doubly dead, since the harness passes three
            // tables so index 3 was out of range as well.
            //
            // `tables_nd` holds genuinely indexed tables and is checked first; anything not found
            // there falls back to the scalar `tables`, so every existing call site keeps working
            // unchanged. Indices are folded row-major and CLAMPED rather than wrapped: a program
            // that computes an out-of-range feature should read the edge of the table, not a
            // wrapped-around unrelated entry, which would be a silent correctness trap of exactly
            // the kind this file has produced three times today.
            Node::TRead(i, args) => {
                // Loop, not map: `val!` early-returns on Flow::Ret and cannot do that in a closure.
                let mut vals: Vec<i64> = Vec::with_capacity(args.len());
                for a in args { vals.push(val!(a).num()); }
                match self.tables_nd.get(*i) {
                    Some(t) if !t.data.is_empty() => {
                        let mut idx = 0usize;
                        for (k, dim) in t.dims.iter().enumerate() {
                            let v = vals.get(k).copied().unwrap_or(0).max(0) as usize;
                            idx = idx * dim + v.min(dim.saturating_sub(1));
                        }
                        Value::Num(*t.data.get(idx).unwrap_or(&0))
                    }
                    _ => Value::Num(*self.tables.get(*i).unwrap_or(&0)),
                }
            }
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
                    // Ceiling reached: unwind with a substitute value rather than crashing.
                    // NOT neutral in every domain -- see `ceiling_hits`.
                    self.ceiling_hits += 1;
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
            // ---- THE PREDICATE PRIMITIVE. This was `=> Value::Bool(false)`: a hardcoded stub.
            //
            // GRAMMAR 2.1 declares `pred` as primitive #5 with a full signature and a declared
            // PredId list, and NOTHING implemented it. Consequences, all measured before this fix:
            // capture_extension (ladder rung 6) had a byte-identical eval count to the seed
            // (4,127,466 both) because its condition was always false, so it was the seed plus a
            // dead branch costing 1.6% -- while GRAMMAR 6 called it "faithful". And Op::WrapIfPred
            // was a disguised DELETE: wrapping a statement in `if false` removes it.
            //
            // Rules-derived only. `is_capture` and `is_promotion` read the move's own FLAG, which
            // the move generator sets; `gives_check` applies the move and asks the board. No piece
            // values, no ordering heuristics, nothing about which of these is GOOD -- that is what
            // the search has to discover, and GRAMMAR 2.1 forbids it here explicitly.
            Node::Pred(mv, pv, id) => {
                let m = val!(mv).mv();
                let pos_v = val!(pv);
                match (m == MOVE_NONE, pos_v.pos()) {
                    (false, Some(pos)) => {
                        use board::types::MoveFlag as F;
                        let b = match id {
                            PredId::IsCapture => {
                                matches!(m.flag(), F::Capture | F::EnPassant | F::PromoCapture)
                            }
                            PredId::IsPromotion => matches!(m.flag(), F::Promo | F::PromoCapture),
                            PredId::GivesCheck => {
                                let mut q = pos.clone();
                                q.make_move(m);
                                let opp = q.stm;
                                q.in_check(opp)
                            }
                            // SPEC GAP, recorded rather than invented. GRAMMAR types `pred` as
                            // returning Bool, but CapturedType, MovingType, FromSquare and
                            // ToSquare name quantities that are not booleans. Any Bool reading of
                            // them ("captures a non-pawn", "moves to the centre") would be
                            // smuggling in chess knowledge that GRAMMAR 2.1 forbids -- "No values,
                            // no importance, no piece worth". They stay false until the spec says
                            // what they mean.
                            _ => false,
                        };
                        Value::Bool(b)
                    }
                    _ => Value::Bool(false),
                }
            }
        };
        Flow::Normal(v)
    }
}
