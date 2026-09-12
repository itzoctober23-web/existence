# PRE-REGISTRATION — a SECOND training seed for Candidate A, written before the arms run

**2026-09-12 03:36.** `candidate_a_budget_loses_RESULT.md` reports the node-budget arm losing at
**0.4383 over 672 pairs and 3 match seeds**, every interval below 0.5. That result is deliberately
stated as a **bound, not a refutation**, for one reason:

> `|mean - 0.5| = 0.0617` is **1.31x** the project's **0.047** between-seed sd — and that 0.047 is
> the spread between **RE-TRAINED** runs, not re-played matches. The three match seeds re-play the
> match on the **same two nets**, which is why their spread is a tiny 0.005. **There is one training
> run per arm.**

This registers the replication that closes that gap, with its decision rule fixed in advance.

## Design

Identical to the original in every respect except the TRAINING seed:

```
arm A2  --init cand_start.net --gens 2000 --games 8 --threads 1 --depth 3 --epochs 3
        --lr 0.0002 --lr-decay 1.0 --blend 0.85 --seed 777777
arm B2  ... the same, plus --datagen-budget 5269
```

* same frozen start net (`cand_start.net`, md5 `9545a35289e9`, verified at launch not assumed)
* same binary `learn_cand` for both arms — the binary whose equivalence to `learn_cand2` was measured
  byte-identical on both the A and B code paths
* matched on GENERATIONS, per the original PREREG; wall-clock matching would confound, because a
  budget changes per-generation cost
* judged by the same gate: `candB2 vs candA2`, 224 pairs, depth 4, 3 match seeds

The original used training seed 20260912. **777777 is chosen before any arm runs** and is not
reused from anything on file.

## Pre-registered decision rule

| outcome | reading |
|---|---|
| B2 vs A2 pooled interval lies **wholly below 0.5** | the effect reproduces across training seeds. The claim upgrades from a BOUND to **established**: a node budget is weaker than fixed depth at matched generations. |
| B2 vs A2 pooled interval **contains 0.5** | the original result does NOT generalise. It stands only as "these two nets differ", and the budget's effect is within training-seed noise. `candidate_a_budget_loses_RESULT.md` must be amended to say so. |
| B2 vs A2 lies wholly **above** 0.5 | the sign is seed-dependent, which would mean the 0.047 between-seed sd dominates the whole comparison and NO conclusion about the budget survives from either run. |

**Predicted, so it can be wrong:** B2 vs A2 lands below 0.5, near the original 0.438. The mechanism
measured tonight is not seed-specific — the budget gives narrow positions more depth and wide ones
less on all 3 seeds (`budget_realised_depth_RESULT.md`), and produces +26.7% rows
(`budget_makes_games_decisive_RESULT.md`) — so if the loss is caused by that reallocation it should
reproduce.

## Why this and not a new candidate

`structural_track_exhausted_STATUS.md` records all three registered structural candidates as answered
or closed. The temptation is to pick a successor. But every successor would rest on the A result, and
that result is currently one training seed per arm. Replication is ~35 minutes per arm on one core
against an unknown cost for a new candidate built on a possibly-spurious foundation.

## Power, stated rather than assumed

Each arm pair yields 672 pairs across 3 match seeds, giving a pooled ci95 of ~0.0375. The effect
under test is 0.0617. So this can resolve the effect it is looking for, but only just — and a NULL
here is "not reproduced at this power", never "no effect".

## Status at registration

Not launched. Waiting on cell C (1728/2000) and the Candidate A gate to free cores on 6-11, so the
arms do not contend with measurements already in flight.
