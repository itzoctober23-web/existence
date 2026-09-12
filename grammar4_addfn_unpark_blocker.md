# The AddFn unpark condition is blocked by a SECOND ordering bias — the function sweep short-circuits

**2026-09-11 23:55.** Code-reading finding, not a runtime measurement. Recorded because it changes what
the next GRAMMAR 4 step is, and because the measurement that would confirm it is cheap and named below.

## Why this was looked at

`p2_fitness_RESULT.md` closed the fitness-composition route today, and both it and
`gate_power_RESULT.md` name the same successor: **GRAMMAR 4 — the type checker and mutation
operators**. The live gap `shape_reachability.rs` asserts is:

> *"0 of 858 applied mutations change `Program::funcs.len()`, so GRAMMAR 4's `add-fn` remains absent
> and every program the search can reach has exactly one function, against a type checker that admits
> 1..4."*

`Op::AddFn` is the only operator that can raise `funcs.len()`, and it is in `PARKED_OPS`. Its unpark
condition (`mutate.rs:148`) is recorded as:

> *"UNPARK WHEN there is a reason for a second function to EARN its call — i.e. once something can
> diverge the lifted body from its origin, or the cost model stops charging a bare call."*

`add_fn_drops_writes_RESULT.md` fixed AddFn TODAY (it had been silently dropping alpha-beta's score
update). So the first clause deserved re-checking against the code as it now stands.

## What the code says

Mutation sites CAN address any function — `mutate()` at `mutate.rs:450` picks
`fi = rng.below(p.funcs.len())`, uniformly. But the search does not call that. It calls
`mutate_program_n`, which sweeps functions **in order and breaks on the first successful placement**:

```rust
for fi in 0..cur.funcs.len() {
    let total = count_nodes(&cur.funcs[fi].body);
    ... shuffled positions ...
    for k in order { if let Some(next) = mutate_at(&cur, op, rng, fi, k) { ok = Some(next); break; } }
    if ok.is_some() { break; }          // <-- funcs[1..] reached ONLY if funcs[0] matched nowhere
}
```

**So a lifted body can diverge only when the drawn operator cannot be placed anywhere in `funcs[0]`.**
For broadly-applicable operators that is essentially never — the same file records that `InsertMax`
wraps *any* Score node and won 32 of 67 ledger placements, while `Tweak` matches ~6 of the seed's 71
nodes and won 0.

## The shape of this is a bias ALREADY FIXED one level up

The comment directly above that loop describes the identical problem for OPERATORS and its fix:

> *"The original drew a fresh random operator on every retry and kept whichever applied first, which is
> a race that broadly-applicable operators always win … PICK THE OPERATOR FIRST, then retry that SAME
> operator on different nodes."*

That fix made the drawn operator set the declared one. **The same race remains across FUNCTIONS**:
positions are shuffled *within* a function, but functions are tried in index order and the first hit
wins. Pick-first-then-place was applied to the operator dimension and not to the function dimension.

## What this means for the unpark decision

The unpark condition is **not** met, but not for the reason previously recorded. It is not that
divergence is impossible — the code supports it. It is that the search reaches `funcs[1..]` only as a
fallback, so a lifted function would sit inert, costing its `Call` (measured ≤1.004x the parent) and
earning nothing. Unparking AddFn alone would therefore still produce only strictly-worse candidates,
exactly as the park says — via a different mechanism than the one written down.

## The measurement that would settle it — cheap, and NOT run here

Construct a 2-function program (apply AddFn once, directly), then draw N mutations via
`mutate_program_n` and count how many land in `funcs[1]`. The prediction from the code is **≈0 for
broadly-applicable operators**. If confirmed, the GRAMMAR 4 step is to make function choice
independent of placement order — the same fix already applied to operator choice — and only then
re-examine unparking AddFn.

**Not done tonight:** that is a `cargo test -p grammar` build, and the Existence slot is running the
953-pair promotion resolve. Nothing here changes a live search: AddFn stays parked, and no operator
behaviour was modified.

---

## MEASURED 2026-09-12 00:10 — my prediction was WRONG. `funcs[1]` IS reached, 17–79% of draws.

The section above predicted, from reading the short-circuit, that `funcs[1..]` would be reached "**≈0
for broadly-applicable operators**". I named the measurement and ran it
(`crates/grammar/tests/mutation_function_bias.rs`, 4000 draws of `mutate_program_n` per multi-function
reference program). **It refutes the prediction.**

```
program                      node counts   funcs[0] hit   uniform-by-size   funcs[1] hit
table reduction (rung 7)      [13,  73]       19.18%          15.12%           78.70%
bound-gap stopping            [69,  58]       80.80%          54.33%           17.22%
proof-number search           [27, 148]       53.42%          15.43%           44.10%
draws that applied: 4000/4000 in every case; abandoned: 0
```

**Where the reasoning failed.** The loop breaks only when a placement SUCCEEDS in `funcs[0]`. I treated
"broadly-applicable operators almost always place in funcs[0]" as if it made funcs[1] unreachable, but
the operator set is mixed: any draw of an operator that matches nothing in `funcs[0]` falls through, and
that is common — overwhelmingly so when `funcs[0]` is small (13 nodes → funcs[1] takes 78.7%).

