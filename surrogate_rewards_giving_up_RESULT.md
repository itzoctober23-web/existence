# The grammar search's fitness surrogate can be improved by SOLVING FEWER POSITIONS

**2026-09-10.** Two evolve arms ran 13h and 7h and accepted **nothing** — 0 accepts across 19 gate
decisions. That reads as "the search is slow". It is not. The arms answered a question, and the
answer is about the fitness function, not the search.

## Every candidate verified at or below 0.500. Every one.

Nineteen independent VERIFY readings (96 pairs each, independent seed) across both arms:

```
0.422  0.430  0.490  0.398  0.435  0.500  0.435  0.500  0.432  0.487
0.432  0.487  0.430  |  0.422  0.430  0.490  0.398  0.435  0.500
```

The **maximum is exactly 0.500**, hit three times. Not one candidate in either arm ever verified
stronger than the program it was mutated from. A search that never once produces a fitter individual
is not exploring slowly; it is being pointed somewhere the games disagree with.

## And the surrogate said those same candidates were 26% BETTER

The gate lines carry both numbers. The MAIN lineage seed and its best-scoring candidate:

| | mates solved | cost | `mates/Mcost` |
|---|---|---|---|
| seed | **23 / 23** | 9,235,450,584 | 0.002490 |
| candidate | **20 / 23** | 6,367,398,918 | **0.003141** |
| change | **−3 mates (−13.0%)** | **−31.1%** | **+26.1%** |

**The surrogate is a ratio, so giving up three mate-in-1s is a 26% IMPROVEMENT as long as it buys a
31% cost saving.** The candidate is rewarded for failing to solve positions. That is not a subtle
mis-weighting; it is the objective working exactly as written, in the direction nobody wants.

This is the degenerate-solution family FITNESS 10 already lists first — "prune everything / return
eval" — reached by a different road. The earlier instance was a transposition table that returned a
constant without ever calling eval (`reference.rs`, `ab_hash`). Same shape: the cheap way to improve
a cost-sensitive score is to stop doing the work.

## Why this was invisible until now

The surrogate and the game gate disagree, and the surrogate is the thing that runs first and cheaply.
`ABOVE:7` in the logs means seven candidates cleared the surrogate filter and were sent to games;
all seven were then rejected at LLR ≈ −3. The pipeline was functioning — the expensive check caught
what the cheap one waved through, every time, which is why nothing bad ever shipped. What was
wasted was the search itself: the mutation operators were being steered by a signal that inverts on
the axis that matters.

## What this does NOT say

**Not that the mutation operators are broken.** They produced programs that are cheaper and still
solve 20 of 23 — that is real work, correctly executed toward the stated objective.

**Not that the gate is too strict.** The gate is the only thing that noticed, and it agreed with the
independent 96-pair VERIFY every time.

**Not that grammar search is a dead end.** It says the current fitness cannot rank candidates, so
any conclusion about reachability drawn from these runs — including GRAMMAR 9's "a path of single
fitter mutations exists" — is untested rather than refuted. **The ladder question is still open, and
these 19 rejections are not evidence against it.**

## The concrete next step — and a wrong fix I nearly wrote down

My first recommendation was "make the floor the seed's own score instead of a constant chosen once".
**Both halves of that were wrong, and reading the code rather than the logs is what caught it.**

The floor is *already* seed-anchored — `guard_floor: f.saturating_sub(guard_tolerance)` at
`evolve.rs:2720`, documented at 2658 as "ABSOLUTE guard floor for this lineage, anchored to its
SEED's score". With `guard_tolerance = 4` (`configs/search_track.conf:110`) the seed's 23 becomes a
floor of 19, which is how a 20-mate candidate qualified.

And the tolerance is **deliberate, with its reasoning recorded** (`evolve.rs:1236`): it exists so
the search can reach **capture extension**, which solves 18/25 and is the only reference program
that scores on the hard set at all. Tightening the floor to the seed's score would make a genuine
discovery target permanently unreachable — a correct measurement frozen into a law that forbids the
thing it was meant to find.

**So the defect is not the floor. It is that the surrogate is a RATIO, which makes mates and cost
substitutable.** Dropping a mate helps twice: it shrinks the numerator a little and the denominator
a lot. The floor's job is to say what may be TESTED; the ratio then also lets a mate loss be BOUGHT.

* **Rank lexicographically — mates descending, then cost ascending — rather than by a ratio.** A
  candidate that solves fewer positions can then still be *tried* (the floor and its tolerance are
  untouched, so capture extension remains reachable) but can never be *preferred* on the strength
  of what it stopped doing.
* **The surrogate needs its precision measured before it steers anything.** These runs give the
  first estimate and it is damning: `ABOVE:7` candidates cleared the surrogate and went to games,
  and **7 of 7 were rejected** at LLR ≈ −3. A filter that is wrong every time it fires is worse
  than no filter, because it spends the search's budget pointing away from the answer.

## Cost of finding out

~20 core-hours of two arms. Recorded here so the next person reads the answer instead of re-running
the arms — and specifically so "0 accepts in 19 gates" is never re-diagnosed as bad luck, a
too-strict gate, or a search that needed longer.
