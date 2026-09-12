# Cell C is the weakest of the three arms — and the permitted reading is the one fixed in advance: decoupling the label from the played move is harmful

**2026-09-12 04:19.** The cell C gate completed all four matches. Reported against
`cell_c_interpretation_PREREG.md`, whose rule was written with one of four readings in hand.

```
                 seeds                pooled            interval          verdict
C vs A    0.395 / 0.386     0.3905 +/- 0.0452    [0.345, 0.436]    wholly BELOW 0.5 — RESOLVED
C vs B    0.461 / 0.478     0.4695 +/- 0.0462    [0.423, 0.516]    CONTAINS 0.5 — UNRESOLVED
B vs A    .439/.433/.443    0.4383 +/- 0.0375    [0.401, 0.476]    wholly BELOW 0.5 — RESOLVED
```

Two seeds and 448 pairs per cell C comparison — deliberately weaker than the A/B verdict's three
seeds and 672 pairs, because this is supplementary.

## The pre-registered rule, applied

The rule said: *second seed also wholly below 0.5 near 0.395 → report as "decoupling the label from
the move played is harmful", NOT as "labels are the damaging channel".* Both C-vs-A seeds are below
0.5 (0.395 and 0.386), so that reading applies.

**Why the stronger claim stays off the table.** Cell C varies TWO things against arm A, not one:

* In arm **A** the recorded label and the played move come from the SAME depth-3 search.
* In arm **B** they come from the SAME budget search.
* In cell **C** the move comes from the depth-3 search and the label from a DIFFERENT budget search.
  The target describes a search that did not choose the move that was played.

A and B are self-consistent; C is not. Being worse than both is exactly what a label/move mismatch
predicts, and this design cannot separate that from label quality.

## The correction the second seed forced

With one seed, `C vs B` read 0.461 and looked like a clean "C is worse than B too". With both, it
pools to **0.4695 with an interval containing 0.5** — **unresolved**. So:

* **C is worse than A**: resolved.
* **C is worse than B**: NOT established. The ordering `A > B > C` is a best estimate, not a measured
  fact at the B/C boundary.

`cell_c_transitivity_FINDING.md` was written when CvB had one seed and predicted the discrepancy
"would have to grow more than 4x to leave the noise". It grew from 7.1 to **13.1 Elo** against
intervals of ±26 to ±45 — still inside, so that finding stands, but its margin is half what it was.

## What this does not establish

* **Not** that labels are the damaging channel in Candidate A. `budget_label_ab.rs` — fixed corpus,
  no self-play, one column swapped — is the design that could test that, and it is queued to run
  last.
* **Not** that any arm improved. `A vs start` is 0.535 ± 0.030, inside the 0.047 between-seed band.
* **Not** a ship. All three nets are diagnostic artefacts.

## Status

Cell C complete. Three arms, six matches, 1,568 pairs total. Nothing shipped.
