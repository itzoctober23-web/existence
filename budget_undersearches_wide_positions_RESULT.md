# The budget picks a different move in 58% of the WIDEST positions — because it searches them 0.85 plies shallower

**2026-09-12 06:10.** Three seeds, ~6,000 positions each, positions HELD FIXED. This identifies the
channel that `candidate_a_replication_RESULT.md` and `budget_label_channel_RESULT.md` narrowed to
"what the search explores", and it is a dose-response, not a correlation.

## The measurement

`label_delta_budget_vs_depth.rs`, extended to stratify by **branching factor** (legal moves at the
position) and to report the budget's **realised depth**. One trajectory driven by the depth-3
control, so both labellers score byte-identical positions.

Pooled over seeds 20260912 / 777001 / 424242:

```
width      realised d   same move   control      NET   mean |dcp|
<=12             3.50       78.5%     92.7%    -14.2          248
13-20            3.17       86.8%     92.3%     -5.5           28
21-28            2.93       83.1%     96.5%    -13.4           34
29-36            2.43       56.7%     97.3%    -40.6           95
>36              2.15       41.9%     97.2%    -55.3          237
```

**The control searches every bucket at exactly 3.00.** The budget searches the widest positions
**0.85 plies shallower** and the narrowest **0.50 plies deeper**, and realised depth is monotone in
width on **all three seeds** — no seed inverts any adjacent pair.

Where the search goes shallow, move selection collapses: in positions with more than 36 legal moves
the budget picks a **different move than depth-3 in 58% of cases** (41.9% agreement against a 97.2%
control), and its evaluation differs by **237 centipawns** on average.

## Why the control is not optional

Running `budget=0` — the same depth-3 search twice — was the first thing done, and it changes the
reading twice over:

* **Scores are identical.** `|dcp| = 0.0` in every bucket, so depth-3 search is deterministic in
  score. Every centipawn of difference in the treatment is caused by the budget.
* **Move agreement is NOT 100%.** It runs 89–98% depending on width, because `shuffle_children`
  advances the searcher's rng and ties break differently. Without this baseline the treatment's
  disagreement would be read as entirely signal, and the narrow buckets — where the control is
  *worst* — would look far more damaged than they are.

The `NET` column is treatment minus control. It is the only column worth quoting.

## What this explains

`budget_realised_depth_RESULT.md` established that an equal-node budget reallocates effort **toward
narrow positions**, because a wide position costs more per ply and exhausts the budget sooner. That
was a statement about node accounting. This is the consequence: **the reallocation is away from
exactly the positions where the move choice is hardest.**

A position with 42 legal moves needs discrimination among 42 candidates; the budget gives it depth
2.15. A position with 5 legal moves needs almost none; the budget gives it 3.50. The treatment
spends its extra depth where it buys least and takes it from where it buys most.

That is a coherent account of why the budget arm ends **below the net it started from** in both
independent training runs (0.445 and 0.422 vs the shared frozen start) while the fixed-depth arm ends
above it. The arm trains on move choices that are wrong more than half the time in its most complex
positions.

## The asymmetry, which matters for the fix

Disagreement is high at BOTH ends, and it does not mean the same thing at each:

* **Wide (>36), depth 2.15 — shallower than control.** Disagreement here is the budget being
  *worse*: less search, same position.
* **Narrow (<=12), depth 3.50 — deeper than control.** Disagreement here is plausibly the budget
  being *better*: it saw further. Its NET is also the smallest once shuffle noise is removed
  (−14.2 against a −55.3 at the wide end).

So the budget is not uniformly bad. It is making a **trade**, and the trade loses. Any fix should
keep the deeper search on narrow positions and stop the shallow search on wide ones — a **minimum
depth floor** rather than abandoning budgets.

**That experiment is NOT run and must not be read as recommended yet**, because a floor at depth 3
would make the arm spend strictly more compute than the control on wide positions while keeping its
surplus on narrow ones, so it would no longer be compute-matched. `budget_realised_depth_RESULT.md`
records that exact compute-matching as the property the A/B needs. Designing a compute-matched floor
is the open problem, not a detail.

## What this does NOT say

* **Not that 237 cp of label difference is the harm.** `budget_label_channel_RESULT.md` measured the
  label channel in isolation at **0.4960, null**, with 47.8% of labels changed. Labels move and it
  does not matter. What this file adds is that the *move* — which determines the position
  distribution the arm then trains on — moves too, and hardest where positions are most complex.
* **Not a strength measurement.** No games were played here. This is a search diagnostic; the
  strength claims live in the two replication results.
* **Nothing ships**, and no figure here is quoted as Elo.
