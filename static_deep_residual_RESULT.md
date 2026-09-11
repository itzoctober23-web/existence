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

**Proposed repair, not yet implemented:** score the deep side with a **fixed reference** — one
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
