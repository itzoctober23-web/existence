# PRE-REGISTRATION — the w64 cells, written before the games were read

**2026-09-10, written while the two 120-game runs are in flight.** The held-out fit is already
measured; the strength readings are not. Recording the prediction first so the result cannot be
narrated to fit whatever comes back.

## What is already known

| | train loss (SF) | held-out (SF) | held-out (self) |
|---|---|---|---|
| w16 | 0.02918 | 0.04711 | 0.01935 |
| w64 | 0.01643 | 0.04163 | 0.01969 |

Quadrupling width fits the TRAINING set 1.8x better and improves held-out fit by **12% on the SF
label and not at all on the self label** (0.01935 -> 0.01969, marginally worse). The train/holdout
gap widens from 1.6x to 2.5x. In this data regime — 18,188 training positions — the arms are
**data-limited, not capacity-limited**.

## Prediction

If strength tracks held-out eval accuracy, the extra width buys almost nothing:

* `w64 x SF` lands within noise of `w16 x SF` (−241 ± 58), plausibly a few Elo better.
* `w64 x self` lands within noise of `w16 x self` (−290 ± 69), with no reason to move at all.
* Neither reaches the champion's −104, and nothing approaches the 1600 the original decision rule
  named.

## What each outcome means

* **Both w64 cells flat** — as predicted. Capacity is not binding at 18k positions, and the open
  lever is DATA VOLUME (more SF-labelled positions), not width. It also confirms held-out MSE is a
  usable cheap surrogate for this question, which would be the first cheap proxy in this repo to
  survive — `proxies_RESULT.md` records that every previous one failed.
* **A w64 cell jumps substantially** — the prediction is wrong and something more interesting is
  true: strength does NOT track held-out eval accuracy, so MSE is not a valid surrogate here and the
  eval->play link is doing something the loss cannot see. That would also mean the 12%/0% fit
  differences above are not the right thing to be reading.
* **A w64 cell drops** — width costs strength at equal search depth despite equal-or-better eval,
  which would point at the fixed-depth instrument (a wider net buys no extra nodes) or at
  overfitting hurting play in a way held-out MSE on THESE positions does not capture.

## The trap this avoids

`width_RESULT.md` and `width_clock_RESULT.md` both concluded width is not the ceiling, measured under
the loop's own labels. `label_source_RESULT.md` argues those labels are the easy ones (w16 already at
0.019), so widening had no residual error to remove and could not have paid. This pre-registration
exists so that a flat w64 result here is not quietly read as "width confirmed dead" a third time: a
flat result under a 12%-better held-out fit says DATA is short, which is a different claim.
