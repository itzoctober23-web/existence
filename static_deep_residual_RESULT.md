# Half of the P1 kill criterion has never been measurable — the static-vs-deep residual, built and refuted

**2026-09-11.** A diagnostic that two specification documents treat as load-bearing did not exist in
the code. It exists now, and the first thing it establishes is that **it cannot do the job
MASTER_PLAN assigns it.**

## What was missing

```text
MASTER_PLAN.md:273  (P1 kill)   "no iteration-over-iteration gain across iterations 4-8 AND the
                                 static-vs-deep residual is not shrinking -> pipeline bug"
FITNESS.md:270      (§8 check)  "the candidate's explanation-layer confidence (PV stability,
                                 static-vs-deep residual class) must be LOW on at least 80% of
                                 positions where the candidate's move differs from its own
                                 32x-cost move"
```

A grep for `residual` across every crate returns only profiling breakdowns and R² spreads. **The
quantity was never implemented.** The first conjunct of the P1 kill was measured many times tonight
and read as a plateau; the kill was never evaluated, because the conjunction was never evaluable.

`crates/pipeline/examples/static_deep_residual.rs` now measures it: for a fixed position set, the
gap between a net's **static** eval and what a **search** returns from the same position, both
converted to White's point of view.

## The measurement — the matched lr pair, plus an untrained control

600 positions, depth 4, seed 20260911. `lrA` (lr 0.01) and `lrB` (lr 0.002) are the **finished**
arms of tonight's A/B: same start, same seed, 2,000 generations each.

| net | mean\|r\| | rms r | sd static | sd deep | rms/sd | **corr** | **rms TANH** |
|---|---|---|---|---|---|---|---|
| origin(random) | 8.6 | 10.9 | 25.0 | 21.8 | 0.501 | **0.900** | **0.0181** |
| lr_start | 85.9 | 124.1 | 237.9 | 281.5 | 0.441 | 0.899 | 0.1681 |
| lrA (lr 0.01) | 92.2 | 128.1 | 262.7 | 293.0 | 0.437 | 0.901 | 0.1666 |
| lrB (lr 0.002) | 149.5 | 215.0 | 388.3 | 452.5 | 0.475 | 0.880 | 0.2601 |

**The untrained random net wins on the trainer's own metric.** Its tanh residual, 0.0181, is an
order of magnitude below every trained net's.

## Why — two mechanical defects, neither fixable by more positions

**1. The two sides are the same function.** `datagen.rs:187` shows the search evaluates leaves with
*the net being measured*. Static and deep are therefore one function at two depths, and they agree
by construction: **corr 0.900 for a net with no training at all.** High static-vs-deep agreement is
not evidence of anything.

**2. The residual scales with the net's output range.** The random net's eval is nearly constant
(sd 25.0), so search cannot move it far and its residual is tiny. A net that learns to distinguish
positions *necessarily* widens its output range — `lr_start` 237.9 → `lrB` 388.3 — and its residual
grows with it. **Ranking nets by residual rewards a small output range, not a good evaluation.**

This is why `lrB` looks worst here: it has the widest output scale. That is an artefact of the
metric, **not** evidence against tonight's lr result — which rests on `netmatch`, a paired game
instrument with no dependence on output scale.

## What this does to the P1 kill criterion

> Kill: no iteration-over-iteration gain across iterations 4-8 **AND the static-vs-deep residual is
> not shrinking** → pipeline bug; stop and find it.

**As written, this criterion cannot fire**, because its second conjunct is not a well-defined
quantity across checkpoints: it moves with output scale, and the direction that indicates *learning*
(a widening range) is the direction that makes the residual *grow*. A conjunctive kill with one
unmeasurable conjunct is a kill that never fires — which is what happened tonight.

**Proposed repair — IMPLEMENTED an hour later, see the second half of this file:** score the deep side with a **fixed reference** — one
frozen strong net for every checkpoint — so the target stops moving with the net under test. That
makes the residual a comparable quantity and removes both defects at once. It is a different
measurement from the one the docs specify, so it is proposed here rather than quietly substituted.

## What survives

**FITNESS.md:270 is well-posed and untouched by this.** It asks a *per-position* question *within
one net* — does this net anticipate its **own** 32×-cost search — and uses the answer to classify
positions, never to rank nets. Self-coupling is not a defect there; it is the subject. That check
remains implementable as specified.

## It agrees with two standing results, for a third reason

`lrB` fits the labels **worse** and plays **better** (0.692 vs 0.499 by netmatch). Scale domination
explains the residual reading, so this is weak evidence — but the direction matches two independent
findings already on file:

* `epochs_ab_RESULT.md` — more epochs drove train_loss 0.0417 → 0.0253 while mcnemar_z went +0.359 → −0.445.
* `depth5_vs_depth3_RESULT.md` — *better* labels (depth 5) scored 0.397 against their own start.

> **⚠ CORRECTED 2026-09-11 03:5x — "anti-correlated" OVERCLAIMS, and the correct word is
> UNINFORMATIVE.** `proxies_RESULT.md` had already settled this at a scale none of tonight's
> readings approach: the held-out surrogate at **r = −0.095 over 239 paired gate results**, training
> loss at **r = +0.379, CI [−0.249, +0.783], n=12** — and crucially it contains examples pointing
> **both** ways (w64 has the lower loss and LOSES; the draw filter has the lower loss and WINS).
> A signal that fails in both directions is noise with respect to strength, not a reversed predictor.
>
> The specific defect in tonight's version: the arms it compares differ in **LEARNING RATE**, which
> changes how much the net fits per generation *independently* of how good the net is. lr moves both
> columns, so the monotone ordering is expected with no causal link between loss and strength.
> n=3 against n=239 besides. This was a closed question, already indexed in `RESULTS_INDEX.md`.
>
> **What survives:** loss must not be used to select or veto candidates here. That conclusion is
> unchanged — it just rests on `proxies_RESULT.md`, not on these three points.

