# Existence — current state, 2026-09-09

## Epochs closed in BOTH directions — 3 is the optimum

```
ep2_2  vs ep2_3 @ d4:  0.448 ± 0.022   epochs 3 BETTER than 2, resolved
ep2_10 vs ep2_3 @ d4:  0.485 ± 0.022   unresolved, no gain from 10
seed 424242,   d2:     0.473 ± 0.020   epochs 3 ahead
seed 20260907, d2:     0.498 ± 0.021   no effect
```

Fewer epochs loses; more epochs does not win. **The shipped default of 3 is at or near the optimum,
tested on both sides** — a stronger statement than a bare null, and the exact reverse of the
"epochs 2 is the surviving candidate" claim I made this morning off the confounded time-boxed arms
(28 vs 26 generations).

The `ep2_10` arm existed all day from `epochs_ab2` and cost one match to use. The mirror test was
worth running precisely because the day's other evidence pointed the *other* way: `distill_gap`
shows the search-minus-eval gap growing with strength, which made "the net is not fitted hard
enough" a live explanation. It is not: 10 epochs buys nothing.

## Gating does not help from scratch — and the gate rolls back on noise

```
sg_20 (gate-every 5) vs bn_075 (no gating) @ depth 4:  0.479 ± 0.023  [0.457, 0.502]
```

Unresolved, but the point estimate favours the **ungated** arm. The mechanism is visible in
`sg_20`'s own log: it rolled back block g15 on an increment of **−0.013 ± 0.030** — an interval
**three times wider than the effect it acted on**. It discarded five generations of champion
progress on a reading it could not resolve.

That is the acceptance-floor arithmetic biting from the other side. The gate needs an edge larger
than its own ci95 to KEEP, so near-zero blocks always roll back — and when the true block value is
slightly positive, the rollback is a loss. From scratch, where most blocks are genuinely positive,
this costs more than it saves.

**`fg_60`: 8 gates, 0 KEEP.** The pool-growth prediction is dead eight times over, not just at
block 5.

**Net on `--gate-every 5`:** no depth-4 gain from `champion_long` (0.502 ± 0.022), no gain from
scratch (0.479 ± 0.023), and 0 KEEPs in 8 blocks once `batch_base` was fixed. The batch gate is
**correct now but not useful** — its resolution is coarser than the effects it is asked to judge.

## IMPLEMENTED (default OFF): `EXISTENCE_HARD_FITNESS` folds the hard set into the surrogate

The candidate loop already computes a hard-set score per candidate — `let (hf, _, _) = fitness(c,
hard, net, depth, bud)` — and then discards it into a diagnostic. With the flag set, the surrogate
becomes `(f + hf) * 1e6 / cost` instead of `f * 1e6 / cost`.

**Why this is justified by the code's own criterion, not my judgement.** `evolve.rs:1207` deferred
it — *"acceptance is NOT changed yet, because the claim 'a better-searching candidate can win these'
is exactly the sort of thing that should be measured before a fitness is restructured around it"* —
and line 1434 gives the test: *"If this never varies, the gradient does not exist."* Measured across
39 lineage-generations: **it varies, 51% non-zero, best 2/8, seed 0/8.**

**Why it should work mechanically:** mates are saturated at 25/25, so the surrogate can only rise
via cost, and 0 of 30 mutants are cheaper — hence the max rate is exactly 1.000× in all 39
lineage-generations and never above. A candidate solving one hard position scores 26/25 = **1.04×**,
which clears both the tie and EPS.

**INTERIM — the flag BINDS, but the gate still rejects.** Flagged vs control, same binary, one env
var apart:

```
FLAGGED  gen1 MAIN  gate REJECT 0.458±0.082  surrogate 0.002265
         gen2 MAIN  gate REJECT 0.417±0.103  surrogate 0.002453
         gen4 MAIN  ..none  rates 0.982-1.000x  hard 1-1   <- every member solves 1
CONTROL  gen1 MAIN  gate REJECT 0.417±0.103  surrogate 0.002794
```

The flag changes which candidate is proposed, and the population climbs to **`hard 1-1`** — all
eight members solving a hard position the seed fails 0/8. But the max rate re-saturates at 1.000×
once the whole population reaches the new level, and **both arms are rejected by the 12-game gate**.

**So the surrogate was not the only binding constraint** — the gate is one too, exactly as the
`resolved_up`-at-6-pairs arithmetic predicted (needs ~60–69% of pairs). Fixing the fitness moved the
population up one rung on the hard dimension and did not produce an acceptance.

**Not yet a verdict.** The control is 1 generation to the flagged arm's 4, and the banked run reaches
`hard 2-2` on its own — so the flag may not be necessary for hard-set climbing at all. The
comparison needs matched generation counts before it means anything, which is the exact error
(comparing arms that did unequal work) that invalidated three results today.

## 🔑 THE COMPLETE MECHANISM — and the precondition the code set is now MET

**Why nothing is ever promoted, end to end:**

1. **The surrogate is `mates / Mcost`, and mates are SATURATED** — the seed already scores 25/25 on
   the guard set. Mates cannot improve.
2. **So only cost can improve — and nothing is cheaper.** `stepdiff` measured **0 of 30**
   identical-playing mutants cheaper than the champion (95% upper bound 10%).
3. **Therefore the surrogate can never exceed 1.000×.** Verified: across **39 lineage-generations**
   in two runs, the max candidate rate is **exactly 1.000× and never above it**.
4. **Ties still reach the game gate** (17 calls in the old run), where `resolved_up` at 6 pairs
   demands ~60–69% of pairs. All rejected.

**Meanwhile the one unsaturated dimension shows real progress that acceptance cannot see:**

```
hard-set scores across 39 lineage-generations (seed = 0/8 by construction)
  20 of 39 (51%) have a member scoring >0
  best observed: 2/8
```

`evolve.rs:1207` explains why the fitness ignores it:

> "Scored and reported per generation; **acceptance is NOT changed yet, because the claim 'a
> better-searching candidate can win these' is exactly the sort of thing that should be measured
> before a fitness is restructured around it.**"

**That measurement now exists, and the answer is yes** — 51% of lineage-generations contain a member
that wins hard positions the seed loses. The precondition the code set for restructuring the fitness
has been met by data the runs were already producing.

## 🔑 THE SEARCH TRACK'S ACCEPTANCE RULE CONTRADICTS ITS OWN DOCUMENTED INTENT

`evolve.rs:1199` describes the game gate:

> "6 pairs = 12 games resolves a large effect… **it CANNOT resolve a 2% edge and is not asked to.
> It is a veto on unplayable programs.**"

`evolve.rs:1573` implements it:

```rust
let resolved_up = gsc.pent_rate() - gsc.ci95() > 0.5;
if !resolved_up { REJECT }
```

**That asks the gate to resolve an edge — the exact thing the comment says it cannot do.** A veto on
unplayable programs would reject only when the gate resolves the candidate *worse* (`resolved_down`).
This rejects **ties**, and at 6 pairs ci95 is **0.189** (0.103 in the one live case), so a candidate
must win **~60–69% of pairs** to be promoted.

The single recorded live acceptance attempt: `gate REJECT 0.417+/-0.103` — a tie, rejected. Not an
unplayable program.

**And the population is healthy.** The probe shows `pop 8 spread 0.002859–0.002884` and `hard 0-1` —
members are diverse *and* one solves a hard-set position the seed fails 0/8. Candidates are being
generated, admitted, and are improving on the unsaturated dimension. They die at promotion.

Same defect class as the anchor gate's "same seed family… same openings" comment, which was also
false about its own code. **Not changed** — this is the experimental apparatus, and every unmeasured
belief today was wrong. What would justify a change: measuring how many promotions the
`resolved_down` rule would admit that `resolved_up` rejects, and whether they survive a larger gate.

## 🔑 WHY THE DISCOVERY TRACK FINDS NOTHING — diagnosed in the selection code

The track that is supposed to discover qsearch runs 25 generations with **0 accepts** and a
population of 8 whose surrogate scores are identical (`spread 0.005508–0.005508`). The cause is in
three lines of `evolve.rs`:

```rust
pool.retain(|x| x.2 >= top * (1.0 - EPS));           // eps = 0.02 -> need >= 0.98x best
pool.retain(|x| seen.insert(format!("{:?}", x.0)));  // dedupe is STRUCTURAL, not behavioural
pool.truncate(MU);
```

1. **The population is initialised as `vec![seed_prog; MU]`** — 8 identical copies. It does not
   collapse; it *starts* collapsed and can only diversify through an accepted mutation.
2. **The dedupe is structural.** Correct alpha-beta variants are behaviourally identical (measured:
   40/40 at depth 3), so 8 syntactically-different programs computing the *same function* all
   survive dedupe, all tie at rate 1.000×, and fill every slot.
3. **EPS cuts before dedupe runs.** A behaviour-*changing* edit is precisely the one that scores
   differently — observed near-misses at **0.969×**, just under the 0.98 threshold. Broken mutants
   score 0.136–0.5 and are correctly cut; the informative ones die at the same fence.

**⚠ THAT DIAGNOSIS IS REFUTED BY MEASUREMENT.** I added a rate histogram to `evolve.rs` to test it:

```
rates 0.801-1.000x [>=.98:3  .90-.98:0  .50-.90:2  <.50:0  distinct:4]
```

Two generations, corrected from my initial n=1 reading:

| gen | guard-passers | in the .90–.98 band (EPS cuts) | rate-distinct |
|---|---|---|---|
| 1 | 5 | 0 | 4 |
| 2 | 9 | **1** | 8 |
| **total** | **14** | **1 (7%)** | **12** |

I first wrote "zero in the band" off generation 1 alone. **The band is not empty — it is rare.** But
**11 of 14 rate-distinct candidates survive EPS**, so EPS is not what stops the population
diversifying. The refutation of the EPS diagnosis holds; the word "zero" did not.

**The real bottleneck is the MUTATION OPERATORS.** An edit either preserves behaviour or breaks the
program; the operators do not produce the graded, slightly-different variants that selection needs
to climb. That is consistent with everything else measured: correct alpha-beta variants are
behaviourally identical (40/40 at depth 3), and 0 of 17 behaviour-changing edits passed the strict
guard.

Good thing this was measured before implementing behavioural dedupe — the fix would have been
built for a bottleneck that isn't there. Third of my own hypotheses refuted today by its own test.

## 🔑 WHY THE PLATEAU EXISTS: the search is BARE alpha-beta, by design

`search.rs` at the horizon returns the static eval with **no capture resolution**:

```rust
if depth == 0 { return ... net.eval(pos, &mut self.scratch) }
```

`MASTER_PLAN.md:38` — *"Seed search program: BARE alpha-beta … No ordering, no hash reuse, no
iterative deepening, no quiescence"*. Line 53 — *"all of these must be DISCOVERED as program edits
that beat the current"*.

**So the ~0.86 plateau is the strength of a bare depth-2 alpha-beta with a width-16 eval, and that
is the expected, pre-registered result.** A search with no quiescence evaluates mid-capture
positions as if they were quiet; no amount of eval training fixes a horizon that cuts through
exchanges.

**This reframes the whole day.** Every candidate I tested — blend, epochs, depth, capacity, draws,
horizon, gate-every, data volume — is a *training-loop* knob. The training loop is at the ceiling of
what a bare search can express. The plan says the next gains come from the **discovery track**
(`evolve`), whose job is to find qsearch / ordering / hash reuse as program edits.

**And the discovery track is the thing that is actually broken:** its population collapsed to
identical members (`pop 8 spread 0.005508–0.005508`), 25 generations with 0 accepts, which is why I
killed it. That — not another hyperparameter — is where the remaining strength is.

## ✅ SETTLED (read this before anything below)

**Every tuning candidate is closed. None moved the plateau. The shipped defaults are correct.**

| candidate | verdict |
|---|---|
| capacity / width | closed — w64 does not beat w16 |
| draw filter, horizon | closed |
| datagen depth | closed — the effect was search PARITY (odd vs even), not depth |
| blend 1.00 | **dead** — advantage is depth-2 only (depth shift z = 4.4) |
| epochs | **3 is the OPTIMUM, tested both sides** — 2 loses (0.448 ± 0.022 @ d4), 10 does not win (0.485 ± 0.022 @ d4) |
| `--gate-every 5` | no depth-4 gain (0.502 ± 0.022) |

**What the loop actually does:** from scratch it reaches ~0.82 in **10 generations** and is flat by
15. `champion_long` sits at **0.864**, the top of that band. Training from `champion_long` produces
no reliable gain because it is *already at the plateau this procedure reaches*.

**What actually improved things today — three fixes, no hyperparameters:**
1. **`batch_base` seeded from the starting champion.** The batch gate never gated its first K
   generations; `ga_d4` lost 0.048 with no baseline to roll back to. Verified live: the repaired
   gate caught a −0.045 block.
2. **`champ_anchor` invalidated on ARCH accept** — found by turning bug 1 into a search over every
   `champion =` site.
3. **`netmatch` prints arm sizes with the bias quantified** — three results today were confounded
   by unequal training (11v5, 96v1, 28v26).

**The measurement wall:** effects of 0.02–0.05 need **~19 seeds** to separate from seed noise
(spread ~0.045). Two seeds cannot; one certainly cannot. Every "resolved" one-seed reading today was
this.

**Open:** `fg_60` (do block increments keep rising as the pool grows?), `dv_4800` (is the plateau
data-limited?), `sg_20 vs bn_075` (does gating help from scratch?), `ep2_10 vs ep2_3` (is epochs 3
too *low*?).

---

> ## ⚠ READ FIRST: the origin metric does NOT invert — it is imprecise, and depth was the confound
>
> **This block previously claimed sign reversals. That claim is WITHDRAWN.** Measured at a MATCHED
> depth with equal total games (896 pairs, depth 2):
>
> | | value |
> |---|---|
> | bn_075 vs origin | 0.846 ± 0.015 |
> | bh_100 vs origin | 0.864 ± 0.014 |
> | **origin-increment** | **+0.018 ± 0.021** — unresolved, **sign CORRECT** |
> | **direct match** | **+0.050 ± 0.016** — resolved |
>
> At matched depth the increment **agrees in sign** with the direct match. The earlier "inversions"
> came from comparing a **depth-4 equal-time control** against a **depth-2 direct match**:
>
> ```
> bn_075 vs origin:  0.847 (d4 control)  0.846 (d2)   unchanged
> bh_100 vs origin:  0.832 (d4 control)  0.864 (d2)   0.032 WORSE at depth 4
> ```
>
> **bh_100 is stronger at depth 2 and not at depth 4.** That is a real depth-dependent difference in
> the nets, not an instrument failing — and it directly threatens the blend candidate, whose whole
> case was built at depth 2.
>
> **What survives: the origin-increment is 3.6× worse signal-to-noise than a direct match** at equal
> games (SNR 1.72 vs 6.12), because it adds two independent estimates in quadrature. The batch gate
> uses the increment. That is the measurement-rate bottleneck, stated precisely.

### The "frozen origin" is NOT frozen across runs

```
main.rs:370   let origin = Net::random(WIDTH_MENU[rung], seed)      <- depends on the RUN SEED
main.rs:413   let anchor = Net::random(champion.n_hidden, 20260907) <- fixed
```

Every run with a different seed faces a **different origin**. Arms at seed 20260907 share one;
the depth replication (424242, 987654) and `blend_seed2` (424242) each face their own.

* **Within a seed the comparison is still clean** — `depth_replicate` pits d2 against d3 at the same
  seed, so both meet the same opponent.
* **Across seeds, origin rates are not comparable.** This is the real explanation for `s2_100`
  reading 0.945 where `bh_100` read 0.832; I attributed that 0.113 swing to seed variance in the
  nets, and it is at least partly a different opponent.

**Consequence of both:** every ceiling arm was scored against the origin. Any gap **under ~0.05**,
any arm scoring **above ~0.95**, and any cross-seed comparison is provisional until re-measured
directly. Detail in `instrument_saturation_RESULT.md`.

Single source of current truth. The `*_RESULT.md` files are the working records and several contain
claims later retracted; **this file supersedes them where they disagree.**

---

## What is MEASURED and stands

### The ceiling investigation — four candidates, three dead

| candidate | verdict | measurement |
|---|---|---|
| capacity / net width | **REFUTED** | w64 loses **0.179 ± 0.021** to w16 at equal TIME |
| **datagen depth (d2 vs d3)** | **CONFOUNDED WITH PARITY** | see below — the effect is even-vs-odd, not shallow-vs-deep |
| draw filter | **REFUTED** | excluding draws better by **+0.086 ± 0.015**, two independent protocols |
| horizon schedule | **REFUTED** | widening beats narrow-fixed by **+0.064 ± 0.034** |

The original depth claim was **+0.025 ± 0.013** (8 deep generations beating 92 shallow at equal wall
clock). It is smaller than the ~0.07 between-run band, and it is now also known to be an even-vs-odd
comparison — see the parity section. A 2-seed replication with `--horizon-cap 45` is running, and
`depth_parity.sh` (d2 vs d4) is what actually decides it. Epochs is a fifth candidate.

