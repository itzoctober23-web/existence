# PRE-REGISTRATION — does the depth-1 gate reject real improvements? Holdout test

**Written 2026-09-10 BEFORE running the holdout, and committed before the result exists.** The
observation being tested was generated POST-HOC and must not be banked until it survives data it
did not come from.

## What happened, and why it does not yet count

`reject_audit` replayed 13 rejected candidates against the champions they lost to, at depth 4, 32
pairs each. The pre-registered statistic was the POOLED rate, with three declared readings (below
0.5 = gate transfers; at 0.5 = gate uninformative; above 0.5 = gate discards real gains).

**The pre-registered statistic came back UNRESOLVED:** pooled 0.5102 ± 0.0226, candidate-level mean
0.5102 ± 0.0248. Interval contains 0.5. By the rule I wrote down first, that is the result, and it
licenses nothing.

Looking at the per-candidate rows afterwards, they are not scattered around 0.5 — they rise:

| candidates | mean | 95% CI |
|---|---|---|
| early, gen ≤ 80 (n=3) | 0.4507 | [0.4235, 0.4778] — below 0.5 |
| late, gen ≥ 140 (n=9) | 0.5302 | [0.5078, 0.5526] — above 0.5 |

Correlation of generation with depth-4 score: r = +0.581, t(11) = 2.37.

**Three reasons this is not a finding yet:**

1. **The split is post-hoc.** I chose ≤80 / ≥140 after seeing the numbers. Nothing stops that
   boundary from being the one that best separates noise.
2. **The trend is fragile.** With 12 candidates r was +0.853 (t = 5.16). The 13th point alone
   dropped it to +0.581 (t = 2.37). A correlation that moves that far on one observation is not
   something to build on.
3. **The early group is n=3**, drawn from immediately after a trainer restart. The replay pool
   refills to steady state by generation 10 and these are gens 40–80, so refill does not explain
   them — but three candidates is three candidates.

Ruled out already: a draw-rate trend, which would produce drifting rates for a boring reason.
Draws vs generation is r = +0.145, i.e. nothing.

## The test

21 reject pairs from generations **340–760**, banked by the same run, **none of them touched by the
audit above** (overlap verified = 0). Same conditions: 32 pairs each, depth 4, seed 20260907, each
candidate against the champion it actually lost to.

## The prediction, and what refutes it

**Predicted:** if the depth-1 gate really discards candidates that are better at depth 4 once the
champion matures, these 21 — all from generations later than any tested above — score **above 0.5**,
with the candidate-level 95% CI **excluding 0.5**.

**Refuted if:** the candidate-level CI contains 0.5, or the mean falls below it. In that case the
early/late split was a post-hoc slice through noise, the gate is not demonstrably discarding
improvements, and **the whole line of inquiry is dropped** — no re-slicing at a different boundary,
no "needs more candidates". The aggregate already said UNRESOLVED and would stand.

**Not predicted, and it matters:** any specific magnitude. The earlier late-group mean was 0.5302,
but that number was computed on the data that generated the hypothesis, so it is inflated by
selection. Predicting it would be predicting my own overfit.

## Why this is worth an hour of compute

If confirmed, the loop is rejecting real improvements at ~1800 generations/hour, and the gate — not
the training, not the data — becomes the binding constraint on Existence's strength. That is
directly actionable: it would justify the deeper tiebreak that `pooled_runs_RESULT.md` currently
lists as an unjustified lead.

If refuted, one hour spent, a plausible story killed before it entered the docs as fact, and the
cheap depth-1 gate keeps its clean bill of health.
