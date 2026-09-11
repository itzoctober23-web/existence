# The gate's own candidates are anomalously game-neutral — 14.3% decisive against 31-42% for reference programs

2026-09-11. Two reference matches, 24 pairs each, run through `evolve refmatch` — the SAME
`match_progs` the search track's gate calls, so the numbers are commensurable with in-loop results.
Both readings were pre-registered (`refmatch_discrimination_PREREG.md`), including a mechanism and
its falsifier, before either number existed.

## The measurements

```
capture extension vs seed   10W-28D-10L   rate 0.500 +/- 0.093   decisive 20/48 = 41.7% +/- 7.1%
hash reuse vs seed           8W-33D-7L    rate 0.510 +/- 0.020   decisive 15/48 = 31.2% +/- 6.7%
real gate candidates         W=2 D=216 L=34 over 21 matches       decisive 36/252 = 14.3% +/- 2.2%
```

```
capture vs gate    +0.274 +/- 0.074   z = +3.68   SIGNIFICANT
hash    vs gate    +0.170 +/- 0.070   z = +2.41   SIGNIFICANT
capture vs hash    +0.104 +/- 0.098   z = +1.07   not significant
```

## What is established

**The candidates reaching the search track's gate diverge in games far less than either reference
program does.** Two structurally different programs — a transposition table, and a capture extension
built to see tactics a flat search misses — both produce 2-3x the decisive rate of the mutations the
loop actually produces. That holds against both, at z = 2.41 and z = 3.68.

## What is NOT established, and I am not reporting it as a finding

**Whether the decisive rate is GRADED or SATURATING.** capture (41.7%) against hash (31.2%) is
z = 1.07. Tempting to call that saturation — the rate not scaling with how different the programs
are — but the contrast is badly underpowered: detecting a 10-point gap at 80% power needs **335
games per arm and I ran 48, short by 7x.** It is unresolved, not equal.

**Which program is stronger, in either arm.** Both runs print "UNRESOLVED at this pair count" and
they are right. The quantity read here is the DECISIVE RATE — how often the games separate at all —
which is a different statistic from the score and is well enough determined at 48 games to support
the comparisons above. Nothing here is an Elo claim.

## The registered mechanism survives its falsifier

Registered before arm 2 reported: **the mate guard selects for game-neutrality.** `WHY_NOTHING`
establishes the guard is `f >= best_found` with the seed at 25/25, so every candidate reaching the
gate solves the SAME tactical positions as the champion — anything that changed tactical behaviour
was filtered upstream. Tactical equivalence then implies similar play in the moments that decide
weak-engine games, the games draw, and the collapsed variance is what
`gate_arithmetic_RESULT.md` proves makes acceptance impossible at 6 pairs.

The falsifier was stated as: *capture extension breaks tactical preservation by construction, so a
decisive rate at or below the gate's 14.3% means the mechanism is WRONG.* It came in at **41.7%,
z = +3.68 above the gate.** The falsifier did not trigger.

That is survival, not proof. The mechanism predicts a difference and a difference appeared, but a
single confirmed prediction does not exclude other explanations for why gate candidates are
game-similar — selection on the surrogate rate, or the population being dominated by neutral twins,
would both produce it too.

## What follows

If the mechanism holds, the fitness's two halves are in **direct tension**: the mate guard demands
behavioural PRESERVATION and the game gate demands behavioural DIFFERENCE. The search track's zero
promotions in 489 gate decisions would then follow from the fitness's own structure rather than from
any bug — and no amount of extra pairs fixes a candidate pool selected to be indistinguishable.

The next question is whether the mate guard can be a GRADIENT rather than a pass/fail filter, which
is what the HARD set was meant to supply before `hardn_probe` turned out to be reading an inert knob
(`hardn_inert_RESULT.md`). That is now the highest-value open question on this track.

## Caveat carried forward

The 21 gate matches mix MAIN (budget 16) with MCTS (budget 256-1024) while `refmatch` runs at budget
16 throughout, so the gate-vs-reference comparison is a first calibration rather than a controlled
contrast. Matching the budgets would sharpen it and has not been done.

## A natural experiment already in the logs: the SATURATED lineage almost never reaches the gate

`search_track_WHY_NOTHING.md` argues qualitatively that MAIN's mate dimension is saturated — the
seed scores 25/25, the guard is `f >= best_found`, so mates is "a pass/fail filter, never a
gradient". The two lineages run side by side in every arm and differ in exactly that respect,
which the run headers state:

```
lineage MAIN  seed  71 nodes, budget 16    -> 19/19 mates (floor 16)   SATURATED
lineage MCTS  seed 131 nodes, budget 1024  -> 11/19 mates (floor 10)   PARTIAL
```

Counting how often each reaches the game gate at all, over the same 29 generations:

```
MAIN    3 gate calls / 29 generations = 0.10
MCTS   26 gate calls / 29 generations = 0.90
difference +0.79 +/- 0.08   z = +9.92
```

**A nine-fold difference.** The lineage whose fitness dimension has room to improve reaches the gate
in nine generations out of ten; the saturated one reaches it in one. That is the consequence of
saturation, measured, where the existing account had only the mechanism.

### This is an ASSOCIATION, not a controlled test, and the confound is large

MAIN and MCTS differ in far more than saturation: budget 16 against 256-1024, guard floor 16 against
10, and two different search paradigms. Any of those could drive the gap. **Nothing here attributes
the 9x to saturation** — it is consistent with the mechanism and would have embarrassed it had the
saturated lineage gated MORE, which is the useful thing about it.

The controlled version is the HARD-set experiment (`hardn_probe.sh`, now pointed at a binary where
`EXISTENCE_HARD_N` actually works): add an unsaturated dimension to MAIN itself and see whether its
gate rate moves, holding budget, floor and paradigm fixed.

### Robustness of the numbers published above

The gate figures in this file were computed on 21 completed matches; the arms have since produced
29. Recomputed on all of them (348 games): draw rate 0.853 against 0.857, decisive 51/348 = 0.147
against 0.143, and the two comparisons move from z = +3.68 to +3.67 and from +2.41 to +2.39. The
conclusions are unchanged on a 38% larger sample.