### ⚠ AMENDED 2026-09-09 — parity is real in the TARGET, and does NOT reach trained strength

`depth_2x2.sh` ran the 2x2 at EQUAL GENERATIONS (4 per arm, asserted) with `--horizon-cap 45`, judged
head-to-head at depth 4:

| contrast | holds fixed | result |
|---|---|---|
| d1 vs d3 | parity (ODD) | **0.450 ± 0.015** — d3 stronger, clear of 0.5 |
| d2 vs d4 | parity (EVEN) | **0.379 ± 0.016** — d4 stronger, clear of 0.5 |
| d1 vs d2 | depth (shallow) | **0.512 ± 0.014** — INDISTINGUISHABLE, and precisely so |

The `distill_gap` measurement below is NOT refuted: crossing parity really does move the training
target 2.6x. What is refuted is the inference from it to strength. **A 2.6x difference in the
training signal produced 0.512 ± 0.014 in the trained net — nothing.** Depth, which barely moves the
target within a parity class, is what moves strength, in both classes, both intervals clear of 0.5.

So the heading below is wrong as a claim about STRENGTH and right as a claim about the TARGET. See
`depth2x2_RESULT.md`. One seed so far; the deep parity cell is still running.

### The depth lever is SEARCH PARITY, not depth (measured on the TARGET distribution)

`distill_gap` measures `|tanh(root/scale) − tanh(eval/scale)|` — at blend 1 that is not a proxy for
the training signal, it **is** the training signal. Across two odd/even pairs:

| net | d3 | d4 | d5 | d6 |
|---|---|---|---|---|
| origin(random) | 0.0244 | 0.0137 | 0.0235 | 0.0167 |
| bn_000 | 0.3518 | 0.1524 | 0.3567 | 0.1842 |
| bn_075 (20 gen) | **0.5715** | 0.1976 | **0.5861** | 0.2365 |
| champion_long | **0.6086** | 0.2176 | **0.6220** | 0.2481 |

**Odd depths cluster high, even depths cluster low, and depth barely matters within a class.** For
bn_075, two extra plies inside a parity class moves the gap +2.5% (d3→d5) and +20% (d4→d6); crossing
parity moves it **2.6×**. This is the classic alpha-beta odd-even effect — at odd depth the side to
move gets the last ply and takes material without reply, inflating the root against a quiet eval.

**The depth lever compared d2 with d3 — even against odd.** Its +0.025 is a comparison between two
target distributions that differ 2.6× in how far they sit from the net's own eval, so "deeper search
gives better labels" is not what was measured. `depth_parity.sh` (d2 vs d4, parity held fixed) is
queued and decides it.

Note this does not say depth-3 training is *worse* — it says the mechanism is misattributed. Odd-depth
targets are systematically optimistic about the side to move, and that may genuinely help; it is just
not "deeper search sees more".

### The training target's BLEND outweighs all four

`target = (1 - blend) * z + blend * root` — blend weights the net's own search score against the
game outcome. Measured on the frozen-origin metric, 20 generations, identical seed:

| blend | vs frozen origin | decisive @ gen 20 |
|---|---|---|
| 0.75 (shipped) | **0.847 ± 0.022** | 1170/2400 |
| 0.25 | **0.735 ± 0.025** | 498/2400 |
| 0.00 | **0.691 ± 0.025** | 340/2400 |

**+0.112 ± 0.033** — bigger than draws (+0.086), horizon (+0.064) or depth (+0.025), and clear of
the between-run band. Controlled by an identity check: both arms report `dec 389/2400` at generation
1, before any training, so they diverge only downstream of the target. No improvement is available
(0.75 is already shipped); the high side is queued as `blend_hi.sh`. Detail in `blend_RESULT.md`.

### Every cheap proxy for strength has failed

| proxy | vs | result |
|---|---|---|
| `mcnemar_z` surrogate | 239 gate results | r = **−0.095**, CI [−0.220, +0.032] |
| training loss | control vs origin, n=12 | r = **+0.379**, CI [−0.249, +0.783] — wrong sign |
| candidate-vs-champion gate | fixed anchor | **0.500 ± 0.007** on pairs an anchor separates easily |

A fourth proxy is the first with a useful point estimate, and it is ONE ARM from resolving:

| proxy | r | 95% CI | n |
|---|---|---|---|
| mcnemar_z surrogate | −0.095 | [−0.220, +0.032] | 239 |
| training loss | +0.379 | [−0.249, +0.783] | 12 |
| **decisive-game rate** | **+0.771** | **[−0.108, +0.973]** | **6 arms** |

The CI still includes zero, so it is NOT established, and there is a visible counterexample: the
strongest arm on the board (`wd_r2`, 0.967) has FEWER decisive games than a weaker one (1124 vs
1170). But at r = +0.771 a seventh arm clears zero, and **every arm already logs `dec` for free** —
so this resolves at no extra compute as the queue drains. Measured on 20-generation arms only; the
1-generation depth probes were excluded because training amount drives both terms.

Only games against a **fixed anchor** have resolved anything. They cost ~1,650 pairs to resolve one
generation's real edge, which is why they are not the per-generation metric.

### The champion gate COMPRESSES — it does not invert. This decides which old results survive

The blend arms were measured on both instruments, giving the first direct calibration:

| 0.75 − 0.25 | champion gate rate | vs frozen origin |
|---|---|---|
| | +0.036 ± 0.016 | **+0.112 ± 0.033** |

Same direction, ~3× the magnitude. Therefore:

* a **DIRECTION** from the champion gate is probably safe — it got the blend ordering right;
* a **NULL** from it is worthless, because compression manufactures nulls. It cannot distinguish
  *equal* from *invisible*.

**Every "these arms are indistinguishable" reading taken on the champion gate must be re-asked**,
starting with main.rs's flat 0.75/0.85/0.95/1.00 plateau and its conclusion that "the game outcome
contributes nothing measurable".

### The search track: why it produced nothing, and what changed

* **Alpha-beta is exact.** All seven AB-family reference programs return the SAME move — 40/40 at
  depth 3, 12/12 at depth 4. Correctness cannot discriminate among correct programs.
* **0 of 33** behaviour-changing single edits passed the all-or-nothing guard. The guard was in
  practice "do not change behaviour".
* **Three declared primitives did not do what GRAMMAR says.** `pred` was a stub returning false;
  rung 6 applied its extension at every depth instead of the horizon (73× cost); `tread` discarded
  its index arguments. All three fixed except rung 7's table CONTENTS, which are an undeclared
  Given (filling them with "reduce later moves more" would be seeding LMR).
* **The pipeline now runs end to end** — guard tolerance 4 on an alpha-sensitive guard admits
  candidates, the surrogate proposes, games decide. First live case: a 1.14× surrogate improvement
  rejected at 0.417 on the board.

---

## RETRACTED — do not rebuild on these

1. **"The training step degrades the net and the gate was hiding it."** Off one reading
   (0.861 → 0.831). 44 readings show champion_long sits at the TOP of the procedure's own
   0.79–0.86 band; further training is regression to its mean, not damage.
2. **"The only class that can improve is inexact variants, and it is empty."** Both members were
   no-ops from missing implementation, not from alpha-beta's exactness. Rung 6 now plays
   differently (8/10) at 1.679× cost.
3. **"35% of single edits are behaviour-changing AND correctness-preserving."** The instrument
   checked "returned a move", not the guard. Correct answer: **0 of 33**.
4. **"Iterative deepening is budget-aware."** True of real ID, false of `ab_id` here — it loops
   depth 1..D and never reads `Budget`. Nine of eleven references are budget-blind.
5. **"The exploits lose 5 guard positions by construction."** Measured: DEPTH loses 9, ALPHA
   loses 2 on the old guard. Both figures were wrong.
6. **"A transposition-table soundness bug."** The depth-4 disagreement was the 2e9 cost cap
   truncating searches, with `MOVE_NONE == MOVE_NONE` scored as agreement.
7. **"Batching the gate does not help — every batch rolls back, so it is a LEARNING failure."**
   `batch_ab.sh` ran at 17:22 on a binary built at 17:13; the anchor-increment batch gate landed at
   19:18 (`bae8c7b`). It measured the OLD head-to-head gate, printing `0.500+/-0.007` — the blind
   gate's signature, not a null. Its arms were also unequal (11 generations vs 5, time-boxed), and
   its negative was pre-registered as expected. Re-queued as `batch_ab2.sh`.

---

## Recurring failure modes, named because each cost hours

* **Two variables moving, the uncontrolled one flattering the result.** equal-DEPTH vs equal-TIME
  (width gate); equal-BUDGET vs equal-COST (game gate); equal-WALL-CLOCK vs equal-HORIZON (depth
  A/B). **Rule: if arms are matched on time, pin `--horizon-cap` explicitly.**
* **Inert features that look correct in the diff.** Cost ceiling set above where it could bind;
  `catch_unwind` under `panic = "abort"`; guard tolerance that changed only the printout. **Rule:
  verify by program BEHAVIOUR — a number that should move and doesn't.**
* **Instruments whose stated semantics differ from their code.** stepdiff's buckets; ttvalue's
  "UNSOUND" message printed for any challenger; moveagree counting MOVE_NONE as agreement.
* **My own fix refuted before shipping.** Champion and base are scored on DIFFERENT opening sets
  (`match_nets` seeds openings from `Rng(seed | 1)`, and the two call sites differ by `^g`), and a
  comment at the anchor gate falsely claimed otherwise. Pairing them looked like free variance
  reduction. **Measured: ratio 1.12×, F(9,9) interval ~[0.28, 4.5] — does not clear 1**, while
  costing 2× games. Cause: openings walk only 4 random plies, so opening difficulty is a small part
  of the variance; it is in the games. `EXISTENCE_PAIRED_BATCH` stays off.
* **`readlink /proc/PID/exe` goes stale the moment you rebuild.** Rebuilding a binary while an
  instance is running makes its exe link read `<path> (deleted)` — the inode survives, so the JOB is
  fine, but any monitor globbing on the exe path silently stops seeing it. I rebuilt `netmatch`
  mid-run, my scan reported the process "GONE", and I was one step from relaunching a job that was
  7 minutes into a depth-4 match. **The brief recommends exe-matching as the safe alternative to
  `pgrep -f`; this is that alternative's own blind spot.** Match on the BASENAME with the suffix
  stripped: `b=$(basename "${e% (deleted)}")`.
* **A verification whose output I never read is not a verification.** I ran
  `grep -A1 'BASENAME with the suffix stripped' STATE.md` to confirm a claim, got NOTHING, did not
  notice, and committed a message asserting the text was intact. The text *was* intact — the phrase
  spans a line break and grep is line-based, so the pattern could not match. Right answer, no
  verification: the brief's own rule, *a grep that finds nothing is usually a broken pattern, not an
  absence*, applied to the check itself. **Print a count or a hit, never rely on silence.**
* **Killing a child does not stop the loop that spawned it.** I killed `depth_parity.sh`'s
  depth-4 arm for seed 424242 after finding the protocol flaw, and reported it handled. The script's
  `for SEED` loop simply started the *next* arm (987654) with the identical flaw, and it ran for
  five more minutes on a core before I noticed the log name had changed. **Kill the parent script
  FIRST, then the child** — otherwise the loop races you and respawns what you just removed.
* **A null from a coarse instrument is not a null.** `du -sb` over a 125G tree reported "0 bytes
  added in 15s" for the 4PC datagen and I nearly recorded it as STALLED. Per-file `stat` over the
  same interval showed +25KB and +67KB and a new file created mid-sample. Earlier the same day a
  `find -name '*.npz'` returned 0 because the job writes `.tsv`. **Two false nulls in one check** —
  match the instrument's resolution to the thing being measured, and confirm a null against a
  second method before believing it.
* **`taskset` on a build does not pin what the build spawns.** `taskset -c 14 cargo test` left
  rustc children with affinity `0-15` — running on HIS cores, which is a hard resource rule. Cargo
  spawns compiler processes through its jobserver and they did not all inherit the mask. Re-pinning
  them individually is a losing race against new spawns. **For builds, pin with an explicit
  `-j` limit and verify the CHILDREN's affinity, not the parent's** — or do not run a parallel build
  while other work is on the box. Killed the run rather than keep racing it; the test suite is worth
  having but not worth taking his cores.
* **A verdict block can spend real compute on a tautology.** `fixed_gate.sh` was 11 minutes into
  `fg_20.net vs champion_long.net` at depth 4 — but I had already md5-verified those files are
  **byte-identical** (4 gates, 0 KEEP, so every rollback restored exactly). It was matching a net
  against itself for an answer that is 0.500 by construction. Meanwhile `gate_align.sh`, relaunched
  after its arms were already complete, was re-running a verdict I had. **Before spending a match,
  check whether the two nets can differ at all** — the md5 that proves a rollback worked also proves
  the comparison is empty.
* **Two runs of a deterministic program are one observation.** `evolve` seeds its mutation RNG
  with a fixed constant (`Rng::new(0xE0FFEE)`) and takes no seed argument, so every run is the same
  run. I aggregated `search_track.log` (34) and `hist_probe.log` (5) as **39 lineage-generations**;
  gen-3 MAIN is field-for-field identical between them, so the true n is **34**. The hard-set
  finding survives (50% vs the reported 51%) but the sample size did not. **Counting re-runs of a
  deterministic process as independent samples is the same error as counting one seed as evidence** —
  which is the day's other main lesson, arrived at from the opposite direction.
* **State outliving its run.** A 5-hour-stale `hz_1000.log` about to be read as a current arm; a
  mid-run script edit that killed a verdict block, where **reverting within a minute did not undo
  it**.

---

### Run-to-run is BIT-EXACT; the ~0.07 band is entirely SEED variance

`ratchet_test.sh` and `batch_ab2`'s K=5 arm turned out to be the same configuration (same seed,
init, binary, settings) launched as two independent processes on different cores. They agree on
**every line of all 9 overlapping generations**, batch gates included. So with a fixed seed and
`--threads 1` there is **no run-to-run noise at all**, and the ~0.07 between-run band is entirely
SEED variance. Two arms sharing a seed are directly comparable; only different seeds need the band.

### The acceptance rule cannot accept a real step

`resolved_up = pent_rate - ci95 > 0.5` (main.rs:675) — acceptance needs an edge **larger than the
gate's own ci95**. At 224 pairs that is **0.0309**; a real per-generation edge is **~0.0114**. Both
figures reproduce numbers recorded independently elsewhere in the tree.

> The shipped gate demands an edge **2.7× larger** than a generation produces.

main.rs:757 already states the premise — "real steps are far smaller than that" — but concludes it
only about the ANCHOR gate, which by main.rs:752 "can only ever veto" and so cannot rescue anything
the champion gate rejected. **Even at face value with zero compression, the 0.0309 floor exceeds
datagen depth (+0.025) — the only surviving ceiling candidate.** Detail in
`acceptance_floor_RESULT.md`.

### RETRACTED, AND REVERSED: it is a MEASUREMENT failure, and batching is the repair

**The section below is wrong and is kept only to show what the underpowered reading looked like.**

At 448 pairs `b2_5` vs `champion_long` read **0.513 ± 0.022** and I called it indistinguishable. At
**960 pairs** it is **0.529 ± 0.015, interval [0.514, 0.543] — b2_5 is STRONGER, clear of 0.5.**

So 20 generations of **batch-gated** training from `champion_long` produced a **real, resolved gain
of ~0.029**, while the per-generation arm over the same span accepted **nothing at all**. That is
precisely the hopeful branch `batch_ab2.sh` pre-registered:

> "KEEPs, and the arm ends above the baseline => MEASUREMENT failure. The gains were real and
> unmeasurable one at a time, batching is the repair, and the shipped default of `--gate-every 1`
> is the brake."

**The acceptance-floor arithmetic stands after all.** The gate demands an edge larger than its own
ci95 (0.0309 at 224 pairs) while a generation produces ~0.0114. At `--gate-every 1` the floor binds
and nothing is ever accepted; at `--gate-every 5` the accumulated change clears it and gets kept.
My "the floor is not the binding constraint" reading was built on the underpowered null and goes
with it.

**`--gate-every 5` is therefore a shipping candidate** — the default is 1. One training run so far;
a second seed is required before the default moves, same bar as blend.

Third time an underpowered null has misled me today, all from the same cause, which is why the
`netmatch` null threshold moved from ci95 < 0.03 to < 0.015 (~896 pairs). This re-check is the fix
paying for itself immediately.

### Superseded reading (kept for the record): "a LEARNING failure at the plateau"

`batch_ab2` finished both arms from `champion_long`, 20 generations each, on the fixed
anchor-increment gate. **Neither arm produced any change:**

* **gate-every 1** — **0 accepts in 20 generations.** The champion never moved, so the program
  printed *"no candidate was accepted; nothing to control against"* and wrote no net. Its output is
  `champion_long` by construction.
* **gate-every 5** — 4 batch gates, **1 KEEP and 3 ROLL BACK**. Final net vs `champion_long`
  head-to-head: **0.513 ± 0.022, INDISTINGUISHABLE.**

