# GRAMMAR.md — The Search Primitive Grammar

This file is the real Given column for the search track. Everything in it is a human
choice, so every item is declared, defended, and its effect on the prior stated as a
number. If the thesis fails, it most likely fails here.

## 0. Purpose and constraints
- The engine's search is a PROGRAM in this grammar, evolved with the gate as fitness.
- The grammar must be able to express bounded minimax (alpha-beta), Monte-Carlo tree
  search, proof-number search, and hybrids of them, with none of them hard-coded.
- The grammar must contain NO chess knowledge. Test: swapping the movegen for Shogi's
  changes nothing here. Every predicate available to programs is rules-derived.
- Program length is a prior. The lengths of the reference programs (Section 6) ARE the
  declared prior. They are reported, not hidden.
- Evolved programs compile to a bytecode that runs at near hand-written speed; the
  grammar may not force per-step work that a hand-written search would not do.

## 1. Types
| Type | Meaning |
|---|---|
| Pos | a game state (rules-defined: board planes, side to move, rules-carried non-board state) |
| Move | a legal move |
| List | an ordered list of Move |
| Score | a signed integer, mover-relative |
| Outcome | symbolic terminal result: WIN / LOSS / DRAW / NONE (rules-defined; NOT a number) |
| Int | a signed integer (counts, depths, budgets, constants) |
| Bool | true/false |
| Key | a hash key of a Pos (Zobrist; engineering, no content) |
| Slot | a hash-table record: {score: Score, depth: Int, flag: Int, count: Int, sum: Score, move: Move} — all fields optional; a program uses the ones it reads/writes |
| Tab | a reference to a learned integer table |
| Fn | one of the program's own functions (for recursion / calls) |

Programs are statically typed. Mutations must type-check; ill-typed candidates are
discarded at generation time (cheap) rather than at gate time (expensive).

## 2. Primitives (29)
Grouped by role. "Prior" notes which reference programs each primitive shortens.

### 2.1 Rules access (5) — content comes only from the movegen
| # | Primitive | Signature | Prior |
|---|---|---|---|
| 1 | moves | Pos -> List | all |
| 2 | apply | Pos x Move -> Pos | all |
| 3 | terminal | Pos -> Outcome | all |
| 4 | key | Pos -> Key | AB (hash), MCTS (stats), PN (stats) |
| 5 | pred | Move x Pos x PredId -> Bool | ordering/pruning programs. PredId is drawn from a DECLARED list of rules-derived predicates: is_capture, gives_check, is_promotion, captured_type, moving_type, from/to square. No values, no "importance", no piece worth |

### 2.2 Evaluation (1)
| 6 | eval | Pos -> Score | the learned net. The only non-rules information a program can read about a position |

### 2.3 Control (8)
| 7 | foreach | List x (Move -> Unit) -> Unit | sequential iteration. Prior: favors depth-first programs (AB) |
| 8 | loop | Int x (Unit -> Unit) -> Unit | repeat N times or until budget exhausted |
| 9 | if | Bool x T x T -> T | branching |
| 10 | call | Fn x args -> T | call one of the program's own functions (recursion) |
| 11 | ret | T -> exits current function with T | early return / break-out. Prior: alpha-beta cutoff needs it; so does PN's proof exit |
| 12 | budget | Unit -> Int | COST UNITS remaining in the current search budget, where cost is the running sum of per-primitive modeled cost (Section 8) accumulated as primitives execute. Not nodes (alpha-beta vocabulary) and not evaluations (proof-number search proves mates with zero eval calls, so eval-count is gameable by terminal-driven programs). Every primitive costs something; cost units are the one unit every paradigm consumes |
| 13 | let | name x T x body -> T | local binding (immutable) |
| 13b | set | name x T -> Unit | assign to an existing local of the same type. Needed for accumulators (best, alpha, count). This is primitive #29; numbering kept as 13b to keep the rules/eval/control/arith/memory/selection/tables grouping intact |

### 2.4 Arithmetic and comparison (6)
| 14 | arith | Int x Int x Op -> Int, Op in {add, sub, mul, div, neg, sqrt, log} | Prior: sqrt and log exist ONLY so UCT-style exploration terms are expressible; they shorten MCTS, not AB. Declared |
| 15 | cmp | (Int x Int x Rel) -> Bool, Rel in {<, <=, ==, >=, >, !=}; also (Outcome x Outcome x Rel) -> Bool with Rel in {==, !=} | comparisons on numbers and on symbolic outcomes; no Outcome-to-number coercion exists except via score_of |
| 16 | max | Int x Int -> Int | Prior: minimax backup |
| 17 | min | Int x Int -> Int | Prior: minimax backup |
| 18 | avg | Score x Int -> Score (sum / count) | Prior: MCTS backup |
| 19 | mix | Int x Int x Int -> Int (a*w + b*(K-w))/K | blends; hybrids |

### 2.5 Memory (3)
| 20 | probe | Key -> Slot? | hash read |
| 21 | store | Key x FieldId x Int -> Unit | hash write of ONE named slot field. **Amended from `Key x Slot -> Unit`:** the grammar has no primitive that CONSTRUCTS a Slot, so the original signature was unimplementable. Writing a named field is what every reference program actually needs and avoids adding a record-constructor primitive (which would be a new Given row). Found while making the faithful UCT executable |
| 22 | field | Slot x FieldId -> T, where T is fixed by FieldId (score/depth/flag/count/sum -> Int; move -> Move) | read a slot field; the FieldId determines the static type, so the tree stays well-typed |

### 2.6 Selection (3)
| 23 | argmax | List x (Move -> Int) -> Move | pick by a quantity the program computes |
| 24 | sort | List x (Move -> Int) -> List | order by a quantity |
| 25 | sample | List x (Move -> Int) -> Move | stochastic pick proportional to weights; seeded RNG for reproducibility. Prior: exploration programs |

### 2.7 Learned tables and outcomes (3)
| 26 | tread | Tab x Int... -> Int | read a learned integer table by index features. Table contents are SPSA-tuned, never written by programs |
| 27 | score_of | Outcome x Int(depth) -> Score | maps a terminal Outcome to a Score via a LEARNED table indexed by (outcome, depth). WIN/LOSS magnitudes, DRAW value, and any mate-distance preference are table contents, not constants. This is how "draw = 0" and "faster mates preferred" stay out of the Given column |
| 28 | const | Int literal in [-8, 8] | small integers only; anything larger must come from a table read or arithmetic. Prevents programs from hard-coding magnitudes |

Not present, deliberately: any notion of piece value, material, mobility, king safety,
"quiet move", "good capture", or move quality. `pred` exposes rules facts only.

## 3. Program structure
- A program is 1 to 4 typed functions. Function 0 is the entry: `choose(Pos, Int budget) -> Move`.
- Functions may call each other and themselves (`call`). Recursion depth is bounded by
  the budget primitive and a hard runtime ceiling (Section 8).
- Total size cap: **staged**. 96 nodes through P2; **160 nodes** from P3 onward
  (declared, not learned — the cap is a budget the experimenter sets, like the TC).
  Rationale: Section 6 puts AB + hash + ID + qsearch + LMR at ~72; the plan's
  rediscovery order continues through history/killers, null move, and futility margins
  at ~5-10 nodes each, i.e. ~100+. A 96 cap would bind exactly at the P3 milestone and
  silently cap the search below Stockfish-class structure. 160 leaves headroom for
  hybrids and for structure nobody has named. Max tree depth per function: 12.
- Every node has a type; the tree is well-typed by construction.

## 4. Mutation operators (10) — all type-preserving
| Op | Effect | Enables |
|---|---|---|
| replace | swap a node for another of the same type (fresh leaves for children) | everything |
| insert | wrap a node in a new parent of the same result type (e.g., wrap Score in max(_, const)) | growth |
| delete | replace a node by one of its same-typed children | shrinkage |
| tweak | change a const by +/-1..3 or a PredId/Op/Rel/FieldId to another | tuning |
| dup | copy a subtree to another same-typed site | reuse |
| swap | exchange two sibling subtrees of the same type | ordering |
| wrap-if | wrap a statement in if(cond, stmt, noop) with a fresh Bool | conditional behavior |
| wrap-loop | wrap a statement in loop(N, _) or foreach(moves(p), _) | lookahead, iteration |
| add-arg | add an Int/Score parameter to a function and thread a value at each call site | windows, depth counters |
| add-fn | split a subtree into a new function and call it | recursion |

Mutation rate: 1-3 operators per candidate, chosen uniformly. A candidate identical to
any ancestor in the ledger is discarded.

**MEASURED on the bare alpha-beta seed, 2026-09-08** (`cargo test -p grammar --test mutate --
--nocapture`), 160 placement attempts per operator:

| operator | applied | no matching node | ILL-TYPED |
|---|---|---|---|
| Tweak | 5 | 155 | **0** |
| WrapIf | 2 | 158 | **0** |
| WrapLoop | 2 | 158 | **0** |
| Delete | 9 | 151 | **0** |
| Dup | 2 | 158 | **0** |
| SwapSiblings | 3 | 157 | **0** |
| InsertMax | 11 | 149 | **0** |
| ReplaceConst | 2 | 158 | **0** |

Two things this settles rather than assumes:

1. **"All type-preserving" is verified.** Zero ill-typed programs across every operator and
   every placement. The claim in the heading above was previously an intention.
2. **Applicability varies 5.5x** (InsertMax 11 sites, four operators only 2). That asymmetry is
   harmless now and was not before: operator selection used to be a RACE — a fresh random
   operator was drawn on each retry and whichever applied first was kept — so an operator's
   usage was proportional to its site count. Measured over 67 real proposals: InsertMax 32,
   Tweak 0. The search was drawing from a subset of the declared set, weighted by how easy each
   operator is to place. Selection now picks the operator FIRST and tries it at every position,
   so each declared operator gets an equal draw regardless of how many sites it has.

