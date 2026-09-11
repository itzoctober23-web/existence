# The engine is MORE confident where its cheap search is wrong — FITNESS §8, implemented and failing

**2026-09-11 02:44.** The second specified-but-never-implemented check. It runs, it is consistent
across three nets, and the direction is the bad one — **with one confound still untested, named
below.**

## The check

```text
FITNESS.md:269-272
  "the candidate's explanation-layer confidence (PV stability, static-vs-deep residual class) must
   be LOW on at least 80% of positions where the candidate's move differs from its own 32x-cost
   move. A confident wrong answer is a regression in the property the project sells."
```

Confidence = **small** static-vs-deep residual (the net's static opinion already matches what search
finds). The threshold is each net's **own median**, which is self-calibrating across nets whose
output scales span sd 237–420 — and which builds in its own control: **exactly 50% of all positions
sit above the median by construction.** So the spec's 80% is a question about enrichment over a coin
flip.

## The measurement

120 positions, depth 3 against depth 6, seed 20260911.

| net | used | flips | flip rate | **low-conf on flips** | measured cost | verdict |
|---|---|---|---|---|---|---|
| p1_champion | 118 | 51 | 43.2% | **35.3%** | 709.8× | NO SIGNAL |
| p1_champion_prev_pre_lr002 | 118 | 49 | 41.5% | **30.6%** | 643.9× | NO SIGNAL |
| lrs_00005 | 118 | 50 | 42.4% | **32.0%** | 751.3× | NO SIGNAL |

**The spec asks for ≥80%. All three nets sit near 32%, against a 50% base rate.** They do not merely
fail to be *un*confident where they are wrong — they are **enriched for confidence** exactly there.
Three nets from two different lineages, all the same direction.

## The confound, stated before the conclusion is drawn

**A move-flip is not the same thing as an error.** Two kinds of position flip when you add depth:

* a genuine error — the cheap search picked a move that actually loses something;
* a **tie-break** — two moves the engine rates as equal, where the argmax turns over on noise.

Tie-breaks should concentrate in quiet positions. Quiet positions are *also* where search revises the
score least, i.e. where the residual is small and the net looks "confident". **That confound predicts
precisely the anti-correlation measured above**, with no calibration failure required.

So the honest reading today is: **the check as specified fails, and it is not yet established that
the engine is miscalibrated** — a metric can fail because the subject is bad or because the metric
counts the wrong events.

**The discriminator is written and not yet run.** For each flip, play the cheap move, let the
opponent search at rich depth, negate: that is what the rich search thinks the cheap move was worth,
so `rich_score − cheap_value` is what the flip actually **cost**. Flips costing ≥10cp are real
errors; the rest are tie-breaks. The question then becomes the one FITNESS actually cares about — *is
the engine confident on the flips that COST something* — and that is a different number from the one
in the table. It is queued behind the decay A/B rather than run now, because three netmatch verdicts
have the cores.

## "32x-cost" is not reachable this way, and the tool says so

The spec says 32×. The measured cost of +3 plies is **~710×** — about 8.9× per ply, so 32× is
bracketed by +1 (~9×) and +2 (~79×) and **no integer ply count reaches it**.

That number is printed beside every row precisely because the header comment originally asserted
"+3 plies lands near 32x at this branching factor". It was wrong by a factor of 22 and announced
itself on the first clean run. An approximation that is stated is honest; one that is hidden is not.

## Method notes — three harness bugs, all the same mistake

This tool was wrong three times before it measured anything, and every one was *assuming the search
API's semantics instead of reading it*:

1. **Mate filter ate the whole sample.** `best_move_capped` returns `-INF` when the budget expires
   before any root move completes; `|-INF|` trips a mate-score filter. Every position was excluded
   and the table printed "no usable positions" for all three nets — an empty result from a broken
   probe, not an absence. `datagen.rs:189-196` documents this exact behaviour and guards it.
2. **The node cap starved at every setting.** `best_move_capped` searches root moves one at a time
   at full depth and `break`s on abort **before** updating `best_s` (search.rs:310). A usable cap
   must exceed one root subtree yet stay below the whole search — a window that moves with each
   position and cannot be set from the command line. Hence plies, not caps.
3. **The cost ratio read 1.0× for depth 3 vs depth 6.** `best_move_capped` sets `self.nodes = 0`;
   **`best_move` does not.** `s.nodes` is a running total across every position, and two running
   totals divide to ~1.0. Node counts are now per-search deltas, with a gate that exits 5 if the
   median ratio is ≤ 1.0, because three extra plies cannot cost nothing.

Bugs 1 and 3 were caught only because a printed number was **impossible on its face** — an empty
table, and a 1.0× ratio. Bug 2 was caught by finally reading the function. The standing rule that
two wrong hypotheses mean the harness is wrong fired here at three, and the fix each time was the
same: read the code path being measured before naming what it does.
