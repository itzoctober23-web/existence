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

| Program | Nodes (MEASURED) | Fidelity |
|---|---|---|
| depth-one | 9 | faithful (purity seed) |
| bare alpha-beta | 71 | faithful (main seed) |
| alpha-beta + hash reuse | **175** | faithful (validity marker, depth, EXACT/LOWER/UPPER bounds) |
| proof-number search | 83 | faithful (proof/disproof numbers, most-proving-node, back-up) |
| UCT MCTS | 104 | faithful (select/expand/evaluate/backpropagate) |

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
budget). Rung 7 (table-driven reduction) is still unwritten.

The capture-extension rung is the expressibility check MASTER_PLAN item 2 asks for -- "verify
alpha-beta, hash reuse, ID, and qsearch are expressible in the grammar", using the test eval
"for this rig only". It is NOT seeding qsearch: MASTER_PLAN line 53 requires the SEED to hold
"no quiescence ... all of these must be DISCOVERED as program edits that beat the current
program on the clock", and `bare_alpha_beta()` is untouched -- still exactly **71** nodes, which
the same run re-measures every time. The reference set is read by the ladder rig alone; neither
the search nor the evolution loop imports it.
The original parenthetical estimates (8/18/29/41/52/61/72) were the same hand guesses that
measured 2.4x wrong elsewhere in this document and have been removed rather than corrected.

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
