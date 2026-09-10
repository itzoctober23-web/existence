# PATH 1 (behaviour-preserving speedup) is CORRECT and EMPTY

The `evolve` loop has two acceptance routes. PATH 2 is the game gate. **PATH 1 accepts with NO GAME
at all**: a candidate that plays IDENTICALLY to the champion on every guard position and costs less
is a pure speedup, so no games are needed to justify it. That is the route the one known real
improvement — hash reuse, +2.4% — would have to take, because it changes nothing about play.

`evolve.rs` poses the question and leaves it open:

> *"IS THE SPEEDUP PATH REACHABLE? ... But 93 generations produced ZERO survivors cheaper than the
> champion, so the path may be correct and EMPTY — a fourth inert feature. Counting it here instead
> of waiting to find out."*

## The count, and it is the answer

`evolve stepdiff`, single edits from the seed, 125 of 200 attempts so far:

    broken     14
    identical  70   <- plays identically on every position
      of which CHEAPER: 0
    different  41   (guard-ok 0)

**70 behaviour-preserving mutants. Zero cheaper. Not one.**

Three independent measurements now agree that PATH 1 is empty:

| source | measurement |
|---|---|
| `evolve.rs` comment | 93 generations, ZERO survivors cheaper than the champion |
| `stepdiff` (this run) | 70 identical-playing single edits, **0 cheaper** |
| live arms | PATH 1 has never fired in any currently-running arm |

## The path is CORRECT — verified, not assumed

`gate_spec_veto_arm.log` (15:58) contains four `ACCEPT speedup` lines reading
`0.002490 was 0.002490` — identical play at IDENTICAL cost, promoted. That looks like a live defect
and is not one. `evolve.rs` records the cause: the "costs less" half was *"silently guaranteed by the
caller, because the strict filter picks only when `popn[0].2 > best_rate`"*, and
`EXISTENCE_SPEC_FILTER` breaks that unstated invariant by picking on `r >= 0.9 * best_rate`. Fixed in
`92a542a`. **Verified live**: the current condition is

```rust
if same_play && rate > best_rate {
```

a STRICT inequality, so identical-play-at-identical-cost can no longer promote. Those four lines are
pre-fix history from a SPEC_FILTER arm.

## What this means

**The one route that could admit hash reuse without playing a single game is open, correct, and
receives nothing.** Combined with the rest of tonight's chain:

* reachability is SOLVED — 5.6% of `ab<-uct` crossovers carry both TT halves;
* the rung is +104 nodes, so it is not one edit from the seed;
* single edits that preserve behaviour never reduce cost — 0 of 70;
* single edits that change behaviour never survive the guard — 0 of 41 (extends the repo's recorded
  0 of 33 to a larger sample, same result).

So a program can be *carried* to the rung by crossover, but it cannot *walk* there: every single step
is either a no-op that costs the same, or a change that breaks correctness. **There is no gradient in
either currency** — not in cost, not in mates.

## Honest limits

* Single edits **from the seed only**. The live loop applies 1-3 edits from evolved parents, and a
  no-op rate measured on the pristine seed need not hold on a mutated one.
* `cheaper` is measured on the guard set at depth 3 with `cost_cap` 20e9. A speedup that only pays at
  greater depth would be invisible here — and `ab_hash` is exactly such a case in miniature (+2.4% at
  this depth, more where a TT sees repeat traffic).
* 125 of 200 attempts at the time of writing; the ratios have been stable across all five checkpoints
  (0 cheaper at 15, 30, 40, 55 and 70 identical), but the run is not finished.
* This measures the SPEEDUP path only. It says nothing about whether PATH 2 (the game gate) is
  correctly calibrated — that is `gate_bounds_RESULT.md`.

## The target QUALIFIES — verified directly, so the path is not merely open, it is aimed

`evolve moveagree 40 3` compares each reference program's chosen move against the seed's, on 40
random positions:

| program | agree | pct | cost | class |
|---|---|---|---|---|
| bare alpha-beta (main seed) | 40/40 | 100.0% | 1.000x | EXACT — must be 100% |
| **alpha-beta + hash reuse** | **40/40** | **100.0%** | **0.971x** | EXACT |
| alpha-beta + iterative deepening | 40/40 | 100.0% | 1.075x | EXACT |
| alpha-beta + hash + ID | 40/40 | 100.0% | 1.046x | EXACT |
| depth-one (purity seed) | 10/40 | 25.0% | 0.000x | different paradigm |
| UCT-style MCTS | 13/40 | 32.5% | 0.534x | different paradigm |

**Hash reuse plays IDENTICALLY on every position and costs 0.971x.** That is exactly PATH 1's
condition, `same_play && rate > best_rate` — identical play, strictly better rate. (The 0.971x here
is a COST ratio, lower being better; the valley table's 1.024x is mates-per-cost, higher being
better. `1/0.971 = 1.030`, so the two agree within the difference between a 40-random-position set
and the 25-position mate/depth/window set.)

**And it is the UNIQUE qualifying rung.** Iterative deepening and hash+ID also play identically —
they are exact transformations too — but both cost MORE (1.075x, 1.046x), so PATH 1 correctly
refuses them. Of everything in the reference set, only hash reuse is both behaviour-preserving and
cheaper.

