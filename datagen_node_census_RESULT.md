# Depth-3 datagen costs 5,269 nodes/move, not 10,309 — and effort per position varies 18-25x

**2026-09-12.** Two pre-registrations were blocked on the same unmeasured number, and the only figure
on disk was ~2x too large for the use they intended.

* `structural_next_PREREG.md:100` — node budget set to "the depth-3 arm's **measured mean** nodes/move"
* `p1_compounding_PREREG.md:82` — set to "the **measured median nodes/move** of depth-3 datagen"

Neither existed. The only number available was the cost table at `crates/pipeline/src/main.rs:190`
(`d3 10,309`), whose own comment says it is "midgame (**the expensive case**, so this never
overspends)". A conservative upper bound is not a mean and is not a median.

## Measured

`crates/pipeline/examples/datagen_node_census.rs`, champion net `p1_champion.net`, 40 games/seed,
depth 3, using a faithful copy of `datagen::play_game_ext`'s move loop (same random `open_plies`,
same `ply < 6 && rng % 4 == 0` temperature, same 160-ply cap, same 100-halfmove rule). Node cost is a
property of the POSITION DISTRIBUTION, so sampling anywhere other than datagen's own distribution
would answer a different question.

| seed | positions | mean | median | p90/p10 | CV |
|---|---|---|---|---|---|
| 20260912 | 3972 | 5885.4 | 5020 | 24.54x | 0.790 |
| 777001 | 4198 | 4742.4 | 3868 | 23.45x | 0.821 |
| 424242 | 4133 | 5179.5 | 4257 | 18.26x | 0.787 |

```
MEAN   nodes/move, depth 3 = 5,269   (3 seeds, sd 577, se 333)
MEDIAN nodes/move, depth 3 = 4,382   (3 seeds, sd 586)
```

Full distribution on seed 20260912 (n=3972): min 47, p10 504, p25 1,877, p50 5,020, p75 9,024,
p90 12,368, p99 18,877, max 26,750.

## The table figure is 1.96x the measured mean, and that mattered

Only **18.5%** of depth-3 moves cost more than 10,309 nodes. The table figure sits near the 85th
percentile of the cost distribution, exactly as its "expensive case" comment claims — it is doing its
job correctly for its own purpose (choosing a depth that never overspends a budget).

**But it was the only number on disk, and both pre-registrations call for a mean or a median.** Setting
the budget to 10,309 would have handed the node-budget arm roughly **twice the compute per move** of
the fixed-depth control it is supposed to be matched against. Any win would then be confounded with
compute, and the A/B would have measured the wrong thing — the same class of error as
`resume_transient` invalidating a published A/B by matching on wall clock.

## The pre-registered gating check passes, decisively

`structural_next_PREREG.md:92-96` registers this as running FIRST and gating the rest: the node
budget can only beat fixed depth if effort per position is currently UNEQUAL. If a depth-3 move costs
about the same everywhere, a budget has nothing to reallocate and the candidate measures nothing.

It is not close to equal. **p90/p10 is 18-25x and the coefficient of variation is ~0.80 on all three
seeds** — the most stable quantity in the whole census. A depth-3 move costs anywhere from 47 to
26,750 nodes.

**Stated precisely, because this is not literally the quantity the PREREG names.** The PREREG's check
is the variance of *realised depth under a budget*, which requires the code change and does not exist
yet. What is measured here is the variance of *cost under fixed depth*. The first implies the second
is non-trivial: if cost at fixed depth varies 18-25x, then under a fixed node budget cheap positions
must reach deeper before exhausting it, so realised depth cannot be constant. So this is strong
evidence the gate will pass — not the gate itself.

## Cost by game phase

| ply bucket | n | mean nodes |
|---|---|---|
| 0-9 | 396 | 4,423 |
| 10-29 | 780 | 7,418 |
| 30-59 | 1080 | 6,696 |
| 60-99 | 940 | 4,989 |
| 100-159 | 776 | 5,049 |

Midgame IS the expensive phase, confirming the table comment's direction. Its magnitude is still
above even the midgame mean (10,309 vs 7,418).

## Instrument validity

The census tracks depth monotonically and at the right order of magnitude (table figures in
parentheses): d2 **316** (352), d3 **5,269** (10,309), d4 **35,755** (72,977). The d4/d3 ratio is
**6.8x** measured against **7.1x** in the table — close agreement on the shape of the depth cost
curve, while the absolute d3/d4 figures sit at ~0.5x the table because the table reports the
expensive tail and this reports the mean. d2 is the exception (0.90x), which is expected: it is the
cheapest and least variable case (CV 0.72).

## What this does NOT show

It does not show that a node budget beats fixed depth. That is the 2000-generation A/B the PREREG
registers, and it has not been run. This supplies its budget parameter and clears its gating
precondition. No arm has been launched and nothing is shipped.

## Files

* `crates/pipeline/examples/datagen_node_census.rs` — the probe, seeded (4th arg) because one sample
  is a lottery; the mean moves ±11% between seeds while the CV does not move at all.
