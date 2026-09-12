# The P1 kill criterion: conjunct 1 is now SATISFIED. Conjunct 2 is measurable but not yet measured.

**2026-09-12 00:55, day 7.** `MASTER_PLAN.md` §Phases declares the P1 kill:

> *"Kill: no iteration-over-iteration gain across iterations 4-8 AND the static-vs-deep residual is
> not shrinking -> pipeline bug; stop and find it."*

This file records where each conjunct stands, because the retro has just cleared the gate the
directive put in front of new work, and the docs specify this as the next thing that decides.

## Conjunct 1 — "no iteration-over-iteration gain" — SATISFIED, measured today

`WEEK1_RETRO.md`, day-7 reading:

```
prodk1926   63 rungs, gens 132-53,881   1541 ± 8   slope +0.0 ± 0.5 per 1000 gens   z +0.04   FLAT
prodk1056   67 rungs, gens 126-53,862   1522 ± 8   slope +0.6 ± 0.5                 z +1.30   FLAT
prodk1658   24 rungs, gens 152-20,156   1551 ±13   slope +0.5 ± 2.2                 z +0.22   FLAT
```

Corroborated by the PAIRED instrument overnight: `auto_promote`'s seven readings across 42,200
generations have mean 0.5054, and the one promotion resolved to **0.503 ± 0.014** at 953 pairs
(`promo_g39836_RESULT.md`). No gain, by two independent instruments.

## Conjunct 2 — the static-vs-deep residual — REPAIRED but NOT YET EVALUATED

`static_deep_residual_RESULT.md` (2026-09-11) established that the metric as written **could never
fire**: the search evaluates leaves with the net under test, so static and deep are one function at two
depths, and an **untrained random net scores corr 0.900**. The validated replacement scores the deep
side with ONE FROZEN REFERENCE net (`static_deep_residual --ref`); its untrained control falls to
**−0.019**, sign agreement 47.8% — chance, as it must be.

**It has not been run across the current run's checkpoints.** So the conjunction has still never been
evaluated, and with conjunct 1 now satisfied, conjunct 2 alone decides whether the spec says *"pipeline
bug; stop and find it."*

## The hazard that must be avoided when running it, named before running

**The reference net must be INDEPENDENT of the series being measured.** The first `--ref` run used
`p1_champion.net` as the reference. That net is the ENDPOINT of the champion lineage, so scoring that
lineage against it guarantees the last checkpoint correlates best and manufactures a rising trend —
the same circularity that made the original metric report 0.900 for an untrained net.

Series available:

```
p1_champion.net.r9.gen100 .. gen2200    ~22 checkpoints, one run
p1_champion_prev_* -> p1_champion.net     8 points, promotion-by-promotion
prodk1926                                 only 2 nets exist (start, live) -- NO checkpoint series
```

So the run must (a) pick a series, (b) pick a reference OUTSIDE it — a net from a different lineage, or
the series' own START rather than its end — and (c) state which, because the choice is the result's
main assumption.

## Status

**Not run tonight.** It needs a build and a reference choice that is defensible rather than convenient,
and getting that wrong reproduces the exact defect the repaired instrument exists to remove. Conjunct 1
is recorded as satisfied; conjunct 2 is the next measurement, and it is a spec-declared gate rather than
a new idea.