**What IS real, and is the honest version of the finding:** `funcs[0]` is over-represented relative to
its share of nodes in EVERY program measured — mildly at [13,73] (19.18% vs 15.12%) and by **3.5x** at
[27,148] (53.42% vs 15.43%). So the index-order short-circuit does bias placement toward the first
function; it does not exclude the others.

**Consequence for the AddFn unpark decision — the blocker moves.** The unpark condition has two
clauses: *"once something can diverge the lifted body from its origin, OR the cost model stops charging
a bare call"*. The first clause is **satisfiable**: a lifted function would be edited on a substantial
fraction of subsequent draws, so it can diverge. **The binding constraint is therefore the COST clause**
— a lift costs ≤1.004x its parent and under FITNESS 3 (mates per cost) that is strictly worse, so the
first lift still cannot survive long enough to be diverged.

**Caveat on generalising these numbers.** All three fixtures are hand-written REFERENCE programs, not
lifted-by-AddFn ones. An AddFn lift's `funcs[1]` would be a subtree cut from `funcs[0]`, so the size
ratio — which these numbers show is what drives the split — would differ. The direction (funcs[1] is
reached) is robust; the exact percentage for a lifted program is not measured.

**The test asserts nothing about the value.** It prints. A threshold invented before the first
measurement would have been a guess wearing a test's clothes — and had I written one from my
prediction, it would now be failing on correct behaviour.

---

## 00:15 — the recorded blocker conflates ACCEPTANCE with RETENTION, and the gap is 5x

Three things were verified tonight, each from the code rather than from the doc comment that asserts it:

**1. `Node::Call` really is charged 2** — it has NO explicit arm in `interp::cost_of` and falls through
the catch-all `_ => 2`. The unpark note's claim is accurate. Worth noting that the same function warns
about exactly this catch-all:

> *"`cost_of` ends in `_ => 2`, so a new node that reads the net would otherwise be charged 2 — eighty
> times under its real cost … an underpriced primitive is a standing invitation for the search to spend
> everything on it for free."*

`Node::Unc` was given an explicit arm for that reason. `Call` was not — nobody has asked whether 2 is
right for it, and FITNESS's own revisit trigger for the cost model (**cost-vs-time correlation < 0.95**,
`FITNESS.md:116`) appears in **no result file**: it is a declared check that has never been measured.

**2. `funcs[1]` is edited 17–79% of draws** (measured above, refuting my own prediction).

**3. The retention rule is `x >= top*(1-EPS)`** (`evolve.rs:1679`, `:1699`), with `EPS = 0.02` from
`configs/search_track.conf`. A lift's rate is `parent / 1.004 = 0.996x`. **0.996 >= 0.98 — the lift is
RETAINED.**

### Why that matters

The unpark note says a lift is *"strictly worse, so it cannot be accepted ON ITS OWN"*. That is true of
ACCEPTANCE and irrelevant to survival. The population is explicitly there for worse-than-best members —
`search_track.conf` says so: *"The population exists to carry intermediates that are WORSE than the
best."* The lift's penalty is **0.4%**; the retention band is **2%**, five times wider.

So the route the unpark condition asks for already exists, unmentioned by either clause:

```
lift (0.996x, retained by EPS)  ->  a later draw edits funcs[1] (17-79%)  ->  diverged candidate
```

**What I am NOT doing: unparking `Op::AddFn`.** The unpark condition is a recorded rule, changing it
alters a live search's behaviour, and `add_fn_drops_writes_RESULT.md` showed one day ago that the
"obvious" claim about this operator was false for months. The evidence above is an argument that the
condition deserves re-examination, not authority to change it. What would settle it is a direct
measurement — enable AddFn in a TEST harness, run N generations, and count how many lifted functions
survive to be edited — and that is a run, not a code reading.

**Kept honest:** point 3 rests on the lift costing 1.004x, which is `add_fn_drops_writes_RESULT.md`'s
measured figure for the seed. A lift of a LARGER subtree pays the same flat +2 against a larger base, so
the ratio only improves. A lift of a tiny subtree in a tiny program could exceed 2% and fall outside the
band — untested, and the reason this is an argument rather than a conclusion.

---

## 00:22 — measured on an ACTUAL lift: `funcs[1]` is edited 0.00% of the time. My 00:15 conclusion is WRONG.

The 00:10 section measured hand-written multi-function REFERENCE programs and found `funcs[1]` edited
17–79%, refuting my original "≈0" prediction. I flagged the caveat myself: an AddFn lift produces a
`funcs[1]` that is a subtree CUT FROM `funcs[0]`, so its size — the thing that drives the split — is
different. `lifted_function_edit_rate.rs` measures the lifted case:

```
depth-one (purity seed), AddFn applied directly via mutate_at
  funcs sizes after the lift: [9, 2]      (seed body was 9 nodes)
  draws applied: 767
  funcs[0] edited  767 = 100.00%   (by size 81.82%)
  funcs[1] edited    0 =   0.00%   (by size 18.18%)   <- THE LIFTED BODY
```

