# Decisiveness measures the SEARCH, not the net — the budget's +14.5 points is there at generation 1

**2026-09-12 05:55.** Measured from four arm logs already on disk (64,000 self-play games). **No new
compute.** It refutes a hypothesis I formed an hour earlier and sharpens `decisiveness_RESULT.md`.

## The hypothesis it kills

`candidate_a_replication_RESULT.md` established that the node-budget arm finishes **below the net it
started from** in both independent training runs (0.445 and 0.422 vs the shared frozen start).
`decisiveness_RESULT.md` established that self-play decisiveness **falls as a champion improves**
(0.3235 → 0.2810 within one run, resolved).

Those two suggested an attractive mechanism: the budget arm degrades, a weaker net produces more
decisive games, the more-decisive data trains a weaker net still — a self-reinforcing spiral, with
decisiveness as the symptom that reveals it.

**It is wrong.** If degradation caused the decisiveness, the gap would have to grow as the arm
degrades. It does not grow, and it does not wait.

## The measurement

Decisive fraction by quartile of each arm's own 2000-generation run (500 generations = 4,000 games
per cell):

```
arm                      Q1               Q2               Q3               Q4
A  fixed (orig)     0.5810+/-0.015   0.5258+/-0.015   0.5500+/-0.015   0.5597+/-0.015
A2 fixed (repl)     0.5730+/-0.015   0.5540+/-0.015   0.5415+/-0.015   0.5500+/-0.015
B  budget (orig)    0.7010+/-0.014   0.6947+/-0.014   0.6973+/-0.014   0.7043+/-0.014
B2 budget (repl)    0.6953+/-0.014   0.7017+/-0.014   0.6857+/-0.014   0.6950+/-0.014
```

The budget-minus-fixed gap, by quartile:

```
original      +0.1200  +0.1690  +0.1472  +0.1445    mean +0.1452
replication   +0.1223  +0.1477  +0.1442  +0.1450    mean +0.1398
```

**Flat.** No trend across 2000 generations, and reproducible to 0.005 across two independent
training seeds.

And the decisive part — the first 50 generations only, when both arms have barely moved off the same
frozen start net:

```
              fixed    budget      gap
original     0.5925    0.7275    +0.1350    (400 games per arm)
replication  0.5300    0.7250    +0.1950    (400 games per arm)
```

**The gap is already there.** At generation 1–50 both arms are running what is nearly the same net —
both resumed from `cand_start.net`, md5 `9545a35289e9` — and their decisive rates differ by 13.5 and
19.5 points.

## What follows

**Decisiveness is a property of the SEARCH PROCEDURE, not of the net's strength.** The same net,
given a node budget instead of fixed depth, resolves 14.5 points more of its games. Causality runs
treatment → decisiveness, immediately, with no room for a strength change to mediate it.

This is an independent confirmation of `decisiveness_RESULT.md`'s headline, and a sharper one. That
file showed decisiveness cannot be a strength signal because it moves the *wrong way* as a champion
improves. This shows it moves **14.5 points with no strength change at all** — the two arms at
generation 1–50 are the same net. A metric that swings that far while the thing it supposedly
measures is held fixed is not a weak signal; it is measuring something else.

The mechanism `budget_makes_games_decisive_RESULT.md` proposed is consistent and survives: a budget
gives more depth to NARROW positions, which are disproportionately forcing — checks, recaptures,
single-reply lines — so seeing further exactly there converts drifting games into finished ones. That
is a claim about the search, and this is what it looks like from generation 1.

## Consequence for the open channel question

`candidate_a_channel_FINDING.md` named three channels the budget moves at once. With the labels
eliminated (`budget_label_channel_RESULT.md`, null at 0.4960 with 47.8% of labels changed), two were
said to remain: the position distribution and the decisive-game rate.

**Those two are not separable, and should stop being counted as two.** The decisive rate is not an
independent knob — it is what the changed position distribution looks like when you count outcomes.
The budget changes where the search spends its nodes; more decisive games and a different position
mix are the same fact reported two ways. An experiment that "isolates decisiveness from position
distribution" is looking for a seam that is not there.

The honest remaining statement is one channel, not two: **the budget changes what the search
explores, and that is the channel that costs strength.** Narrowing further needs an intervention on
the search's allocation itself, not on a downstream statistic of it.

## What this does NOT say

* **Not that decisiveness is useless.** It is a fine diagnostic of what a search is doing. It is
  unusable as a *strength* signal, which is the use `decisiveness_RESULT.md` was evaluating and
  rejecting.
* **Not that the fourth cell is now pointless.** `budget_makes_games_decisive_RESULT.md` recorded "a
  fourth cell would be needed: the control arm run to the same number of TRAINED ROWS rather than the
  same generations". That remains unrun — but note it was proposed against the possibility that arm B
  might *win* on volume. B **lost** while holding +26.7% more rows, so the volume channel works
  against the observed effect and cannot explain it. Its priority should drop accordingly.
* **Nothing ships**, and no figure here is quoted as Elo gained.
