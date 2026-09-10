# REFUTED: the depth-1 gate is NOT discarding depth-4 improvements. The deeper-gate lead is closed.

**2026-09-10.** The prediction in `reject_holdout_PREREG.md` was committed before the data existed.
It failed, on both of the conditions it named. This file records that, and closes the lead.

## The prediction and the outcome

> **Predicted:** these 21 — all from generations later than any tested — score **above 0.5**, with
> the candidate-level 95% CI **excluding 0.5**.
> **Refuted if:** the CI contains 0.5, or the mean falls below it.

| | result |
|---|---|
| candidates | 21, none seen by the discovery set (overlap verified = 0) |
| games | 1344 (286W-747D-311L) |
| candidate-level mean | **0.4907** |
| 95% CI | **[0.4755, 0.5059]** |

**Both refutation conditions fired**: the interval contains 0.5, and the mean is below it.

## The trend did not merely fail to replicate — it reversed

| set | n | mean | trend of score vs generation |
|---|---|---|---|
| discovery (generated the hypothesis) | 13 | 0.5102 | **r = +0.581**, t(11) = +2.37 |
| holdout (tests it) | 21 | 0.4908 | **r = −0.258**, t(19) = −1.16 |

A correlation that flips sign on fresh data is noise, not a weak effect. The discovery set's
early/late split — early (n=3) 0.4507 with CI excluding 0.5, late (n=9) 0.5302 with CI excluding
0.5 — looked like two resolved groups pointing opposite ways. It was one post-hoc boundary drawn
through 13 scattered points.

## What is now established, pooling both sets

All **34** candidates the gate rejected, each played against the champion it actually lost to, at
depth 4:

**mean 0.4982, sd 0.0401, 95% CI [0.4847, 0.5117]** — a tight interval centred on 0.5.

So a rejected candidate is, on average, **exactly as strong as the champion that beat it at the
gate**. Two consequences, and they point in opposite directions:

1. **The cheap gate is not costing strength.** It is not discarding improvements. The worry that
   motivated all of this — the gate sees +0.043 where depth 4 sees +0.139, so it must be throwing
   away real gains — is **measured false**. **The deeper-tiebreak lead recorded in
   `pooled_runs_RESULT.md` is CLOSED.** A depth-4 gate costs ~200x (≈7 min/generation against ~2s)
   and would buy nothing on this evidence. That saving is the value of this result.
2. **But the gate's "no" is uninformative, not correct.** Rejects are not weaker; they are
   indistinguishable. The gate is not identifying bad candidates, it is declining to switch when it
   cannot tell — which is the right default, and also means rejection carries no signal about the
   judged game.

## What this does NOT settle

Only REJECTED candidates were measured. Nothing here says whether the gate's ACCEPTS are the best
available, or whether a candidate better than the champion would be recognised — the accept path
was never sampled. That is a different experiment and would need accepted candidates saved too.

## The method note, which is the reusable part

The discovery-set result was tempting: two subgroups whose intervals both excluded 0.5, a
significant correlation, and a mechanism that made sense. Writing the prediction down first — with
an explicit "no re-slicing at a different boundary, no needs-more-candidates" — is the only reason
this is a one-hour negative result instead of a paragraph in the docs asserting that the gate
throws away improvements.

**Cost:** ~2 hours of E-core time across both runs. **Bought:** a closed lead, a vindicated cheap
gate, and one fewer false claim.
