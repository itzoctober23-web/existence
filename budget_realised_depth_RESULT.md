# The node-budget gate PASSES — but the budget reallocates effort in the OPPOSITE direction to the one pre-registered

**2026-09-12.** `structural_next_PREREG.md:92-96` registers a check that "runs FIRST and gates the
rest" of Candidate A: under a node budget, *the variance of realised depth across positions must be
> 0*. Until now that quantity did not exist — the budget primitive itself did not exist. Both do now
(`best_move_budget`, `--datagen-budget`), so this is the gate, measured.

## The gate passes

Budget = **5,269** nodes/move, the measured mean cost of a depth-3 move
(`datagen_node_census_RESULT.md`), on the champion, over datagen's own move loop, 40 games/seed.

| seed | depth 2 | depth 3 | depth 4 | depth 5 | depth 6 | mean | variance |
|---|---|---|---|---|---|---|---|
| 20260912 | 31.2% | 57.8% | 10.1% | 0.9% | — | 2.81 | 0.413 |
| 777001 | 35.7% | 47.4% | 14.7% | 2.1% | — | 2.83 | 0.562 |
| 424242 | 37.8% | 52.7% | 8.6% | 0.8% | 0.1% | 2.73 | 0.430 |

Realised depth spans **2 to 6** and variance is **0.41-0.56** on every seed. The mechanism's
precondition holds: a budget does have effort to reallocate. **Candidate A is not measuring nothing.**

Budget adherence is exact. Every move spends **exactly** 5,269 nodes — min = p10 = median = p90 =
max — because the abort fires at `nodes >= node_cap`. Overshoot is zero, so the budget arm and the
fixed-depth-3 control are compute-matched to within rounding, which is the property the A/B needs.

## The direction is backwards from the registered rationale

`structural_next_PREREG.md:81-83` states the mechanism as:

> "iterative deepening on each position until the budget is spent, so **a hard position gets more
> depth and a simple one less**."

That is not what an equal-node budget does, and the data is unambiguous. Branching factor against
realised depth, all three seeds:

| realised depth | seed 20260912 | seed 777001 | seed 424242 |
|---|---|---|---|
| 2 | 37.42 | 39.06 | 38.43 |
| 3 | 21.82 | 20.96 | 21.68 |
| 4 | 9.15 | 12.68 | 9.55 |
| 5 | 2.79 | 3.64 | 4.10 |

**Monotone decreasing on every seed, by more than 10x end to end.** A wide position costs more per
ply, so it exhausts the budget SOONER and gets FEWER plies. The budget gives more depth to *narrow*
positions and less to *wide* ones — the reverse of the sentence above.

This follows necessarily from equal-node accounting; it is not a bug in the implementation. The
implementation does exactly what "equalise effort per position" means. It is the PREREG's gloss on
what that would buy that does not follow.

## What this does and does not do to Candidate A

It does **not** refute it. Narrow positions are disproportionately forcing ones — checks,
recaptures, single-reply lines — and depth is both cheapest and most valuable exactly there, so
spending the saved effort on them may well produce better labels. That is a live hypothesis.

What it does is remove the registered *reason*. The PREREG's argument was "label quality is worst
where positions are hardest, and a budget fixes that by giving hard positions more depth". Measured:
hard (wide) positions get **less** depth under the budget, not more. So if the budget arm wins, it
cannot be attributed to the registered mechanism, and if it loses, the registered mechanism was
never the thing under test. The rationale has to be corrected **before** the 2000-generation arms
run, or the result will be read against a mechanism that is not operating — the
`right_measurement_wrong_conclusion` failure.

One practical consequence for the arm design: a budget set to the depth-3 **mean** does not
reproduce depth 3. Mean realised depth is **2.73-2.83**, and only 47-58% of positions reach depth 3
at all, because the cost distribution is right-skewed (mean 5,269 against median 4,382). The arms
are compute-matched, which is what the design asks for — but "budget 5269" must not be read as
"equivalent to depth 3".

## Files

* `crates/pipeline/examples/datagen_node_census.rs` — `[net] [games] [depth] [seed] [budget]`;
  budget > 0 switches to `best_move_budget` and reports the realised-depth histogram and the
  branching factor against it. In budget mode the node-spend columns are degenerate by construction
  (they equal the budget) and are labelled as such rather than under the PREREG headings.

## Status

Gate cleared, mechanism direction corrected, **no arm launched and nothing shipped.**