**0 of 767.** The lifted body is never edited, so it can never diverge from its origin.

### The reconciliation, and why the caveat was the whole story

```
fixture                         funcs[1] size    funcs[1] edited
reference: table reduction           73                78.70%
reference: proof-number search      148                44.10%
reference: bound-gap stopping        58                17.22%
ACTUAL AddFn lift                     2                 0.00%
```

A lift takes a small subtree, so `funcs[1]` is tiny and offers almost no placement sites while
`funcs[0]` — still 9 of the 11 nodes — absorbs every operator. The reference programs were the wrong
fixtures for this question: they have second functions comparable in size to the first, which no lift
produces.

### Consequence: the unpark blocker moves BACK

My 00:15 conclusion — "divergence is satisfiable, so the binding constraint is the COST clause" — **is
refuted.** For an actual lift, divergence does not happen at all, so BOTH clauses of the unpark
condition are unmet:

* **divergence:** 0.00% measured, so a lifted body cannot earn its call;
* **cost:** a lift still pays +2 for its `Call`.

The EPS retention argument from 00:15 stands on its own (a lift at 0.996x IS retained), but retention
without divergence just carries an inert, slightly-costlier twin — which is precisely what `PARKED_OPS`
says.

### Limits of THIS measurement, stated rather than buried

* **One seed lifted.** Only `depth-one` (9 nodes) produced a legal 2-function program; `contains_set`
  refuses every Set-bearing subtree, and the other seeds offered no lift site in the sites tried.
* **767 of 4000 draws applied** — a 11-node program offers few placement sites, so most draws abandon.
  0/767 is still decisive for this program; it is not a claim about every possible lift.
* **A larger lift is untested.** If AddFn could lift a big subtree, `funcs[1]` would be large and the
  reference-program rates suggest it would be edited often. Whether such a lift site exists under
  `contains_set` is not measured here.

**What this chain shows about method:** three measurements tonight, each overturning the previous
reading — prediction (≈0), reference programs (17–79%), actual lift (0.00%). The caveat I attached to
the middle one was not a formality; it was the entire result.

---

## 00:28 — the framing was wrong: BOTH live lineages already seed from 2-FUNCTION programs

`crates/grammar/tests/reachability.rs:199-202` records the claim this whole chain rests on:

> *"0 of 858 applied mutations change `Program::funcs.len()`, so GRAMMAR 4's `add-fn` remains absent
> and **every program the search can reach has exactly one function**, against a type checker that
> admits 1..4."*

**The second half is false for the live search.** A census of `reference::all()` — a descriptive fact I
had been inferring from filters instead of measuring:

```
program                                     funcs   sizes
depth-one (purity seed)                       1     [9]
bare alpha-beta (MAIN seed)                   2     [13,  58]     total 71
UCT-style MCTS                                2     [16, 115]     total 131
... 12 of 13 reference programs have 2 functions; only depth-one has 1
```

And `evolve.rs:641` seeds from `reference::bare_alpha_beta()`. The live P2 run's own header confirms
it independently:

```
lineage MAIN  seed  71 nodes      = bare alpha-beta [13, 58]
lineage MCTS  seed 131 nodes      = uct_mcts        [16, 115]
```

**So both live lineages start with two functions.** The reachability claim is true as written only of a
search seeded from a 1-function program — which is the PURITY lineage (`depth-one`, 9 nodes), not the
two lineages that actually run.

### What that does to the AddFn argument

`PARKED_OPS` values AddFn as *"the only operator that can raise `funcs.len()`"*, closing a 1 -> 2 gap.
For the live lineages there is no 1 -> 2 gap: they begin at 2. AddFn would be taking them 2 -> 3.

And the MAIN seed's second function is **58 of 71 nodes (82%)** — the same shape as `table reduction`
[13, 73], which my 00:10 measurement showed has its `funcs[1]` edited **78.70%** of draws. So the live
MAIN lineage already carries a large, actively-mutated second function. The thing AddFn was parked as
an enabler FOR is already present in the lineages that matter.

Where AddFn's 1 -> 2 gap is real — the purity lineage — is exactly where I measured a lift to be tiny
(max 5 nodes of 9) and never edited (**0 of 767**).

### Status of the chain, four measurements in

```
prediction from code        funcs[1] edited ~0%        WRONG for reference programs
reference programs          17-79%                     RIGHT, wrong fixtures for a lift
actual AddFn lift           0.00% (0/767)              lifts are tiny and inert
reference census            MAIN seed already 2 funcs  the 1->2 gap does not exist live
```

**AddFn stays parked, and the park now has a better reason than the one recorded**: not merely that a
lift is costlier, but that in the lineages that run there is no missing second function to create, and
in the lineage that lacks one, every legal lift is too small to be edited.

**Not claimed:** that `funcs.len()` 2 -> 3 would be worthless. That is untested, and the type checker
admits up to 4. What is claimed is that the recorded justification for AddFn's VALUE — closing a
1 -> 2 gap — does not describe the live search.