**TWO OF THE TEN DECLARED OPERATORS DO NOT EXIST — measured 2026-09-10, `tests/shape_reachability.rs`.**

The table above declares ten operators. `mutate::ALL_OPS` implements twelve names (eleven before 2026-09-10), which reads like
a superset and is not: `replace`/`insert` ship as the narrower `ReplaceConst`/`InsertMax`, three
memory operators were added later (`ProbeRead`, `StoreHere`, `WrapIfPred`), and the last two rows of
the table have no implementation under any spelling.

| declared | status | measured consequence |
|---|---|---|
| `add-arg` — "add an Int/Score parameter and thread a value at each call site" | **still absent** | no operator adds a FUNCTION PARAMETER. (`Op::TReadIndex`, added 2026-09-10, lengthens a tread's index list — a different move, see the correction below) |
| `add-fn` — "split a subtree into a new function and call it" | **absent** | **0 of 823** applied mutations changed `funcs.len()`, across all 10 reference programs |

**⚠ THE `add-fn` ROW ABOVE IS OUT OF DATE — IT WAS IMPLEMENTED THE SAME DAY, AND IS PARKED.
Corrected 2026-09-11.** The "absent / no implementation under any spelling" wording was committed at
`9a56364` (2026-09-10 09:38). `try_add_fn` landed eight hours later at `21d7d73` (17:56) —
`mutate.rs:485`, with `crates/grammar/tests/add_fn.rs` asserting it raises the function count,
scope-checks, type-checks, never lifts a `ret`, and applies somewhere. It threads the lifted
subtree's free variables as typed parameters, and it also SUBSUMES `add-arg` here: `add-arg` as
declared cannot apply to a 1-function program at all, because `check_program` pins the entry to
`choose(Pos, Int) -> Move`, so there is no function to add a parameter to until this operator makes
one.

**The CONSEQUENCE is unchanged; only the CAUSE is.** `Op::AddFn` sits in `mutate::PARKED_OPS`, not
in `ALL_OPS`, so it is never drawn: function count still never changes in a real run, and §4's
"the search is confined to one function forever" below remains true as written. The operator is not
missing — it is deliberately not dealt.

**REFINEMENT to the parking rationale, verified 2026-09-11 by reading the code path.** `mutate.rs:144`
justifies parking with "its parent plus a `Call` node, which `interp::cost_of` charges 2". The
charge is 2, but it is not paid once. `Interp::exec` does `self.cost += cost_of(n)` on EVERY NODE
VISIT (`crates/interp/src/lib.rs:637`), against a cost cap — cost is RUNTIME, not static program
size. So a lift costs **2 × the number of times the lifted site executes**, which for a site inside
alpha-beta's recursion is thousands, not 2. The park decision is therefore better supported than its
own comment claims, and it carries a corollary: if a lift is ever to be cheap, it must land on a
COLD site, and nothing in `try_add_fn` currently prefers one — `get_nth` picks by index.

This also kills an attractive-looking fix before anyone builds it: *lift a subtree that occurs k
times and replace all k sites with calls, so one body of N nodes replaces kN.* That arithmetic is
about STATIC SIZE and this cost model does not measure static size. Sharing a body does not reduce
how often it runs; it adds a call charge to every execution. A multi-site lift is strictly worse
than the single-site lift, not better. (Recorded as reasoning from the code, NOT as a measurement
on a program.) **Superseded the same day: lifts HAVE now been benchmarked — see below.**

**AND THE OTHER HALF OF THE PARKING RATIONALE WAS FALSE — measured 2026-09-11.** "AddFn is
behaviour-preserving by construction" was asserted in three places and tested in none. It lifted
`Set("best", Max(Var("best"), Var("vv")))` — alpha-beta's own score update — out of the seed and
dropped the write, because `Node::Call` discards the callee's frame and `free_vars` does not report
an assignment target as free (its comment says so explicitly, and that is right for what a site
REQUIRES and wrong for what a lift must THREAD OUT). The program stayed well-typed and well-scoped,
played a different move, and cost **91x LESS** — which under mates-per-cost makes a gutted candidate
look FITTER, not worse, so the safety argument for parking pointed the wrong way.

Fixed by a `contains_set` refusal mirroring `contains_ret`. Behaviour is now pinned by
`interp/tests/add_fn_is_behaviour_preserving.rs` (180 of 180 comparisons identical, all returning a
real move), the cost claim is measured at **≤ 1.004x** the parent rather than asserted as a bare
"+2", and coverage is intact (539 applications still). Full account, including the controls that
cleared the harness first, in `add_fn_drops_writes_RESULT.md`.

Two consequences follow, and both were previously open:

**CORRECTION 2026-09-10, same day, before anything was built on it.** The row above pairs the
missing `add-arg` with rung 7's missing `TRead/2`, and they are NOT the same operator. `add-arg`
adds a FUNCTION PARAMETER and threads a value at each call site. Rung 7 needs
`TRead(3, [d, i])`, where `d` is `ab`'s EXISTING depth parameter and `i` is an existing local —
no new parameter anywhere. What it needs is an operator that appends an IN-SCOPE Int expression
as a tread INDEX, which §2.7 already permits: `tread : Tab x Int... -> Int` is variadic, so this
is inside the Given column and needs no new primitive.

The measured fact is unchanged and is what the test asserts — *no operator lengthens any argument
list* — but the two gaps need two different operators, and implementing `add-arg` as declared
would NOT make rung 7 reachable. Recorded because the imprecise version was committed first.

**⚠ SHAPE-REACHABLE, BEHAVIOURALLY INERT, AND THEREFORE PARKED — `Op::TReadIndex`, 2026-09-10.**
Read the two points below as the diagnosis that motivated the operator. What changed, and what
did not:

| | before | after `Op::TReadIndex` |
|---|---|---|
| TRead arities constructible in ONE edit | `[]` | **`[1]`** |
| `TRead/2`, which rung 7 needs | unreachable at any edit count | **reachable in TWO edits — measured** by composing the operator with itself (but the operator is PARKED, see below) |
| does appending an index change BEHAVIOUR? | n/a | **no — measured inert**, `tables_nd` is never populated |
| function count changed by a mutation | 0 of 823 | **0 of 858 — still zero**. (`add-fn` was implemented later the same day and PARKED, so this stays zero in any real run — see the correction above. "remains absent" was the original wording and is superseded) |

The operator appends ONE in-scope Int as a tread index and never picks the table id, so reaching
`TRead(3, [d, i])` still costs two edits plus finding table 3. That is deliberate: MASTER_PLAN:53
requires the technique be DISCOVERED, and `mutate.rs:205` already refused an operator that emitted
probe-and-store together as making the discovery vacuous. It adds no primitive — §2.7 #26 declares
`tread : Tab x Int... -> Int` variadic already.

**AND IT IS NOT IN THE DRAWN SET.** `mutate::ALL_OPS` stays at eleven; the operator lives in
`mutate::PARKED_OPS`. The reason is measured, not cautious: `interp/tests/tread_index_is_inert.rs`
shows appending an index changes NOTHING. `Interp` resolves a TRead through `tables_nd` first and
falls back to the scalar `tables`; `tables_nd` is never populated and every `Interp::new` call site
passes at most three scalars, so the indices are never consulted. Every candidate the operator can
produce today is behaviourally its parent and strictly LARGER — and FITNESS 3 is mates per COST, so
each one is strictly worse while still consuming a `1/|ALL_OPS|` share of every draw. Enabling it
now would be a regression to the search wearing the costume of new capability.

This is the same shape as the `GATE_VETO` result earlier the same day: the mechanism fired exactly
as predicted, and firing was not evidence the change helps.

UNPARK WHEN both halves of point 1 below clear — `tables_nd` populated AND tables in the genome.
Nothing here claims the search would then climb rung 7 either: reachability was a PRIOR blocker and
the valley is a separate one; `reachability.rs`'s header records hash reuse as reachable since
2026-09-08 and still blocked by a conjunctive valley.

1. **Rung 7 of the ladder was UNREACHABLE, and this is how it was proven.** `table_reduction` needs `TRead(3, [d, i])`.
   The seed contains `TRead(0, [])` and `TRead(1, [])`, so the kind-granularity check in
   `reachability.rs` correctly reported "nothing missing" and recorded the rung as NOT PROVEN
   EITHER WAY. At shape granularity the missing element is exactly `("TRead", 2)` and no operator
   builds it. `reachability.rs:153` named this follow-up; this is it.
2. **The search is confined to one function forever.** `typecheck.rs:19` admits 1..4 functions, but
   neither mutation nor `crossover()` can change the count — crossover writes
   `out.funcs[rfi].body` and never pushes a func. Seeded with a 1-function program, three quarters
   of the declared program space is unreachable. Not unlikely: unreachable.
   **STILL TRUE OPERATIONALLY, for a changed reason (2026-09-11).** `try_add_fn` now exists and does
   push a func, so "neither mutation nor `crossover()` CAN change the count" is no longer true of the
   code. `Op::AddFn` is in `PARKED_OPS` and is never drawn, so the count still never changes in a
   run and three quarters of the space is still unreached. The barrier moved from *cannot* to
   *not dealt*, and the unpark condition is recorded at `mutate.rs:148`.

This is an EXPRESSIVENESS gap and it is narrow. `reachability.rs`'s header records that the last
claim of this shape was stale — hash reuse turned out to be reachable, and the barrier there is a
conjunctive fitness valley, not expressibility. Nothing here says the missing operators would make
the search succeed; it says the ladder's third rung and all multi-function programs are outside what
the current operator set can construct, which is a fact about the Given column rather than about
any run.

**⚠ THE OPERATORS COULD PRODUCE PROGRAMS WITH HOLES IN THEM — measured and FIXED 2026-09-10,
`tests/scope_escape.rs`.**

A program that reads a variable nothing binds used to type-check, run, and silently compute with
`Unit` in place of the missing value. Four fallbacks conspired to hide it:

```text
  typecheck  Var(name) => *env.get(name).unwrap_or(&Ty::Unit)      unbound -> Unit
  typecheck  want(got, expect) accepts got == Ty::Unit             Unit    -> fits anywhere
  typecheck  Foreach/Argmax/Sort/Sample insert their binder and NEVER remove it,
             so a loop variable stays live for the rest of the function
  interp     lookup(..).unwrap_or(Value::Unit)                     unbound -> Unit at RUNTIME
```

| | before | after `typecheck::scope_check` |
|---|---|---|
| `Op::WrapIfPred` applications that read an unbound var | **26 of 50 (52%)** | **0** |
| ...of those, how many passed `check_program` and reached the GATE | **26** | 0 |
| all operators, escapes / applications | 26 / 823 (3.2%) | **0 / 797** |
| reference programs affected | 0 of 10 | 0 of 10 |

`WrapIfPred` hardcodes `Var("m")` and `Var("p")` and never asks whether `m` is in scope at the wrap
site, so wrapping any statement outside a `Foreach(_, "m", _)` reads an unbound move. The `If`
condition then evaluates on `Unit` and is effectively constant, making the candidate either
inert-but-larger or silently statement-disabling — under FITNESS 3 (mates per COST) both are
guaranteed-worse. GRAMMAR 3 states the economics exactly: *"ill-typed candidates are discarded at
generation time (cheap) rather than at gate time (expensive)."* Those 26 were being paid for in
GAMES. The operator is not disabled — it still applies at its 24 legal sites.

**The first version of that measurement reported 0 escapes and was a BROKEN PROBE**, recorded because
the failure is instructive: it counted a name as bound if it appeared anywhere in the function as a
binder, and the reference programs reuse loop variables, so a subtree lifted OUT of the `Foreach`
binding `m` still looked bound whenever any other `Foreach` also bound `m` — precisely the case being
hunted. A positive control (`escape_is_detectable`) now guards the predicate.

**CONSEQUENCE FOR CROSSOVER, and it is a Given-column fact rather than a bug.** `reachability.rs`
previously measured crossover carrying `{"Max", "Set"}` from alpha-beta into UCT, and that assertion
PASSED. It was measuring holed programs. In alpha-beta those constructs exist only in terms of its own
accumulators — `Set("a", Max(Var("a"), Var("vv")))` — and UCT has no `a` and no `vv`. With scope
enforced the direction is **empty**, and selecting donors by free variables does not recover it: no
alpha-beta subtree containing `Max` or `Set` is closed over anything UCT supplies. The direction is
genuinely unreachable, not unluckily sampled.

What would fix it is named, and it is not crossover's job: moving an accumulator-shaped subtree across
lineages requires INTRODUCING the binding it reads — which is exactly the two declared-but-missing
operators above, `add-arg` and `add-fn`. The missing operators and the one-directional crossover are
the same gap seen from two sides.

## 5. Seeds
### 5.1 Purity lineage seed (depth-one), 8 nodes
```
choose(p, B) = argmax(moves(p), m -> neg(eval(apply(p, m))))
```
### 5.2 Main lineage seed (bare alpha-beta), 29 nodes
```
choose(p, B) = argmax(moves(p), m -> neg(ab(apply(p,m), D, neg(INF), INF)))
ab(p, d, a, b) =
  if (cmp(terminal(p), NONE, !=)) ret score_of(terminal(p), d)
  if (d == 0) ret eval(p)
  let best = neg(INF)    -- mutable via set
  foreach m in moves(p):
    let v = neg(call ab(apply(p,m), sub(d,1), neg(b), neg(a)))
    set best = max(best, v); set a = max(a, v)
    if (cmp(a, b, >=)) ret best
  ret best
```
D and INF are table reads (`tread`) — not constants — so even the search depth of the
seed is a tuned value, not a given. Children are visited in `moves` emission order,
which is shuffled by the engine (declared: ordering carries no opinion at the seed).

## 6. Reference programs and THE DECLARED PRIOR
Node counts are of the well-typed tree, counting every primitive and variable reference
once; lambdas count as one node plus their body. **STATUS: MEASURED by the parser.** Every
number in this section is produced by `cargo run --release --example prior -p grammar`.
(This line read "hand estimates, +/-20%, the absolute skew is not yet exact" while the table
one row below was already labelled MEASURED and the text below it said "no sketches remain" —
the document contradicted itself inside the paragraph that states the project's central claim.
The grammar was implemented and the counts replaced; the status line was not updated with it.)

RE-DERIVED FROM THE PARSER 2026-09-08 (`cargo run --release --example prior -p grammar`). The
table previously listed FIVE of the nine programs the parser measures; the other four were quoted
only as prose in section 9, where they could drift from the counter without anything failing.
Every row below is printed by that command.

| Program (named EXACTLY as `reference::all()` returns it — see note below) | Nodes (MEASURED) | vs seed | Fidelity |
|---|---|---|---|
| depth-one (purity seed) | 9 | −62 | faithful (purity seed) |
| bare alpha-beta (main seed) | **71** | +0 | faithful (main seed) |
| capture extension (rung 6) | 84 | +13 | faithful |
| extend-by-uncertainty (yardstick a) | 85 | +14 | faithful |
| mix-backup (yardstick b) | 78 | +7 | faithful |
| bound-gap stopping (yardstick c) | 127 | +56 | faithful |
| table reduction (rung 7) | 86 | +15 | faithful |
| alpha-beta + iterative deepening | 100 | +29 | faithful |
| UCT-style MCTS | **131** | **+60** | **faithful — VERIFIED BY EXECUTION, 23/23 forced mates** at K >= 600 |
| UCT-style MCTS (blend selection, historical) | 132 | +61 | PARTIAL — 20/23 at best, and it CANNOT reach 23/23 at any weight. See below |
| alpha-beta + hash reuse | **175** | +104 | faithful (validity marker, depth, EXACT/LOWER/UPPER bounds) |
| proof-number search | **175** | +104 | faithful — VERIFIED BY EXECUTION, 23/23 forced mates |
| alpha-beta + hash + ID | 204 | +133 | faithful |

**The names in the first column are the literal strings `reference::all()` returns, and
`crates/grammar/tests/prior_table.rs` asserts EXACT equality between this table and the parser.**
That is deliberate and slightly ugly. On 2026-09-10 this table said `capture extension (rung 6) = 80,
+9` while the parser printed **84, +13**: commit `064112f` ("rung 6 fixed to extend AT THE HORIZON")
grew the program by 4 nodes on 09-08 20:29, and the row -- last written 09-08 17:08 -- never followed.
Nine of the ten rows were correct, which is exactly why nobody looked.

This section had already been bitten once and had already tried to fix it: four programs used to be
quoted only as prose in section 9, "where they could drift from the counter without anything
failing", and the fix was to move them into this table. That RELOCATED the drift without gating it --
`cargo run --example prior` printing the truth is worthless if nothing compares its output to the
claim. The test now does the comparison, and it fails loudly if it parses fewer rows than there are
programs, so a heading rename cannot make it pass vacuously.

Matching is exact string equality rather than an alias map, because a hand-kept list of names is the
defect that bit this workspace three separate ways on the same day (a hardcoded arm-name alternation
in `tick.sh` that missed a probe; a whitelist grep that dropped four alarms and had to be deleted; a
`pgrep -x evolve` enumerator that reported 5 arms while 6 ran, because one had been renamed). A
rename in the code must FORCE an edit here.

**The correction does not touch the conclusion.** At +13 the capture extension is still far nearer
the seed than either rival paradigm (+60 MCTS, +104 PN), so the skew direction and its rough factor
of two are unchanged. A second test, `every_alpha_beta_variant_is_nearer_the_seed_than_either_rival_paradigm`,
now asserts that ordering directly -- row-by-row count agreement would not have caught an inversion.

**THE SKEW IS NOW RESOLVED, AND IT IS TOWARD ALPHA-BETA.** This section recorded the direction as
UNRESOLVED for a specific and correct reason: MCTS and PN were SKETCHES, so their counts were
lower bounds and a lower bound cannot establish which paradigm the grammar favours. Both are now
written out in full and executed — PN solves 23/23 forced mates, and UCT 23/23 at K >= 600 — so the
counts are real lengths rather than floors, and the comparison is meaningful for the first time:

* the seed alpha-beta is **71** nodes; MCTS is **+60** and PN is **+104** from it.
* every alpha-beta VARIANT is nearer the seed (+13, +15, +29) than either rival paradigm is.

> **Two corrections to this paragraph, 2026-09-10.** It said "+59" and "MCTS 20/23", and both
> disagreed with the table twenty lines above it — the table that `grammar/tests/prior_table.rs`
> exists to keep honest. The node count is settled by arithmetic on gated values: the table records
> UCT at **131** and the seed at **71**, so the distance is **+60**, and 131 − 71 is not a matter of
> opinion. The fidelity figure was stale rather than wrong-at-the-time: "20/23" is the `slot 2 = 1`
> row of the sweep below, and the section headed *RESOLVED 2026-09-09 — UCT reaches 23/23* closes
> that status explicitly. **The conclusion is untouched** — +60 against +13/+15/+29 is the same skew,
> the same factor of two, in the same direction.
>
> Worth naming the shape of it: `prior_table.rs` gates the TABLE's node counts, and the numbers that
> drifted were the ones re-typed into PROSE beside it. A test that pins a table does not pin the
> sentences quoting it, which is the same class of gap as a monitor whose output nobody reads.

So the grammar is biased toward alpha-beta-shaped programs, by roughly a factor of two in edit
distance. That is a real cost of this Given column and it is now a measured number rather than an
open question. It does NOT say MCTS is unreachable — it says a search starting from this seed
must cross ~59 nodes of neutral or worse territory to get there, which the valley result below
(`ladder_valley_RESULT.md`) shows is exactly the kind of distance a strict hill climb cannot
cross.

**WHY MCTS IS PARTIAL, EXPLAINED 2026-09-08 — the exploration term saturates at 2.**

The fidelity note above records UCT as PARTIAL (20/23 forced mates) without saying why, and the
budget sweep produced an anomaly that demanded an answer: mates FALL as the budget rises.
Measured (`evolve mctsbudget`, 25 positions, depth 3): 1 mate at budget 16, 6 at 64, **12 at 256**,
then **10 at 1024 and 10 at 4096**. More playouts making UCT worse is not what UCT does.

Read off the interpreter's arithmetic rather than guessed:

* `Log` is `ln` TRUNCATED to an integer (`(a.max(1) as f64).ln() as i64`), so `Log(256) = 5`,
  `Log(1024) = 6`, `Log(4096) = 8`.
* `Div` is integer division. The exploration term is
  `u = Sqrt(Div(Log(visits(parent)), visits(child) + 1))`, so once a child has more than `Log(N)`
  visits the quotient is **0** and `u` is **0 — permanently, for that child**.
* `Sqrt` is truncated too, and `Div <= 8` at these budgets, so **`u` can only ever be 0, 1 or 2.**

Meanwhile `q` is a SCORE: mate is +/-29936, so `q` spans roughly +/-30000. Selection is
`Mix(q, u, c) = (q*c + u*(16-c))/16` with the declared `c = 8`, i.e. `(q + u)/2`. **An exploration
bonus of at most 2 against an exploitation term of +/-30000 is numerically inert** — it breaks ties
near zero and nothing else. And `Mix` is a CONVEX BLEND, not a scaling, so `c` cannot amplify `u`:
`c = 0` gives pure exploration over `u` in {0,1,2}, which is almost all ties, and `c = 16` gives
pure greed. No setting makes the two commensurable.

That is the mechanism behind the anomaly. At budget 256 some children still have fewer than
`Log(N)` visits and exploration still operates; by 1024 and 4096 essentially every child is past
the threshold, `u` is 0 everywhere, and the search is greedy over averages from a weak eval —
which is worse than the partially-exploring version. The decline is the expected consequence, not
noise.

**THIS IS A DEFECT OF THE REFERENCE PROGRAM, NOT PROOF THAT THE GRAMMAR CANNOT EXPRESS UCT**, and
the distinction matters because the second is a claim about the Given column. `arith` includes
`mul` (Section 2.4), so a program CAN scale the bonus to the value range — `uct_mcts` as written
simply does not, and no operator would ever discover that it should, because the reference is the
declared seed rather than something the search produced. Rewriting it to scale `u` would change
the declared prior's node count and is therefore a deliberate, recorded act, not a silent fix.

**CONSEQUENCE FOR THE MCTS LINEAGE**, which is seeded with exactly this program: its mate guard is
`f >= 12`, and 12/25 is the score of a near-greedy searcher, not of UCT. Any claim that the lineage
"discovered" something must be read against that, and any hybrid built on it inherits an
exploration term that cannot see past 2.

**RESOLVED 2026-09-09 — UCT reaches 23/23. The blend was inverting the exploration term.**

The "PARTIAL" status above is closed, and the *reason* recorded for it was wrong.

`uct_mcts` reads table slot 2 **twice**: once to scale the exploration term inside the sqrt, and
once as the weight of `Mix(q, u, c)`, which `interp/src/lib.rs:696` computes as
`(q*c + u*(16-c))/16`. The two uses fight. Raising the slot enlarges `u` inside the sqrt while
shrinking `u`'s blend coefficient `(16 - c)` toward zero — and past it. Swept at budget 256 over the
23 mate-in-one positions:

| slot 2 | coefficient on `u` | mates |
|---|---|---|
| 1 | +15 | **20/23** |
| 4 | +12 | 18/23 |
| 8 | +8 | 15/23 |
| 16 | **0** | 14/23 |
| 24 | −8 | 12/23 |
| 64 | −48 | 9/23 |
| 360000 | −359984 | **0/23** |

Monotone, crossing the greedy baseline exactly where the coefficient reaches zero.

**This corrects the explanation on record.** The K = 360,000 result was read above as "exploration
swamps exploitation so the search never exploits". It is the opposite: at that weight the
coefficient on `u` is hugely negative, so the program is *penalised* for exploring. Proof by
substitution — at the **same** K = 360,000, selecting on `q + u` instead of `Mix` scores **23/23**
rather than 0/23. Magnitude was never the problem.

**And the derivation that was discarded was correct.** `interp/src/lib.rs` records C = one eval unit
= 600 as "REFUTED by measurement". The sum-selection encoding first reaches 23/23 at exactly
**K = 600**, the net's declared eval scale, and holds it at 4096 and 360000. That derivation was
right in form *and* in magnitude; it was defeated by the convex blend it was fed through.

`uct_mcts_sum` (`reference.rs`) selects `argmax(q + u)`. It introduces no primitive — `arith add` is
already declared in 2.4 — and it is **131 nodes, one FEWER than the Mix form's 132**, because `Mix`
takes three children and `Add` takes two. Both encodings are kept and counted: the declared program
is unchanged, so GRAMMAR 6's recorded count still refers to a program that exists.

**THE DECLARED PROGRAM IS NOW THE SUM ENCODING (2026-09-09), and the prior is +60.** This is the deliberate recorded change this section previously said such a swap would require. `uct_mcts()` selects `argmax(q + u)`; the blend form is kept as `uct_mcts_mix()` and still counted at 132, so every ladder distance measured against it still refers to a program that exists.

The evidence for making it the default rather than an alternative: it solves 23/23 forced mates where the blend form's ceiling is 20/23 at ANY weight; at matched cost (~1.0x bare alpha-beta) on the 25-position mate set it scores **17/25 against 11/25**; and it is one node CHEAPER. The MCTS lineage seed and `budget_mcts` (512 -> 1024, the measured cost-parity point) were changed with it.

**Consequence for the MCTS lineage**, which is seeded with the Mix form at slot 2 = 8: its seed
scores 15/23, and the ceiling of that encoding is 20/23 at slot 2 = 1. Any lineage claim must be read
against a seed whose exploration term is partly cancelled by its own blend weight.

**CORRECTION 2026-09-08 — "faithful" meant COUNTED AND READ, and for PN it was wrong.**

`uct_mcts` and `proof_number` were referenced exactly once each in the whole tree, from
`reference::all()`, which examples/prior.rs uses to COUNT NODES. Nothing had ever executed them.
That is the same state `alpha-beta + hash reuse` was in when it was labelled faithful while
returning the constant 0 — and it was caught the same way, by running it
(`crates/interp/examples/reference_audit.rs`, judged against bare alpha-beta as a control on the
same positions at the same depth).

**MCTS DID NOT PASS, AND MY FIRST VERDICT SAYING IT DID WAS WRONG.** I held the two programs to
different bars: PN had to SOLVE forced mates, while MCTS only had to return a legal move, call
`eval`, and spend its budget. It passed that weaker bar and I wrote "faithful — VERIFIED BY
EXECUTION" into this table on the strength of it. The ladder sweep had independently reported MCTS
finding **0 mates out of 120** the day before, which I had not reconciled. Applying PN's bar:
**0/23 forced mates at budgets 64, 256 AND 1024**, and only 2 distinct moves returned across 23
different positions. Two defects, both now fixed:

* **No first-play urgency.** The interpreter is integer arithmetic and `Div` by zero returns 0, so
  an unvisited child computed `u = Sqrt(Div(Log(N), 0)) = 0` and `q = Avg(sum, 0) = 0`. An
  unexplored child scored the LOWEST possible value where UCT requires the highest, so the search
  locked onto the first child it expanded. Denominator is now `visits(child) + 1`.
* **The Q term had its sign inverted.** Backprop stores `val = Neg(simulate(child))`, so a node's
  `sum` is in its OWN mover's frame; reading a child's sum from the parent needs another flip and
  did not get one. A checkmate child stores `ScoreOf(Loss,0) = -29936` — exactly what the parent
  wants — and un-negated that reads as the single most repellent move on the board.

After both: **20/23**, first-move 3/23, 19 distinct moves. Better by every measure and still short
of the bar, so this entry says PARTIAL rather than faithful. The shortfall is neither budget (20/23
at 256 AND at 1024) nor the learned exploration weight (swept: k=1 gives 20/23, k=16 gives 14/23,
so the audit's k=1 is already the best of those tested) — it is a real remaining limitation.

Both MCTS defects are the same root cause as PN's: an unvisited slot is `Slot::default()`, all
zeros, and nothing distinguishes "no data" from a real value. For PN zero meant PROVEN WIN (the
best); for MCTS it means worthless (the worst). Same missing distinction, opposite directions,
three reference programs affected counting hash-reuse.

**Proof-number search was degenerate:** it returned the FIRST legal move on 23/23
mate-in-one positions, found 0/23, and agreed with alpha-beta 0/23, while the control found 23/23.
Three separate defects, each measured:

1. **Unvisited nodes read as PROVEN.** `proof(n)` was a bare `Probe(Key(n)).Score`; an unvisited
   slot is `Slot::default()`, all zeros, and proof==0 means proven win. Every unexplored child
   looked already proven. Fixed with the `Flag` validity marker: `effective = stored + (1 - flag)`.
2. **No expansion step, so every iteration hit the recursion ceiling.** Real PN descends to the
   most-proving LEAF and expands one ply; this recursed until terminal. Measured 3840 ceiling hits
   over 60 positions at budget 64 — exactly 60 x 64, every iteration. At the ceiling `Call` returns
   `Num(0)`, documented as "a neutral value", and for proof numbers 0 means PROVEN WIN, so each
   iteration asserted the line was won and the back-up carried it to the root. (`Interp::ceiling_hits`
   was added to make that visible; alpha-beta hits the ceiling too and survives, because for it 0 is
   a score and not a claim.)
3. **No AND/OR alternation, and a mate SCORE stored as a proof NUMBER.** The back-up took min/sum of
   the child's PROOF for both quantities, treating every node as an OR node, so an expanded child's
   proof stayed 1 forever and the descent never left child_0. And the terminal branch stored
   `ScoreOf(Terminal(p), 0)` = **-29936**, a mate score where a proof count belongs — and a negative
   proof number is always the minimum. Rewritten in negamax form, which needs no AND/OR flag:
   `pn(n) = min over children of dn(child)`, `dn(n) = sum over children of pn(child)`, terminal =
   (pn INF, dn 0), expansion sets dn to the CHILD COUNT so an expanded node stops being attractive.

After the rewrite: **23/23 forced mates, 0/23 first-move, 23/23 agreement with alpha-beta, 0 evals**
(zero is correct — PN is terminal-driven, which is exactly why FITNESS 3 denominates mates-per-COST
rather than per-evaluation). At budget 64 it scores 21/23 and at 256 it scores 23/23, so the
shortfall at 64 is RESOURCE, not a defect — checked by sweeping the budget rather than assuming.

The count moved 83 -> 175 and the PN distance with it, +70 -> **+104**. The old number counted a
program that could not prove a mate in one, so it was never a PN distance in the first place.

MCTS is now 131 nodes (+60), PN 175 (+104).

<!-- 2026-09-10: this line read "130 nodes (+59)". The gated table in GRAMMAR 6 records 131 against a
     seed of 71, and `grammar/tests/prior_table.rs` asserts that count against `reference::all()`, so
     +60 is the value with a test behind it. Corrected here rather than in the table. -->


**All entries measured by `crates/grammar` (examples/prior.rs) at EQUAL FIDELITY.** Run it to
reproduce.

**CORRECTION 2026-09-07 — the hash-reuse entry was 89 and was not a transposition table.**
The encoded program was `if probe(p).depth >= d: ret probe(p).score` with a store that wrote
only `Score`. `Depth` was never written, so every slot held `depth = 0` — and so does an EMPTY
slot, since `Slot::default()` is all zeros. The guard therefore read `0 >= d`, true at every
leaf, and the program returned an empty slot's `score`, i.e. the constant **0**, without ever
calling `eval`. Worse, `Node::seq` desugars to nested `Let("_", stmt, rest)`, so the body's
tail `ret best` unwound straight past the trailing store: the table was never written at all.

Measured before the repair (`crates/interp/examples/tt_pressure.rs`, 60 random positions):

| depth | evals (hash) | evals (bare) | cost ratio | agrees with bare |
|---|---|---|---|---|
| 2 | **0** | 516,829 | 0.285x | 0/60 |
| 3 | **0** | 7,206,527 | 0.218x | 2/60 |
| 4 | **0** | 66,932,291 | 0.072x | 1/60 |

Zero evaluations at every depth. The apparent "14x cheaper than alpha-beta" was a search that
had stopped searching — the degenerate solution FITNESS 10 lists first, except it returns a
constant rather than an eval, and it was sitting in the REFERENCE SET used to calibrate the
prior, labelled `faithful`, under a line reading "no sketches remain".

After the repair: agrees 60/60 at every depth (a sound TT does not change alpha-beta's value),
and it now earns its keep in evaluations — 59,347,691 vs 66,932,291 at depth 4, **-11.3%**.

## DECLARED PRIMITIVES THAT DO NOT DO WHAT THIS DOCUMENT SAYS — measured 2026-09-08

Three defects, found by pulling one thread: capture extension "played identically to the seed and
cost 1.6% more", which is impossible for an extension that re-searches captures. All three were
INVISIBLE while the first one stood, because a dead branch masks everything downstream of it.

### 1. `pred` (primitive #5) was a stub — FIXED

`interp/src/lib.rs` read `Node::Pred(..) => Value::Bool(false)`. Every predicate always false.

Measured consequence: `capture_extension` had a BYTE-IDENTICAL eval count to the seed
(4,127,466 both). It was the seed plus a dead branch costing 1.6% — while section 6 called it
"faithful". `Op::WrapIfPred` was also a disguised DELETE, since wrapping a statement in `if false`
removes it.

Now implemented for the three PredIds with unambiguous Boolean readings (`is_capture`,
`is_promotion`, `gives_check`), strictly from move flags and a board query. The other four
(`captured_type`, `moving_type`, `from_square`, `to_square`) name quantities that are NOT booleans
while the signature says Bool; any reading of them would smuggle in the chess knowledge section 2.1
forbids, so they remain false and the SPEC GAP is recorded rather than papered over.

### 2. `capture_extension` contradicts rung 6's own wording — NOT FIXED

Section 9 rung 6 says "capture extension **at horizon**". The reference applies `if is_capture:
nd = d` inside the move loop at EVERY depth, so captures are free throughout the tree — not
quiescence, but full-width search with captures unbounded.

Measured once `pred` worked, 3 positions at depth 3:

| program | evals | cost | agreement |
|---|---|---|---|
| bare alpha-beta | 441,471 | 1.000x | 3/3 |
| **capture extension** | **24,440,705** | **73.363x** | 1/3, and 2 searches hit the cost cap |

**73x, and it truncates.** The 0.985x recorded for this rung in the ladder was the cost of a dead
branch; the real figure is 73x and unusable. The rung needs re-encoding to match its own
specification before any ladder number for it means anything.

### 3. `tread` (primitive #26) ignored its index arguments — MECHANISM FIXED, CONTENTS UNDECLARED

Declared as "read a learned integer table **by index features**". Implemented as
`Node::TRead(i, _) => tables.get(i)` — the arguments are DISCARDED. So `TRead(3, [d, i])` returns
one scalar regardless of depth or move index, and table-driven reduction (rung 7) cannot vary by
anything. It is doubly dead: the harness also passes only three tables, so index 3 is out of range
and returns 0.

Measured: `table_reduction` still has a byte-identical eval count to the seed (441,471) at 1.007x —
a no-op costing 0.7%, which the `pred` fix does not touch because this is a different primitive.

**The MECHANISM is now implemented.** `Interp` carries `tables_nd: Vec<NdTable>` — genuinely
indexed tables with `dims` and row-major `data` — checked before the scalar `tables`, so every
existing call site is unchanged and `TRead(i, args)` folds its arguments into an index. Out-of-range
features CLAMP rather than wrap: a program reading past the edge should get the edge, not a wrapped
unrelated entry, which is exactly the silent-correctness trap this file has produced three times.

**THE TABLE CONTENTS REMAIN AN UNDECLARED GIVEN, and that is where rung 7 stops.** GRAMMAR 2.7 says
table contents are "SPSA-tuned, never written by programs", so *something* must declare what R[d, i]
holds before rung 7 can be measured at all. And the obvious filling — "reduce later moves more" — is
late move reduction, which is precisely the technique MASTER_PLAN requires to be DISCOVERED rather
than supplied. Writing my own intuition into R and then measuring "the search found a reduction
schedule" would be circular.

So rung 7 is not measurable today, and the reason is a genuine gap in the Given column rather than a
missing implementation. Recorded rather than resolved by inventing a table.

### Why this matters beyond the three fixes

`search_track_WHY_NOTHING.md` concluded that the only class of candidate that CAN improve is the
inexact variants — extensions and reductions — and measured both as no-ops. Both were no-ops
because the primitives underneath them were missing or mis-specified, not because of alpha-beta's
exactness. **The class was not empty by mathematics; it was empty by omission.** The uniformity
measurements stand as measurements; their explanation was incomplete.

## 7. Fitness interface (details in FITNESS.md)
A program exposes `choose`. Exactness is NOT program-settable. The compiler derives a
static taint per returned Score: a score is **exact** iff it provably came through
`max`/`min` over the FULL child list of `moves(p)` at every level of the returning path
(no `ret` inside the `foreach` before the list is exhausted, no `avg`/`mix`/`sample`/
`tread` on the path, no depth reduction relative to the declared depth argument).
Anything else is **inexact**. The taint is computed once per program from the tree and
cannot be influenced by runtime control flow. (If the flag were program-settable,
evolution's optimal policy would be to never claim exactness, disabling the strong
check — the same class of exploit the oracle exists to catch. Added to 10.5.)

The correctness oracle (FITNESS.md) compares programs against a reference full-width
search on a fixed position set: exact scores must match the reference exactly; inexact
scores must not be provably impossible (e.g., a claimed forced win where the reference
finds none within the searched depth; a score outside [LOSS, WIN] bounds; a returned
move that is not in `moves(p)`). Mates-per-cost uses `terminal`-derived outcomes only.

## 8. Cost model and compilation
- Cost model: a declared per-primitive cost (cycles estimate) table. The interpreter
  accumulates it at runtime; that running sum IS the budget unit (`budget`, #12).
- The static per-node cost cap (3x the seed) that earlier drafts used is REMOVED. Under a
  cost-unit budget an expensive-per-step program simply takes fewer steps in the same
  budget, and the time-based gate charges for the rest — the cap was redundant, and it
  was a declared bias against per-simulation paradigms (10.4). Removing it deletes a bias
  rather than renaming a metric. What remains: a sanity ceiling — a candidate must
  complete `choose` on a fixed smoke-test position within budget or it is rejected
  (catches runaway programs, not paradigms).
- Hard runtime ceilings: recursion depth 128; total cost units per `choose` = budget; wall
  time enforced by the harness.
- **ACCEPTANCE MEASUREMENT: PASSED at 0.98-0.99x (line is 0.50), by the tree-walker alone.**
  Measured by `crates/interp` (examples/bench_interp.rs) at depths 3 and 4, with an
  EQUIVALENCE CHECK that both arms evaluated the same number of leaves (2099 vs 2099;
  19675 vs 19675) before any ratio is believed.
  Consequence: the register bytecode of CRATE 4 is NOT needed to clear this gate and can be
  deferred. `Net::eval` dominates both arms identically (~10k nps either way, a dense forward
  pass with no incremental accumulator), so interpretation overhead is near-free at the
  current eval cost. Re-measure when the eval gets fast: the ratio only becomes informative
  once eval stops dominating.
  **An earlier reading of 0.14x was a HARNESS BUG, not a result** -- three stacked errors:
  (1) the seed's `choose` expands the root itself, so passing D=depth searched one ply deeper
  than the reference; (2) a `Let` written as a statement in the encoded seed scoped only over
  its own placeholder, so the `best` accumulator was invisible to the loop and read 0;
  (3) the reference narrowed alpha across root moves while the seed gives each a full window.
  Two "fixes" aimed at the interpreter (removing string-keyed env lookup, making Position
  refcounted) changed the number by nothing, which is what exposed the harness as the culprit.
  The eval-count check is now permanent so a ratio can never again be printed for two
  different trees.
**THE DEFERRAL NEEDS REVISITING, for a reason it did not consider (2026-09-08).** The bytecode
was deferred because the RATIO is fine — the tree-walker runs at 0.98x of hand-written Rust, so
interpretation overhead is nearly free. That is still true and it is the wrong question for the
search track, which is gated by ABSOLUTE throughput.

Measured cost of the seed's own full search, per position, under the per-primitive cost model:

| depth | cost units | wall time at ~14M units/s |
|---|---|---|
| 1 | 3.15M | 0.22 s |
| 2 | 37.3M | 2.7 s |
| 3 | 411M | 29 s |

A game-based PROGRAM gate needs both sides to complete their search at every move. At depth 2
that is ~2.7s per move, so one 160-ply pair costs ~14 minutes and a single candidate screened
over 4 pairs costs an hour. The search track is therefore affordable only at DEPTH 1 — and
depth 1 is precisely where search technique does not matter, because alpha-beta's advantages
(cutoffs, ordering, transpositions) all appear at depth >= 2.

So the honest statement is: **the current interpreter cannot afford to gate search programs by
games at a depth where the thing being searched for exists.** That is a throughput argument for
CRATE 4, and it is independent of the 0.98x ratio that deferred it. The alternative is to lean
on the mates-per-cost surrogate (FITNESS 3) as the primary PROGRAM signal, with games as a
confirmation for the few candidates that clear it — which is what the track does today, and
which should be recorded as a limitation rather than a design choice.

**RE-MEASURED 2026-09-08, after the eval got fast — and it ANSWERS the question against my own
argument above.** That note said the deferral needed revisiting because the search track is
gated by absolute throughput rather than by the ratio. It does not, and the reason is that the
ratio did not move:

    hand-written        3740 nodes    0.029s
    interpreted     11731918 cost     0.030s
    EQUIVALENCE     evals 3320 vs 3320 -- same tree
    ratio           0.971x   (was 0.98-0.99x when eval dominated)

The interpreter is still at parity with hand-written Rust AFTER the incremental accumulator
made eval ~8x cheaper at width >= 64. So interpretation overhead was never the thing hiding
behind eval cost: a register bytecode would buy at most ~3%, and the search track's expense is
the SEARCH ITSELF — evals, movegen, make/unmake — which a compiled program pays identically.

CRATE 4 therefore stays deferred, now for a measured reason rather than an untested one. The
route to affordable depth-2 game gating is a cheaper search, not a cheaper interpreter.

- Compilation target: a register-based bytecode with a Rust interpreter. Acceptance
  criterion for the interpreter design: the compiled main seed runs at >= 50% of the NPS
  of a hand-written Rust bare alpha-beta with the same net. If not met, the grammar
  design is revisited before P0 proceeds.

  Elo per doubling of speed — the constant that prices interpreter overhead. Derivation
  inlined so no private document is needed:
  - 2-player literature at normal TC: ~50-90 Elo/doubling.
  - Author's 4PC engine, threat-input builds gated at movetime 0.15s, back-solving
    (Elo cost) / (log2 of speed ratio), holding the fixed-node quality delta constant:
      old build: 0.329x speed = 1.60 doublings, cost 219 Elo -> 137 Elo/doubling (48 pairs)
      new build: 0.647x speed = 0.63 doublings, cost 107 Elo -> 170 Elo/doubling (148 pairs)
    The source (the 4PC engine's threat_v2/README.md) notes the combined ~176 "is high
    enough that it is probably an artefact of the 48-pair sample". Later internal notes
    cite 176 as measured without that caveat; the better-supported single measurement is
    170 from 148 pairs, and the author's own flag applies to the whole cluster.
  - Caveats carried from the source: one TC (0.15s), one engine, one variant, and the
    assumption that the fixed-node eval delta transfers unchanged to the shallower depths
    reachable in 0.15s. Decisive-heavy 4PC games also inflate Elo per game relative to
    2-player chess.
  - Honest bracket for Existence, which gates at fixed time: **50-170**. The interpreter
    argument holds at every point in it (at 50 it is ~3.4x weaker than at 170). A dedicated
    calibration — same engine, same net, halved movetime — is the first measurement to
    take once the seed runs, and it replaces this bracket in the ledger.
  Interpreter overhead is the cheapest way to lose the project.
- Determinism: fixed RNG seeds per game for `sample` and for `moves` shuffling; all
  candidates reproducible from (program, tables, net, seed).

## 9. The ladder check (run offline before any compute is spent)
Using the author's existing net as the test eval (methodology only), verify each step
is expressible, is 1-3 mutations from the previous, and beats it on mates-per-cost
and/or fixed-time games:
1. depth-one (**9**, measured)
2. depth-two: wrap-loop + add-arg + call -> minimax without bounds
3. add window args + set accumulators + wrap-if(cmp(a,b,>=)) ret -> bare alpha-beta (**71**)
4. probe before recursing, store after -> hash reuse (**89**)
5. loop over depth in choose -> iterative deepening
6. wrap-if(pred(m,p,is_capture)) around depth check -> capture extension at horizon
7. tread(reduction, depth, index) in the recursive depth -> table-driven reduction

Counts in bold are MEASURED by `crates/grammar`; the unbolded rungs are not yet written out.

**Rung 5 (iterative deepening) and rung 6 (capture extension) are now WRITTEN and MEASURED**
(`examples/prior`): `alpha-beta + iterative deepening` = **100** nodes, `alpha-beta + hash + ID`
= **204**, `capture extension` = **80** (+9 from the seed, inside GRAMMAR 4's 1-3 mutation
budget). Rung 7 (table-driven reduction) is **WRITTEN BUT INERT** — corrected 2026-09-10, it was described here as unwritten. `reference::table_reduction()` exists (`reference.rs:76`) and is locked by `crates/interp/tests/rung7_table.rs`. It does not work, and the reason is specific: `Interp` checks `tables_nd` (genuinely indexed tables) before `tables` (scalars), and repo-wide `tables_nd` appears four times — the declaration, an init to `Vec::new()` (`lib.rs:519`), a comment, and the read. **Nothing populates it.** So `TRead(3, [d, i])` misses `tables_nd` and falls through to `tables.get(3)`, and every caller passes a THREE-element vector (`vec![depth, 32_000, uct_exploration()]`), so index 3 is absent and the read yields `0`. With `R = 0` the reduced depth `max(d - 1 - R, 0)` is just `d - 1`: the rung makes exactly the seed's moves while carrying the extra move-counter nodes it declares, so on this ladder it reads as **strictly worse than the seed** — and "table-driven reduction does not help" would be a measurement of an unpopulated table, not of the idea.

What it needs is NOT hand-written LMR values: this section's own wording is that `R` is "a LEARNED table" whose contents "are meant to be searched, so that 'reduce late moves more' is something the loop DISCOVERS rather than a rule written in by hand". So the missing piece is a table 3 that EXISTS and is REACHABLE BY MUTATION, not one filled in by me. `rung7_table.rs::plays_identically_to_the_seed` is the canary and MUST START FAILING when that lands; a rung indistinguishable from the seed is inert by definition.

**And that missing piece is bigger than a population step — measured 2026-09-10.** `R` is not merely
unpopulated; it is **outside the search space entirely**, for two independent reasons:

1. **Tables are not in the genome.** `Program { funcs, lineage }` (`ast.rs:173`) has no table field.
   Tables arrive as a CONSTRUCTOR ARGUMENT — `Interp::new(net, tables: Vec<i64>)` (`lib.rs:511`) —
   supplied by whoever runs the program. The evolution loop mutates `Program`s, so **no mutation can
   ever change a table's contents.** Across every `Interp::new` call site in the repo the widest
   vector passed is three entries; not one passes a fourth.
2. **`TRead` itself is unreachable.** `tests/reachability.rs` already records that "no operator
   constructs a TRead or appends an argument to one", so even the READ could not be introduced by
   the operator set.

So GRAMMAR 9's requirement that "reduce late moves more" be "something the loop DISCOVERS rather than
a rule written in by hand" is **not satisfiable as the system is currently structured**, and no
amount of running the search will satisfy it. Making rung 7 discoverable is an ARCHITECTURAL change —
tables would have to become part of `Program` (a genome extension, with its own mutation operators)
rather than a runtime parameter. Hand-filling `R` would produce a working rung and a vacuous
discovery claim, which is the trade MASTER_PLAN line 53 exists to forbid.

**Not claimed:** that the genome extension is the right call, or worth its cost. Only that the
current gap is architectural rather than a missing initialiser, which is what the previous wording
("still unwritten") and the test's own framing ("nothing populates it") both understate.

The capture-extension rung is the expressibility check MASTER_PLAN item 2 asks for -- "verify
alpha-beta, hash reuse, ID, and qsearch are expressible in the grammar", using the test eval
"for this rig only". It is NOT seeding qsearch: MASTER_PLAN line 53 requires the SEED to hold
"no quiescence ... all of these must be DISCOVERED as program edits that beat the current
program on the clock", and `bare_alpha_beta()` is untouched -- still exactly **71** nodes, which
the same run re-measures every time. The reference set is read by the ladder rig alone; neither
the search nor the evolution loop imports it.
The original parenthetical estimates (8/18/29/41/52/61/72) were the same hand guesses that
measured 2.4x wrong elsewhere in this document and have been removed rather than corrected.

> **SUPERSEDED 2026-09-08 — READ THE RESOLUTION AT THE END OF THIS SECTION BEFORE THIS TABLE.**
> Everything from here to "CONSEQUENCE" is the FLAT-COST-MODEL era and its conclusion ("step 4 is
> not a rung", "a 25% LOSS") no longer holds. Two instrument defects and one real engineering
> change were found afterwards: the flat model charged a full NNUE forward pass the same as
> `const 3`, and `key` cost 97 units because the Zobrist hash was rebuilt from scratch at every
> probe. Under the measured per-primitive cost model with an incrementally-maintained key, hash
> reuse is **0.98x the seed at D=3 — a GAIN**. The cost scales are not comparable either (61.8M
> here vs 50.9 BILLION there), so the two tables cannot be read against each other.
>
> It is kept, not deleted, because it is the honest record of what the engine really cost at the
> time, and because the reasoning below it — a TT needs repeated visits to pay for itself — is
> what motivated the ID rungs and is still correct.
>
> COST OF LEAVING IT UNMARKED, on 2026-09-08: I read this table, read the corrected 0.98x
> elsewhere in the same file, and spent a turn treating one document's two sections as three
> contradictory measurements of one quantity before noticing the era difference.

**FIRST RUNGS MEASURED (`crates/interp/examples/ladder.rs`), MATE-1 set of 120 positions:**

| rung | mates found | cost | mates per Mcost |
|---|---|---|---|
| depth-one | 1/120 | 24k | 41.1 (cheap, but blind to mate) |
| bare alpha-beta | **120/120** | 61.8M | **1.9** |
| alpha-beta + hash reuse | **120/120** | 77.2M | **1.6** |

**STEP 4 IS NOT A RUNG. Corrected 2026-09-07, and the correction reverses the conclusion.**

This table previously read `19.2M cost, 6.3 mates/Mcost` and concluded "Step 4's predicted gain
is confirmed on paper: hash reuse finds the SAME mates for 3.2x less cost." That was measured
against the BROKEN `ab_hash` documented in section 6 — a program that never called `eval` and
returned the constant 0 at every leaf. Mate-in-1 is found by the TERMINAL guard, not by the
eval, so a program that had stopped searching still scored 120/120 while costing a third as
much. The "confirmed gain" was the bug.

Re-measured against the faithful transposition table: **77.2M cost, 1.6 mates/Mcost — a 25%
LOSS against the seed**, not a 3.2x gain.

**Three independent measurements now agree, and they contradict the plan's expected order:**

| measurement | says |
|---|---|
| program length (§6) | hash reuse is **+104 nodes**, the FARTHEST reference program (UCT +33, PN +12) |
| cost at fixed depth (`tt_pressure.rs`) | **1.12–1.27x more expensive** than bare alpha-beta at depths 2–4 |
| mates-per-cost (this table) | **1.6 vs 1.9** — a loss |

**Why, and what it implies for the ORDER.** A transposition table pays for itself when the same
position is reached repeatedly. A single fixed-depth search offers almost no such traffic — a
few transpositions by move-order permutation — so the probe/store machinery costs more than it
saves. The thing that CREATES repeated searches of the same positions is ITERATIVE DEEPENING,
which this ladder lists as step 5, AFTER hash reuse.

So the ladder's order is wrong, and the plan's "Expected rediscovery order" (which opens with
hash reuse) is wrong with it. **Iterative deepening has to come first, or the two have to
arrive together**; a TT discovered before ID has nothing to hit and would be rejected by the
very fitness function meant to reward it. Step 4 and step 5 are provisionally SWAPPED, pending
a measurement of ID alone, which is the next thing this ladder should cost out.

This is precisely what the offline ladder check is for: a predicted rung was measured, failed,
and the sequence changed on paper — before any compute was spent chasing it.

MCTS and PN score 0 on this set at a budget of 16 simulations; both need many simulations to
prove anything and neither is a rung of this ladder.
Each step's expected gain type is recorded (2-3: mates-per-cost; 4-7: fixed-time Elo).
If any step fails to be a gain, the grammar or fitness is changed HERE, on paper.

**RESOLVED 2026-09-08: step 4 IS a rung. Hash reuse measures 0.98x the seed's cost at D=3.**

It took two instrument fixes and one real engineering change to see it, and the order matters
because each step was a genuine correction, not a search for a friendlier number:

| what changed | hash reuse vs seed, D=3 |
|---|---|
| (as first measured) flat cost model, from-scratch zobrist | 1.218x — a 22% LOSS |
| per-primitive cost model MEASURED (eval 1365, not 1) | 1.01x — break-even |
| Position maintains the Zobrist key INCREMENTALLY | **0.98x — a GAIN** |

1. The flat model charged a full NNUE forward pass the same as `const 3`, which prices a
   transposition table's entire trade — spend a cheap probe, skip an expensive eval — at zero.
2. `key` then cost 97 units because `Position::zobrist()` rebuilt the hash from scratch at
   every probe (~32 XORs plus bitboard iteration). The key is now maintained through
   make/unmake and verified equal to the from-scratch value at every node of a perft walk
   (`tests/perft.rs`), so it is a field read: 97 -> 1.

Both earlier readings were HONEST measurements of a system that was genuinely worse than it
needed to be. The engine really was paying 97 units per probe; the ladder really did say
"not a rung". What was wrong was concluding anything about hash reuse AS A TECHNIQUE from a
measurement dominated by two fixable implementation costs.

The prediction that survives is the DIRECTION: the overhead falls with depth, so the gain
should widen deeper. The claim that does NOT survive is the earlier "break-even near depth
5-6, so evolution at depth 2 can never discover it" — at depth 3 it is already ahead.

**MEASURED 2026-09-08 — THE RUNG IS REAL AND STILL UNREACHABLE, FOR A DIFFERENT REASON.**

`examples/evolve valley` scores the rung's two HALVES with the search track's own fitness
(`ladder_valley_RESULT.md`), 25 positions at D=3, deterministic:

| program | nodes | mates/Mcost | vs seed |
|---|---|---|---|
| bare alpha-beta (seed) | +0 | 0.002518 | 1.000x |
| probe only (never stores) | +69 | 0.002494 | **0.991x** |
| store only (never probes) | +39 | 0.002510 | **0.997x** |
| hash reuse (both halves) | +104 | 0.002578 | **1.024x** |

Both halves are WORSE than the seed; only the conjunction is fitter. `evolve` accepts on
`rate > best_rate`, STRICTLY, so neither half can ever be taken and the pair can never be
assembled one edit at a time.

**So GRAMMAR 9's premise — "a path of single mutations from the seed exists where every step is
fitter" — is measured FALSE for the only rung that is verifiably fitter.** The valley is
structural, not a property of this position set: `Interp::run` clears the table per position, so
probe-only searches a table nothing wrote (guaranteed misses) and store-only writes entries
nothing reads. Neither half pays alone in ANY set, so a different set moves the magnitudes and
cannot move the sign.

This RETIRES the next task it looked like the plan implied. `tests/reachability.rs` proves the
mutation operators cannot construct `Probe`/`Key`/`Field`/`Store`, and the obvious response was to
add operators that can. That would not have helped: reachability is necessary, not sufficient, and
no monotone path exists regardless. The bottleneck is the SEARCH — strict hill climbing over a
0.3-0.9% valley — not the grammar. An operator that inserted probe+store as ONE edit would cross
it, and is refused: MASTER_PLAN line 53 requires these to be DISCOVERED, so a gadget-inserting
operator makes the discovery vacuous.

**FIDELITY NOTE 2026-09-08 — `ab_id` is not budget-aware, so it is not iterative deepening as
practitioners mean it.** Real ID searches successively deeper *until its allowance runs out*;
`with_iterative_deepening` loops depth 1..D and stops, never reading `Budget`. Verified across the
whole reference set: only `uct_mcts` and `proof_number` contain `Node::Budget`, and all nine
alpha-beta-family programs are budget-blind. The node count and the ladder measurement (0.914x) are
correct for what is written; what is written is a DEPTH ladder, not a time-limited one, and the
distinction matters because FITNESS 3's cost term only becomes meaningful for a budget-aware
program (see FITNESS.md).

**MEASURED 2026-09-08 — THE LADDER IS A COST LADDER, NOT A STRENGTH LADDER, AT DEPTH 3.**
`evolve moveagree` runs every reference program on 40 random positions and compares the move
returned against the seed's. All SEVEN alpha-beta-family programs agree with the seed 40/40 —
including capture extension and table reduction, which change effective depth and were expected to
differ. Only other paradigms diverge (depth-one 25%, UCT 10%, PN 0%). So the rungs of this ladder
are indistinguishable as PLAYERS at the depth the search track runs; they differ only in cost, and
two of them (0.985x, 0.993x) are pure overhead for identical play. Limit: 40 positions at depth 3,
so this is "no disagreement observed here", not "never differs".

### A5 — THE LADDER AS A STANDING TEST (`evolve valleyall`), MEASURED 2026-09-08

Every reference program scored on the search track's OWN fitness set (25 positions: 15 mate-in-1,
5 depth-requiring, 5 window-sensitive) at depth 3. UCT is scored at its declared budget of 256,
not 16, for the reason given in `configs/search_track.conf`.

| program | nodes | mates | mates/Mcost | vs seed | verdict |
|---|---|---|---|---|---|
| depth-one (purity seed) | −62 | 5/25 | 2.499200 | **992.711x** | loses answers |
| bare alpha-beta (seed) | +0 | 25/25 | 0.002518 | 1.000x | — |
| **alpha-beta + hash reuse** | +104 | 25/25 | 0.002578 | **1.024x** | **FITTER** |
| alpha-beta + iterative deepening | +29 | 25/25 | 0.002300 | 0.914x | not fitter |
| alpha-beta + hash + ID | +133 | 25/25 | 0.002348 | 0.933x | not fitter |
| UCT-style MCTS | +59 | 12/25 | 0.001409 | 0.560x | loses answers |
| capture extension (rung 6) | +9 | 25/25 | 0.002479 | 0.985x | not fitter |
| table reduction (rung 7) | +15 | 25/25 | 0.002499 | 0.993x | not fitter |
| proof-number search | +104 | 4/25 | 0.022854 | 9.078x | loses answers |

> **This table is a SNAPSHOT, and its "vs seed" column is stale on two rows (noted 2026-09-10).**
> It reads `+59` for UCT and `+9` for the capture extension; the gated GRAMMAR 6 table now records
> **+60** and **+13**. The numbers were NOT edited in place, because the distance column is not the
> only thing that moved: the `12/25` beside UCT is the pre-`RESOLVED 2026-09-09` blend program, and
> the `25/25` beside rung 6 predates commit `064112f` ("rung 6 fixed to extend AT THE HORIZON").
> Refreshing the distances alone would pair current programs with fitness measured on their
> predecessors — a row that never existed. Re-run `examples/prior.rs` to regenerate the whole table
> if these fitness figures are needed for current programs.

**HASH REUSE IS THE ONLY FITTER RUNG ON THE WHOLE LADDER.** That was previously asserted from one
measurement of one program; it is now measured across all nine, and every other rung the plan
declares — ID, capture extension, table reduction — is a LOSS at this depth.

**AND THE SURROGATE ALONE IS WORTHLESS, quantified.** depth-one scores **992x** the seed's
mates-per-cost while answering 5 of 25; proof-number scores **9x** while answering 4. Both are
"fitter" by the surrogate and both are useless. The only thing standing between this loop and a
program that answers nothing instantly is the mate guard `f >= best_found`, which is why it is
never relaxed and why plateau tolerance applies to COST ONLY.

**CONJUNCTIVE, CONFIRMED:** whole 1.024x fitter, best half 0.997x not fitter (probe-only 0.991x,
store-only 0.997x). A strict `rate > best_rate` climb cannot take the first step.

No generic single-primitive decomposition is attempted for the other rungs. probe-only/store-only
is specific to a transposition table, and inventing an equivalent for ID or capture extension would
be inventing an instrument rather than using one.

**CONSEQUENCE: hash reuse is not a rung AT THESE DEPTHS, and that is a statement about the
regime rather than about the technique.** `examples/ladder.rs` now sweeps depth. Cost relative
to the bare alpha-beta seed:

| program | D=2 | D=3 | D=4 |
|---|---|---|---|
| hash reuse | 1.250x | 1.218x | **1.113x** |
| iterative deepening | 1.084x | 1.098x | 1.086x |
| hash reuse + ID | 1.357x | 1.341x | 1.218x |

The overhead SHRINKS with depth and the shrinking accelerates (-0.032 D2->D3, then -0.105
D3->D4), which is what "a table needs transpositions and a shallow search has none" predicts.
Break-even lands near depth 5-6 on two independent estimates: extrapolating this trend, and
`tt_pressure.rs` measuring an 11.3% eval saving at depth 4 against a ~22% cost overhead (so the
table must remove >18% of nodes to pay for itself).

So the ladder's step 4 is real, and it is simply **not reachable from a depth-2 fitness
function**. Evolution searching at depth 2 would correctly reject it as a 25% loss. That
constraint is now recorded in MASTER_PLAN's rediscovery order.

**TWO HYPOTHESES DIED HERE AND BOTH ARE KEPT, because the wrong ones were instructive.**

1. *"Hash reuse should be a gain, the ladder confirms it."* — that confirmation was measured
   against a broken encoding that never called `eval`; see the correction above.
2. *"Iterative deepening is what creates the traffic a table needs, so steps 4 and 5 should be
   SWAPPED."* — written into this document and into MASTER_PLAN, then measured and REFUTED. ID's
   cost is FLAT at ~1.086x across D2-D4, and hash+ID is worse than hash alone at every depth.
   ID re-searches from scratch; the entries it stores at depth d-1 are rejected by a probe
   needing depth >= d. The swap has been reverted in both documents.

Two wrong hypotheses in a row was also the signal that the HARNESS was the thing to doubt: the
ladder had a single hardcoded D=2, a depth at which no transposition-table effect can exist.
Sweeping depth is what turned a flat "hash reuse is a loss" into a trend with a break-even.

## 10. Audit
### 10.1 Neutrality (Shogi test, per primitive)
All 29 primitives are unchanged under a Shogi movegen. `pred`'s PredId list is
rules-derived and would be regenerated from Shogi's rules (captures, checks, promotions,
drops). `score_of` maps symbolic outcomes through a learned table, so no outcome
magnitude or draw value is given. PASS, with `pred` and `score_of` flagged as the two
primitives whose declared lists/tables are game-specific in FORM but derived, not chosen.

### 10.1b `pred` and what it enables (pre-empting the "smuggled ordering" objection)
`captured_type` and `moving_type` are rules facts. `pred(m, p, captured_type)` fed into a
learned table indexed by (captured_type, moving_type) is exactly MVV-LVA waiting to be
discovered — the ORDER is given nowhere; the table starts at identity and SPSA fills it
if it wins. Likewise `gives_check` + a table is check-extension-ready. This is the
intended mechanism: rules facts in, ordering learned. Nothing in `pred` ranks anything.
### 10.2 Hidden chess opinions — checked and removed
- Draw = 0: removed (score_of table).
- Prefer shorter mates: removed (score_of table indexed by depth; the engine may learn it).
- Search depth of the seed: removed (D is a table read).
- INF magnitude: removed (table read).
- Emission order of `moves`: shuffled at the seed; ordering must be learned.
- Constants: capped at |8| so magnitudes come from tables or arithmetic, not literals.
- `pred` exposes no value or quality; is_capture is a rules fact, "good capture" is not available.
### 10.3 Expressiveness — verified by construction (Section 6)
Alpha-beta, hash reuse, ID, qsearch-like extension, LMR-like reduction, UCT, policy-
weighted MCTS, PN-search, and two hybrids are all written out and counted. PASS.
### 10.4 Known biases — declared, not removable
- `foreach` is sequential: depth-first programs are shorter than breadth-first ones.
- `max`/`min` are single primitives: minimax backup is one node; averaging is `avg` over
  slot fields, i.e., 3-4 nodes. Backup asymmetry ~3 nodes in AB's favor.
- The main seed IS alpha-beta. The purity lineage exists to measure what this costs.
- (Removed) The per-node static cost cap was a second, independent bias toward
  alpha-beta — MCTS's cost is per-simulation with `sqrt`/`log` per child per visit. It
  has been deleted (Section 8): with cost-unit budgets it was redundant, so the bias is
  gone rather than declared. Recorded here so the history is visible.
- Total declared skew: mutation distance ~10-12 toward alpha-beta vs MCTS (robust);
  node gap nominally ~15 but see Section 6 for the interval.
### 10.5 Degenerate-solution resistance
- "Prune everything": punished by mates-per-cost (missing forced mates) and caught by the
  correctness oracle (impossible claimed scores).
- "Search nothing, return eval": is the depth-one program; only wins if the eval is good
  enough, which is the dial working as intended, not a bug.
- "Exploit hash collisions for free Elo": correctness oracle.
- "Never claim exactness so the strong check never fires": impossible — exactness is a
  compiler-derived taint (Section 7), not a program flag.
- "Infinite loop / stack blow": budget primitive and hard ceilings.
### 10.6 Corrections applied in first self-audit
- `cmp` Rel set was {<, <=, ==}; the seed used >= and !=. Expanded, and Outcome
  comparison added, so the seed type-checks in its own grammar.
- No assignment primitive existed; the seed mutated locals. Added `set` (#29).
- `field` had a union return type; now typed by FieldId.
- Node counts relabeled as hand estimates pending the parser.
- Seed rewritten to use only primitives in Section 2.
### 10.6b Corrections applied in second (external) audit
- `claims_exact` was program-settable and therefore gameable; replaced with a
  compiler-derived static taint (Section 7); exploit added to 10.5.
- Node cap raised and staged (96 -> 160 at P3); binding at the P3 milestone was implicit.
- Per-node cost cap declared as a second, independent anti-MCTS bias (10.4).
- Primitive count reconciled to 29 everywhere (header, Control group, 10.1).
- +/-20% now propagates: prior leads with mutation distance; node gap given as an interval.
- `pred` -> MVV-LVA / check-extension path made explicit (10.1b).
- Elo-per-doubling derivation inlined; bracket 50-170 (2-player literature low end; 4PC 137 @ 48 pairs and 170 @ 148 pairs; the combined 176 is flagged by its own source as a likely small-sample artefact).
### 10.6c Corrections applied in third audit
- Budget unit: nodes -> evaluations -> COST UNITS (evaluations were gameable by terminal-
  driven search). Static per-node cost cap removed; the 10.4 bias it caused is gone.
- Mates metric renamed mates-per-cost to match FITNESS.md.
### 10.6c Corrections applied in third self-audit (implementation)
- `store` was declared `Key x Slot -> Unit`, but no primitive constructs a Slot — the signature
  could not be implemented. Amended to `Key x FieldId x Int -> Unit` (write one named field).
- The interpreter stored a single scalar per key, so MCTS's `count` and `sum` collided: a
  faithful UCT would have measured the right SIZE while computing garbage if RUN. Slots now
  carry score/depth/flag/count/sum/move separately.
- `Node::seq` scoped a `Let` statement over its own placeholder instead of the rest of the
  sequence, which made the alpha-beta seed's `best` accumulator read 0. Fixed; the seed's node
  count fell 75 -> 71 once the placeholder nodes went away.

### 10.7 Open items (to resolve in FITNESS.md / CRATE.md)
- Exact `claims_exact` semantics for mixed (max-then-avg) backups.
- Whether `sample`'s RNG seed is per-game or per-node (reproducibility vs. diversity).
- Interpreter benchmark against the 50% criterion — this is a measurement, not a decision.
- Whether the const cap |8| is too tight for early window-widening programs (may be raised
  to |16|; declared either way).
