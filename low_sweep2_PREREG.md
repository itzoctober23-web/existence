# Pre-registration: the full-length low-lr sweep tests a claim the truncated run made

**Written 2026-09-11 06:16, at ~80 of 2000 generations per arm — before any verdict exists.**
Git's timestamp on this commit is the evidence it was written first.

## What the truncated run claimed

`low_sweep_RESULT.md`, cut short at ~650 generations when `timeout` killed the frozen arms:

| lr | vs shared start | interval | gens |
|---|---|---|---|
| 0.0005 (shipped) | 0.422 ± 0.028 | [0.394, 0.450] | 666 |
| **0.0002** | **0.551 ± 0.028** | [0.523, 0.579] | 645 |
| 0.0001 | 0.516 ± 0.031 | [0.485, 0.547] | 652 |

Two things were claimed from it, both hedged: **0.0002 beats 0.0005 with disjoint intervals**, and
**0.0001 is not better than 0.0002** — the first turning point in a sequence (0.01 → 0.002 → 0.0005
→ 0.0002) that had won at every previous step.

## What is different this time

* **Full 2,000 generations**, cap raised 5400s → 18000s so it cannot expire mid-run.
* **No pause, ever.** The truncated arms died because they were SIGSTOPped under a `timeout` whose
  clock kept running while they were frozen.
* **A different start**: the current champion `34a5ace752d8` (prod4 at gen 2052), not
  `91c6eb472d04`. So this is an independent test, not a continuation — and the control arm is again
  the **shipped** rate.
* Fresh seed 20260916.

## PRE-REGISTERED READING

* **0.0002 beats 0.0005 again, disjoint** → replicated at full length on a new seed and a new start.
  That is enough to switch production to 0.0002, and it would be the fourth rate improvement in the
  chain.
* **0.0002 ≈ 0.0005** → the truncated result was an artefact of 650 generations, i.e. 0.0002 moves
  faster early and the two converge. The shipped rate stands and the sequence has ended at 0.0005.
* **0.0001 beats 0.0002** → the turning point was not real and the optimum is lower still. Sweep
  again below 0.0001 before settling.
* **All three near 0.5** → the rate lever is SPENT from this champion. That closes the learning-rate
  question and sends the next effort to the target or the architecture, which is where
  `ceiling_ANALYSIS.md` pointed before the rate turned out to be the ceiling.
* **The control arm (0.0005) clears 0.5** → note it explicitly: in the truncated run it *lost* to
  its own start (0.422), and a control that behaves differently at full length means the 650-gen
  readings describe a transient, not a rate.

## What will NOT be claimed

No promotion on a single seed if the margin is under `netmatch`'s between-seed sd of **0.047**. The
0.0002-vs-0.0001 gap in the truncated run was 0.035 — smaller than that noise floor — and those two
remain indistinguishable until something separates them by more.
