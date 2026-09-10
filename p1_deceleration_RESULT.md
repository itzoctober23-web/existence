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
* **A claim about the recent window needs SEEDS, not pairs** — the interval at 200 pairs is ±0.029
  while the between-seed sd is 0.047, so more pairs on one seed buys precision the seed noise
  swamps. How many seeds depends on which sd, and `netmatch` deliberately uses the most
  conservative of four measured estimates:

  | sd | source | seeds for 80% power |
  |---|---|---|
  | 0.047 | 2 estimates — kept high ON PURPOSE as a guard | ~14 |
  | 0.043 | 4 estimates, canonical in STATE.md | ~12 |
  | 0.039 | 3 estimates | ~10 |
  | 0.0285 | measured DIRECTLY across five seeds, 2026-09-09 | ~5 |

  The `~14` the tool prints is the safe end. Even at the directly-measured 0.0285 it is ~5, which
  is still more than the one in hand.

* **BUT SEEDS ARE THE EXPENSIVE ROUTE. Buy effect size with GENERATIONS instead.** The seed
  requirement scales as `(2.8·sd/effect)²`, and for a WITHIN-LINEAGE trend the effect grows with
  the window while the seed noise does not:

  | window | expected gain at +24 Elo/100 | score | effect | one seed enough? |
  |---|---|---|---|---|
  | 100 gens | +24 | 0.534 | 0.034 | no — below sd |
  | **200 gens** | **+48** | **0.569** | **0.069** | **yes — clears 0.047** |
  | 300 gens | +72 | 0.602 | 0.102 | yes, comfortably |

  At ~1200 generations/hour a 200-generation window costs about **10 minutes of loop time**, against
  five to fourteen full re-runs for the seed route. **Next measurement: gen500 against gen300.** If
  it resolves, the trend is real at one seed; if it does not, that is evidence the trend has stopped
  — decisive either way, which the 100-generation window was not.

**Not claimed:** that the deceleration will continue, or that it reflects a ceiling. Two points
(+154 over 271 generations, +24 over the last 100) are a trend of two, and the second is marginal.


---

## ⚠ 2026-09-10, live: THE TWO INSTRUMENTS DISAGREED IN SIGN on the same pair

The gen-300 origin control landed after the head-to-head above, and it points the other way.

    control @gen 100   0.873 +/- 0.022
    control @gen 200   0.871 +/- 0.021
    control @gen 300   0.847 +/- 0.023      <- DOWN

| instrument | gen300 − gen200 | reading |
|---|---|---|
| via the FROZEN ORIGIN | **−0.024 ± 0.031** | gen300 WEAKER (not resolved) |
| HEAD-TO-HEAD at depth 4 | **+0.035 ± 0.029** | gen300 STRONGER (interval clear of 0.5) |

Same two nets, same window, opposite signs. `instrument_saturation_RESULT.md` predicted exactly
this — it records the frozen-origin metric reversing sign **twice** at the top of its range, and
0.847–0.873 is that range. This is the first time it has been caught on a live pair rather than
in retrospect.

**Which to believe: the head-to-head.** Not because it agrees with the hoped-for answer, but
because the project already decided this on measured grounds — `netmatch.rs` exists precisely
because "beating a random net is a saturating measurement", and depth-4 head-to-head is the
declared strength standard. The origin reading also has the weaker statistics of the two here: its
difference is NOT resolved (−0.024 ± 0.031) while the direct one is (interval [0.506, 0.564]).

**What it does NOT license.** The head-to-head effect is still below the between-seed sd, so
"gen300 > gen200" remains a one-seed result. The disagreement does not promote it; it only removes
the origin control as evidence against it. Two weak instruments pointing opposite ways is not one
strong instrument.

**Operational consequence, effective now:** stop quoting the origin control as P1's strength
number. It is inside its documented reversal band, it lacks the resolution for the effect sizes
that remain (±0.021 against a ~+0.015 expected move), and it has now contradicted the standard
instrument on a live pair. It stays useful only as a coarse "has the lineage collapsed" tripwire.

---

## ⚠ THE PRE-REGISTERED PREDICTION FAILED, and the failure is the result

I sized the next measurement on the assumption that gains are LINEAR in generations, predicting a
200-generation window would clear the between-seed noise on one seed:

    PREDICTED   200 gens -> +48 Elo -> score 0.569, effect 0.069 > sd 0.047 -> RESOLVES
    OBSERVED    200 gens -> +27 Elo -> score 0.539, effect 0.039 < sd 0.047 -> does NOT

Both windows are measured against the SAME opponent (the gen-200 net), so there is no
non-transitivity to explain it away:

| comparison | score | Elo |
|---|---|---|
| gen300 vs gen200 | 0.535 ± 0.029 | +24.4 |
| gen400 vs gen200 | 0.539 ± 0.027 | +27.2 |
| **implied gens 300→400** | | **+2.8** |

**The methodological error is worth more than the number.** I used linearity to size an experiment
whose purpose was to test whether the trend continues. If the gain had been linear the design
worked; because it is not, the design under-powered itself by exactly the amount the effect
decelerated. An experiment sized on the hypothesis it is testing cannot fail informatively in one
direction — it just comes back marginal again, which is what happened.

**What is and is not established:**

* **Established:** a 200-generation window produced +27 Elo, not the +48 that a constant
  +24/100 rate predicts. Linear extrapolation from the first window is refuted.
* **NOT established:** that gens 300→400 added ~+3 rather than ~+24. The difference between the two
  windows is +0.004 ± 0.040 — nowhere near resolved. The deceleration is visible in the point
  estimates and nothing more.
* **Still true:** both windows exclude 0.5, so the loop is still gaining. It is the RATE that is
  in question, not the sign.

**What would actually settle it,** now that window-widening is known not to buy effect size here:
seeds. `netmatch` says ~12 at sd 0.047, ~5 at the directly-measured 0.0285. At ~600–1200
generations/hour a 5-seed replication of a 200-generation window is roughly 1–2 hours of box time,
which is affordable and is the honest next step rather than a third window.