**So the path is not merely open and correct; it is aimed at exactly one target, and that target
qualifies.** What it never receives is a candidate. The failure is entirely upstream: 0 of 70
single edits are identical-and-cheaper, and the one program that would satisfy the condition is
+104 nodes from the seed.

## 2026-09-10 — NEITHER operator can produce a PATH-1 candidate. The dichotomy is complete.

PATH 1 accepts with **no games at all**: identical play everywhere, and cheaper. `moveagree` showed
the target qualifies (`ab_hash`: 40/40 identical, 0.971x). The remaining question was whether either
operator can deliver such a child. Both have now been tested against this route specifically — which
is NOT the same test as the mate guard, and had never been run on crossover output.

    MUTATION (stepdiff, 200 single edits from the seed)
      plays identically : 100      <- preserves behaviour readily
      ...AND cheaper    :   0      <- never reduces cost

    CROSSOVER (ttgraft, both-halves children, ab <- uct)
      plays identically :   0 / 40 <- never preserves behaviour
      ...AND cheaper    :   0 / 40   (n=40 completed; the n=12 interim read the same)

**POSITIVE CONTROL, run before the children and printed on every run:**

    CONTROL ab_hash: same-play true  cost 9698559036 vs seed 9930290911  -> PATH-1 acceptable: true

A `0 / 12` is exactly the kind of clean zero that is indistinguishable from a broken comparison, and
`moveagree` gives independent ground truth, so `ab_hash` must read same-play here or `base_moves` is
wrong. It reads true, and correctly identifies the rung as PATH-1 acceptable. The zeros are real.

### The dichotomy

| operator | preserves behaviour? | reduces cost? | PATH-1 capable |
|---|---|---|---|
| mutation (1 edit) | **yes, 100 of 200** | **never, 0 of 100** | no |
| crossover (both-halves graft) | **never, 0 of 40** | (moot) | no |

**Mutation preserves behaviour but never reduces cost. Crossover changes cost but never preserves
behaviour.** PATH 1 requires BOTH at once, and no operator in the grammar produces both. That is why
the route is open, correct, aimed at a qualifying target, and permanently empty — not because of a
threshold, a bound, or a tolerance, but because the operator set cannot express the conjunction.

### Honest limits

* **n = 40 crossover children, drawn from 499 attempts** — the launched run completed and the ratio
  did not move from the n=12 interim (0 either way). The mutation side is n = 200. Both are firm
  enough for the conjunction claim; neither is large enough to bound a rare event below ~2%.
* Crossover here is `ab <- uct` only — the graft that carries TT primitives. A crossover between two
  MAIN-lineage members is a different distribution and is not measured.
* "Cheaper" is on the 25-position set at depth 3 with `cost_cap` 20e9. A speedup that only pays
  deeper is invisible, and `ab_hash` itself is a small-margin case (2.9% here).

## ⚠ 2026-09-10 — CORRECTION: it is NOT an expressiveness gap. The primitives ARE reachable.

The section above concluded that "no operator in the grammar produces the conjunction" and called it
an expressiveness gap. **That explanation is wrong**, and the repo's own instrument says so:

    operators can introduce: {Budget, Const, Field, Key, Loop, Max, Pred, Probe, Store}

`ALL_OPS` contains **`Op::ProbeRead`** — which turns an Int-typed leaf into
`Field(Probe(Key(Var("p"))), f)`, i.e. Probe, Key AND Field in ONE edit — and **`Op::StoreHere`**,
which introduces Store. Their own doc comment states the intent plainly: *"Introduces Store. Paired
with ProbeRead this makes hash reuse REACHABLE -- not assembled."*

So mutation can build every TT primitive, and hash reuse is two well-placed edits, inside the loop's
1-3 edit budget. Nothing is inexpressible.

**What survives, unchanged:** the MEASUREMENTS. 0 of 100 behaviour-preserving single edits are
cheaper; 0 of 40 crossover children preserve behaviour; PATH 1 has never fired. Those are counts and
they stand.

**The correct explanation is the VALLEY, which `ladder_valley_RESULT.md` already established.** A
single `ProbeRead` adds a probe against a table nothing has written — a guaranteed miss, so pure
overhead (measured 0.991x). A single `StoreHere` writes entries nothing reads (0.997x). Both halves
are individually WORSE, and only the pair pays (1.024x). So "0 of 100 identical mutants are cheaper"
is not evidence that the operators cannot express a speedup — **it is the valley showing up in the
PATH-1 statistic**, exactly where the valley predicts it.

**Why I got it wrong, and it is the same mistake as four earlier tonight:** I read
`tests/reachability.rs`'s HEADER — *"they cannot introduce a PRIMITIVE the program does not already
contain... hash reuse needs Probe/Key/Field/Store"* — and treated it as current. That header is
stale; the test BELOW it prints the constructible set and that set contains Probe and Store. The
refutation was in the same file as the claim, in the output rather than the prose. Prose in a repo
this active is a hypothesis; the measurement next to it is the fact.

**Net effect on the diagnosis:** the barrier is a conjunctive VALLEY, not expressiveness, and the
route out is whatever crosses a valley — accepting a non-improving intermediate, or a single edit
that installs both halves at once. The second is what MASTER_PLAN line 53 forbids as hand-coding the
answer; the first is what EPS was for, and EPS is separately measured inert because the band it
widens into is empty.
