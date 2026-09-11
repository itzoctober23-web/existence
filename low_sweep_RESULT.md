# Below the shipped rate: 0.0002 wins, 0.0005 LOSES to its own start — on a run that was cut short

**2026-09-11.** Three arms from champion #4 (`91c6eb472d04`, itself trained at lr 0.0005), seed
20260915, control arm at the **shipped** rate. Verdict by `netmatch` against the shared start.

| lr | vs shared start | interval | generations |
|---|---|---|---|
| **0.0005 (SHIPPED)** | **0.422 ± 0.028** | [0.394, 0.450] | 666 |
| **0.0002** | **0.551 ± 0.028** | [0.523, 0.579] | 645 |
| 0.0001 | 0.516 ± 0.031 | [0.485, 0.547] | 652 |

## Read this caveat before the numbers

**The run was cut short at ~650 of 2,000 planned generations, and not for a reason that respects the
experiment.** I SIGSTOPped the arms at 03:52 to protect an asymmetric 4PC benchmark; `timeout`
measures wall-clock and kept counting while they were frozen, so all three were killed at their
5,400s cap having done no work for the final 42 minutes
(`sigstop-under-timeout-is-death` in memory). The arms are therefore **not exactly matched** —
666 / 645 / 652, a 3.2% spread — and the budget is a third of what was pre-registered.

So this is a **partial** result and the pre-registered reading, which was written for 2,000
generations, does not strictly apply. What follows is what the data supports at 650.

## What it supports

**The shipped rate lost to its own start.** 0.0005 scored 0.422, interval entirely below 0.5 — over
650 generations it made the champion *worse*. That is the control arm, so it is not a claim about a
treatment; it says that continuing at 0.0005 from a net already trained at 0.0005 does not pay.

**0.0002 gained**, interval entirely above 0.5, and its interval is **disjoint** from 0.0005's
([0.523, 0.579] against [0.394, 0.450]). That is the one comparison here strong enough to survive
the truncation: same start, same seed, near-equal generations, non-overlapping intervals.

**0.0001 is not better than 0.0002** (0.516 against 0.551, intervals overlapping heavily). So the
sequence 0.01 → 0.002 → 0.0005 → 0.0002, which had improved at every step, finally stops improving.
**This is the first evidence of a turning point**, and the reason that matters is that every earlier
step down won, which is exactly the shape that cannot locate an optimum.

## What it does NOT support

* **Not a promotion.** No arm is being shipped off a truncated run with unmatched generations.
* **Not "0.0002 is the optimum".** One seed, 650 generations, and `netmatch`'s between-seed sd of
  0.047 is larger than the 0.035 gap between 0.0002 and 0.0001. Those two are indistinguishable here.
* **Not a refutation of 0.0005 as the shipped rate.** 0.0005 was promoted on matched 2,000-generation
  arms from a *different* start and replicated three times. A control arm losing from a net that
  already had that rate is the expected shape of a rate that has done its work — the same pattern
  lr 0.002 showed once the champion had been trained at 0.002.

## The next run, and what it must fix

Re-run at the full 2,000 generations with **no pause**, arms matched on generations, testing
**0.0005 / 0.0002 / 0.0001** again on a fresh seed. If 0.0002 repeats, it ships; if the three
converge, the rate lever is spent and the next ceiling is elsewhere. The box can now carry this
beside 4PC work, because the 4PC job running alongside it is a paired A/B whose contention cancels.
