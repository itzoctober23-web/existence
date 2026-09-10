# The shipped guard tolerance admits an exploit AND excludes the target — worst of both

**2026-09-10.** `crates/pipeline/tests/exploit_regression.rs::shipped_configuration_admits_no_known_exploit`
is **failing, and was already failing before today's changes** (verified by stashing the diff and
re-running on the pre-change tree). This records what it is saying, because a red test nobody reads
is the same as no test.

## What it reports

```
the shipped configuration (guard tolerance 4) admits 2 known exploit(s):
  MAIN gen 1 under strict:      2 mates vs champion 6 (floor 2), rate 110x, games 0.208
  MAIN gen 1 under spec_filter: 2 mates vs champion 6 (floor 2), rate 110x, games 0.208
```

A specimen that solves **2 of the champion's 6 mates**, lands exactly ON the floor (6 − 4 = 2),
carries **110× the champion's fitness rate**, and scores **0.208 in games** — i.e. it is crushed.
Both selection rules admit it.

## The trap is that the tolerance cannot be tuned out

The mates floor is `champion_mates − guard_tolerance`, and the tolerance is doing two incompatible
jobs at once:

* **Lower it** (4 → 3) and the 2-mate exploit is excluded — but capture extension, which solves
  **18/25** against a 25/25 seed, needs a tolerance of **7** to be reachable at all
  (`evolve.rs:1236`). It is already unreachable at 4.
* **Raise it** (4 → 7) and capture extension becomes reachable — and so does every cheaper
  non-searching specimen in the same band.

**So the shipped configuration is the worst of both:** it excludes the one reference program that
scores on the hard set, and admits a program that gives up two thirds of the mates to be cheap.
There is no value of a single scalar tolerance that admits the first and excludes the second,
because on the mates axis they look the same — both give up mates.

## Why this is the same finding as the ladder, from the other end

`surrogate_inverts_RESULT.md` measured the full 10-program round robin: the three strongest programs
in games are proof-number search and both iterative-deepening variants — all of which **search
more** — and `mates/Mcost` ranks them 9th, 5th and 6th of 10 while the three cheapest sit on top.

This test is the same defect seen through the guard rather than the ranking: the fitness cannot
distinguish "gave up mates because it searches differently and better" from "gave up mates because
it stopped working". The tolerance is a scalar and the distinction is not.

## Status and what would actually fix it

**NOT fixed here, and not fixable by tuning the number.** Recorded so the red test is understood
rather than silenced. The fix has to give the guard information the mates count does not carry —
the obvious candidate being the games themselves, which already separate these cases perfectly
(0.208 vs the seed's ~0.481) and which `search_track_WHY_NOTHING.md` independently names as the
only trustworthy signal.

That is the P2 restart condition: not a new tolerance, a guard that consults the game gate.
