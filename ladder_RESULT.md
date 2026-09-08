# GRAMMAR 9 ladder, re-run with the repaired reference programs (2026-09-08)

`ladder_sweep.log` predated today's PN rewrite (0/23 forced mates -> 23/23) and MCTS fixes
(0/23 -> 20/23), so it was re-run. 120 mate-in-1 positions, mates per unit COST (FITNESS 3, which
is denominated in cost rather than evaluations precisely so an eval-free prover cannot dominate).

## The headline: NO RUNG BEATS THE SEED, and this one is budget-independent

At D=2, budget 16 or 256 alike (alpha-beta IGNORES the budget and searches to table D):

| program | mates | mates/Mcost | cost vs seed |
|---|---|---|---|
| bare alpha-beta (main seed) | 120 | 0.026 | 1.00x |
| alpha-beta + hash reuse | 120 | 0.026 | **1.01x** |
| capture extension (rung 6) | 120 | 0.026 | **1.02x** |
| table reduction (rung 7) | 120 | 0.026 | **1.01x** |
| alpha-beta + iterative deepening | 120 | 0.024 | **1.08x** |
| alpha-beta + hash + ID | 120 | 0.024 | **1.09x** |

Every rung finds the SAME 120 mates for MORE cost. GRAMMAR 9 requires "a path of single mutations
from the seed exists where every step is fitter" — on this metric, at this depth, **no such step
exists**. Every declared rung is a regression.

That is not a small bookkeeping point: it predicts exactly what the search track has been doing.
`evolve` has now run ~690 candidates across two runs (288 before the surrogate repair, ~400 after)
and accepted ZERO. A hill climb cannot climb a surface with no upward step from where it stands.

## FITNESS 3 is neutral in UNITS but not in BUDGET

`ladder.rs` passed budget 16 to every program. Alpha-beta ignores it; MCTS reads it as a simulation
count and PN as an iteration count. Measured effect of that one number at D=2:

| budget | MCTS mates | MCTS mates/Mcost | PN mates | PN mates/Mcost |
|---|---|---|---|---|
| 16 | 1 / 120 | 0.001 | 29 / 120 | **0.031** (beats the seed's 0.026) |
| 256 | 66 / 120 | 0.002 | 109 / 120 | 0.006 |

At 16 the ratio crowns PN; at 256 it crowns alpha-beta. Same programs, same positions, opposite
verdict. A program whose budget is tunable can be placed anywhere on that curve, so comparing
paradigms at ONE arbitrary budget picks the winner arbitrarily. The old table reported MCTS finding
**one** mate in 120 and presented it as a paradigm comparison; it was a throttle reading.

The budget is now a parameter (`ladder --budget N`), and any cross-paradigm claim from this table
has to state the budget it was measured at. The rung comparison above is unaffected, because
alpha-beta variants do not read the budget at all.

## What this does NOT establish

* NOT that MCTS and PN are worse paradigms. At EQUAL COST they have not been measured — PN spends
  4.17x the seed at budget 256, so a fair test needs its budget tuned down to ~60 first. That is a
  measurement nobody has made and it should not be guessed from these rows.
* NOT that the rungs are wrongly encoded. They are faithful and they find every mate; they simply
  cost more, which is what a transposition table or a reduction SHOULD do on a set where every
  answer is one ply deep and there is nothing to reuse or prune.
* Which points at the real suspect: a MATE-IN-1 set may be the wrong instrument for ranking search
  refinements. Hash reuse has nothing to reuse at depth 1, and LMR has nothing to reduce. The
  ladder may need positions where the rungs can pay for themselves before it can show an ascent.
