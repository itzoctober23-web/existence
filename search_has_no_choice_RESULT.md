# The search almost never has a CHOICE: 94% of generations offer ≤1 distinct fitness

Measured 2026-09-09 from the arms' own log fields. No new run — every number below was already on
disk in `gate_*.log` and had never been aggregated.

## The funnel, 79 generations across four arms

    candidates SCORED              580
    passed the MATE GUARD           61    10.5% of scored
    DISTINCT rates among those      46    75.4% of mate-ok

    generations with ANY guard-passing candidate    47 of 79   (59%)
    generations with MORE THAN ONE distinct rate     5 of 79   ( 6%)
    generations whose best candidate exceeded 1.000x 2 of 47   (4.3%)

**In 94% of generations the population is handed zero or one distinct fitness value.** Selection
cannot select from a set of size ≤1. Every knob this project has tuned — gate bounds, EPS, guard
tolerance, surrogate role, population size, lambda — operates on a CHOICE that is not being offered.

## The two-stage collapse

**Stage 1, the mate guard: 10.5% survival.** 580 candidates in, 61 out. This is the intended
behaviour of `f >= best_found - guard_tolerance` and it is doing exactly what it was built for; the
point is its severity, which nothing downstream can compensate for.

**Stage 2, degeneracy among survivors: only 6% of generations see >1 rate.** Of the 61 that pass,
they cluster onto 46 distinct values, and they arrive so unevenly that five generations account for
nearly all the variety.

## Why this reframes tonight's other results

* **`eps_is_inert_RESULT.md`** showed the `.90-.98` band holds 0 of 60 candidates and concluded EPS
  0.10 is inert. This is why: with ≤1 distinct rate per generation there is no distribution for a
  retention width to cut into.
* **The collapse to `pop 2`** in every arm follows directly. Retention keeps distinct survivors;
  there are ~0.58 distinct rates per generation to keep.
* **"0 accepts" is not primarily a gate-strictness result.** The gate has been reached and has
  correctly rejected (`REJECT llr -3.18`, `-3.03`, `-2.97`, `-3.31`, all with VERIFY ≈ 0.42-0.49). It
  is downstream of a stage that hands it almost nothing.

## The unit error, recorded because it nearly became the headline

My first pass computed `distinct / candidates_scored` and reported **"92% of candidates duplicated a
rate already present"** — a dramatic claim about mutation redundancy. Wrong denominator: `distinct`
counts distinct rates among the **mate-ok** candidates, not among all scored ones, because a
candidate that fails the guard never gets a rate at all. Against the right denominator the
duplication is **25%**, which is unremarkable — and the real finding moved to a different stage of
the pipeline entirely. Same class as the earlier `result==0` row-vs-game confusion tonight.

## Honest limits

* Arms are young (gens 1-5) and three of four share `EXISTENCE_EVOLVE_SEED=1`, so trajectories are
  correlated rather than independent. The funnel shape is consistent across all four, but n is not
  four independent samples.
* `distinct` counts distinct RATES, not distinct programs. Two genuinely different programs with
  identical mates and identical cost count once — which is the right unit for SELECTION (the loop
  ranks on rate) and the wrong one for measuring operator diversity.
* This measures what reaches SCORING. It says nothing about the 52 ill-typed candidates rejected
  before that.

## What it points at

The lever is upstream of every knob tried so far: **raise the number of guard-passing, distinctly-
scoring candidates per generation.** That is a statement about the mutation operators and the mate
guard's severity together, not about selection pressure — and it is the first suspect tonight that
none of the four running arms varies.
