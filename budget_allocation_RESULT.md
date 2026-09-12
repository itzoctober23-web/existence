# Reallocating the budget toward wide positions makes it WORSE — depth is logarithmic in nodes, and the exchange rate loses

**2026-09-12 06:35.** Three seeds, ~4,000 positions each, positions held fixed, arms compute-matched
to **0.0001%**. This tests the fix that `budget_undersearches_wide_positions_RESULT.md` pointed at,
and it fails — which is the useful outcome, because it rules out the whole class.

## The design, and why it is exactly compute-matched

A minimum depth FLOOR is the obvious repair and cannot be tested cleanly: a floor at the control's
depth 3 makes the arm spend strictly more than the control on wide positions while keeping its
surplus on narrow ones, destroying the compute-matching the A/B depends on.

Proportional allocation has no such problem. Give position `i` a budget of `B · wᵢ / mean(w)`; the
total is `B · Σwᵢ / mean(w) = B · N` — the flat arm's total exactly, with no tuning constant and no
clamp. Budget adherence is exact (every move spends precisely its cap), so the match is provable:

```
TOTAL NODES   flat 20928468   proportional 20928455   skew 0.0001%
```

Four searches per position against a depth-4 reference: the reference, a **self-agreement control**
at the same depth (two identical depth-4 searches — the tie-noise ceiling), fixed depth 3, the flat
budget, and the proportional budget. Every arm gets a fresh searcher seeded from the ply index.

## Result 1 — proportional is WORSE, on every seed

```
OVERALL   prop - flat   [-1.5, -1.2, -1.5]   mean -1.40, sd 0.17   all negative
          prop - depth3 [-2.5, -0.4, -2.4]   mean -1.77            all negative
```

It does what it was designed to do in the wide buckets — at >36 legal moves it lifts realised depth
2.15 → 2.30 and agreement 24.3% → 25.8% on seed 1 — and it loses far more at the narrow end, where
the allocation drops to 835 nodes and depth falls 3.55 → 2.71 for −8.3 points.

**The mechanism is that depth is logarithmic in nodes.** Giving a wide position 45% more nodes
(5269 → 7640) buys **+0.15 plies**, because each additional ply costs a factor of the branching
factor. Taking 84% of a narrow position's nodes (5269 → 835) costs **0.84 plies**, because plies are
cheap there. Any reallocation at fixed total compute pays that exchange rate, and it is against you
in both directions.

**This rules out the class, not just this instance.** Width-proportional was the most natural member,
and the argument against it is structural rather than a bad constant: you cannot buy depth where
depth is expensive by selling it where it is cheap.

## Result 2 — the crossover replicates, and it is the real finding

Flat budget minus fixed depth 3, agreement with the depth-4 reference:

```
width      seed1   seed2   seed3     mean     signs
<=12         1.2     6.7     4.4      4.1    3/3 positive
13-20        2.6     3.5     7.6      4.6    3/3 positive
21-28        6.2     3.0    -1.4      2.6    2/3 positive
29-36       -5.6    -3.4    -4.3     -4.4    0/3 positive
>36         -2.4    -1.4    -3.3     -2.4    0/3 positive
```

**The two narrowest buckets are positive on 6 of 6 seed×bucket cells; the two widest are negative on
6 of 6.** The budget picks better moves than depth 3 below ~20 legal moves and worse moves above ~28.
That is the same crossover `budget_undersearches_wide_positions_RESULT.md` found through realised
depth, now measured against an independent standard.

## Result 3 — and the OVERALL difference does NOT replicate

```
OVERALL   flat - depth3   [-1.1, +0.9, -0.8]   mean -0.33, sd 1.08   SIGN FLIPS
```

**This must not be reported as "the budget picks worse moves".** Pooled over the whole width
distribution the gains and losses roughly cancel, and the sign is not stable across three seeds.

That is a genuinely awkward result and it is worth stating plainly: the budget arm **loses in
training** (it ends below its own start net in both independent runs, 0.445 and 0.422), yet its
move-agreement with a deeper search is indistinguishable from the control's overall. So one of these
holds, and this experiment cannot tell which:

* agreement with a depth-4 reference is not the quality that matters for training, or
* the harm is not in the average move but in **which positions the different moves lead to** —
  a distribution effect that compounds over 2,000 generations and cannot appear in a
  single-position diagnostic.

The second is consistent with everything else on file: `budget_label_channel_RESULT.md` found the
label channel null, and `budget_makes_games_decisive_RESULT.md` found the budget's games resolve 14.5
points more often — a trajectory-level change. A per-position metric is structurally blind to it.

## What the control bought

`ctrl` — two identical depth-4 searches — runs **94.7 / 96.7 / 96.0%** across seeds. Every arm sits
near 40%, so the gap to the ceiling is enormous and is *not* tie noise: one ply of depth genuinely
changes the chosen move about 55% of the time at this net's strength. Without the control that 40%
would look like an indictment of the budget rather than a property of depth-3-vs-depth-4.

## What does not follow

* **No strength claim.** No games were played; this is a search diagnostic. The strength results are
  in the two replication files.
* **Not that budgets are useless.** They are better than fixed depth in narrow positions, on every
  seed. The problem is the positions where they are worse, and reallocation cannot fix it.
* **Nothing ships**, and no figure here is quoted as Elo.

## What is now closed, and what is open

**Closed:** compute-matched reallocation as a repair. The exchange rate is structural.

**Open, and now the only live direction:** whether the harm is a trajectory/distribution effect rather
than a per-move-quality effect. That is a training-time question and cannot be answered by scoring
fixed positions, which is what every diagnostic in this chain has done.
