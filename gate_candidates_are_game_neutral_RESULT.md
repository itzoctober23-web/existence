# The gate's own candidates are anomalously game-neutral — 14.7% decisive against 31-42% for reference programs

2026-09-11. Two reference matches, 24 pairs each, run through `evolve refmatch` — the SAME
`match_progs` the search track's gate calls, so the numbers are commensurable with in-loop results.
Both readings were pre-registered (`refmatch_discrimination_PREREG.md`), including a mechanism and
its falsifier, before either number existed.

## The measurements

```
capture extension vs seed   10W-28D-10L   rate 0.500 +/- 0.093   decisive 20/48 = 41.7% +/- 7.1%
hash reuse vs seed           8W-33D-7L    rate 0.510 +/- 0.020   decisive 15/48 = 31.2% +/- 6.7%
real gate candidates         W=4 D=297 L=47 over 29 matches       decisive 51/348 = 14.7% +/- 1.9%
```

```
capture vs gate    +0.270 +/- 0.074   z = +3.67   SIGNIFICANT
hash    vs gate    +0.166 +/- 0.070   z = +2.39   SIGNIFICANT
capture vs hash    +0.104 +/- 0.098   z = +1.07   not significant
```

## What is established

**The candidates reaching the search track's gate diverge in games far less than either reference
program does.** Two structurally different programs — a transposition table, and a capture extension
built to see tactics a flat search misses — both produce 2-3x the decisive rate of the mutations the
loop actually produces. That holds against both, at z = 2.39 and z = 3.67.

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
decisive rate at or below the gate's 14.7% means the mechanism is WRONG.* It came in at **41.7%,
z = +3.67 above the gate.** The falsifier did not trigger.

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

The 29 gate matches mix MAIN (budget 16) with MCTS (budget 256-1024) while `refmatch` runs at budget
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

### Provenance of the gate figures, and the robustness check

This file was first written on 21 completed gate matches. The arms have since produced 29, and
everything above has been RECOMPUTED on all of them (348 games) so the document is internally
consistent rather than carrying two vintages of the same number.

What the 38% larger sample changed: draw rate 0.857 -> 0.853, decisive 0.143 -> 0.147, and the two
comparisons z = +3.68 -> +3.67 and +2.41 -> +2.39. Nothing material. The check is recorded because
"my headline rests on a sample that has since grown" is a failure mode worth catching in one's own
work, not because the answer moved.

## REFUTED: my own mechanism's premise. Gated candidates have LOST tactics, not preserved them.

The mechanism registered in `refmatch_discrimination_PREREG.md` ran:

1. the guard is `f >= best_found`, the seed scores 25/25, so every survivor also scores 25;
2. therefore every candidate reaching the gate solves the SAME tactical positions as the champion;
3. tactical equivalence implies similar play, so the games draw;
4. collapsed variance makes acceptance impossible.

**Steps 1 and 2 are false in the configuration actually running.** `guard_floor` is not
`best_found`; it is `max(seed_mates - tolerance, ceil(0.84 * seed_mates))` (`evolve.rs:399-404`).
With MAIN's seed at 19 and tolerance 4 that is `max(15, 16) = 16`, so a candidate may lose THREE of
nineteen mates and still pass. The run header says so plainly — `19/19 mates (floor 16)` — and I
read past it.

Measured across every gate call on disk:

```
MAIN (seed 19/19, floor 16)   4 of 4 gated candidates scored exactly 16 — THE FLOOR, none at 19
MCTS (seed 11/19, floor 10)  19 at 10 (the floor), 4 at 13, 3 at 14
```

**Gated candidates sit at the minimum tactical score the tolerance permits.** They have lost the
maximum allowed, not preserved anything — and they still draw 85.3% of their games.

### A wrong mechanism made a right prediction, which is worth noticing

The registered falsifier was "capture extension at or below the gate's decisive rate means the
mechanism is WRONG". Capture came in at 41.7% against 14.7% and the falsifier did not trigger. So
the PREDICTION held while the EXPLANATION behind it was false. That is not a rescue — it is the
ordinary case of a mechanism being underdetermined by one confirming test, and it is why a single
survived falsifier was recorded above as "survival, not proof".

### What the measurement actually supports

A program that solves three fewer mate positions plays nearly identically in games. The likeliest
reading is that the lost positions are ones games do not visit — deep or rare tactical shapes the
guard set is built from. That does not rescue the preservation story; it REPLACES it with the
decoupling already measured twice today:

* `identity:12/27` — 44% position agreement, near-total draw;
* hash reuse — ~100% agreement, 31% decisive;
* and now — candidates at the tactical FLOOR, still 85% drawn.

Three independent angles, all saying the position sets and the games measure different things. The
guard is not selecting for game-neutrality by preserving tactics. It is filtering on a dimension the
games are largely blind to, which produces game-neutral candidates as a side effect.

**What this changes downstream.** The HARD-set experiment becomes MORE important, not less: if the
existing position sets are decoupled from games, a new position-set gradient must be shown to track
game outcomes before anything is built on it. That caveat is already in `hardn_probe.sh`'s header,
written before this measurement, and it now has a third piece of evidence behind it.
