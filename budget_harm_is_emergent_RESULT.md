# Both isolated channels are NULL while the full treatment costs real strength — the harm needs the LOOP

**2026-09-12 07:00.** The position channel, measured the same way the label channel was, closes the
single-pass decomposition of Candidate A. Neither channel reproduces the harm, and the full treatment
does. That is the result.

## The decomposition, complete

```
                     isolated, one training pass, matched rows      live self-play arm, 2000 gens
LABEL channel        0.4960  [0.4776, 0.5144]  NULL   -2.8 Elo
POSITION channel     0.5050  [0.4944, 0.5156]  NULL   +3.5 Elo
FULL node budget                                             0.4383 / 0.4530 vs the control arm
                                                             0.445 / 0.422 vs its own START net
```

The budget arm finishes **below the net it started from** in both independent training runs
(`candidate_a_replication_RESULT.md`, joint P = 0.0059 under the null). Isolate either channel that
could plausibly carry that, hold everything else fixed, and it vanishes.

## The position measurement

`budget_position_ab.rs` is `budget_label_ab.rs` with the roles swapped. There the positions were held
fixed and the label column was swapped; here the **labeller is held fixed at depth 3 in both arms**
and only the played move differs, so the arms walk different trajectories from identical openings.

```
ARM depth : 6874 rows, 32 decisive/60 games, 114.6 plies/game
ARM budget: 5982 rows, 43 decisive/60 games,  99.7 plies/game
positions identical in both arms: 6 of 5982 (0.1%)
truncated to 5982 rows each -- volume is NOT the variable
```

Three things make the null trustworthy:

* **The treatment is live.** Only 0.1% of positions are shared, and the run independently reproduces
  `budget_makes_games_decisive_RESULT.md`: more decisive games (43 vs 32), shorter games (99.7 vs
  114.6 plies) and a 0.870 position ratio against the 0.904 that file measured on 16,000 games.
* **Volume is controlled.** That file measured the budget yielding +26.7% usable rows, so both
  corpora are truncated to the smaller count. A result bought with more data would not be a statement
  about composition.
* **Neither arm is undertrained.** Held-out MSE 0.09684 and 0.10008 against 0.46335 untrained —
  ~4.7× better. This is the trap `label_source_RESULT.md` had to rule out before it could report.

`blend = 1.0` switches the outcome term off, which matters more here than it did for labels: budget
games are ~18 points more decisive, and letting `z` through would have smuggled the decisiveness
channel back in and made this a two-variable experiment.

**The null is not underpowered.** The pooled interval is ±0.0106 against a full effect sitting ~0.05
below parity — it would have resolved an effect of Candidate A's size roughly five times over.

## What this means

Every single-pass explanation is now eliminated:

| channel | verdict | file |
|---|---|---|
| labels | NULL at ±0.018, with 47.8% of labels changed | `budget_label_channel_RESULT.md` |
| position distribution | NULL at ±0.011, with 99.9% of positions changed | this file |
| per-move quality | sign FLIPS across seeds, not resolved | `budget_allocation_RESULT.md` |
| allocation as a repair | structurally impossible — depth is logarithmic in nodes | `budget_allocation_RESULT.md` |

**So the harm is emergent.** It is not a property of the data a budget produces in one pass. It is a
property of what happens when a net trained on that data generates the next generation's data, two
thousand times. Each channel is individually too small to measure in a single pass and the loop
compounds them.

That is consistent with everything else on file and explains the awkward result in
`budget_allocation_RESULT.md`, where the budget's average move quality was indistinguishable from the
control's while its training outcome was clearly worse. A per-position metric is structurally blind
to a compounding effect, and so is a single-pass training run.

## What this closes, and what it does not

**Closed: the single-pass decomposition programme.** Five experiments, four diagnostics and a
replication, have taken it as far as it goes. Building a sixth isolation harness would be re-deriving
a closed question — the failure mode `RESULTS_INDEX.md` exists to prevent.

**Not closed: the mechanism.** "Emergent from the loop" names where to look, not what is happening.
The candidates worth distinguishing, and they are distinguishable only with training runs:

1. **Drift.** The budget's trajectory distribution moves further from the evaluation distribution
   each generation, and the net chases it.
2. **Variance amplification.** The budget's move choice is unstable exactly where positions are
   complex (58% disagreement at >36 legal moves,
   `budget_undersearches_wide_positions_RESULT.md`), and noise compounds multiplicatively in a loop
   that trains on its own output.
3. **Decisiveness feedback.** Shorter, more decisive games yield a narrower position mix, which makes
   the next generation's games shorter still.

**A discriminator exists and is cheap in design if not in compute:** run the budget arm with the
trajectory RESET to the control's distribution every N generations. If drift is the mechanism,
periodic resetting removes the harm; if variance amplification is, it does not. That is one arm pair
and it is the first experiment in this chain that requires the loop rather than avoiding it.

**Nothing here ships.** `bpa_depth.net` and `bpa_budget.net` are diagnostic, trained on 5,982 rows
for 12 epochs, and must never be gated. No figure is quoted as Elo gained.

## The limitation, carried forward

Both nulls are at 60 games and 12 epochs against a 2000-generation treatment. **A per-generation
effect too small to resolve here is exactly what a compounding explanation requires**, so these nulls
do not say the channels are inert — they say each is below the resolution of one pass. That is the
same caveat `budget_label_channel_RESULT.md` recorded, and it now applies to both channels, which is
what makes the compounding reading the natural one rather than a consolation.
