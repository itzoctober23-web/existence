# Width is NOT the ceiling — REFUTED, resolved

`width_ab.sh`, 2026-09-08. Pre-registered in three directions before the run; this is the branch
labelled CAPACITY REFUTED.

## The measurement

Two arms from scratch, identical seed, games/generation, epochs and generation count, **nothing
gated in either**. They differ in `--rung` and nothing else.

| arm | width | weights | net |
|---|---|---|---|
| rung 0 | 16 | 12,528 | 50,134 bytes |
| rung 2 | 64 | 50,112 | 200,470 bytes |

Head to head at **equal TIME** (`arch::equal_time_caps`), 600 pairs / 1200 games:

```
depth cap 4, 8964/12818 nodes (full tree at that cap = 12780)
181W-67D-952L   rate 0.179 +/- 0.021   champion WEAKER
```

**Width 64 loses crushingly to width 16.** Resolved far below 0.5 — this is not an unresolved
null, the interval is nowhere near 0.5.

The node caps are the mechanism and they are working as designed: the wider net gets **8964**
nodes to the narrow net's **12818**, because it costs more per node and equal TIME charges it for
that. At fixed DEPTH the comparison would have flattered width by handing it the extra computation
free, which is why the fixed-depth measurement is skipped across widths.

## What this closes

`ceiling_ANALYSIS.md` nominated capacity as the top-ranked explanation for the 0.79–0.86 band that
44 readings across 17 runs all sit in, on the strength of the sibling GPU-RL project breaking a
flat-across-eight-anchors plateau by going from 302k to 3.12M parameters. **That reasoning does not
transfer here, and the transfer was the error** — the same class as the three corrections logged
today, where a result was carried across depths, sets or windows without saying so.

Width is closed as a cheap lever. The ceiling is in the DATA, and the remaining candidates are:

* **depth-2 labels.** The target cannot be better than the search that produced it, and "depth 2"
  is a 3-ply tree here.
* **the horizon schedule**, `10 + (g-1)*5`, reaching 755 plies by generation 150 when
  `datagen.rs:17-20` warns the outcome is nearly independent of a position 40 plies earlier.
* **the draw filter**, which discards ~64% of every batch. `draws_ab.sh` is running.

## The confound, TESTED — and it is substantially weakened

The obvious objection to the above is "width 64 has 4x the parameters on identical data, so it is
simply undertrained at 20 generations". That is checkable from the loss trajectories already on
disk, and it does not hold:

| arm | mean loss, first 5 gens | mid 5 | last 5 | late slope (mid->last) |
|---|---|---|---|---|
| w16 | 0.0273 | 0.0353 | 0.0397 | +0.0044 — FLAT |
| w64 | 0.0276 | 0.0217 | **0.0262** | +0.0045 — FLAT |

**Neither arm was still improving.** Both late slopes are slightly POSITIVE, so neither is a net
still descending that was cut off early. Undertraining predicts a steeply falling loss at the
cutoff, and that is not what either arm shows.

**And the wider net FITS BETTER while PLAYING WORSE.** w64's final loss is 0.0262 against w16's
0.0397 -- as 4x the parameters should manage -- and it still loses at 0.179 ± 0.021 per unit of
clock. That is not a capacity shortfall. It is lower training loss failing to convert into
strength, plus the speed penalty equal-time correctly charges it.

That decoupling is worth more than the width result itself: **loss on this target is not a proxy
for playing strength here.** It is the same lesson as the draws A/B, where the include arm's much
lower loss (0.0191 vs 0.0726) came with measurably WORSE play -- twice in one day, from two
unrelated experiments.

CAVEAT ON THE LOSS COMPARISON: each arm generates its own self-play data with its own net, so the
two losses are computed on different distributions and are not strictly commensurable. The
FLATNESS of each trajectory is a within-arm fact and is unaffected; the cross-arm level comparison
is the weaker of the two claims and is flagged as such.

Both arms' loss also RISES across the run (w16: 0.0273 -> 0.0397). That is the horizon schedule
widening and making the task genuinely harder, which is the next candidate under test.

## The original confound, stated rather than buried

The arms are matched on GENERATIONS, not on training-to-convergence. Width 64 has **4x the
parameters and received exactly the same data**, so "undertrained at 20 generations" is a live
alternative to "architecturally worse per unit of clock". This experiment refutes *widening as a
cheap win* — swap the flag, get strength — which is precisely how it was proposed. It does **not**
prove capacity is irrelevant at convergence.

Testing that properly means training both to convergence, which at this cost is many hours per
arm, and it is not worth it while the ceiling has three cheaper unexplained candidates.

## Harness note

The first run of this produced its answer and threw it away: the script piped the head-to-head
through `tail -12`, which kept the trailing diagnostic block and cut off the rate line. Fixed to
write `width_headtohead.log` in full.

That surviving diagnostic also reads as a verdict on the wrong thing if taken at face value — it
warns "the control gate is BLIND, and 0.500 from it is NO EVIDENCE", but it probes **depth 6**,
while the gate runs at cap depth 4 where the budget (8964) exceeds the full tree (12780 is the
uncapped figure; the cap is what binds). The warning is real for its own configuration and
misleading printed unconditionally next to another.
