# Rung 6 was dead code. Now it plays differently — whether it plays better is UNRESOLVED.

2026-09-08. Found by pulling one thread: "capture extension plays identically to the seed and costs
1.6% more", which is impossible for an extension that re-searches captures.

## Three defects, chained, each hidden by the one before it

1. **`pred` (GRAMMAR primitive #5) was a stub** — `Node::Pred(..) => Value::Bool(false)`. Every
   predicate always false, so capture extension's condition never fired.
2. **The rung contradicted its own spec.** GRAMMAR 9 says "capture extension **at horizon**"; the
   reference applied `if is_capture: nd = d` inside the move loop at EVERY depth, making captures
   free throughout the tree. Invisible while the predicate was dead.
3. **`tread` (primitive #26) discarded its index arguments**, which kills rung 7 independently.

The tell was an eval count identical to the digit: **4,127,466 for both the seed and capture
extension**. Not "rarely fires" — never fires.

## After fixing 1 and 2

10 positions at depth 3:

| program | evals | cost | agrees with seed |
|---|---|---|---|
| bare alpha-beta | 1,413,909 | 1.000x | 10/10 |
| alpha-beta + hash reuse | 1,368,508 | 0.980x | 10/10 |
| **capture extension** | **1,971,912** | **1.679x** | **8/10 — DIFFERS** |
| table reduction | 1,413,909 | 1.007x | 10/10 (still a no-op) |

**73x → 1.679x**, and the first alpha-beta-family program measured to play different chess from the
seed. Node count 80 → 84 (+13 from the seed), a deliberate change to the declared prior.

## Does it play BETTER? Unresolved, and three instruments say so differently

| instrument | result | why it cannot settle it |
|---|---|---|
| value oracle (`ttvalue`) | 5/20 disagreements: 2 better, 2 worse, 1 equal | the oracle searches **depth−1** — SHALLOWER than the program it judges. An extension exists to see what a flat search misses, so a flatter judge is blind to its purpose |
| game match (`refmatch`) | **3W-9D-4L, rate 0.469 ± 0.172, forfeits 0** | 8 pairs resolves ±0.164. Nothing smaller than a rout is visible |
| — | — | and a fixed-DEPTH match does not charge the extension for its 1.679x cost, so a win would be per NODE, not per unit of time |

**Zero forfeits is the load-bearing detail** in the match. A capped program that runs out of budget
returns MOVE_NONE and forfeits, and forfeits fall systematically on the EXPENSIVE side — capture
extension costs 1.679x. A tight cap would have handed the seed a win regardless of play, measuring
cost while looking like it measured strength. The harness now counts forfeits and voids the verdict
if any occur; zero here means this measured play.

## What it would take to settle it

Pair-count arithmetic from the measured pentanomial sd of 0.2362:

```
  8 pairs -> ±0.164      100 pairs -> ±0.046
 24 pairs -> ±0.094      400 pairs -> ±0.023
 to resolve a 0.05 edge: ~86 pairs
```

The first attempt at 24 pairs never finished — a 1.679x program playing ~100 moves a game is
expensive. So a real answer costs roughly 86+ pairs, and it answers only the per-NODE question. The
per-unit-time question needs an equal-cost match, which cannot use a forfeit-on-overrun ceiling for
the reason above.

## Status

* Rung 6: **fixed and alive.** Plays differently, cost known, strength open.
* Rung 7: mechanism fixed (`tread` now indexed with clamped out-of-range features), but the table
  CONTENTS are an undeclared Given. Filling them with "reduce later moves more" is late move
  reduction, which MASTER_PLAN requires to be discovered rather than supplied — so the rung is not
  measurable today and that is a gap in the Given column, not in the code.
