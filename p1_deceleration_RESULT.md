# P1 is still gaining, at ~+24 Elo per 100 generations — and BOTH instruments are now at their limit

**2026-09-10.** Measured with `netmatch` at depth 4 (the declared strength standard) against
saved ancestors, because the origin control had entered the band where it has reversed sign twice.

## The deceleration, head-to-head and non-saturating

| window | score | Elo | effect / between-seed sd | verdict |
|---|---|---|---|---|
| gens 29 → 300 | 0.708 ± 0.031 | **+154** [+129..+181] | **4.4×** | ROBUST |
| gens 200 → 300 | 0.535 ± 0.029 | **+24** [+4..+45] | **0.7×** | MARGINAL — needs ~14 seeds |

Both intervals exclude 0.5, so the loop is **still improving**. But the recent window's effect
(0.035) is *smaller than the between-seed standard deviation* (0.047), which `netmatch`'s own power
note reports: a different training seed could plausibly reverse it. **One seed carries the +154; it
does not carry the +24.**

## Why the origin control saw nothing, and why that is NOT only saturation

    control @gen 100   0.873 +/- 0.022
    control @gen 200   0.871 +/- 0.021      change -0.002 +/- 0.030, not resolved

Twenty candidates were ACCEPTED between those two controls and the metric moved by nothing. The
tempting reading is "the metric has saturated" — it is inside the 0.861–0.967 band where
`instrument_saturation_RESULT.md` records two sign reversals. But the arithmetic gives a simpler
and more useful reason first:

**A +24 Elo gain at rate 0.87 moves the rate by about +0.015, and the control's interval is
±0.021.** The gain is inside the error bar. The control did not fail to see it because it is
saturated; it could not have seen it at 400 pairs whatever its calibration. That is a RESOLUTION
limit, and it is fixable by pairs in a way saturation is not.

Both problems are real and they now bind together: the metric is simultaneously in its
sign-reversal band AND too coarse for the effect size that remains.

## What this means for the next measurement

* **Do not read "0 accepts in 40 generations" or "the control did not move" as a plateau.** Both
  are consistent with a +24 Elo/100-generation trend that neither instrument can resolve. That
  mistake was made once today and retracted within the hour.
* **Head-to-head against a saved ancestor is now the only instrument that works**, which is why the
  loop saves a ladder rung at every control (added today — `{out}.gen{N}.net`). Before that the
  champion's history was overwritten by every accept.
* **A claim about the recent window needs SEEDS, not pairs.** The interval at 200 pairs is ±0.029
  and the between-seed sd is 0.047, so more pairs on one seed buys precision the seed noise
  swamps. ~14 seeds for 80% power on a 0.035 effect.

**Not claimed:** that the deceleration will continue, or that it reflects a ceiling. Two points
(+154 over 271 generations, +24 over the last 100) are a trend of two, and the second is marginal.
