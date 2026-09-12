# The three arms form a consistent ranking A > B > C — a transitivity check the instrument could have failed

**2026-09-12 04:12.** With all three pairwise comparisons in hand (CvB's second seed still running),
the set can be checked against itself. That is worth doing here specifically, because
`nontransitive_walk_RESULT.md` records non-transitivity in this project — three nets need not order
consistently, so transitivity is a test the instrument can fail, not an assumption.

```
B vs A   0.4383   3 seeds, 672 pairs   -43.1 Elo
C vs A   0.3905   2 seeds, 448 pairs   -77.3 Elo
C vs B   0.4610   1 seed,  224 pairs   -27.2 Elo
```

## The check

```
predicted C-vs-A  =  (C-vs-B) + (B-vs-A)  =  -27.2 + -43.1  =  -70.2 Elo
observed  C-vs-A  =                                            -77.3 Elo
discrepancy                                                      7.1 Elo
```

Against per-comparison intervals of **±26 / ±32 / ±45 Elo**, a 7.1 Elo discrepancy is well inside
the noise. **The three comparisons are transitive within their own error bars**, so `A > B > C` is a
single consistent strength ranking rather than three unrelated readings.

## Why this matters more than it looks

It is an *instrument* check, not a result about the budget. Three nets measured pairwise on the same
netmatch at fixed depth 4 **could** have come back circular — `nontransitive_walk_RESULT.md` is the
file saying so. They did not. That raises confidence in the individual numbers, including the
headline `candidate_a_budget_loses_RESULT.md` figure of 0.4383, because that figure now sits inside a
self-consistent system rather than standing alone.

It is also the first thing tonight that could have invalidated the whole arm set and did not.

## What it does NOT establish

* Not that any arm improved — `candA_fixed vs cand_start` is 0.535 ± 0.030, inside the 0.047
  between-seed band, and the 0.539 that fooled the promoter tonight was the same magnitude.
* Not the *cause* of C being last. `cell_c_interpretation_PREREG.md` fixes that reading in advance:
  C is the only arm whose label and played move come from DIFFERENT searches, so "decoupling the
  label from the move played is harmful" is the permitted reading — **not** "labels are the damaging
  channel".
* Not a ship. Nothing here is shippable and no arm's net is a candidate.

## Status

CvB's second seed is still running; the CvB leg rests on one seed and 224 pairs, which is why its
interval is the widest of the three. The transitivity conclusion survives that: the discrepancy would
have to grow by more than 4x to leave the noise.
