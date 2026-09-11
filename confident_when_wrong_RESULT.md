# The engine is MORE confident where its cheap search is wrong — FITNESS §8, implemented and failing

**2026-09-11 02:44.** The second specified-but-never-implemented check. It runs, it is consistent
across three nets, and the direction is the bad one.

> **STATUS, updated 03:09:** the confound named below was the reason this file first withheld its
> conclusion. It has since been tested and **refuted** — see the second half. The finding stands and
> is stronger: on flips that genuinely cost material the champion scores **17.1%** against a spec
> threshold of 80%. Read the caveat below as the reasoning that earned the follow-up, not as an open
> question.

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

---

# The confound was tested and REFUTED — the miscalibration is real

**Added 03:09, the same session.** The section above withheld the conclusion pending a
discriminator. It has been run, and it refutes the caveat rather than the finding.

For every flip: play the cheap move, let the opponent search at rich depth, negate. That is what the
rich search thinks the cheap move was worth, so `rich_score − cheap_value` is what the flip actually
**cost**. Flips costing ≥10cp are real errors; the rest are tie-breaks between moves the engine
rates as equal.

| net | flips | low-conf, ALL flips | real errors (≥10cp) | **low-conf on REAL ERRORS** |
|---|---|---|---|---|
| p1_champion (new) | 46 | 23.9% | **35 of 46 (76%)** | **17.1%** |
| p1_champion_prev_pre_lr0005 | 51 | 33.3% | **42 of 51 (82%)** | 33.3% |

**Both halves of the confound fail.**

1. **Tie-breaks do not dominate.** The premise was that flips would be mostly coin-flips between
   equal moves. They are not: **76% and 82% of flips cost at least a tenth of a pawn.** The
   tie-break population is the minority.
2. **Removing them makes the effect STRONGER, not weaker.** If tie-breaks were producing the
   anti-correlation, filtering to real errors should pull the number toward the 50% base rate. For
   the champion it moves the other way — 23.9% → **17.1%**, further from chance.

**So the conclusion the earlier section declined to draw now stands:** on the positions where this
engine's cheap search makes a move error that genuinely costs material, it is **more** confident
than on an average position, not less. FITNESS.md:271 calls that "a regression in the property the
project sells", and by its own §8 threshold of 80% the engine fails at 17%.

## What this is and is not

**Is:** a measured calibration failure, with the obvious alternative explanation tested and
eliminated rather than argued away.

**Is not:** a strength claim. Nothing here says the engine would play better if calibrated — the
explanation layer is a stated product goal in its own right, not a proxy for Elo. Note also that the
**new, stronger champion is the worse-calibrated of the two** (17.1% against 33.3%), on 35 and 42
events respectively, which is a small sample and is recorded as an observation rather than a trend.

**The method point worth keeping:** I named the confound *before* seeing the discriminator's result
and wrote it into the file as a reason not to conclude. That is what made the follow-up worth
running and its answer worth trusting — the prediction was on record, and it was wrong in a way that
strengthened the finding rather than rescuing it.

## And §8 is not wired into the gate — which is, for now, the only reason the loop still runs

FITNESS.md:273 says of this check:

> **Fail = REJECT even if the ladder passed.** Recorded with the entry.

That makes it a **gate**, not a diagnostic. It is implemented nowhere: `gate.rs` and `main.rs`
contain no adversarial set, no explanation-layer confidence, and no §8 rejection path. The gate
decides on `pent_rate − ci95 ≥ 0.5` and nothing else. This is the third specified-but-absent check
found tonight, after the P1 residual and this measurement itself.

**Do not wire it as written.** The engine scores **17.1%** against §8's 80% threshold, so a literal
implementation would reject *every* candidate, including the one promoted an hour ago on a clean
0.601. The spec would halt the project.

So this is a genuine spec-versus-reality conflict and it is recorded as one rather than resolved by
quietly picking a side:

* if 80% is right, the engine has a large outstanding defect and no candidate should be shipping;
* if shipping is right, the 80% threshold was chosen before anything had measured what this quantity
  actually does, and needs re-deriving from data rather than from intuition.

Nothing here settles which. What it does settle is that the number is now **measurable**, so the
question can be argued from evidence — which it could not be this morning.
