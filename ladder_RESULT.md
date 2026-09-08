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


---

# CORRECTION (2026-09-08, same day): an ascent step DOES exist, at D=3

**The headline above is wrong.** I wrote "no rung beats the seed" and "every declared rung is a
regression". That was the D=2 table generalised across depths, and this file's own comment explains
why D=2 cannot show the effect: *"iteration 1 of iterative deepening stores entries at depth 1,
iteration 2 probes needing depth >= 2 and rejects every one, so a transposition table has literally
no reuse to find ... Measuring hash reuse there and calling it a LOSS says nothing about hash reuse
-- it says the test was too shallow to contain the effect."*

I ran the depth sweep, had D=3 in front of me, and read the conclusion off the wrong table.

## What D=3 actually says (deterministic; re-run reproduces to the digit)

| program | mates | cost | vs seed |
|---|---|---|---|
| bare alpha-beta (seed) | 120 | 50,859,895,712 | 1.00x |
| **alpha-beta + hash reuse** | **120** | **49,815,960,763** | **0.98x** |
| capture extension (rung 6) | 120 | 51,645,048,800 | 1.02x |
| table reduction (rung 7) | 120 | 51,226,197,996 | 1.01x |
| alpha-beta + hash + ID | 120 | 54,850,556,083 | 1.08x |
| alpha-beta + iterative deepening | 120 | 55,846,044,574 | 1.10x |

**Hash reuse is 2.05% cheaper for the identical 120 mates.** That is a single mutation from the
seed that is strictly fitter on FITNESS 3 — exactly the step GRAMMAR 9 requires to exist. It is not
noise: the ladder is deterministic (fixed positions, fixed net, no sampling) and a re-run returned
the same integers.

## The precise state of the GRAMMAR 9 claim

GRAMMAR 9 needs a PATH where EVERY step is fitter. Measured at D=3:

* seed -> hash reuse: 1.00 -> 0.98 — **FITTER. The path starts.**
* hash reuse -> hash + ID: 0.98 -> 1.08 — worse. The path STOPS after one rung.
* seed -> capture extension (1.02), table reduction (1.01), ID (1.10) — none fitter.

So: **one verified ascent step, and the ladder cannot currently continue past it.** That is a far
more useful statement than "no step exists", and it is the opposite conclusion on the first rung.

## D=4 is cost-cap limited and should not be quoted

The seed spends 233,392,934,154 over 120 positions = 1.94e9 each, against `cost_cap` 2e9. Most runs
hit the ceiling and return a default move, which is why the seed finds only 17/120 there. Direction
agrees (hash reuse 23 mates for 230B vs the seed's 17 for 233B — better on both axes) but the
numbers are a ceiling reading, not a search reading.

## Standing implication for the search track

`evolve` has run ~690 candidates with zero accepts, and it evaluates fitness at **D=2**
(`Interp::new(net, vec![2, 32_000, 8])`). At D=2 the one known ascent step is invisible. So the
search track is climbing at the one depth where the rung that WOULD pay is measured as a loss.
That is now the leading explanation for its zero accepts, and it is testable by moving its fitness
depth to 3.


---

# THE STRUCTURAL RESULT (2026-09-08): the fitter rung is UNREACHABLE by mutation

Correcting my own prediction from an hour ago. I moved the search track to D=3 saying it should
"now be able to find the hash-reuse-shaped step that D=2 hid". It cannot, and the node counts say
so outright.

| program | nodes | vs seed | fitness at D=3 |
|---|---|---|---|
| bare alpha-beta (seed) | 71 | — | 1.00x |
| capture extension (rung 6) | 80 | **+9** | 1.02x LOSS |
| table reduction (rung 7) | 86 | **+15** | 1.01x LOSS |
| alpha-beta + iterative deepening | 100 | **+29** | 1.10x LOSS |
| **alpha-beta + hash reuse** | 175 | **+104** | **0.98x FITTER** |
| alpha-beta + hash + ID | 204 | +133 | 1.08x LOSS |

`mutate_program` applies `1 + rng.below(3)` edits — one to three. So:

* The **only** rung that is fitter than the seed sits **+104 nodes away**. That is on the order of
  a hundred edits, not one to three. It is not in the mutation neighbourhood at any depth.
* The rungs that ARE within plausible reach (+9, +15) are both **losses** at D=3.

**So there is no verified single-mutation ascent step from the seed, at any depth measured.** The
search track's ~690 candidates with zero accepts is not a bug, not the wrong depth, and not the
mutation operators failing — its reachable neighbourhood simply contains no known improvement.

## What this says about GRAMMAR 9

GRAMMAR 9 requires "a path of single mutations from the seed exists where every step is fitter".
The declared ladder does not demonstrate that, and cannot: its rungs are whole programs 9 to 133
nodes from the seed, so the ladder verifies **"these programs are expressible, and one of them is
fitter"** — a different and much weaker claim than "a mutation-reachable ascent path exists".

That distinction had not been drawn. The ladder is a fine expressibility check and a fine ranking
of finished programs; it is not evidence for the search track's premise.

## What would settle it

Either (a) find intermediate programs between the seed and hash reuse where each step is both small
and fitter — i.e. actually construct the path GRAMMAR 9 asserts — or (b) accept that the ascent is
not single-mutation and change the search track to match, e.g. much larger edit counts, or seeding
from a program that already has the TT machinery so the remaining edits are small.

Neither is a code change to make on a hunch. What is established today is that the current
configuration cannot climb, and why.

## Note on the D=3 move

Keeping it. D=2 systematically misprices anything TT-shaped (no reuse exists at depth 2, so a
transposition table can only cost), and mispricing a whole class of programs is worse than the 11x
cost. But it should be recorded honestly that the depth change does NOT fix the search track, and
I said it might.