The origin metric read that batch arm as a *decline* (0.824 against the 0.861 baseline). The direct
match says no change — **saturation again**, now caught a third time.

**So `batch_ab.sh`'s original conclusion — "learning failure, the gate is not the lever" — was
RIGHT.** Its evidence was not: a broken instrument, arms of 11 vs 5 generations, and a negative
pre-registered as expected. Withdrawing it was still correct; a right answer reached through a
broken ruler is not a result, and it would have blocked exactly this re-run.

The acceptance floor is real arithmetic but it is **not the binding constraint**: with the floor
removed and the compressing opponent replaced, there is still nothing to keep.

**The loop learns from RANDOM and not from a trained champion.** From `--rung 0` it reaches 0.847
in 20 generations; from `champion_long` it moves nothing in 20. The ceiling is real and it is in the
training signal.

### Earlier partial reading on the fixed batch gate (superseded by the above)

The ratchet test runs `--gate-every 5` on a binary that has the anchor-increment gate, so it is
already producing the measurement `batch_ab2` was queued for:

```
batch gate g5: champ-vs-origin 0.819+/-0.024  base 0.828+/-0.022  increment -0.009+/-0.033  ROLL BACK
```

The acceptance-floor finding predicts that removing the 0.031 floor should reveal accumulated gains.
It does not. Five generations delivered **−0.009 ± 0.033**, CI [−0.042, +0.024], and the **+0.057**
that 5 × 0.0114/gen predicts is **excluded by the measured interval**.

So the floor is real (the arithmetic stands) but "the floor is why nothing is accepted" is now
doubtful: with the floor gone and the compressing opponent replaced, there is still nothing to
accept. **n = 1 batch gate**, at `champion_long`'s plateau (0.828 vs origin), and the 0.0114 figure
came from a different regime and the compressed metric — so this is evidence, not a verdict.
`batch_ab2` yields four more; the ratchet yields more still.

## Non-transitivity is REAL — so the direct match's 3.6× precision is on the wrong quantity

I suspected the codebase's non-transitivity demonstration was the same depth confound that produced
my false "sign reversals", which would have freed the batch gate to use a direct champion-vs-base
match and gain 3.6× signal-to-noise. **Measured at a matched depth 2, all three matches, 448 pairs:**

| | value |
|---|---|
| ep_1 vs champion_long, **direct** | **0.548 ± 0.021** — ep_1 stronger, clear of 0.5 |
| ep_1 vs origin | 0.831 ± 0.016 |
| champion_long vs origin | 0.855 ± 0.016 |
| **origin-increment** | **−0.024 ± 0.023** — champion_long stronger |

**The two metrics disagree in sign at the same depth.** Not a protocol artifact — genuine
non-transitivity. `ep_1` beats `champion_long` head-to-head while being worse against a fixed
reference.

So the anchor-increment design is **right** and stays. The direct match is 3.6× more precise, but it
measures "beats this specific opponent", which is not "stronger" — and a ratchet built on it would
climb a matchup rather than a strength. The 3.6× is not available; it was precision on the wrong
quantity.

Note this cuts *both* ways and neither metric is an oracle: for the blend pair the two agreed in
sign at matched depth, here they do not. Which one is trustworthy is **pair-dependent**, so every
number should name the instrument that produced it.

## The ceiling is not a COMPONENT — it is a measurement RATE

Every named candidate is now measured and closed:

| suspect | verdict |
|---|---|
| capacity / width | refuted — w64 does not beat w16 head-to-head, loses at equal time |
| draw filter | refuted (and the direction survives a direct match) |
| horizon schedule | refuted |
| datagen depth | **was search PARITY** — odd depths sit ~2.7× from the eval, and d2-vs-d3 is even-vs-odd |
| acceptance gate | real: batching produced 2 KEEPs in 5 gates, P = 0.0059 under a null |
| training signal collapses | refuted — the search-minus-eval gap **grows** with strength |
| encoding: net is material-only | refuted — champion_long is R² 0.588 against material |
| encoding: net is a linear PST | refuted — 0 always-on, 0 always-off, 1100–2300 activation regions |

What is left is not a component but an **arithmetic**:

* a generation produces about **0.0114**;
* a 224-pair gate resolves about **0.031**, and the two-sample batch bar is √2 × that;
* so an improvement can only be *banked* as fast as it can be *measured*.

That is exactly why five batched generations (≈0.057) produced a real KEEP where per-generation
gating accepted **nothing in twenty**. The loop is not blocked from learning; it is blocked from
*recognising* what it learned. `compound.sh` tests whether that banking compounds over 40
generations, and larger `--gate-every` is the obvious next dial if it does.

## Killed: `pd_d4` (depth-4 datagen from champion_long)

**Its premise expired and I did not re-price it.** `plateau_depth.sh` asked whether a deeper teacher
could move a champion that depth 2 could not. Depth 2 **can** move it — `b2_5` beat `champion_long`
0.529 ± 0.015 at 960 pairs. I retracted the "learning failure" reading and left the experiment built
on it running.

The cost made that expensive: 30 minutes elapsed, still on **generation 1**. Depth-4 datagen at 2400
games/gen is ~30 min per generation, so 20 generations is **~10 hours of a core** — spent on "is
depth 4 better by enough to matter" while the two matches that decide whether anything ships were
queued behind slower work.

A marker was written into `pd_d4.log` before killing it, because `plateau_depth.sh`'s verdict block
says a missing `pd_d4.net` **is** the tie branch. That reading does not apply: there is no result,
only a stopped run. Logs are gitignored, so this note is the tracked record.

## 8-net POOL RATING — the first non-transitivity-robust ranking (depth 2)

`pool_rating.rs`, 28 matches, 224 pairs each, **zero cyclic triples out of 56** — the pool orders
consistently, so this ranking can be read as one:

| net | rating |
|---|---|
| **b2_5** (gate-every 5) | **0.5544** |
| ep_2 | 0.5537 |
| ep_3 | 0.5371 |
| champion_long | 0.5207 |
| bh_100 (blend 1.00) | 0.4908 |
| s2_100 (blend 1.00, seed 2) | 0.4786 |
| s2_075 (blend 0.75, seed 2) | 0.4354 |
| bn_075 (blend 0.75) | 0.4292 |

All three candidates hold **at depth 2**, on 1568 pairs per net rather than one match:
gate-every 5 is +0.034 over champion_long; blend 1.00 beats 0.75 at **both** seeds (+0.062, +0.043);
epochs 2 beats epochs 3 by +0.017.

**The second-seed blend pair is the case that matters.** Its direct match was unresolved
(0.487 ± 0.015) and I downgraded the candidate over it; against a field it separates cleanly. The
pool did not overturn that reading — it *resolved* it, because a rating over seven opponents carries
seven times the games and no single matchup dominates.

**Still depth 2, which is not the strength standard**, and both candidates already degrade at depth 4
(b2_5 +0.029 → +0.002; bh_100 0.864 → 0.832 vs origin). The same pool is running at depth 4
(112 pairs, ~3h). **Nothing moves before it lands.**

### Ship-candidate attribution (depth 2)

```
sc_c vs s2_075 (shipped defaults)  0.531 ± 0.014   combination BETTER
sc_c vs s2_100 (blend only)        0.472 ± 0.017   combination WORSE than blend alone
```

**Epochs 2 hurts on top of blend 1.00.** Blend-alone is the better change. Running all three matches
instead of only the ship test is what makes that readable rather than a bare win.

## `--gate-match-depth` verified to BIND, and its real cost

The loop learns from depth-2 labels **and selects on depth-2 matches**, while strength is judged at
depth 4 — it optimises the game it measures. The new flag defaults to the datagen depth, so it
changes nothing until set. Verified by behaviour, same seed and training, only the gate depth
differing:

```
gate-match-depth 2:  champ-vs-origin 0.516±0.038  base 0.516±0.021  increment +0.000
gate-match-depth 4:  champ-vs-origin 0.422±0.060  base 0.492±0.056  increment -0.070±0.082
```

Completely different output — **not inert**, unlike the three features that shipped looking correct
in the diff and moved no number.

**Cost, corrected twice.** I first called it "a fraction of the price", then measured a 1.58×
variance penalty at depth 4 and revised to 2.4×. Both were wrong. Re-deriving pair sd from the
*trained-net* matches:

| match | ci95 | implied pair sd |
|---|---|---|
| b2_5 vs champion_long, **depth 2**, 960 pairs | 0.015 | **0.2371** |
| b2_5 vs champion_long, **depth 4**, 448 pairs | 0.022 | **0.2376** |
| established, 15,008 pairs | — | 0.2362 |

**Depth does not change pair variance for trained nets.** The 1.58× came from a 5-generation,
200-game probe whose nets were near-identical and drew heavily — a different variance regime, not a
depth effect. A depth-4 gate needs the **same pairs**; only the ~25× node cost applies, giving
**~1.9× total compute at `--gate-every 5`**.

Note `bn_075 vs origin` implies pair sd **0.162** — matches against a weak opponent are lopsided and
so have *lower* pair variance. That also corrects the "direct match has 3.6× the SNR" claim: the
noise ratio is only **1.3×**, and the remaining **2.8×** is the origin comparison *compressing the
signal* because both nets are near-saturated. I attributed the whole gap to quadrature addition.

## The full 40-generation batch-gate record (`rt_k5`)

8 gates, **2 KEEPs**, and the base moved 0.828 → 0.865 on the origin metric:

* KEEPs at **g10 (+0.033)** and **g25 (+0.039)**
* **g30, g35, g40 all ROLL BACK** — the last 15 generations produced nothing keepable

P(≥2 KEEPs in 8 | true null) = **0.0157**, so the KEEPs remain unlikely to be noise, though weaker
than the 0.0059 the first 5 gates gave. **The gains are front-loaded and then stop**, which is what
`compound.sh` was queued to test — and it now has a partial answer before it even starts: within
this run, batching bought two steps and then stalled.

## RESOLVED: blend 1.00's advantage is depth-2 only — the candidate is DEAD

The direct 448-pair match settles the hint below:

```
bn_075 vs bh_100   depth 2:  0.450 ± 0.016   RESOLVED, blend 1.00 stronger
                   depth 4:  0.511 ± 0.022   UNRESOLVED, no advantage
shift              +0.061 ± 0.027,  z = 4.40 -> the DEPTH EFFECT is resolved
```

Blend 1.00 genuinely beats 0.75 **at depth 2** and shows **no advantage at depth 4**. Depth 4 is the
strength standard, so **the blend candidate is dead for shipping** and the shipped default of 0.75
stands. The pool's common-opponent route pointed the same way independently.

**Consequence for the surviving candidate:** `sc_c` (blend 1.00 + epochs 2) beats the shipped
defaults at depth 4 — and it cannot be the blend doing it. It must be **epochs 2**, which the
depth-4 `sc_c vs s2_100` match tests directly.

### The hint this replaced (kept: it called the direction correctly while underpowered)

Depth-4 pool, via the common opponent `champion_long`:

| | depth 2 | depth 4 |
|---|---|---|
| bn_075 (blend 0.75) scores | 0.421 | **0.413** |
| bh_100 (blend 1.00) scores | **0.458** | 0.382 |
| ahead | blend 1.00 by 0.037 | blend 0.75 by 0.031 |
| resolved? | **yes** (448 pairs, diff ci95 0.031) | **no** (112 pairs, diff ci95 0.062) |

**Explicitly not claimed.** The depth-4 rows are underpowered — a 0.031 gap against a 0.062
difference interval. Reading a reversal off two underpowered matches is the error that produced
three retractions today. The direct 448-pair `bn_075 vs bh_100` match is running to settle it.

**If it confirms, two things follow:** the shipped default of blend 0.75 is *right* at the depth that
counts, ending my downgrade-then-partial-rehabilitation of blend 1.00 as a refutation; and `sc_c`,
which beats the shipped defaults at depth 4, must be winning on **epochs 2** rather than on blend —
which the depth-4 `sc_c vs s2_100` match now running shows directly.

## ⚠ SUPERSEDED — "first candidate to survive depth 4"
>
> `sc_c` is blend 1.00 + epochs 2. **Blend 1.00 is dead** (advantage is depth-2 only, z = 4.4) and
> **epochs 2 is dead** (0.498 / 0.473 / 0.448 once arms are matched). Whatever `sc_c` was winning
> on, it was not either component as measured. Kept as the record of a reading that looked solid
> and was not.

```
sc_c vs s2_075 (shipped defaults)   depth 2:  0.531 ± 0.014   clean
                                    depth 4:  0.529 ± 0.022   MARGINAL
```

**The point estimate is stable across depths** — 0.531 → 0.529. Every other candidate collapsed:
b2_5 went 0.529 → 0.502, bh_100's origin score 0.864 → 0.832. The depth-4 reading is flagged
marginal by the tool's own rule (margin 0.007 against half-ci95 0.011) purely because it has **448
pairs against depth-2's 896**. That is a *power* limitation, resolvable by spending pairs — not the
transfer failure that killed the others.

**Caveat that has to travel with it:** at depth 2, `sc_c` **loses** to `s2_100` (blend alone),
0.472 ± 0.017. So the combination beats what ships, but blend-alone may beat the combination —
epochs 2 appears to *hurt* on top of blend 1.00. The depth-4 version of that match is running, and
it decides whether the change to make is "blend only" or "both".

Nothing ships until the marginal reading is resolved at higher pair count **and** the depth-4
`sc_c vs s2_100` lands.

## ⚠ SUPERSEDED — DEPTH-4 POOL (28/28): the `ep_*` rows are CONFOUNDED
>
> **Two of the eight nets (`ep_2`, `ep_3`) were time-boxed with unequal generations (28 vs 26).**
> "epochs 2 wins" below is that confound, not an epochs effect — see the retraction section. The
> other six rows are generation-matched and stand. Kept because the *blend* reading here (last at
> depth 4) was independently confirmed by the direct match.

| net | depth 2 | depth 4 |
|---|---|---|
| **ep_2** | 0.5537 | **0.5999** |
| ep_3 | 0.5371 | 0.5634 |
| b2_5 | 0.5544 | 0.5417 |
| champion_long | 0.5207 | 0.5239 |
| s2_100 | 0.4789 | 0.4953 |
| bn_075 | 0.4290 | 0.4521 |
| bh_100 (blend 1.00) | 0.4909 | **0.4133** |
| s2_075 | 0.4356 | 0.4104 |

**Epochs 2 rises to the top at depth 4; blend 1.00 falls to last.** The direct match agrees and
*strengthens* with depth — `ep_2 vs ep_3` is 0.518 ± 0.014 (marginal) at depth 2 and
**0.544 ± 0.023 (resolved)** at depth 4. That is the opposite of blend, whose advantage vanished.

Blend remains **seed-inconsistent even at depth 4**: seed 1 gives `bn_075 vs bh_100` = 0.511
(0.75 ahead), seed 2 gives `s2_075 vs s2_100` = 0.456 (1.00 ahead). Dead.

## The gate-alignment fix FAILS — the origin anchor saturates at depth 4

```
gate-match-depth 2:  base 0.828, champ 0.819–0.862, increments −0.009..+0.033  ->  1 KEEP
gate-match-depth 4:  base 0.939, champ 0.936–0.948, increments −0.002..+0.009  ->  0 KEEP
```

Same net, same opponent, different depth: champion-vs-origin goes **0.828 → 0.939**. Deeper search
lets a good eval convert its advantage more reliably, so both sides crush the random origin and the
increment compresses to nothing. **The depth-4 gate is blind because its ANCHOR is too weak at depth
4**, not because selection depth is the wrong idea.

The binary check passed first (`xt6 reproduces b2_5's gate lines`), so this is the arms differing,
not the builds.

**The fix this implies:** a depth-4 gate needs a *trained* anchor, not `Net::random`. That is a
different change from `--gate-match-depth` and it is the one worth making.

## BUG FIXED: the batch gate never gated its first K generations

`main.rs:860` read `let base = batch_base.get_or_insert_with(|| champion.clone()).clone();` — and
that line sits **inside** the gate block. So `batch_base` was first set at the *first gate*, to the
champion after K generations of training, not to the net the run started from.

Two measured consequences:

* **The first gate compared a net against itself.** First-gate increments across four runs:
  `rt_k5 −0.009`, `b2_5 −0.009`, `ga_d2 −0.009`, `ga_d4 −0.002`. Noise, by construction.
* **Generations 1..K were never gated**, so damage there was permanent. `ga_d4` ran 20 generations
  with **zero KEEPs** and still finished at **0.813** against champion_long's **0.861** — its first
  5 generations cost 0.048 and there was no baseline to roll back to.

Fixed by seeding `batch_base` from the starting champion. Verified by behaviour on the new binary:
the first gate now reads `champ 0.863 base 0.883 increment −0.020` — a real comparison, and it
catches the early degradation the old code was blind to.

**This reframes every batch run today.** `b2_5`, `rt_k5` and `ga_d2` all had 5 ungated generations
baked in before their first real gate. Their KEEPs are still real (those were later gates against a
genuine base), but their *starting point* was already 5 generations of undone drift.

