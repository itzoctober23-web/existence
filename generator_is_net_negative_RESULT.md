# Five generations of training make the net WORSE — the gate was never the problem

**2026-09-11.** Twenty direct, paired, unsaturated measurements of what a batch of five generations
is actually worth. The answer resolves the plateau question that `acceptance_floor_RESULT.md` and
`batch_gate_saturated_RESULT.md` were both circling, and it is not about the filter.

## The measurement

`EXISTENCE_DIRECT_BATCH=1` makes the batch gate decide on a **direct champion-vs-base match**
instead of a difference of two frozen-origin scores. Each decision is therefore a clean read of one
question: *is the net after five generations better than the net it started those five generations
from?*

Twenty decisions, 100 generations, 224 pairs each:

```text
0.410 0.412 0.419 0.434 0.438 0.445 0.450 0.454 0.462 0.467
0.468 0.483 0.483 0.485 0.489 0.490 0.503 0.511 0.527 0.538
```

| | |
|---|---|
| mean | **0.4684**, 95% CI **[0.4525, 0.4843]** |
| median | 0.4675 |
| sd across batches | 0.0364 |
| batches scoring **below** 0.5 | **16 of 20 (80%)** |

**The mean is resolved below parity.** Five generations of this loop's training, measured against
the exact net they started from, on the same openings, make it **worse**.

## What this settles

Three files have treated the plateau as a measurement failure:

* `acceptance_floor_RESULT.md` — the gate demands an edge 2.7× larger than a generation produces.
* `batch_gate_saturated_RESULT.md` — the batch gate decides on a metric that saturates at 0.96 and
  has reversed signs.
* Both proposed making the filter better able to *see* a real step.

Both diagnoses are correct about the instruments and neither reaches the cause. **There is no real
step to see.** The gate is not blind to improvement; it is correctly reporting that the average batch
is a regression. A filter cannot be tuned into finding signal that is not in its input.

This is the same conclusion `gate_power_RESULT.md` reached from the other end — *"the generator, not
the gate, is what has no gradient"* — now measured directly rather than inferred, on an instrument
that is unsaturated, paired, and centred at 0.5.

## Why this reading is trustworthy where the earlier ones were not

Every previous attempt at this number was compromised in a way this one is not:

* **Not saturated.** The control arm's 20 decisions all sat at 0.948–0.981 against the frozen
  origin, inside the band where that metric has reversed sign. These sit at 0.41–0.54, the middle of
  the scale.
* **One measurement, not a difference of two.** The origin-based gate subtracted two independent
  scores, giving a combined ci95 of ~0.018 on increments of ±0.02. Here each reading is a single
  match with ci95 ~0.035.
* **Paired by construction.** Both sides play the same openings, so opening luck cancels — the thing
  `EXISTENCE_PAIRED_BATCH` was added to patch, and which had moved one net 0.029 on its own.
* **Prospectively collected.** These are the gate's own live decisions, not a post-hoc selection.

And it agrees with an independent estimate assembled from three other files:
`reject_holdout_RESULT` (rejected candidates 0.4907), `accept_audit_RESULT` (accepted 0.5179) and
`accept_rate_vs_noise_RESULT` (8.87% accept rate) put the mean adopted candidate at **0.4931** per
generation. Compounded over five generations that predicts a batch below 0.5, which is what 20
direct measurements show.

## The consequence for the three filters

| filter | what it does | outcome |
|---|---|---|
| K = ∞ | adopts every batch | net drifts down ~95 Elo by gen 100, recovers to 0.557 by gen 4,327 |
| K = 5 origin | rejects on a saturated scale | 1 KEEP in 20, ends **0.411 ± 0.035** |
| K = 5 direct | rejects on a correct scale | **1 KEEP in 20** — rejecting correctly |

The two gated arms keep the same number of batches for opposite reasons: the origin arm because it
cannot see, the direct arm because it sees accurately and there is nothing worth keeping. **That
symmetry is the finding.** Fixing the instrument did not change the outcome, which is exactly what
should happen when the instrument was not the binding constraint.

## What this does NOT say

* **Not that training is useless.** The ungated arm reaches **0.557 ± 0.032 against its own start by
  generation 4,327** — the loop does climb, over thousands of generations, by a biased random walk
  whose individual steps are net-negative. Slow and real is not the same as absent.
* **Not that the gate should be removed.** A filter that correctly rejects net-negative batches is
  working. The reason to run ungated is that the walk eventually climbs, not that the filter is wrong.
* **Not a verdict on `--games 8`.** Every batch here is five generations of *this* configuration.
  Whether a generation that produces more data, or trains differently, has a positive expectation is
  the question this reframes — and it is untested.

## What it makes next

The lever is the **generator**, not the filter. Concretely: what makes a single generation have
positive expected value? `datagen_depth_RESULT.md` already showed label depth is such a lever (1 → 3
was +128 ± 72 Elo), and `depth5_vs_depth3_RESULT.md` showed it is spent at 3. The remaining knobs on
the generator side — games per generation, epochs, blend, replay window — have not been measured
against this instrument, and this instrument is the one that can now see a five-generation effect
directly.


## ⚠ SCOPE LIMIT I did not state when publishing this — all 20 samples are POST-RESUME

The batch gate rolls back on reject, so the **base only moves on a KEEP**, and there was exactly one
KEEP, at g10. Decisions 1–2 therefore measured from the start net and decisions 3–20 from the g10
champion. **Every one of the 20 samples is "five generations from a net at most ten generations past
the champion."**

That is precisely the regime `resume_dip_RESULT.md` shows is anomalous: a resumed run loses ~95 Elo
by generation 100 before recovering to **+39.8 by generation 4,327**. So **0.4684 may simply be the
resume transient, measured per batch**, rather than a property of the loop in general — in which case
this file and `resume_dip_RESULT.md` are one finding seen from two directions, and "the generator is
net-negative" is overstated.

It also resolves a tension this file left open. If every five-generation step really had expectation
0.4684, a 4,327-generation run could not arrive at **0.557 against its own start** — it would be far
worse, not better. Restricting the claim to the post-resume regime removes the contradiction; leaving
it general does not.

**The discriminator is running and costs no training.** `prod2` is already training ungated, adopting
every candidate, so two snapshots of its output five generations apart *are* base and base+5 in the
steady state. `steady_state_batches.sh` samples ten such pairs from the live production run at
generation ~2,200 and matches them head-to-head. Pre-registered: a mean near 0.4684 makes the finding
general; a mean at or above 0.5 narrows it to the post-resume regime.

Until that lands, **read every number above as applying to the first ten generations after a resume**,
which is the only regime they were drawn from.