Three measurements, three instruments, one direction: **in this pipeline, fitting the search-derived
labels better does not make the engine stronger.** That is the standing "the labels are the problem"
conclusion, and nothing tonight weakens it.

## Method note — my own control check returned a confidently wrong verdict

The first version printed:

```text
control check: best trained 0.437 vs origin(random) 0.501 -- instrument separates trained from untrained
```

It compared **one column**, `rms/sd_deep`, which happens to favour the trained nets. The two columns
added afterwards — `corr` and `rms TANH` — both favour the **untrained** one, and they are the more
diagnostic pair. The check passed on a subset of the evidence and stated the opposite of the truth.

`corr` was worse than unread: it was already being **computed**, for the frame-mismatch gate, and
simply never printed. The number that overturned the conclusion was in the struct the whole time.

The verdict block now tests the two failure modes that actually occur and prints `NOT USABLE AS A
CROSS-NET STRENGTH OR HEALTH RANKING` on this data. **A check that reports a pass on a subset of the
evidence is worse than no check**, because it launders an artefact as a result.

## What was built in, and did its job

* **Frame gate.** `Net::eval` is mover-relative and so is negamax; if either reading were wrong every
  residual would be `|a − (−a)| = 2|a|` — a wrong number, not a crash. Both sides are converted to
  White's POV and the correlation is gated at 0.30, because a frame flip inverts its sign and noise
  does not. (The tighter check, a depth-0 search reproducing the static eval exactly, is unavailable:
  `best_move` computes `depth - 1` on a `u32` at the root and underflows.)
* **Mate exclusion, counted not dropped.** A ±MATE score differs from any eval by ~30,000 and a
  handful would swamp the mean. 8 of 600 excluded, reported in the table.
* **Untrained control, every run.** Following `eval_anatomy`. It is the control that refuted the
  instrument — which is the entire reason for carrying one.

---

# The repair works — and a game-free instrument independently reproduces the lr result

Added the same hour. The proposal above ("score the deep side with a fixed reference") is
implemented as `--ref`, and it does what the self-referential form could not.

## The circular first run, caught by a repeated digit

The first `--ref` run used `p1_champion.net` as the reference while measuring `lrB.net`, and printed
`lrB 0.880`. **0.880 is exactly what lrB scored in the self-referential table.** Three matching
decimals is not coincidence:

```text
p1_champion.net   dfd258b070264e53
lrB.net           dfd258b070264e53      <- byte-identical
```

lrB was promoted to champion hours earlier, so the reference *was* the net under test. It scored
against itself, and every other row measured distance-from-lrB rather than agreement with an
independent opinion. **That run is void.**

The tool now refuses it (`exit 4`) rather than printing: a reference byte-identical to any net under
test aborts. The bug was invisible in the numbers except for that repeated digit, which is exactly
the kind of tell that does not survive a busier table.

## The adversarial re-run

The reference was then chosen to be **biased against the conclusion**: `p1_champion_prev_pre_lr002`,
the champion from *before* the lr change — the lr 0.01 lineage, i.e. lrA's side of the family.
600 positions, depth 4.

| net | corr | sign agree | sd static |
|---|---|---|---|
| origin(random) | **−0.019** | **47.8%** | 25.0 |
| lr_start | 0.664 | 73.5% | 237.9 |
| lrA (lr 0.01) | **0.613** | 66.6% | 262.7 |
| **lrB (lr 0.002)** | **0.743** | **76.7%** | 388.3 |

**lrB > lr_start > lrA**, with lrA landing *below the start it trained from*.

## Why this is worth more than a fourth match

`netmatch` scored the same pair 0.692 (lrB) against 0.499 (lrA) — lrB gained, lrA went nowhere. The
table above is the same ranking from an instrument that shares **no machinery** with it: no games,
no openings, no pairing, no seed lottery, and the between-seed sd of 0.047 that makes a single
`netmatch` reading a lottery does not apply. And the reference was drawn from the **losing arm's**
lineage, so the bias ran against the result.

`learning_rate_is_the_plateau_RESULT.md` closes with "**Not replicated.** One seed." This is not a
second seed, but it is a second *instrument*, which is the stronger of the two things one can add.

The random control is the load-bearing row: **corr −0.019, sign agreement 47.8%** — indistinguishable
from a coin flip. In the self-referential mode the same untrained net scored corr 0.900 and won the
trainer's own metric outright. The mechanical coupling is gone, which is what the repair was for.

## Status of the P1 kill criterion

Still **not repaired as written** — MASTER_PLAN specifies the self-referential form, and that form
remains unusable across checkpoints. What exists now is a working substitute with different
semantics ("agreement with a fixed stronger opinion", scale-free), offered as a proposed amendment
to the document rather than quietly swapped in behind the same name.

Both conjuncts of the kill can now be evaluated. Neither fires: strength gain is **not** absent
(the lr result), and agreement with a fixed reference is **not** flat across the lineage.