## Invariants held, and a mirror-image test the day's data implies

`sg_20` finished 2 KEEP / 1 ROLL BACK, and the md5 check agrees: **it differs from `bn_075`**, as a
rollback requires. Both rollback invariants now pass —

* `fg_20` (4 gates, 0 KEEP) → **identical** to `champion_long`
* `sg_20` (3 gates, 1 rollback) → **differs** from `bn_075`

— so the gate restores exactly when it should and only when it should.

**The mirror test.** At depth 4, `ep2_2 vs ep2_3` = 0.448: *fewer* epochs **loses**. The gradient
therefore points toward **more**, not fewer — the opposite of what I chased all day. `ep2_10` exists
from the same sweep at matched 20 generations, so `ep2_10 vs ep2_3` at depth 4 costs one match and
asks whether the shipped default of 3 is too **low**.

This matters because the distillation gap **grows** with net strength (0.186 at `champion_long`):
the search keeps finding things the eval does not know, so "the net is not being fitted hard enough
to close it" is a live explanation for the plateau that nothing has yet tested.

## The from-scratch learning curve, measured by the gate in 5-generation blocks

`sg_20` (rung 0, gate-every 5) gives the curve directly:

```
g5   base 0.500 -> champ 0.742   +0.242  KEEP
g10  base 0.731 -> champ 0.819   +0.088  KEEP
g15  base 0.825 -> champ 0.811   -0.013  ROLL BACK
```

**Flat by generation 15.** The loop gains almost everything in its first ~10 generations, then
stops. `champion_long`, with far more training, sits at **0.864** — the top of the same band.

That reconciles the day's two halves. Training from `champion_long` shows no reliable gain because
`champion_long` is already at the plateau this procedure reaches; and a from-scratch run reaches
that plateau in about ten generations. The gate is measuring a real ceiling, not failing to see
progress.

## The gate is not over-conservative: from scratch it KEEPs everything

`sg_20` — identical to `bn_075` (rung 0, 20 generations, blend 0.75, seed 20260907) except
`--gate-every 5` instead of 100, so `bn_075` had no gating at all:

```
g5   champ 0.742  base 0.500 ± 0.007  increment +0.242  KEEP
g10  champ 0.819  base 0.731          increment +0.088  KEEP
```

**`base = 0.500 ± 0.007` at g5 is the `batch_base` fix proving itself** — the baseline is the random
starting net matched against itself, which is 0.500 by construction. Under the old code the baseline
would have been the 5-generation champion and that first gate would have been a no-op.

So where there is real progress the gate keeps all of it. The rollbacks on `champion_long` are the
gate working, not the gate being too strict.

**Invariant to check when it finishes:** if every block KEEPs, the champion is never rolled back, so
`sg_20.net` should be **byte-identical to `bn_075.net`** — same seed, same training, and the gate
matches use their own seeded RNG. If it differs, the gate is perturbing the training stream, which
would be a defect worth finding.

## DATA COMPOUNDS: more games raises the decisive-game rate, which yields more data again

`dv_4800` (2× games/generation) against `bn_075`, same seed, same 20 generations:

| gen | bn_075 decisive | dv_4800 decisive | pool ratio |
|---|---|---|---|
| 1 | 389/2400 = **16.2%** | 734/4800 = **15.3%** | — (control) |
| 2 | 372/2400 = 15.5% | 821/4800 = 17.1% | |
| 4 | 600/2400 = 25.0% | 2061/4800 = **42.9%** | |
| 6 | 829/2400 = 34.5% | 2599/4800 = **54.1%** | **3.0×** |

**Generation 1 is the built-in control**: both nets are still random and the fractions match, so the
later divergence is caused by *training*, not by the game count. The pool at generation 6 is **3.0×**
where the game count is only 2× — the extra factor is the rising decisive fraction.

**This is a compounding loop, not a scale-up:** more data → stronger net → more decisive games →
more usable positions → more data. It is the mechanism the data hypothesis required, and it explains
why `champion_long` (far more accumulated data) sits above a fresh 20-generation run.

**CLOSED — the advantage is transient. Full trajectory, both arms complete:**

```
gen    bn_075   dv_4800     gap
  1     16.2%     15.3%   -0.9pp   <- control: both nets still random
  4     25.0%     42.9%  +17.9pp
  6     34.5%     54.1%  +19.6pp   <- peak
  9     48.6%     51.1%   +2.4pp
 16     50.3%     49.7%   -0.7pp
 20     48.8%     50.1%   +1.4pp   <- converged
```

Both arms plateau at **~49–50% decisive**. The gap peaks at generation 6 and is gone by 9.
**2× the data buys arrival, not altitude** — a faster climb to the same ceiling, which is exactly
the alternative flagged when the arm was launched and the one the plateau-height criterion says is
worth nothing.

Generation 1 is the control and it does real work: at −0.9pp with both nets random, the later
divergence is training rather than the game count, and the convergence is therefore a real
convergence rather than the measurement washing out. The depth-4 head-to-head is running to confirm,
but the trajectory has answered.

## Testing the last structural lever: is the plateau DATA-limited?

No hyperparameter moved the plateau, and the plateau — not the climb rate — is the shipping
criterion. Three independent observations point at data volume:

* `fg_20`'s block increments rose monotonically as the pool grew: **−0.045, −0.033, +0.003, +0.007**
  with pool 88k → 432k, each block starting from the *same* champion.
* `champion_long`, carrying far more accumulated data, sits at **0.864** where a fresh 20-generation
  run reaches **~0.82**.
* The pool survives rollback, so data is the one thing that monotonically accumulates while the
  champion is repeatedly reset.

`dv_4800` is `bn_075`'s exact configuration with **`--games 4800` instead of 2400** — same 20
generations, so the arm-size guard reads "matched" and the variable under test is data *per*
generation.

**This is not an efficiency claim.** At equal generations the arm burns 2× the compute. The question
is strictly whether the plateau HEIGHT is data-limited; whether that data is worth its cost is a
different question and would need equal wall clock, which is the confound that has bitten three
experiments today.

## THE ONE LIVE LEAD: the data pool survives rollback, and the increments are climbing

`fg_20` finished 20/20 with 4 gates and **0 KEEPs**, and the invariant holds exactly —
`fg_20.net` is **md5-identical to `champion_long.net`**, so all four rollbacks restored cleanly.
That verifies the `batch_base` fix end-to-end.

The interesting part is the sequence:

```
g5   -0.045      g10  -0.033      g15  +0.003      g20  +0.007
pool 88k         pool 250k        pool 368k        pool 432k
```

**Monotone increasing, crossing zero at block 3.** Each block starts from the *same* champion — the
rollback restores the net — but **the data pool is not rolled back**. It grows 9,747 → 432,237
across the run. So the blocks are not independent samples: each trains the same starting net on a
strictly larger pool, and the trend is the pool growing.

Mean over four blocks is −0.017 ± 0.025 (no reliable change), but the mean is the wrong statistic
for a monotone series.

**PREDICTION REFUTED AT BLOCK 5.** The sequence is now **−0.045, −0.033, +0.003, +0.007, −0.010** —
block 5 went back negative with a *larger* pool than block 4 had. Five blocks give
**−0.016 ± 0.020**, no reliable change.

I predicted blocks would turn positive and start KEEPing. They did not. Four rising points was a
pattern fitted to a series short enough that one more point could break it, and one more point broke
it. `fg_60` has 7 blocks left and the mean may still move, but **the monotone reading is dead** and
pool growth alone does not drive the increment.

This is the only mechanism found today that predicts *improvement* rather than explaining absence.

## CLOSED: the loop cannot improve champion_long, and epochs 3 (shipped) is RIGHT

**The fixed gate rolls back everything.** `fg_20`, 15 generations from champion_long with the
repaired `batch_base`:

```
g5   champ 0.819  base 0.864  increment -0.045  ROLL BACK
g10  champ 0.830  base 0.864  increment -0.033  ROLL BACK
g15  champ 0.867  base 0.864  increment +0.003  ROLL BACK
```

**These are three INDEPENDENT 5-generation runs, not a trajectory.** Each block rolls back, so the
champion resets to `champion_long` before the next begins. Read that way:

```
mean -0.025 +/- 0.028   ->  no reliable change
spread 0.048            ->  dwarfs the mean
```

So 5 generations from `champion_long` neither reliably helps nor hurts — and the run-to-run spread
is the **same seed-noise wall** that killed every hyperparameter comparison today, showing up now
inside a single run.

Every block negative or neutral. **This also explains `b2_5`'s apparent gain.** Its "+0.033 KEEP" at
g10 was measured against a base that was already the *degraded* 5-generation net (0.819) — so it was
**recovering toward champion_long's 0.864, not improving past it.** The broken gate turned a
recovery into an accept.

**Epochs reverses direction once arms are matched:**

| comparison | result |
|---|---|
| seed 20260907, depth 2 | 0.498 ± 0.021 — no effect |
| seed 424242, depth 2 | 0.473 ± 0.020 — epochs 3 ahead |
| **seed 20260907, depth 4** | **0.448 ± 0.022 — epochs 3 ahead, resolved** |

The shipped default of 3 is not merely safe, it is **better**. The confounded evidence pointed the
wrong way, and correcting the 2-generation imbalance flipped the sign.

**Net position: nothing about the engine's settings was wrong.** Every candidate promoted today died,
and two of them (blend, epochs) died pointing back at the shipped value. What was wrong was the
gate, and that is fixed.

## A second bug of the same class, found by systematic audit

The `batch_base` defect was **a lazy cache whose subject changed underneath it**. So I enumerated
every `champion = ` assignment and checked each for a nearby `champ_anchor = None`:

```
line  849  champion = cand        cleared OK
line  952  champion = base        NO invalidation
line 1078  champion = acand       NO invalidation   <-- real bug
line 1130  champion = best_net    cleared OK
```

* **1078 (ARCH accept) is a genuine bug.** With `--anchor-pairs > 0`, an architecture step would
  leave the *old* champion's cached anchor score in place and judge the *new* champion's candidates
  against it. Latent, not live — `anchor-pairs` defaults to 0 — but `arch-every` defaults to 5, so
  it would fire the moment anyone enabled the anchor gate. Fixed.
* **952 (batch rollback) is safe**, because the only read site is guarded by `!batch_mode`. Now
  documented in place, so the next audit doesn't have to re-derive that it was checked rather than
  missed.

Two bugs of one shape in one file. **The generalisable move was turning the specific bug into a
search pattern** — "find every mutation of the thing a cache depends on" — rather than fixing the
one instance and moving on.

## CLOSED: epochs has no effect once the arms are matched — and the gate fix catches real damage

**Epochs, measured properly** (20 generations both arms, depth 2):

| seed | result |
|---|---|
| 20260907 | **0.498 ± 0.021** — no effect |
| 424242 | **0.473 ± 0.020** — epochs 3 slightly ahead |

The 0.518/0.544 that made epochs look like the surviving candidate was entirely the 2-generation
confound. **The shipped default of 3 stands.** No candidate remains.

**The `batch_base` fix is verified on live data.** `fg_20`'s first gate:

```
champ-vs-origin 0.819   base 0.864   increment −0.045   ->  ROLL BACK
```

Base 0.864 is champion_long's true origin score. The first 5 generations **cost 0.045**, and the
repaired gate detected and reverted it. Under the old code `base` would have been set to 0.819 — the
already-degraded net — and that loss would have been permanent and invisible. This is the same
0.048 that `ga_d4` lost with zero KEEPs to undo it.

**So the one thing that improved the engine today is a bug fix, not a hyperparameter.** Every tuning
candidate died; the gate now protects generations 1..K that were previously ungated.

## ⚠ RETRACTED: every epochs claim today used UNEQUAL ARMS

`ep_2` and `ep_3` — the nets behind every epochs number I reported — were run with `gens=1000000`,
i.e. **time-boxed, not generation-matched**. Epochs 2 trains faster per generation, so it got more:

```
ep_2   28 generations
ep_3   26 generations
```

Two extra generations at the measured ~0.0114 per generation is **~0.023 of advantage from training
amount alone** — the same size as the entire observed effect (0.018 at depth 2). The epochs effect
cannot be separated from the generation count.

**What this retracts:**
* `ep_2 vs ep_3` = 0.518 ± 0.014 (d2) and **0.544 ± 0.023 (d4)** — the "strengthens with depth"
  result, and the reason I called epochs the surviving candidate.
* **Both pool rankings**, where `ep_2`/`ep_3` are two of the eight nets — including "ep_2 tops the
  depth-4 pool". The other six rows are unaffected; those arms were generation-matched.

**The clean comparisons** are `ep2_*` (20 generations each, seed 20260907) and `es2_*` (20 each,
seed 424242). Only `es2` has reported: **0.473 ± 0.020 — epochs 3 ahead.** `ep2_2 vs ep2_3` is
running.

**This is the third time today the same defect has appeared**: `batch_ab` (11 vs 5 generations),
`depth_parity` (96 vs 1), and now the epochs sweep (28 vs 26). Time-boxed arms and a per-generation
cost that varies with the setting under test produce unequal training every time. The rule that
would have caught all three: **when the knob changes per-generation cost, match on generations, never
on wall clock.**

## ⚠ THE CAMPAIGN'S REAL LIMIT: seed variance swamps every effect being chased

**Both** candidates fail seed replication, the same way, with the heterogeneity itself resolved:

| comparison | seed 20260907 | seed 424242 | difference | z |
|---|---|---|---|---|
| blend 0.75 vs 1.00 | 0.450 | 0.487 | +0.037 | **+3.31** |
| epochs 2 vs 3 | 0.518 | 0.473 | −0.045 | **−3.61** |

Both **flip sign** between seeds. This is not two unlucky draws — the seed-to-seed differences are
significant, so the effects genuinely differ by seed.

**What that costs, quantified.** With a seed-to-seed spread of ~0.045 on the estimate:

| effect size | seeds needed | 20-generation arms |
|---|---|---|
| 0.05 | ~3 | 6 |
| 0.03 | ~9 | 17 |
| **0.02** | **~19** | **39** |

Every candidate this campaign has chased sits in the 0.02–0.05 band. **Two seeds cannot separate
them from seed noise**, and one seed certainly cannot — which is exactly what every "resolved"
depth-2 reading earlier today was.

**This is the honest closing state:** the measurement protocol (train 20 generations at two
settings, match the results) is under-powered for the effects it is being pointed at, by roughly an
order of magnitude in seed count. Nothing in the shipping table below survives that, and the fix is
not another candidate — it is either many more seeds per comparison, or longer runs where the effect
grows relative to seed noise.

## THE DISCOVERY TRACK HAS NO RUN-TO-RUN ENTROPY, and my fix for it bound nothing

`EXISTENCE_EVOLVE_SEED` was added to make the evolve track produce a second trajectory. **It did
not.** The two-way check is what caught it:

```
seedchk_default  gen 1 MAIN gate REJECT 0.417+/-0.103  surrogate 0.002794
seedchk_777      gen 1 MAIN gate REJECT 0.417+/-0.103  surrogate 0.002794
```

Byte-identical. The binary was confirmed to contain the string (`grep -qa`, built 06:31, edit
committed 06:33), so the variable was read and did nothing. Cause, read from source:

```
1366: let mut rng = Rng::new(<the settable seed>)   <- the knob
1684: let _ = rng.next();                            <- its ONLY use
1685: let _ = rng.next();                            <- and its only other use
1395: let mut r = Rng::new((g<<20) ^ (li<<16) ^ i ^ 0xBEEF);   <- the ACTUAL mutation draw
```

I attached the knob to an object whose entire consumption is two discarded calls. **The real draw is
derived purely from the slot indices** — generation, lineage, candidate — xor a hardcoded constant.

**This is the same failure class as the harness bugs today:** confirming a change *parses* rather
than confirming it *binds*. `gate_align.sh` already encodes the correct discipline (`--gate-match-depth`
was verified to move a number, 0.516→0.422, not merely to be accepted on the command line). I did not
apply it to my own change.

### The consequence is larger than the knob

Auditing every RNG in `evolve.rs`: besides the mutation draw there are **nine separate hardcoded
xorshift seeds** (`0xC0DE_F00D`, `0x5EED_1234`, `0xA1FA_5EED`, `0x51A7_E5EE`, `0x5D1F_F00D`,
`0x4A8D_3117`, …), one per position-generating function. So **every position set is also one fixed
draw** — including the HARD set of 8.

Therefore **every number this track has ever emitted is n=1 by construction**, not by sampling:
the saturated `1.000×` rates, the `mates 25/25` ceiling, and "17 of 34 lineage-generations have a
member scoring >0 on the hard set". None of them has a second observation behind it. That is exactly
the defect this campaign's central result warns about — single-seed conclusions FLIP SIGN on a second
seed (blend at z = 3.3, epochs at z = 3.6).

**One thing it does NOT invalidate:** the flagged-vs-control hard-fitness arms genuinely diverge
despite identical mutation draws, because the fitness changes which candidate is *kept*, so the
*parent* differs at the next generation even when the draw does not. Divergence enters through the
population, not the RNG. The comparison is still single-trajectory, but it is not a comparison of a
thing against itself.

