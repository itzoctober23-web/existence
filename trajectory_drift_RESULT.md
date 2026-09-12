# Drift is real but points the WRONG WAY — the better net moved FURTHER from the start, not closer

**2026-09-12 07:20.** `budget_harm_is_emergent_RESULT.md` named three candidate mechanisms for a harm
that only appears inside the loop. This tests the first of them and it does not survive.

## The measurement

`traj_profile.rs`: every net plays its own self-play at **fixed depth 3**, from openings drawn from
one rng stream seeded identically for all arms. The search is therefore identical across arms and any
difference in the positions reached belongs to the **net**. Three nets, three seeds, 60 games each:

```
net               mean width   decisive %   plies/game
cand_start             27.50         62.8        101.8
candA2_fixed           25.60         62.8        100.5
candB2_budget          26.22         57.2        108.2
```

Paired differences, per seed:

```
A2 - start   (width)        [-2.80, -1.23, -1.65]   mean -1.89   3/3 same sign
B2 - start   (width)        [-2.01, -0.31, -1.50]   mean -1.27   3/3 same sign
B2 - A2      (width)        [+0.79, +0.92, +0.15]   mean +0.62   3/3 same sign
B2 - A2      (plies/game)   [+10.0,  +2.9, +10.1]   mean +7.67   3/3 same sign
B2 - A2      (decisive %)   [-11.7,   0.0,  -5.0]   mean -5.57   SIGN VARIES
```

## Drift exists

Both trained nets steer into **narrower** positions than the net they started from, on 3 of 3 seeds.
The share of positions with more than 36 legal moves collapses from **30.9% to ~18%** over 2000
generations. Training visibly changes where self-play goes, which is the precondition a drift
explanation needs.

## But the direction refutes the hypothesis

The drift story in `budget_harm_is_emergent_RESULT.md` was: *the budget's trajectory distribution
moves away from where the evaluation is good, and the net chases it.* That predicts the harmed arm
drifts further.

**It drifts less.** `candA2_fixed` — the arm that finishes **above** its own start net (0.552 and
0.535 across two training seeds) — moved **−1.89** in mean width. `candB2_budget` — the arm that
finishes **below** its start (0.422 and 0.445) — moved only **−1.27**, staying closer to the start's
wider distribution. The gap is small (0.62) but it is the same sign on all three seeds.

So among these three nets, **moving further from the start distribution is what improvement looks
like**, not what harm looks like. Drift-as-cause is not supported, and the naive version is
contradicted.

## The one difference that is large and consistent

```
B2 - A2 plies/game   +10.0, +2.9, +10.1   mean +7.67   3/3 same sign
```

The budget-trained net plays **longer games** than the depth-trained one, at the same search depth.
It is also worth noting what this is NOT: measured through the same fixed depth-3 search, so it is a
property of the net's evaluation, not of the budget's search allocation. The budget arm has learned
something that makes its games drag.

That is suggestive against the third mechanism (decisiveness feedback) rather than for it: the
budget's *search* produces shorter, more decisive games
(`budget_makes_games_decisive_RESULT.md`, `budget_position_ab`: 99.7 vs 114.6 plies), yet the net it
trains produces **longer** ones. Search and net push in opposite directions, so a simple "shorter
games → narrower data → shorter games still" feedback loop is not what is happening.

## What is NOT claimed

* **Decisiveness is not resolved here.** `B2 - A2` varies in sign across seeds (−11.7, 0.0, −5.0) and
  pooled over 180 games the 5.6-point gap is ~1.1 sd. Reporting a direction would be inventing one —
  which is also a second independent instance of decisiveness failing as a signal, consistent with
  `decisiveness_measures_the_search_FINDING.md`.
* **This measures ENDPOINTS, not the path.** It compares where each arm's final net steers, not how
  the distribution moved generation by generation. A drift mechanism that operates transiently and
  then reverts would be invisible here. Settling that needs per-generation snapshots, which the arms
  did not save.
* **Three nets is a small population.** The "further drift = better" relation is an observation across
  three points, not a law. It is enough to contradict the hypothesis as stated, not enough to
  establish its converse.
* **Nothing ships**, and no figure here is quoted as Elo.

## Where this leaves the mechanism

```
1  drift                    CONTRADICTED -- the better arm drifted further, 3/3 seeds
2  variance amplification   UNTESTED, and now the leading candidate
3  decisiveness feedback    WEAKENED -- search and net push in opposite directions
```

Variance amplification is the survivor: the budget's move choice is unstable exactly where positions
are complex (58% disagreement at >36 legal moves,
`budget_undersearches_wide_positions_RESULT.md`), and noise compounds multiplicatively in a loop that
trains on its own output. It is also the hardest of the three to test cheaply, because the natural
measurement — the variance of the arm's own outputs across generations — needs per-generation
snapshots that a future arm pair would have to be instrumented to save.
