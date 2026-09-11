# Every production run is FLAT on the absolute ruler — all 166 Elo came from BETWEEN runs, not within them

**2026-09-11 09:2x.** His stop condition is "1600 on the pooled ruler **with a rising trend** by
day 7". The level half was answerable; the trend half never has been, because `live_ruler.sh`
takes ONE 120-game sample per rung, so each production run is a sequence of n=1 readings at
+/-50-67. `ruler_trend.py` fits those properly (weighted least squares, weights 1/sigma^2).

## Every run with a real generation span is flat

    run          n   gens          POOLED  +/-   slope/1k gens   z      chi2/dof
    prod1       19   356-17036     1315    11    +1.7 +/- 2.3   +0.71   0.32
    prod3       17   569-18562     1387    13    +2.6 +/- 2.5   +1.06   0.44
    prodk0633   10   8301-15825    1438    18    +8.4 +/- 7.7   +1.10   0.42
    prodk0759   10   26-8584       1481    19    +1.0 +/- 7.1   +0.14   0.66   <- SHIPPED lr 0.0002
    prod5        8   119-7219      1418    20    -4.0 +/- 8.3   -0.48   0.75
    prod2        7   1604-10052    1302    18    -3.3 +/- 6.3   -0.53   0.40
    prod4       11   737-2052      1464    17    +5.6 +/- 43.4  +0.13   0.41

**Not one reaches |z| > 2.** And chi2/dof is 0.32-0.75 everywhere -- BELOW 1 -- so the scatter is
already fully explained by the quoted error bars. There is no signal hiding underneath them.

## The saturation control, run BEFORE believing "flat"

A ruler out of range reports flat no matter how strong the engine gets, and this project has met
that exact failure twice (`instrument_saturation_RESULT.md`; the 4PC anchor at 31-0-0 buying a
lower bound). So:

    pooled 1481 vs the SF-1320 rung  ->  expected score 0.716
    observed scores                 ->  0.758, 0.679, 0.758
    1600 would imply                ->  0.834, still far below the ~0.95 saturation band

The ruler can see 1600 comfortably. **Flat is a measurement, not a ceiling.**

## What actually produced the gains

    prod1 1315 +/- 11  ->  prodk0759 1481 +/- 19     = +166 +/- 22  (7.5 sigma)

Against that, the best-constrained WITHIN-run estimate is prod1: +1.7 +/- 2.3 Elo per 1000
generations over a 16,680-generation span = **+28 +/- 38 Elo for the entire run**, consistent
with zero.

**So essentially all 166 Elo came from CONFIGURATION CHANGES BETWEEN runs, and none from
generations within them.** This replicates and generalises `untrained_baseline_RESULT.md`
("~235 Elo, all of it before generation 200, then flat for 1200 generations") and
`elo_per_ply_RESULT.md` ("twelve hundred generations of training bought ~0") -- now across seven
production runs and ~100 ruler readings, with a trend test and a saturation control attached.

## What this means for the stop condition

We are at **1481 +/- 19**, needing 1600: a gap of 119, or 6.3 SE. The shipped run's trend is
+1.0 +/- 7.1 per 1000 generations. Taking even the 2-sigma UPPER bound (+15.2/1000) at face
value, closing 119 Elo would need ~7,800 more generations; at the point estimate it never closes.

**Waiting is not a plan.** The lr sweep moved the plateau LEVEL by ~96 Elo (1392 -> 1488 on
matched 2000-generation arms) and did NOT restore a rising trend -- which is exactly what this
table predicts: changing the configuration moves the level, running longer does not.

The 1600 target is reachable the way the previous 166 Elo was reached: another configuration
change of similar size. Per `RESULTS_INDEX.md` the two live candidates are
`blend_RESULT.md` ("the training target's blend is the highest-leverage knob measured") and
`depth_RESULT.md` ("deeper labels beat more labels at equal compute -- PROMISING, needs
replication"). Neither is closed.

## Two bugs found while building the instrument, both recorded because either could have shipped a wrong number

1. **Catastrophic cancellation.** The textbook `den = S*Sxx - Sx^2` is non-negative by
   Cauchy-Schwarz, but generation numbers reach ~18,000, so the subtraction cancelled away the
   significant digits and returned **den = -1.27e12** on `prod1`. `math.sqrt` raised ValueError,
   which was luck: the same cancellation one digit smaller returns a plausible WRONG slope
   silently. Fixed by centering x.
2. **Degenerate fit emitting a formatted non-number.** The `sweep*` arms have all four rungs at
   the SAME generation (they are repeat samples of one frozen net, not a time series), so Sxx is
   a floating-point crumb rather than exactly 0 and a `<= 0` guard misses it. The fit reported
   slope +354.7 with an SE of **1.2e17**. Now refused explicitly, and the row still PRINTS as
   "NO TREND DEFINED" -- a dropped row reads as "not measured".

## UPDATE 10:3x — the flat bound tightens by 4x, and a configuration change clears it

`prodk0759` has since reached 22,141 generations and 28 ruler rungs. The same weighted fit, now on
2.6x the span and 2.8x the rungs:

    earlier   n=10  gens 26-8584    1481 +/- 19   slope +1.0 +/- 7.1   z=+0.14
    now       n=28  gens 26-22141   1476 +/- 11   slope -0.6 +/- 1.6   z=-0.38   chi2/dof 0.31

**The slope's error bar shrank 4.4x and the answer did not move.** Over the full 22,115-generation
span the implied total change is `-13 +/- 35` Elo — zero, measured tightly rather than merely
unresolved. chi2/dof of 0.31 says the quoted per-sample error bars more than account for the
scatter.

### What that makes of the blend arms

    production, 22,141 generations at blend 0.75    1476 +/- 11
    blend 0.75 arm, 2,000 generations               1490 +/- 30
    blend 0.85 arm, 2,000 generations               1544 +/- 32

The 0.85 arm sits `+68 +/- 34` above the production lineage — 2.0 sigma. Stated carefully, because
this is NOT a controlled contrast: production and the arms share a champion but differ in elapsed
training and in when they branched. **The controlled comparison remains the paired head-to-head,
0.541 +/- 0.032 = +29 Elo**, and the arms-vs-each-other ruler contrast, `+54 +/- 44`.

What the production number does establish is the BASELINE the configuration change is being asked
to beat: 22,000 generations of training at the shipped blend bought `-13 +/- 35` Elo, and 2,000
generations at a different blend is the only thing on file that has moved the altitude at all.