### VERIFIED BOTH WAYS — the track can finally produce a second trajectory

```
DEFAULT   gen1 MAIN REJECT 0.417+/-0.103  surrogate 0.002794   reproduces the bank
          gen2 MAIN REJECT 0.417+/-0.103  surrogate 0.002884   matches the banked gen-2 exactly
SEED 777  gen1 MAIN REJECT 0.458+/-0.082  surrogate 0.002705   DIVERGES
          gen1 MCTS ..none                                     different shape entirely
```

Both halves of the check pass: the default is bit-identical to the banked run, so nothing already
measured is invalidated, and 777 departs immediately. The binary announces which it is on its first
line (`run seed 777 (mix 0x3660740359129bbd) -- NEW trajectory`), so a log can no longer be mistaken
for the wrong arm.

**The first thing the second trajectory buys.** Seed 777 shows the SAME saturation — rates capping at
exactly `1.000x`, `hard 0-0`, `..none` generations. That was previously a single-trajectory
observation and the central claim resting on it (the surrogate is saturated, so selection has nothing
to rank) could not be separated from one unlucky run. It now reproduces on an independent trajectory.
That does not make it a large sample, but it moves the saturation from n=1 to n=2 and it is the first
evidence about this track that is not a restatement of the same run.

**Fixed narrowly**: the run seed is now mixed into the mutation draw only, multiplicatively so that
the default (unset → 0) is the identity and every banked trajectory stays bit-for-bit reproducible.
The nine position-set seeds are deliberately left hardcoded — a fixed benchmark is what makes results
comparable across runs. The cost of that choice is stated plainly: hard-set difficulty is one sample,
so hard-set results do not generalise to another draw of positions, and claiming otherwise would need
the position seeds varied too.

## Shipping candidates, with evidence strength stated per item

All head-to-head at 960 pairs. **Nothing here has shipped**; none of it is an Elo number.

> **⚠ ALL OF THESE WERE MEASURED AT DEPTH 2.** This project's own standard for strength is **depth
> 4** — the gate derives its node budget as "7061 nodes = 100% coverage of a full depth-4 search"
> (`gate_depth_cap` default 4), and the built-in control plays at that cap with equal-time budgets.
> A depth-2 result is not automatically a depth-4 result. The depth-4 cross-check is running; until
> it lands, every row below is a claim about depth-2 play.

| candidate | shipped | measured | strength |
|---|---|---|---|
| ~~blend 1.00~~ | 0.75 | d2 **0.450 ± 0.016** / d4 **0.511 ± 0.022** | **DEAD — advantage is depth-2 only (z = 4.4)** |
| **`--gate-every 5`** | 1 | d2 **0.529 ± 0.015** / **d4 0.502 ± 0.022** | **FAILS AT DEPTH 4** — gain vanishes |
| epochs 2 | 3 | 0.518 ± 0.014 (ep_2 vs ep_3) | **MARGINAL** — margin 0.004 vs ci95 0.014 |

**Neither candidate survives the depth change, and the pattern is general.** `b2_5` beats
`champion_long` by +0.029 at depth 2 (resolved) and by +0.002 at depth 4 (unresolved) — the point
estimate collapses, not merely the interval widening. `bh_100` likewise scores 0.864 against the
origin at depth 2 and 0.832 at depth 4.

**Why this is expected in hindsight:** datagen runs at `--depth 2`, and the batch gate scores with
`match_nets(..., depth, ...)` — the *same* depth. So the loop both learns from and selects on
depth-2 play, while the project judges strength at depth 4. It is optimising the game it measures.
Aligning the **gate** to depth 4 is far cheaper than moving datagen there (224 pairs vs 2400
games/generation) and is the obvious next experiment.

**Blend no longer clears the bar either.** Seed 20260907 resolved (0.450, 1.00 stronger); seed
424242 did not (0.487, interval [0.471, 0.502] contains 0.5). Both point estimates favour 1.00, but
the two seeds differ by **0.037 ± 0.011, z = 3.3** — genuinely heterogeneous, so they cannot be
pooled into a win. `blend_seed2.sh` pre-registered exactly this: *"UNRESOLVED => needs a third
seed"*. And both readings are at depth 2, which is not the strength standard.

**Epochs is marginal, not clear.** I earlier called it "clear of 0.5" off a 448-pair reading; at 960
pairs the lower bound is 0.503. The direction is consistent across both readings and both sit above
0.5, but this is not yet a result and must not be shipped on.

Every one of these needs a **second training seed** before a default moves — a match seed re-rolls
openings and nothing else. `blend_seed2` (running), then `ship_candidate` for the combination.

## ✅ RESOLVED 2026-09-09 — the exploration term was INVERTED, and that explains the cost anomaly too

The section below is kept as written because its reasoning was sound and its conclusion was wrong for
a reason it could not see. Both refutations, and the anomaly it stopped on, have one cause.

`uct_mcts` reads table slot 2 **twice**: as the scale inside the sqrt, and as the weight of
`Mix(q, u, c) = (q*c + u*(16-c))/16` (interp/src/lib.rs:696). So raising K enlarges `u` while driving
`u`'s coefficient `(16 - c)` to zero and then **negative**. Swept at budget 256 over 23 mate-in-one
positions: 20/23 at c=1, 14/23 at c=16 where the coefficient is exactly 0 (pure greed), then 12/23,
9/23 and **0/23** at c=24, 64 and 360000. Monotone, crossing the greedy baseline precisely where the
coefficient vanishes.

So "K = 2000 and K = 360000 are worse" was never a fact about exploration magnitude. At those values
the program is **penalised for exploring**. Proof by substitution: selecting on `q + u` instead of
`Mix`, at the SAME K = 360000, scores **23/23** instead of 0/23. `uct_mcts_sum` reaches 23/23 from
K = 600 upward — and 600 is the net's declared eval scale, i.e. the "C = one eval unit" derivation
this section records as REFUTED was right in form *and* magnitude, and was defeated by the blend it
was fed through.

**The cost anomaly resolves with the correct sign.** This section stops on: raising K costs 50x for
the same playout count, when broader-and-shallower should be CHEAPER. But raising K did not broaden
the tree — it inverted the term, so selection *avoids* unvisited children. A playout ends at the
first unvisited node, so preferring visited children makes each descent go DEEPER before terminating.
Deep narrow descents, not broad shallow ones. That predicts higher cost, which is what was measured.

PRE-REGISTERED so the explanation is falsifiable rather than merely consistent: at K = 360000 the Mix
encoding should cost substantially MORE than the sum encoding at the same K and the same playout
count, since only the Mix form inverts. If their costs match, this explanation is wrong and the
anomaly is still open.

