# Deeper labels beat more labels at equal compute — PROMISING, needs replication

`depth_ab.sh`, 2026-09-08. The first ceiling candidate to point UP rather than get refuted.

## The measurement

Both arms from scratch, same seed, nothing gated, **equal wall clock (2400s each)**. The cost ratio
was MEASURED first rather than assumed — 300 games costs 3s at depth 2 and 29s at depth 3, ~9.7× —
so the depth-3 arm simply completes fewer generations. That is the trade a practitioner actually
faces: given a fixed budget, label many positions shallowly or fewer deeply?

| arm | generations in 2400s | vs frozen origin |
|---|---|---|
| datagen depth 2 | **92** | 1403W-595D-2L — 0.850 ± 0.010 |
| datagen depth 3 | **8** | 1502W-494D-4L — **0.875 ± 0.009** |

**Difference +0.025 ± 0.013 — resolved within-run.** Eight deep generations beat ninety-two shallow
ones on the same clock.

## Why this is "promising", not "confirmed"

**The effect is smaller than the between-run band.** `ceiling_ANALYSIS.md` measured run-to-run
variation across 17 runs at roughly **0.07 wide** — nearly three times this 0.025 difference. A
single pair of arms can produce a gap this size by chance, and the within-run ci95 says nothing
about that.

The pre-registration named this outcome before the run: *"unresolved => more seeds; the run-to-run
band is ~0.07, wider than one run's ci95."* The number came in on the confirming side, and the
honest reading is unchanged by which side it landed on. **Two more seeds are the price of calling
this a lever.**

## Why it is still the most interesting result of the ceiling work

Four candidates were named. Three are now resolved and two of those are dead:

| candidate | status |
|---|---|
| capacity / net width | **REFUTED** — w64 loses 0.179 ± 0.021 to w16 at equal time |
| draw filter | **REFUTED** — excluding draws is better by 0.086 ± 0.015, confirmed on two protocols |
| **datagen depth** | **promising** — +0.025 ± 0.013, below the between-run band |
| horizon schedule | running now |

It is also the only candidate whose mechanism was independently predicted: a training target cannot
be better than the search that produced it, and "depth 2" here is a 3-ply tree. That the effect
survives an 11× reduction in generation count is the part worth replicating — it says label quality
buys more than label quantity at this strength, which is the opposite of how the loop is currently
tuned.

## Honest limits

* One seed, one pair of arms.
* The arms differ in generation count by construction (92 vs 8) — that IS the experiment, but it
  means the depth-3 net saw far less data diversity, and a longer run might close or widen the gap.
* Both controls use fixed-depth-2 uncapped play, which is independent of either arm's training
  depth. That part is clean.


---

## CONFOUNDED WITH THE HORIZON — found before the replication ran

The two arms differ in more than datagen depth, and the difference runs in the direction that
flatters the winner.

`horizon = 10 + (g-1)*5`, and equal wall clock forces unequal generation counts:

| arm | generations | horizon reached |
|---|---|---|
| depth 2 | 92 | **465** |
| depth 3 | 8 | **45** |

**A 10× horizon difference.** The horizon hypothesis — the fourth ceiling candidate, currently
under test — says NARROW is better, because far-from-terminal labels are anti-signal in near-random
self-play (`datagen.rs:17-20`, and `main.rs:514` measured sign accuracy 0.452 → 0.441 training on
ALL decided positions against 0.543 at ≤10 plies). **The depth-3 arm had the narrow horizon.**

So +0.025 may be the horizon, not the depth, and this experiment cannot tell them apart.

**The confound is STRUCTURAL, not an oversight in one run.** Equal wall clock → unequal generation
counts → unequal horizons, every time. So the replication as originally queued would have
reproduced it faithfully at two more seeds and produced a confident, confounded answer — which is
worse than no answer, because three agreeing runs read as strong evidence.

`depth_replicate.sh` now passes `--horizon-cap 45` to BOTH arms, pinning the horizon at what the
depth-3 arm reached naturally, so datagen depth is the only remaining difference. **The original
seed-20260907 result is not comparable to the capped ones and is excluded from the count**, which
means the replication answers 2 pairs rather than 3.

This is the same class as the equal-time/equal-depth error in the width experiment and the
equal-cost/equal-budget error in the game gate: two things varying at once, with the uncontrolled
one pointing the way I wanted.
