# All three registered structural candidates are now answered — and they point the same way

**2026-09-12 03:34.** `structural_next_PREREG.md` put three structural candidates on the table and
ranked them. As of tonight all three are resolved or closed, so the registered track is exhausted.
Recording the state, and what the three answers jointly imply, without launching anything.

| candidate | status |
|---|---|
| **A** — node-budget datagen | **ANSWERED, negative.** `candidate_a_budget_loses_RESULT.md`: 0.4383 over 672 pairs and 3 match seeds, every interval below 0.5. Reported as a bound (one training seed per arm, 1.31x the 0.047 between-seed sd). |
| **B** — rolling data window | **MEASURED, null, underpowered.** `replay_window_plateau_FINDING.md`: a plateau-regime sweep ran 2026-09-08 — windows 1/8/999 at 0.855/0.852/0.863, spanning 0.011 against CIs of ±0.038-0.046 — and was never written up. The PREREG's "genuinely unmeasured" was wrong. |
| **C** — ARCH width | **CLOSED** before tonight. `width_clock_RESULT.md`: 11 attempts, 0 accepts on the clock. Reopens only on a measured ns/eval drop. |

Against MASTER_PLAN's P1 milestone of **~2000 vs SF-limited**, the pooled ruler reads **1561**
(`prodk0127_plateau_STATUS.md`, 20 readings, trend −2.89 Elo/1000 gens, t=−1.74). P1 is 439 short and
flat on a fourth independent lineage.

## What the three answers jointly imply

The A result is the informative one, because its mechanism was measured rather than inferred:

* a node budget gives **more** depth to NARROW positions and **less** to WIDE ones — branching against
  realised depth is monotone decreasing on all 3 seeds (~38 legal moves at depth 2 down to ~3-4 at
  depth 5)
* it produces **more** usable data, not less: +14.5 points decisive, +26.7% trained rows
* **and it still loses by 0.0617 of score**

So the arm that starved wide positions lost while holding every other advantage. Equalising NODES is
worse than equalising DEPTH, and the natural reading is that **complex positions are where label
quality actually binds** — they need more search, not less, and the current fixed depth already
under-serves them.

That is a directional hypothesis with a measured mechanism behind it, and it is the inverse of what
the PREREG registered. It is written here and NOT acted on: the project's standing practice is to
pre-register a design, its decision rule and its power before spending compute, and
`candidate_a_budget_loses_RESULT.md` is explicitly a bound rather than a refutation — one training
seed per arm. A second training seed on the existing A/B arms is cheaper than a new candidate and
would firm up the foundation any successor rests on.

## Status

Nothing launched. This is a state record plus the direction the evidence points, so that whoever
picks the next candidate is not choosing in the dark or re-deriving a closed one.