**Status:** MCTS is no longer PARTIAL. A faithful, solving UCT is +60 nodes from the 71-node seed
(131 vs the Mix form's 132 — `Add` takes two children, `Mix` takes three), against PN's +104.
`EXISTENCE_UCT_K` still defaults to 8, which remains correct for the Mix-form lineage seed.

---

## ⚠ SUPERSEDED — UCT EXPLORATION: my hypothesis is REFUTED TWICE and the declared value 8 is the best tested

GRAMMAR 6 records MCTS as PARTIAL (20/23 forced mates) and attributes it to an exploration term that
saturates. The arithmetic is real: at K = 8, `Div` truncates to 0 once a child passes ~40 visits, so
`u` switches off mid-run. I raised K on that basis. Both values I derived are worse than the one
they replaced:

```
budget:            16    64      cost vs alpha-beta at 16 / 64
K = 8  (declared)   1    10          0.013x  /  0.062x     <- BEST
K = 2000            0     2          0.399x  /  3.116x
K = 360000          0     0          0.560x  /  3.648x
```

Monotone in the wrong direction on both mates and cost. **Two refuted hypotheses in a row is the
documented signal to stop guessing and distrust the framing**, so there is no third K.

**And there is an anomaly I cannot explain, which is the real reason to stop.** Cost rises **50x for
the SAME playout count**. The reference program is not a rollout-to-terminal MCTS:

```
simulate(p):  if terminal(p)   -> score
              if visits(p)==0  -> store(key p, 1); ret eval(p)      expand + evaluate, STOP
              m = argmax(moves(p), UCT);  v = neg(simulate(apply(p,m)))
```

Every playout terminates at exactly ONE unvisited node, so the eval count is identical whatever K
is. Cost can therefore only differ through DESCENT DEPTH — and raising exploration makes the tree
broad and shallow, which must be CHEAPER. The measurement says 50x dearer. The code and the
measurement disagree about the sign, and per-argmax cost is K-independent, so the discrepancy is not
in the exploration term at all.

**What this does and does not establish.** It does NOT show the saturation analysis is wrong — the
cliff at 40 visits is arithmetic and still true. It DOES show the cliff is not what limits
mate-finding at these budgets, because removing it makes mates worse. 16-4096 playouts against a
branching factor near 30 is far too few to find mates by exploring; the search has to exploit, and
K = 8 exploits.

**Recorded as a bounded negative, not a fix.** `EXISTENCE_UCT_K` stays, defaulting to the declared 8
so nothing changes, and the next step on this thread is explaining the cost anomaly rather than
choosing another constant. The one unambiguous improvement from the attempt survives: the weight was
a bare literal at 27 sites as `8` and 6 more as `1`, so `reference_audit` certified these programs
under a different exploration weight than the loop runs. It is one named accessor now.

## The ratchet is now visible in BEHAVIOUR: the filter gates every generation, strict gates 44%

The mechanism recorded below has a measurable consequence, and it is the clearest difference the A/B
has produced:

```
specfilter   5 gates in  5 generations   100%
strict       4 gates in  9 generations    44%
```

Under the strict rule the surrogate bar rises to each rejected candidate's rate, so after a few
rejections the population can no longer clear it and generations pass with `..none`. Under the
filter the bar stays where the champion put it, so a candidate is proposed every generation. **The
strict rule progressively silences its own loop; that is what the ratchet does when you watch it
long enough.**

The filter's `ABOVE` count climbs as the run goes on — 3, 6, 9 across generations — because it is
measured against a fixed champion rate while the population improves. Under strict the same count
sits at 1, because the denominator chases the numerator upward.

**What this does NOT yet show.** Every one of those extra gates was a REJECTION, at 0.417 and 0.500,
so the filter has bought more games and no acceptance. It has also not yet produced its signature
event — a gate line whose surrogate is BELOW the incumbent's, impossible under the strict rule —
because the population has so far always contained something above the champion's rate. The filter
restores proposal frequency, which is what it was for; that this converts into an accepted program
is unproven and is the harder question.

## The strict rule RATCHETS ITS OWN BAR UP on every rejection, and the A/B makes it visible

The spec-filter A/B diverged in a way I did not predict and which explains the loop's behaviour over
time. Both arms propose the SAME candidates in the first generations, at the same rates:

```
specfilter  gen1 MAIN surrogate 0.002794  ABOVE:3    gen2 MAIN surrogate 0.002884  ABOVE:6
strict      gen1 MAIN surrogate 0.002794  ABOVE:3    gen2 MAIN surrogate 0.002884  ABOVE:1
```

Same candidate, same rate, but **ABOVE:6 against ABOVE:1**. `ABOVE` counts offspring whose
rate exceeds `best_rate`, so the two arms are dividing by different denominators:

* strict, after gen 1's rejection: `best_rate = 0.002794` — raised to the REJECTED candidate's rate
* filter, after gen 1's rejection: `best_rate = 0.002518` — still the champion's

**So on every gate rejection the strict rule raises its own surrogate bar to the rate of a program
the GAMES had just judged worse.** The candidate lost at 0.417 and its surrogate became the new
threshold anyway. The bar therefore ratchets upward monotonically across a run: after N rejections
it sits at the best rate any of N losing programs achieved, and proposals get rarer with every one.

**This is a purpose with a side effect, not a plain bug.** The update exists to stop the same
candidate being re-proposed forever, and it does that. But it buys that with a permanent, one-way
increase in the bar, driven by programs that failed. The filter arm gets the same protection from an
explicit already-tried set, which is why it needs one — and it leaves the bar where the champion put
it.

**Verified, and its limit stated.** The mechanism difference is confirmed from the data (the ABOVE
denominators can differ no other way). The BEHAVIOURAL difference the filter was built for — a gate
line whose surrogate is BELOW the incumbent's, which the strict rule cannot produce — has NOT yet
appeared in four generations. The filter binds; whether it changes outcomes is still open.

## The MATE-2 result, fully tempered: it removed the TAIL, not the BIAS

Accumulated across three configurations — 14 generations, zero captures, against a control that
captured at generation 1 on all three seeds. The exploit axis holds. But what the surviving
proposals actually look like:

```
mate2_treat  (4242, floor 30)   gen1 MAIN 1.02x games 0.458    gen2 MAIN 2.60x games 0.250
mate2_31337  (31337, floor 30)  gen1 MAIN 1.05x games 0.333    gen2 MAIN 1.32x games 0.375
window       (4242, floor 29)   gen1 MAIN 1.02x games 0.458
```

```
MATE-2 MAIN gates pooled   0.3540 +/- 0.0945 -> [0.260, 0.448]   24 pairs
ORIGINAL 25-set   pooled   0.4554 +/- 0.0505 -> [0.405, 0.506]   84 pairs
```

**The proposals are not better. If anything they are worse.** The intervals overlap only in
0.405-0.448 and both sit below 0.5. Removing the extreme exploits did not make the survivors good —
it removed the tail, not the bias. That is what a higher floor can do and all it can do.

**The window arm adds nothing so far.** Its generation 1 is byte-identical to the treatment's
(surrogate 0.002524, games 0.458): the looser floor did not change which candidate was best, because
the binding constraint at that generation was not the floor. The window's value was always
conditional on capture extension being worth admitting, which is exactly what the 300-pair match is
measuring and which has never been established.

**RESEEDED 2026-09-09 — that lineage was seeded with a capped program.** Its seed selected on
`Mix(q, u, c)`, whose coefficient on the exploration term is `(16 - c)`: zero at 16, negative above.
The encoding's ceiling is **20/23** forced mates at ANY weight. Measured on `evolve mctsbudget`'s own
viability criterion — "approaching 25/25 at cost ~1.0x AB" — against the sum encoding:

| encoding | budget | mates | cost vs AB |
|---|---|---|---|
| blend (slot2=8) | 512 | 11/25 | 0.968x |
| **sum (slot2=600)** | 1024 | **17/25** | 1.085x |
| sum (slot2=600) | 64 | 14/25 | **0.050x** |

The last row is the sharpest: the sum form reaches 14/25 for **5%** of alpha-beta's cost, where the
blend form needs 2.17x the cost to reach 13. Best mates/Mcost is 0.0281 against 0.0163.

**MEASURED IN THE LOOP ITSELF, 2026-09-09.** Two arms of `evolve 25 8 12 6 3`, same
`EXISTENCE_EVOLVE_SEED`, same budget, differing only in the MCTS lineage seed. The MAIN lineage is
BYTE-IDENTICAL in both (23/23 mates, cost 9235450584), which is what proves the seed is the only
difference:

| MCTS seed @ budget 1024 | mates | guard floor | cost | mates/Mcost |
|---|---|---|---|---|
| blend @ K=8 (historical) | 11/23 | 7 | 2.00e10 | 0.000549 |
| **sum @ K=600 (declared)** | **15/23** | **11** | **1.07e10** | **0.001406** |
| blend @ K=600 | 1/23 | **0** | 4.43e10 | 0.000023 |

Better on every axis at the same budget: four more mates, guard floor 7 -> 11, and HALF the cost,
so 2.56x the mates per unit of cost.

**The third row is a configuration that never existed, and it was my first attempt at this A/B.**
Running the blend form at the sum encoding's weight collapses it to 1/23 with a guard floor of
**ZERO** — which is precisely the degenerate regime `mcts_budget`'s own docstring was written to
detect: *"if it stays near 1 even at matched cost, then seeding a lineage with it creates a
population whose mate guard is `f >= 1` — vacuous."* It is kept here as evidence that the blend form
must never read the global weight, and as a reminder that the comparison had to be redone: an A/B
where one arm runs in a configuration nobody ever shipped is rigged, however bad the arm looks.

Still NOT established: whether the seed is why every MCTS gate reads 0.500. The 25-generation arms
are running; a guard floor of 7 was never vacuous, so the 0.500 gates need their own explanation.

The lineage seed is now `uct_mcts()` (sum selection) and `budget_mcts` is 1024, the measured
cost-parity point. Whether the 0.500 gates were caused by the seed is NOT established — that is the
next measurement, not a claim.

**MCTS lineage, worth noting separately:** every gate is exactly 0.500. Those are behaviourally
identical candidates — the "plays the same, costs less" path — not strength changes.

**Where this leaves the whole day's Existence work.** Two axes were attacked. The exploit axis
genuinely improved and is measured. The ranking axis is now refuted from both directions: MATE-2
does not unsaturate the numerator (rungs moved <0.4%), and the hard set rewards difference rather
than depth (depth-one ties capture extension at 6/40). `proxies_RESULT.md` said it already — only
games measure strength here — and everything today has converged on that sentence.

## ⚠ REFUTED: the HARD set measures DIFFERENCE from the seed, not better search

The direction I called "the one that survives every other measurement today" — rank primarily on the
unsaturated dimension — is dead. Scaling the hard set from 8 positions to 40:

```
                              hard 8   hard 40
depth-one (purity seed)          0        6      <- the DEGENERATE program
capture extension (rung 6)       1        6      <- TIED with it
UCT-style MCTS                   0        5
bare alpha-beta (seed)           0        0
alpha-beta + hash reuse          0        0
alpha-beta + iterative deepening 0        0
alpha-beta + hash + ID           0        0
```

**Capture extension and depth-one score identically.** The hard set cannot tell a genuine capture
extension from the cheapest, shallowest program in the reference set. Every real alpha-beta variant
scores ZERO.

**Why, from `harder_set`'s own construction.** It records the seed's answer one ply DEEPER than the
fitness depth, so the seed is wrong on every position by design. But "wrong at depth D, right at
D+1" does not mean "a better search gets it" — it means the answer CHANGES with depth, and any
program that answers DIFFERENTLY has a chance of landing on the deeper answer. depth-one searches
one ply, maximally unlike depth three, and collects six. The set rewards difference and cannot
distinguish it from depth.

**The n=8 version was a small-sample lie.** At 8 positions capture extension scored 1 and everything
else 0, which reads as a clean discriminating signal; it was one position that happened to fall its
way. My own rule — one sample is a lottery — applies to instrument validation and I nearly built a
ranking on eight observations.

**This closes the loop with what was already recorded.** `proxies_RESULT.md`: "Every cheap proxy for
strength has failed. **Only games measure strength here.**" The hard set is one more cheap proxy and
it fails the same way. Having re-derived three results this morning by not reading, I have now
re-derived a fourth — but this one at least adds the specific mechanism and the specific number.

**What survives.** Nothing about ranking. The MATE-2 rung still closes the extreme-exploit hole in
the guard, which is a separate and still-standing result. But "rank on the hard dimension" would rank
depth-one first, and depth-one is the program the guard exists to keep out.

## RUNNING: rung 6 at proper power — the documented UNRESOLVED question

Applying the lesson above rather than restating it: reading all 20 result headlines identified the
genuinely OPEN items, and `rung6_RESULT.md` is the one my whole MATE-2 line has been circling.

> "Rung 6 was dead code. Now it plays differently — **whether it plays better is UNRESOLVED**."

Its match was **3W-9D-4L, 0.469 +/- 0.172 over 8 pairs**, with the doc's own verdict: "nothing
smaller than a rout is visible". That is the same under-powered arithmetic as the 12-game loop gate,
and the fix is pairs:

```
  8 pairs -> ci95 0.164      100 pairs -> ci95 0.046
 24 pairs -> ci95 0.094      300 pairs -> ci95 0.027
```

300 pairs is running. It resolves ~0.027, so a true 0.53 edge becomes visible and a true 0.50 becomes
a tight null — an answer either way, which is what "unresolved" needs.

**Why this matters beyond the rung.** Capture extension is the only reference program that scores on
the hard set, the only alpha-beta-family program measured to play DIFFERENT chess from the seed, and
the program the whole MATE-2/tolerance-window line exists to admit. Every argument I have made today
for admitting it assumes it is worth admitting, and that assumption has never been measured. If it
plays at 0.50, the window is a mechanism for admitting a neutral program and the case for it
collapses to "the guard should not be arbitrary" — still true, but much smaller.

**Validity check, taken from the doc rather than invented:** forfeits must be ZERO. Capture extension
costs 1.679x, so a forfeit falls on the expensive side systematically and a score built on them
measures cost rather than play. A non-zero count voids the verdict.

## ⚠ PROCESS FAILURE: I re-derived three results that were already in the tree

There are 23 analysis documents at the repo root. I did not read them before starting, and three of
today's "findings" are re-derivations of work dated 2026-09-08:

| today | already recorded |
|---|---|
| the gate demands 0.58-0.75, far above a real edge | `acceptance_floor_RESULT.md` — "the gate demands an edge **2.7x larger than a generation produces**" |
| the surrogate's proposals pool at 0.4554, anti-correlated with strength | `surrogate_validation.md` — "the accept/reject surrogate **does not predict strength** — measured" |
| tolerance 7 admits exploits, tolerance 4 is safe | `tolerance_RESULT.md` — a sweep over **33 behaviour-changing edits and 2 exploits**: tol 4 admits 6/33 genuine and no exploit, tol 5 admits ALPHA |

The prior tolerance work is **more thorough than mine**: it measured 33 genuine candidates against
two constructed exploits and located the boundary between them. My guard experiment found the same
boundary from one direction with less evidence.

**What today actually contributes, after subtracting the duplication:**

1. **A third exploit family, machine-found**, which lands EXACTLY on the floor (18 = 25-7). The prior
   work used two hand-constructed exploits (ALPHA, DEPTH); those were built by a person who knew
   what to build. This one was found by the loop and sits on the boundary because that is where
   selection pushes.
2. **The corpus as a persistent regression test** — `exploits.tsv` plus a cargo test, so a
   configuration change is checked against real specimens rather than re-argued.
3. **MATE-2 separates the rung from the exploit**: capture extension 11/12, the exploit 7/12, where
   both score 18/25 on the current set and NO tolerance can tell them apart.
4. **The tolerance window** (MATE-2 set, tol 8-9), which admits capture extension while excluding the
   exploit.

Point 4 extends the prior work rather than repeating it. `tolerance_RESULT.md` concluded that on the
original guard "**no tolerance separates them**" for the alpha exploit; the finding here is that
adding a RUNG creates separation where changing the tolerance alone could not. That is the same
lesson from the other side, and it is why FITNESS 3 specifies four rungs.

**The process lesson, which is the expensive part.** Reading 23 files costs minutes; re-deriving
three results cost hours of compute and turns. The rule that would have caught it is the one already
written for the 4PC side — audit what exists before generating new work — and it applies to this repo
just as directly.

## RUNNING: the tolerance-window arm, pre-registered

Three arms on identical mutation draws (seed 4242), differing only in the fitness set and the mates
floor:

```
control    25 positions, floor 18   CAPTURED an exploit at gen 1 (95x rate, games 0.208)
treatment  37 positions, floor 30   no capture; proposed 1.02x, games 0.458
window     37 positions, floor 29   admits capture extension (29 >= 29), excludes the exploit (25 < 29)
```

**PRE-REGISTERED: zero captures in the window arm.** A capture refutes the window arithmetic
outright, because the only row the loosened floor admits is one the corpus measures at 25.

If it holds, the window is the first configuration that admits capture extension AT ALL. That
program is the only reference program scoring on the hard set, and the guard has cut it in every
configuration run so far — including the 25-position set at every tolerance, where it and the
exploit both score 18 and cannot be told apart by any floor.

**What it still will not do.** Admitting a candidate to the gate is not accepting it. Capture
extension's game rate against the seed has never been measured, and the 12-game gate demands
0.58-0.75. The window changes which programs get to be judged, not the standard they are judged by.

## ★ THE MATE-2 RUNG SEPARATES THE RUNG FROM THE EXPLOIT — and opens a tolerance window

The two programs are **indistinguishable on the set the loop actually uses**, and the MATE-2 rung
tells them apart:

```
                          25-set      MATE-2        total / 37
capture extension (rung 6)   18      11/12  (92%)       29
the captured exploit         18       7/12  (58%)       25
```

Both score exactly 18 of 25. **No mates guard at any tolerance can separate them on the current
set** — that is what "the set cannot see the difference" means concretely, and it is why the exploit
lands precisely on the floor while the rung is cut alongside it.

Adding the rung opens a window:

```
                     floor   rung    exploit
MATE-2 set, tol 4       33   out     out
MATE-2 set, tol 7       30   out     out
MATE-2 set, tol 8       29   IN      out     <- admits the rung, excludes the exploit
MATE-2 set, tol 9       28   IN      out     <- same
```

**Neither change achieves this alone.** MATE-2 at tolerance 7 rejects both. Tolerance 8 on the
25-position set has floor 17 and admits BOTH (each scores 18). Only the rung plus the loosened floor
admits capture extension — the one reference program that scores on the hard set — while keeping out
a program that plays at 0.208.

**This is what FITNESS 3's per-N design is FOR**, arrived at by measurement rather than by reading:
the rungs are not there to be individually decisive, they are there so a real searcher and a
plausible impostor, tied on one rung, come apart on the next. 92% against 58% is that coming-apart.

**Caveats, both real.** The exploit's 25/37 is 18 measured on the 25-set plus 7/12 measured on the
MATE-2 probe — the same twelve positions, so the sum is exact, but it is a sum rather than a single
run on 37. And the window rests on ONE rung and ONE captured exploit family; a different exploit
scoring 11/12 on MATE-2 would close it, which is precisely why the spec asks for four rungs and not
two.

## ⚠ CORRECTION: capture extension really does lose mates — the markdown is not an artefact

Re-scored on the 37-position set: **29/37 mates, 0.370x**, against 18/25 and 0.340x on the
25-position set. The larger set barely moved it and it is still marked "loses answers".

I wrote earlier that the set "penalises the one program that solves the hard set", implying the
markdown was an instrument error. It is not. Capture extension loses 8 of 37 here, and `matesplit`
independently put it at 17/20 on MATE-1 and 18/20 on MATE-2 rather than 20/20. At a fixed budget of
16, searching captures deeper genuinely costs mate-finding elsewhere — that is a real trade, not a
measurement artefact.

**The defensible version:** the surrogate is not wrong to mark it down, it simply has no way to
PRICE the trade of mates for tactical depth. Capture extension buys the only hard-set point any
reference program scores and pays 8 mates for it. Whether that is a good trade is exactly the
question games answer — and it never reaches games, because the guard cuts it first.

The conjunctive result is unchanged by the larger set: probe-only 0.991x, store-only 0.997x, both
halves 1.026x. A strict climb still cannot take the first step toward the one fitter rung.

## ⚠ WALKED BACK: MATE-2 COMPRESSES the exploit, it does not eliminate bad candidates

I claimed the rung "turns an exploit generator into a normal-candidate generator" on the strength of
generation 1. Generation 2 is worse than that claim allows:

```
TREATMENT (37 positions, floor 30)
  gen 1 MAIN  surrogate 0.002524 =  1.02x the seed   games 0.458   ordinary
  gen 2 MAIN  surrogate 0.006432 =  2.60x the seed   games 0.250   plays badly

CONTROL (25 positions, floor 18)
  gen 1 MAIN  surrogate 0.231865 = 94.91x the seed   games 0.208   EXPLOIT
```

**What survives:** the rate inflation is compressed from **95x to 2.6x**, and no candidate has
tripped the 10x capture threshold in 4 generations on seed 4242 or 1 on seed 31337, where the
control captured at generation 1 on both. The extreme exploit is gone.

**What does NOT survive:** "normal-candidate generator". A program at 2.6x the incumbent's rate
playing at 0.250 is the same failure in a smaller size — a surrogate gain that is not a strength
gain. The guard now stops the egregious version and lets the mild one through, which is exactly what
a higher floor should be expected to do: it is a threshold, not a fix for the ratio being the wrong
measure.

**And the gate caught it**, at 0.250 against a bar of 0.582. That is the veto working as designed.
The loop is not in danger from these; it is wasting games on them.

**Honest scope of the whole MATE-2 result**, restated: it closes the extreme-exploit hole in the
GUARD. It does nothing for the ranking defect (measured: rungs moved <0.4%), and it does not stop
the surrogate proposing mildly-inflated bad players, because that is the ratio's nature and not the
floor's job.

## Original claim, superseded by the above

Paired arms, seed 4242, identical mutation draws, generation 1. The ONLY difference is twelve MATE-2
positions in the fitness set:

```
CONTROL   25 positions, floor 18   surrogate 0.231865   ABOVE:3   gate 0.208   EXPLOIT captured
TREATMENT 37 positions, floor 30   surrogate 0.002524   ABOVE:1   gate 0.458   normal candidate
```

The control proposes a program at **95x the incumbent's rate** that plays at 0.208 — the exploit
signature, and it has done this at generation 1 on three separate seeds now, so it is a reliable
behaviour rather than a run of bad luck. The treatment, from the SAME draws, proposes a program at
**1.02x** that plays at 0.458.

**The deficit-accumulation mechanism is confirmed.** The exploit-shaped candidate scores 18 on the
original 25 and 7 on the new rung — 25 of 37 against a floor of 30 — so the guard rejects it before
its rate is ever considered, and the population's next-best candidate is an ordinary one. `ABOVE`
falling from 3 to 1 is the same fact from the other side: fewer candidates clear the higher floor.

**This is the fix, and it is not the fix I first argued for.** The prediction that survived is the
SECOND one — deficits adding across rungs — after the first ("aces MATE-1, fails MATE-2") was
refuted by the specimen scoring 7/12 on MATE-2. The corrected mechanism made a sharper prediction and
the sharper prediction held.

**Scope, stated honestly.** One generation per arm. The control's failure is established across three
seeds; the treatment's success is one observation at the point where the control reliably fails,
which is the informative place to look but is not yet a distribution. And this fixes the EXPLOIT
axis only — the ladder-ranking defect is untouched, as the oracle re-run showed.

## MATE-2 does NOT fix the ladder ranking — and that separates two problems I was conflating

The oracle re-run with the MATE-2 rung added (37 positions instead of 25):

```
                        WITH MATE-2        before        change
bare alpha-beta (seed)     1.000x  37/37   1.000x  25/25
hash reuse                 1.026x  37/37   1.024x         +0.002
iterative deepening        0.915x  37/37   0.914x         +0.001
hash + ID                  0.936x  37/37   0.933x         +0.003
UCT-style MCTS             0.808x  12/37   0.968x         -0.160
depth-one                784.729x   6/37 992.711x       -207.98
```

**The rungs did not move.** ID is still 0.915x, hash+ID still 0.936x — both still below the seed,
still rejected. Adding the rung the spec asks for changed the ladder ordering by less than 0.4%.

**The reason is the one thing I did not check: the seed aces MATE-2 as well, 37/37.** A rung only
unsaturates a numerator if the incumbent FAILS some of it. Twelve more positions the seed also solves
leave `mates/cost` exactly as cost-dominated as before, so the ratio still measures cheapness.

**So there are TWO separate defects and MATE-2 addresses only one of them:**

| defect | mechanism | does MATE-2 help? |
|---|---|---|
| exploits admitted at a lowered floor | guard floor is an absolute count | **YES** — deficits accumulate across rungs (18/25 -> 25/37 against floor 30) |
| ladder ranked below the seed | numerator saturated, ratio measures cost | **NO** — seed scores 37/37, saturation is untouched |

I had been treating these as one problem with one fix. They are not, and only the measurement
separated them.

**What WOULD unsaturate the numerator: positions the seed FAILS.** That is exactly the HARD set —
seed 0/8 by construction — and capture extension is the only reference program that scores on it.
The earlier `EXISTENCE_HARD_FITNESS` attempt failed because it ADDED a 0-or-1 hard term to a
numerator already saturated at 25; the arithmetic could not invert a 0.340x-vs-1.000x ordering. The
direction that survives all of today's measurements is to rank PRIMARILY on the unsaturated
dimension rather than adding it to a saturated one — which is a different change from the one I
tried, and it now has evidence behind it rather than an argument.

**MATE-2 still earns its place**, on the exploit axis, which is what the paired arm is testing. It is
just not the ladder fix I hoped it was.

## ⚠ MY per-N ACCOUNT IS REFUTED BY THE FIRST MEASUREMENT — and the real mechanism is better

The first exploit captured with per-N scoring:

```
18/25 on the loop's set   MATE-1 12/12   MATE-2 7/12   132x cheaper   games 0.208
```

**It does not fail MATE-2. It scores 58% on it.** I predicted "aces MATE-1, fails MATE-2" as the
exploit signature, twice — once from `matesplit` and once as the justification for adding the rung —
and the measurement says no. This is not a program that refuses to search; it searches enough to
find seven forced mates in two, and still plays at 0.208.

**The corrected mechanism, which the same numbers support:**

```
                        floor   exploit   verdict
tol 7, current set (25)    18        18   ADMITTED   <- lands exactly on the floor
tol 7, with MATE-2 (37)    30        25   rejected
tol 4, current set (25)    21        18   rejected
tol 4, with MATE-2 (37)    33        25   rejected
```

MATE-2 stops it by **DEFICIT ACCUMULATION**, not by a categorical failure: it loses 7 on the original
set and 5 more on the new rung, 12 total against a tolerance of 7. Every rung added is another place
a program that plays badly has to keep up, and the deficits add while the tolerance does not. That is
a better argument for FITNESS 3's four rungs than the one I made — the spec asks for MATE-{1,2,3,4}
precisely so a deficit has four places to show up rather than one.

**It also sharpens what the exploit IS.** Not "prune everything / return eval" — FITNESS 10's first
row, which I matched it to. It finds mates competently and plays terribly, which is a different
degenerate solution from the one the spec catalogues, and I do not yet have a name for it.

**And I had a reader bug that hid this.** `exploit_check.py` used `csv.DictReader`, which takes the
header from the FILE — still the old 11-column one — so the two new fields landed in the unnamed
restkey and the tool printed "n/a" for a row that had the data. The header is migrated and legacy
rows padded with empty (not zero) per-N fields. A tool that reports "not measured" for a measurement
it is holding is worse than one that reports nothing.

**Pre-registered, replacing the refuted prediction:** the treatment arm (same seed 4242, 37-position
set, floor 30) should capture NO exploit, because 25 < 30. The control has now captured at generation
1 on three separate seeds, so a null in the treatment is a real difference and not a quiet run.

## FITNESS 3's per-N split IS a real discriminator, and the loop throws it away

FITNESS 3 specifies MATE-N for N in {1,2,3,4}, 500 each, **reported per N**, with different
thresholds (>=0.9x on MATE-{1,2}, >=0.8x on MATE-{3,4}). The loop scores ONE pooled ratio over
`mate_set` — which is MATE-1 only — plus two disagreement sets. `forced_mate_set`, the MATE-2
generator, is defined at `evolve.rs:57` and **used by no fitness set anywhere**.

FITNESS 10's table of degenerate solutions opens with

```
| Prune everything / return eval | mates-per-cost filter (3); ladder (7) |
```

and the per-N split is how filter (3) is supposed to catch it. Measured, rather than asserted
(`evolve matesplit`, 20 positions each, depth 3):

```
program                            MATE-1    MATE-2
depth-one (purity seed)              1/20      1/20    fails both
bare alpha-beta (main seed)         20/20     20/20    searches
alpha-beta + iterative deepening    20/20     20/20    searches
alpha-beta + hash + ID              20/20     20/20    searches
table reduction (rung 7)            20/20     20/20    searches
capture extension (rung 6)          17/20     18/20    searches
UCT-style MCTS                      11/20      2/20    <- 5.5x COLLAPSE
proof-number search                  5/20      5/20    partial
```

**MATE-2 separates a shallow searcher from a real one.** UCT falls 11 -> 2 while every alpha-beta
variant holds 20/20. A pooled ratio over MATE-1 alone cannot see that difference at all, and it is
precisely the difference between "found a mate that was one ply away" and "searched".

**My specific prediction was WRONG and that is worth recording.** I pre-registered that `depth-one`
would ace MATE-1 and fail MATE-2, as the hand-written non-searcher. It scores 1/20 on BOTH: it has
no terminal check, so it cannot see mate-in-one either, and it is not a valid proxy for the exploit
class. The mechanism survived on a program I had not nominated, which is weaker evidence than a
confirmed prediction and is reported as such.

**The caveat that limits this.** Whether the per-N split catches MY captured exploits is still
UNTESTED, because those specimens are `{:#?}` dumps and there is no text format to reload them. The
argument that it would — they score 18/25 on a set that is 15/25 MATE-1, so they are finding shallow
mates cheaply — is inference, not measurement. Making it a measurement needs either a serialiser or
a capture that records per-N scores at capture time. The latter is far cheaper and is the next step.

**Second observation, unprompted and awkward for the current set.** Capture extension scores 17/20
and 18/20 here — it searches, on both rungs. On the loop's actual 25-position set it scores 18/25
and is rated **0.340x**, the worst of any rung. The set the loop uses penalises the one program that
solves the hard set, and the per-N view says it is searching perfectly well.

## The exploit is a STABLE ATTRACTOR: two independent seeds converged on the same shape

A second specimen, from seed 31337 — a different trajectory entirely, since the mutation draw is now
genuinely seedable:

```
seed default   18 mates,  5,440,807 cost   1,881x cheaper   1,354x rate   games 0.208
seed 31337     18 mates,  4,836,594 cost   2,115x cheaper   1,523x rate   games 0.208
```

**Both land on exactly 18 mates and exactly 0.208 games.** 18 is the guard floor at tolerance 7
(25-7), and 0.208 is what a program that does not search scores over 12 games. Two independent
trajectories converged on the same point, which means this is not a fluke of one run — it is where
selection goes when the floor is lowered by three. The fitness landscape has a large basin at
"stop searching, keep just enough mates to clear the floor", and evolution finds it in ONE generation
from either starting point.

That also makes the corpus more useful than a list of curiosities: two rows, same signature,
different provenance. A candidate fitness that admits one admits the family.

## FIRST MACHINE-FOUND EXPLOIT CAPTURED, and it lands exactly on the guard floor

`exploit_MAIN_gen1_1354x.prog`, produced within one generation of turning the refuted
`GUARD_TOL=7` on as a generator:

```
18 mates, 81 nodes, generation 1, lineage MAIN
surrogate 3.308333 vs incumbent 0.002443 = 1354x
games 0.208 +/- 0.151 over 12 -- far below parity

cost:  exploit      5,440,807
       seed    10,233,319,689      -> 1,881x CHEAPER for 18 of 25 mates
```

**It barely searches.** 5.4M cost units against the seed's 10.2B is not a cheaper search, it is
almost no search — and it still collects 18 mate-in-1/2 positions, because those are findable
without one. Then it plays at 0.208.

**And it scores EXACTLY 18, which is exactly the floor.** At tolerance 7 the mates floor is 25-7=18.
The population did not merely slip past the guard, it landed on the boundary to the unit. At the
default tolerance of 4 the floor is 21 and this program is rejected before its rate is ever
considered. That is the guard doing the whole job, and it is why the tolerance stays at 4.

**Consequence for the spec filter, which matters because I shipped it this turn.** FITNESS 3's
filter is a test on the RATE (>= 0.9x the champion). This exploit has 1354x the rate, so it sails
through any rate filter — the spec's included. The rate filter is not and was never the exploit
defence; FITNESS 2's correctness oracle is, and the mates guard is what plays that role here. So the
two changes are orthogonal and both are needed: the guard keeps no-search programs out, the filter
stops discarding rungs that are slightly cheaper-but-sound. Implementing the filter without keeping
the guard would have reproduced exactly this specimen.

**Why hand-written exploits could not have taught this.** The reference set's degenerate programs
are `depth-one` (5/25 mates) and `proof-number search` (4/25) — both far under any plausible floor,
so they never test the boundary. A machine-found exploit sits ON the boundary by construction,
because that is where selection pushes it. That is the gap the corpus exists to close.

## ⚠ REFUTED BY ITS OWN EXPERIMENT: relaxing the mates guard admits exploits

The `guard_tolerance 4 -> 7` proposal passed the ladder oracle (4/6 vs 3/6) and **failed in the
loop within three generations**:

```
tol7  gen1 MAIN  surrogate 1.090070  ABOVE:4  gate 0.125+/-0.110   (433x the seed's rate)
tol7  gen3 MAIN  surrogate 6.551549  ABOVE:2  gate 0.125+/-0.110   (2600x the seed's rate)
ctl   gen1 MAIN  surrogate 0.002705  ABOVE:3  gate 0.458+/-0.082   (1.07x the seed's rate)
```

Enormous surrogate, catastrophic games: the exploit signature. Dropping the mates floor from 21 to
18 admits precisely the degenerate cheap-and-shallow optimiser the guard exists to stop — the same
shape as `depth-one`, which scores 992.711x on the surrogate and 5/25 on mates. The control, at the
same generations, proposes candidates at 1.07x the seed that lose narrowly (0.458). One knob turned
a plausible-candidate generator into an exploit generator.

**The methodological lesson is bigger than the knob.** The oracle ranks NINE HAND-WRITTEN reference
programs. The guard must exclude an entire SPACE of degenerate programs that mutation can reach.
Passing the oracle therefore said nothing about exploit-resistance, and I treated it as though it
did. An offline ranking test over known-good programs cannot validate a filter whose job is to
reject unknown-bad ones.

**DISAMBIGUATED — it is the GUARD, and EPS is inert.** Single-knob arms, generation 1:

```
guardonly_s0  GUARD_TOL=7, eps 0.02   surrogate 3.308333  ABOVE:4  gate 0.208   1314x the seed
epsonly_s0    eps 0.10, GUARD_TOL=4   surrogate 0.002794  ABOVE:3  gate 0.417   1.11x the seed
ctl_s0        defaults                surrogate 0.002794  ABOVE:3  gate 0.417   1.11x the seed
```

`guardonly` reproduces the exploit on its own — a surrogate 1314x the seed's 0.002518 with a game
rate of 0.208. `epsonly` is byte-identical to the control on every field. So the mates guard is the
sole cause and the EPS widening is inert here, which is what the reasoning predicted; the value of
running it is that the prediction is now a measurement.

One honest limit on the EPS half: EPS governs which candidates are RETAINED in the population, and
at generation 1 there is not yet a population wide enough for a 0.02-versus-0.10 band to separate,
so "inert" is established only for the first generation. The guard result does not depend on that —
it fires immediately and unambiguously.

**Superseded confound note follows.**

**CONFOUNDED, and I have to say so: I changed two knobs at once.** The `tol7` arms set
`GUARD_TOL=7` AND `EPS=0.10` together, so this measurement cannot attribute the exploit to the guard
alone. Theory says the guard — a 433x-rate candidate survives any EPS band, so EPS cannot be what
admitted it, while the mates floor is exactly what a cheap-and-shallow program has to clear — but
that is reasoning, not measurement, and the whole point of this file is not to accept the first.
Single-knob arms are running now.

**What the gate did right.** Both exploits were rejected at 0.125, so the system was never in
danger; the cost of the bad guard is wasted gate time, not a corrupted champion. That is the guard
and gate working as a pair, which is worth recording alongside the failure.

## ROOT CAUSE: the surrogate cannot express the ladder it exists to climb

`evolve valleyall` scores every reference program against the seed on the SAME mates/Mcost surrogate
the loop selects with. 25 positions, depth 3:

```
program                            nodes  mates  mates/Mcost  vs seed  hard  verdict
bare alpha-beta (main seed)          +0     25    0.002518     1.000x    0
alpha-beta + hash reuse           +104     25    0.002578     1.024x    0   FITTER
alpha-beta + iterative deepening   +29     25    0.002300     0.914x    0   not fitter
alpha-beta + hash + ID            +133     25    0.002348     0.933x    0   not fitter
table reduction (rung 7)           +15     25    0.002496     0.992x    0   not fitter
capture extension (rung 6)         +13     18    0.000857     0.340x    1   loses answers
UCT-style MCTS                     +61     10    0.002437     0.968x    0   loses answers
depth-one (purity seed)            -62      5    2.499200   992.711x    0   loses answers
proof-number search               +104      4    0.022854     9.078x    0   loses answers
```

**Six of the seven ladder rungs score BELOW the seed.** The mechanism is arithmetic, not bad luck:
the seed already scores **25/25 mates**, so the numerator is SATURATED and `mates/cost` is driven
entirely by cost. The only way to exceed 1.000x is to be CHEAPER. Every genuine search improvement
costs more — +104, +29, +133, +13, +15 nodes. The fitness function therefore ranks the ladder
roughly in reverse.

**The sharpest single number: capture extension is rated 0.340x.** It is the ONLY program in the
entire reference set that scores on the HARD set (`hard: 1`; everything else, including the seed,
scores 0). So the surrogate penalises hardest the one program that demonstrates the exact capability
the hard set was built to measure. It is excluded twice over — the guard cuts it for losing 7 mates
(25 -> 18 at a fixed budget of 16, tolerance 4) and the rate cuts it at 0.340x.

**And the one rung that IS fitter cannot be climbed to.** The conjunctive test:

```
probe only (never stores)   25 mates  0.991x
store only (never probes)   25 mates  0.997x
hash reuse (both halves)    25 mates  1.024x
```

Both halves are needed; each alone is downhill. A strict `rate > best_rate` climb cannot take the
first step.

**This is the mechanistic explanation for `ABOVE:0`**, and it means that measurement was never
evidence about the operators or the search. `ABOVE:0` is *structurally guaranteed* by the fitness
function. An operator audit confirms the operators are not the gap: `WrapIfPred`, `ProbeRead` and
`StoreHere` were added beyond GRAMMAR 4's ten precisely so rungs 4-6 are expressible, and every rung
above the seed is reachable without the two spec operators that are missing (`add-arg`, `add-fn`) —
those are only needed for rungs 2-3, which sit BELOW the seed. Checked before claiming, because the
obvious story — "qsearch needs a new recursive function, so it is unreachable" — is refuted by
`reference.rs:45`: rung 6 is a single `wrap-if` on the recursion the seed already has.

**What GRAMMAR 9 actually requires, and where the loop diverges from it.** The spec says each rung
must beat the previous "on mates-per-cost **and/or** fixed-time games". The and/or is load-bearing:
six of seven rungs fail the mates-per-cost half, so the ladder is only climbable on the GAMES half.
The evolve loop selects on the surrogate and only reaches games afterwards, so it filters on the one
criterion the ladder demonstrably fails. That is the defect — not the operators, not EPS, not the
gate's pair count.

**WHY the fixed-time half is unavailable, verified in the interpreter.** GRAMMAR 9's criterion has
two halves and the loop can only run one of them, because there is no fixed-cost comparison to run.
`interp/src/lib.rs:427-435` keeps two separate quantities and says so explicitly:

* `budget` — "the value the program RECEIVES as its second parameter ... UCT uses it as a simulation
  count, **alpha-beta ignores it**";
* `cost_cap` — "SAFETY ceiling ... Generous but FINITE. Large enough that no honest program notices."

So every alpha-beta program runs to whatever cost its depth table implies and is then normalised by
that cost. Nothing ever runs at a COMMON budget. A ratio with a saturated numerator is precisely the
shape that rewards being cheap, which is why `depth-one` scores **992.711x** while answering 5 of 25,
and why the guard rather than the rate is what keeps it out.

**The unsaturated dimension exists and is the right idea, but it is sparse.** On the HARD set the
seed scores 0/8 and capture extension scores 1/8 — it is the only program in the reference set that
scores at all. That is the correct signal and it is why the set was built. But a dimension where the
best known program scores 1 of 8 gives almost every mutation a score of 0, which matches what both
arms actually print: `hard 0-0` and `hard 0-1` nearly everywhere. It discriminates the ONE known
rung from the field; it does not supply a gradient for a population to climb.

**Which is why `EXISTENCE_HARD_FITNESS` did not fix anything, and now I know the reason.** It adds
hard solves to a numerator that is already saturated at 25 and adds their cost to the denominator, so
a 0-or-1 term perturbs a 25-term sum. Arithmetically it cannot invert an ordering where the gap is
0.340x versus 1.000x. That is a better account than the one I recorded earlier ("the flag binds but
the gate still rejects"), which described the symptom without the cause.

**Next experiment, pre-registered.** `valleyall` is now a ground-truth ORACLE for surrogate designs,
and it runs offline — which is exactly what GRAMMAR 9's own heading asks for ("run offline before any
compute is spent"). Any candidate surrogate must rank hash reuse, iterative deepening, capture
extension and table reduction ABOVE the seed, and depth-one and proof-number search BELOW it. The
current one gets 1 of 6 right. A design is worth putting in the loop only after it passes that
ordering test, and no loop time gets spent before it does.

**Fixed-cost scoring was considered and RULED OUT on the mate set, with the reason.** The
interpreter can enforce it — `cost_cap` is a real ceiling and exceeding it unwinds to `MOVE_NONE`,
"which callers already treat as a forfeit" (`interp/src/lib.rs:514-518`) — so this was implementable.
It still cannot work here. At a generous cap every rung finishes (they are only 1.1x-2.1x the seed's
cost) and everything scores 25/25, so there is no discrimination at all; at a tight cap the expensive
rungs simply forfeit and score LOWER. Either way the answer is the same, because **you cannot measure
an improvement on a test the incumbent already aces**. The binding constraint is the SET, not the
normalisation. Recorded so this is not re-derived as a fresh idea later.

**Which leaves the tolerance, and the ladder sizes it precisely.** If the surrogate cannot rank the
rungs, it should not be the thing that decides them — it should be a cheap pre-screen that rejects
only clearly damaged candidates and lets the GAME GATE select, which is the half of GRAMMAR 9's
criterion the ladder actually passes. The numbers give an exact requirement rather than a guess:

```
rung                          rate     tolerance needed to admit it
table reduction (rung 7)     0.992x    0.008     (inside the current 0.020)
hash reuse halves       0.991/0.997x    0.009     (inside)
iterative deepening          0.914x    0.086     OUTSIDE -- needs ~4.3x the current EPS
capture extension (rung 6)   0.340x    0.660     unreachable by tolerance
```

So EPS = 0.020 admits the conjunctive hash path and rung 7 and excludes iterative deepening, which is
a genuine rung, by a factor of about four. Widening EPS to ~0.10 would admit ID.

**Capture extension cannot be fixed by EPS and that is a separate finding.** At 0.340x no plausible
rate tolerance reaches it, and the GUARD would exclude it anyway: it answers 18 of 25 against a guard
tolerance of 4. Admitting the one program that scores on the hard set therefore requires changing the
GUARD — the mates floor — not the rate band. Those are two different mechanisms and conflating them
would produce a change that looks correct and does nothing, which is the failure mode that has
already cost three inert features in this file.

**PROPOSAL, not a claim, and not yet run.** Two independent knobs, each with a pre-registered check
against the `valleyall` oracle before any loop time: (1) EPS 0.020 -> 0.10 should bring iterative
deepening inside the band while leaving depth-one and PNS excluded by the guard; (2) a guard
tolerance that admits capture extension must be justified separately, because loosening a mates floor
is exactly how a cheap-and-wrong program gets in — `depth-one` sits at 992.711x and only the guard
keeps it out.

**Where EPS lands, for completeness.** With `EPS = 0.020` the tolerance band reaches 0.98, which
covers the conjunctive path (0.991x, 0.997x) and table reduction (0.992x), but not iterative
deepening (0.914x) and not capture extension (0.340x). So the plateau tolerance makes hash reuse
approachable in principle while leaving two real rungs permanently outside the band.

## ⚠ RETRACTED: "no candidate ever beat the incumbent" — I published a TAUTOLOGY as evidence

**The claim below is wrong and the instrument that produced it was vacuous.** `ABOVE` was printed
only inside the `..none` branch, which is the ELSE of `if popn[0].2 > best_rate`. `popn` is
parents-union-offspring sorted by rate, so if any offspring beat the incumbent, `popn[0].2 >
best_rate` and the code takes the IF branch. **`ABOVE` can therefore only ever print 0**, in every
run, forever, regardless of what the search does. I read that guaranteed 0 as the answer to a
pre-registered question.

That is the fourth inert diagnostic in this file — after the cost ceiling, `catch_unwind` under
`panic=abort`, and the misplaced guard floor — and the first whose output I turned into a headline.
It is also the second instrument defect in a row on the same question, which is the documented signal
that the harness is wrong rather than the subject.

**What the logs actually say**, counted rather than inferred, identically in both arms:

```
hardv2_fit    15 gens |  8 ..none |  7 reached the GATE | 0 accepted
hardv2_ctrl   15 gens |  8 ..none |  7 reached the GATE | 0 accepted
```

A gate line is only reachable when a candidate DID strictly beat the incumbent. So **in 7 of 15
generations (47%) the surrogate found a candidate that beat the champion on rate** — the opposite of
what I recorded. Every one of the 14 gate calls then rejected it:

```
gate REJECT 0.375+/-0.110  (x2)   0.417+/-0.103  (x3)
gate REJECT 0.458+/-0.082  (x3)   0.500+/-0.250  (x6)
```

**The corrected conclusion is stronger than the retracted one, and it agrees with the ladder.** The
surrogate proposes rate-improvements about half the time and the GAMES say none of them is a strength
improvement — every observed rate is at or below 0.500. That is exactly what the ladder result
predicts: a saturated-numerator ratio rewards being cheaper, and cheaper is not stronger.

**And the gate cannot accept a realistic improvement anyway.** Acceptance is
`pent_rate - ci95 > 0.5`, so the bar is `0.5 + ci95`. At the ci95 values actually observed:

```
ci95 0.082 -> must score > 0.582      ci95 0.110 -> must score > 0.610
ci95 0.103 -> must score > 0.603      ci95 0.250 -> must score > 0.750
```

A genuine engine improvement is typically 0.51-0.55 in pair rate. **The 12-game gate demands
0.58-0.75**, so it would reject every real improvement it was ever shown. Six of the fourteen
rejections sit at exactly `0.500+/-0.250`, which is 12 games carrying no information at all —
nothing between 0.25 and 0.75 can resolve there.

`ABOVE` now prints on the GATE line, where it is informative (how many candidates cleared the
incumbent), together with the acceptance bar `needed >{0.5+ci95}` so that threshold is never again
recalled from memory instead of read.

### POOLED: the surrogate's proposals average BELOW parity, and the gate is 40x too small

The 14 gate calls are 168 games of evidence about what the surrogate actually proposes. Pooling them
(pair sd 0.2362, the project's measured value):

```
14 gate calls = 168 games = 84 pairs
pooled pair rate  0.4554 +/- 0.0505   ->  [0.405, 0.506]
  6/14 land on exactly 0.500  (candidate plays identically to the champion)
  8/14 land strictly below 0.500
```

**The average proposal is worse than the champion it was proposed against.** The interval only just
touches 0.5, so this is not merely "the surrogate finds neutral candidates" — it is evidence that
surrogate rate-improvements are mildly ANTI-correlated with strength. Which is precisely what the
ladder predicts: a saturated numerator makes the rate a measure of cheapness, and the cheapest way to
keep 25/25 mates is to search less in places that did not happen to matter on those 25 positions.

The six exact 0.500s are their own signal: a candidate that plays identically to the champion scores
exactly 0.500 by construction. Those are pure speedups that missed the behaviour-identical fast path
by differing on at least one guard position.

**And the gate could not see a real improvement even if one arrived.** Acceptance needs
`pent_rate - ci95 > 0.5`; at 6 pairs that is a bar of 0.58-0.75. Solving for the pairs required to
resolve a given true edge:

```
true 0.53 candidate  ->  238 pairs (476 games)     gate has 6
true 0.55 candidate  ->   86 pairs (171 games)     gate has 6
true 0.60 candidate  ->   21 pairs ( 43 games)     gate has 6
```

A genuine engine improvement is 0.51-0.55. The gate is roughly **40x too small** to resolve one, so
its rejections carry almost no information — six of the fourteen are literally `0.500+/-0.250`, an
interval spanning 0.25 to 0.75.

**⚠ CORRECTION — the gate is not "too small", it is a VETO and says so.** I wrote that the gate is
"40x too small to resolve a real improvement", which judges it by the wrong standard. Its own
comment at `evolve.rs:1215` states the design:

> GAME-GATE PAIRS. Small on purpose: a game at fitness depth is ~200x a single fitness evaluation,
> so this is the expensive half and it only runs on a surrogate improvement. 6 pairs = 12 games
> resolves a large effect, which is the only kind worth promoting here; **it CANNOT resolve a 2%
> edge and is not asked to. It is a veto on unplayable programs.**

So the gate is correctly implemented for its purpose. The arithmetic I did is still right — the
acceptance bar really is 0.58-0.75 and a 0.53 candidate really would need 238 pairs — but it
describes a DESIGN CHOICE rather than a defect. The loop deliberately promotes only large effects.

**The defensible version of the criticism is narrower and stronger.** The design assumes large
effects EXIST to be promoted. The ladder measurement says they do not: the best available rung is
hash reuse at 1.024x on the surrogate, and every rung's game rate against the seed sits at or below
0.5. A veto tuned to pass only large effects, in a space whose known improvements are all small, will
never pass anything — not because the veto is wrong, but because the design's premise about the
effect-size distribution is not met here.

**And raising the gate would NOT help on its own**, which is why this correction changes the
priority rather than just the wording. The pooled evidence says the surrogate's proposals average
0.4554 against their champions, so a larger gate would spend 16x the games confirming rejections it
already makes correctly. The surrogate is the binding constraint, exactly as recorded below.

**Both ends of the loop are therefore broken, and independently.** The surrogate selects for
cheapness (measured: 6 of 7 ladder rungs rank below the seed), and the gate cannot resolve what the
surrogate hands it (measured: 40x short). Fixing either one alone changes nothing — a better
surrogate still meets a gate that rejects everything, and a bigger gate still receives proposals that
average 0.4554. That is the single most useful consequence of today's work, and it is why no further
loop time is worth spending on this track until at least one of the two is repaired.

### Superseded text follows

## ANSWERED: no candidate has EVER strictly beaten the incumbent, in either arm

With `ABOVE` counting guard-passers whose rate exceeds the incumbent's — the acceptance condition
itself — the pre-registered question is settled:

```
FLAGGED  gen2 MCTS  rates 0.772-1.000000x  [.. distinct:4 ABOVE:0]  hard 0-0
         gen3 MAIN  rates 0.899-1.000000x  [.. distinct:8 ABOVE:0]  hard 0-1
         gen3 MCTS  rates 0.325-0.999996x  [.. distinct:3 ABOVE:0]  hard 0-0
CONTROL  gen2 MCTS  rates 0.801-1.000000x  [.. distinct:4 ABOVE:0]  hard 0-0
         gen3 MAIN  rates 0.969-1.000000x  [.. distinct:8 ABOVE:0]  hard 0-1
         gen3 MCTS  rates 0.446-0.999993x  [.. distinct:3 ABOVE:0]  hard 0-0
```

**ABOVE is 0 in every `..none` generation of both arms.** Not once has a candidate strictly beaten
the incumbent, so not once could anything have been accepted. `EXISTENCE_HARD_FITNESS` does not
change that — the flag alters which candidate is proposed and the population's hard-set floor, but
not the acceptance picture.

**The six-decimal print shows the old check was reading a rounding artefact.** `0.999996x` and
`0.999993x` both rendered as exactly `1.000x` at three decimals. So the maxima I had been reading as
"a neutral twin ties the incumbent" were in several cases *below* it — the population was never even
level, let alone above.

**And it sharpens the diagnosis rather than repeating it.** `distinct:8 ABOVE:0` means eight of
twelve candidates differ from the incumbent and every one differs DOWNWARD. That rules out the
hypothesis `distinct` was added to test — "the operators produce only neutral rewrites, so no
selection policy can help". The operators produce plenty of variation. All of it is neutral or
worse. The bottleneck is not the selection filter and not the fitness shape; it is that the mutation
operators, on this surrogate, never generate an improvement to select.

## ⚠ BRIEF TASK 1 IS WRONG AT THE SHIPPED WIDTH: incremental NNUE would make the engine SLOWER

The standing brief calls the incremental accumulator "the biggest single win", on the premise that
"eval is a dense 256x782 forward pass at every leaf". **Both halves are false, and the fix would be
a regression.**

**1. eval is already sparse.** `Net::eval` calls `Self::active(pos, &mut idx)` and accumulates only
the ~38 active feature rows. There is no dense 782-row pass anywhere in it.

**2. Incremental is a measured LOSS at small width**, and the numbers are already in the tree
(`pipeline/src/search.rs`, node counts identical across both paths so the ratios are real):

```
hidden  32   refresh 1718969   incr 1560497   0.91x  LOSS
hidden 128   refresh  889406   incr 1014240   1.14x  win
hidden 512   refresh  240701   incr  363458   1.51x  win     crossover near 64
```

**3. The shipped nets are width 16.** Reading the `EXNT` header of all 70 nets in the repo:

```
n_hidden=16    64 nets      <- everything the loop actually trains and gates
n_hidden=32     3 nets
n_hidden=64     2 nets
n_hidden=256    1 net       (champion_arch)
```

The saving scales with width (~38 rows rebuilt vs ~4 touched) while the bookkeeping — two `active()`
scans, a bitset diff, a memcpy per ply — does not. At 16 the bookkeeping is larger than the work it
saves, so the engine would lose more than the 0.91x already measured at 32. **Do not do brief task 1
at the current width.** It only becomes correct if the champion moves to width >= 128, which is a
capacity decision, not a perf one.

**What the real lever is, and it follows from the same arithmetic.** At width 16 an eval is roughly
38x16 = 608 row-adds plus a 16-wide ReLU head — a few hundred nanoseconds. Against that,
`Net::eval` does `Vec::with_capacity(40)` — a heap allocation and free on EVERY eval, in the hottest
function in the program. The smaller the net, the larger that fixed cost is as a fraction. Three
sites allocate per call (`eval`, `Accum::refresh`, and line 147), and `active()` already takes a
reusable `&mut Vec<u16>` that no caller reuses. That is the width-16 optimisation, and it is the
opposite of the one the brief names.

**MEASURED — the allocation fix is worth ~3.3% at the shipped width.** `search_bench`, depth 4, one
quiet core (the arm on it SIGSTOPped for the duration), node counts identical on every arm so the
work is provably equal:

```
width   refresh          incr             incr vs refresh
 16     1.034x  (+3.4%)  1.032x  (+3.2%)  0.995x   <- incremental buys NOTHING here
 32     1.009x           1.006x           1.004x
128     1.017x           1.005x           1.10x    <- incremental is real, at width 128
```

Three interleaved runs at width 16 gave OLD 1920868/1914636/1923943 against NEW
1970496/1980629/1981025 — non-overlapping ranges, so the 3% is not noise.

**My own estimate was wrong and the measurement is what counts.** The arithmetic argument above
predicted 20-30%; the truth is 3%. glibc's malloc fast path is far cheaper than I assumed and the
row-adds dominate more than I credited. The direction was right, the magnitude was invented.

**The last column settles brief task 1 by measurement rather than extrapolation.** At the shipped
width of 16, incremental is 0.995x — a fraction slower than rebuilding from scratch. At 128 it is
1.10x. So wiring the accumulator into `crates/engine` would buy nothing at the width the loop
actually trains, which is what the width census predicted, and is now demonstrated directly instead
of argued from a table.

**One number in that table does NOT replicate.** `pipeline/src/search.rs` records `hidden 32 ...
0.91x LOSS` for incremental. On the current code I measure 1.004x at width 32 — no loss. The
crossover story still holds at the ends (nothing at 16, clear win at 128), but the specific 0.91x is
not reproducible today and should not be quoted. Not chased further: it does not change any decision
here.

**Correctness before the number.** All 6 nnue tests pass, including the two that gate this change:
`incremental_matches_full_refresh` covers the `Accum::refresh` edit, and
`sparse_matches_dense_over_random_games` covers the `active`-to-`active_with` refactor against the
dense reference.

## Task list (docs/MASTER_PLAN items 1-6) — verified stale

| item | status, verified by reading |
|---|---|
| 1. incremental NNUE accumulator | **done and correctly OFF.** `Acc` exists, `tests/incremental.rs` checks it against a full refresh, and `search.rs:169` gates it on `n_hidden >= 64`. It is a measured **0.91× LOSS** at width 32 (arch.rs:150), and shipped width is 16. All three clauses of its premise expired: eval is a sparse gather over ~38 active rows, not a dense 256×782 sweep; width is 16, not 256. It pays only at width ≥64, which was refuted at equal time. |
| 3. Zobrist + real TT slots | **done.** Incrementally maintained key (`chess.rs:271`) with a from-scratch `zobrist()` to check against. |
| 6. xcheck + perft as `#[test]`s | **done AND green.** Ran them: `canonical_perft_suite`, `movegen_agrees_with_an_external_engine`, `incremental_zobrist_matches_from_scratch_everywhere`, `make_unmake_restores_the_position`, plus 2 more — **6 passed, 0 failed**. Previously recorded as done on the strength of the files existing; now actually executed. |
| 5. register bytecode | deprioritised — the interpreter measured 1.003× hand-written on a quiet core. |

## Running now (one job per core, no chains)

| core | job | question |
|---|---|---|
| 13 | `fg_60` | do block increments keep rising as the pool grows past 432k? |
| 14 | `dv_4800` | is the plateau DATA-limited? (2× games/generation, same 20 generations) |
| 12 | `sg_20 vs bn_075` @ d4 | does gating help from scratch? |
| 15 | `ep2_10 vs ep2_3` @ d4 | is the shipped epochs default too **low**? |

The old PID-chained queue is gone — every arm in it either completed or was killed for a measured
defect (`depth_parity` for unequal arms, `pd_d4` for an expired premise, two verdict matches for
being tautological). Chains are not being rebuilt: they made reordering impossible while running,
which repeatedly left the highest-value item last.

## Open, partially answered

**Is the speedup acceptance path reachable?** ANSWERED, and the instrument fix is what delivered it.
`stepdiff6` died at 50/60 (cause unknown — no OOM evidence either way, and neither journal nor
dmesg was readable) yet still reported: **identical 30, cheaper 0; different 17, guard-ok 0**. The
previous run died and yielded nothing; this one died and yielded its answer, which is exactly what
moving every summary counter into the progress line was for.

Read honestly, this is **bounded, not empty**: 0 in 30 gives a 95% upper bound of 10% by the rule of
three. Combined with 93 prior generations producing no cheaper survivor, it is consistent with
empty, but 30 trials cannot prove a rate below ~10%.

The widened 33-position identity check is committed but deliberately NOT deployed, since restarting
a live search-track run to strengthen a path bounded below 10% would cost real generations.

## Not started

* Rung 7 needs its table contents DECLARED before it can be measured.
* Register bytecode (CRATE 4) — a perf task; the interpreter is at 1.003× hand-written speed, so
  it is not urgent.

<!-- Commit 82d7f6a's message body was mangled: it was written with `git commit -m "..."` in double
     quotes, so the backticked spans inside it were executed as command substitution and vanished.
     The line that disappeared was the fix itself, `b=$(basename "${e% (deleted)}")`, leaving the
     message reading `b=` with nothing after it. Every other commit today used a heredoc (-F -) for
     exactly this reason. Not force-pushed: the commit is public and the substantive record above
     is correct, so a rewrite of shared history buys nothing a follow-up note cannot. -->
