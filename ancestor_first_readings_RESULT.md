# First readings from the ancestor control: no measurable gain over 400-generation windows

**2026-09-10.** The ancestor control shipped this afternoon because the origin control had
saturated — gen 400 and gen 800 both read 0.882 ± 0.022 on the previous run, identical to three
decimals across 400 generations containing accepts. These are its first two production readings.

## What it says

| instrument | @gen 400 | @gen 600 | @gen 800 |
|---|---|---|---|
| vs the FROZEN ORIGIN | 0.895 ± 0.021 | — | 0.857 ± 0.024 |
| vs its OWN SELF 400 generations back | — | **0.464 ± 0.053** (vs gen 200) | **0.498 ± 0.052** (vs gen 400) |

Both ancestor readings sit on 0.5. Intervals: [0.411, 0.517] and [0.446, 0.550] — neither excludes
0.5, and neither shows the champion beating the net it was trained from 400 generations earlier.

## What that does and does not establish

**It is NOT "the loop is getting worse."** Both intervals contain 0.5. The honest statement is that
any gain over a 400-generation window is **smaller than this instrument can currently see** — at 160
pairs, ci95 ≈ 0.052, so improvements under roughly 0.05 (order 35 Elo) are invisible to it.

**It IS the first non-saturating progress signal this loop has ever had.** The origin control cannot
distinguish these cases at all: it reports 0.895 and 0.857, inside the band where
`instrument_saturation_RESULT.md` records it REVERSING SIGN twice (0.861 and 0.967). A fall from
0.895 to 0.857 there means nothing; 0.464 and 0.498 against a moving opponent mean something.

**It does not contradict this morning's +0.139.** That measured two nets ~49 minutes apart at depth
4 with 448 pairs, on a different run. These compare 400-generation windows at 160 pairs on run 7.
Different opponents, different windows, different power — comparing them directly would be the
pooling error this repo has already recorded twice today.

## The immediate consequence

**The question "is the loop still improving?" is now askable, and the answer is "not by more than
~0.05 per 400 generations".** For a loop running ~1800 generations/hour, that is the number that
decides whether to keep spending on it or change something — and until this afternoon there was no
instrument that could produce it.

## What to do with it, stated as next steps and not as findings

1. **Raise `--ancestor-pairs`.** 160 pairs cannot resolve what is plausibly a small per-window gain.
   The cost is linear and the reading is every 200 generations, so this is cheap to tighten.
2. **Shorten `--ancestor-lag`.** A 400-generation window conflates "no gain" with "gain that
   saturates early". A 100-generation lag against the same rung cadence would show the shape.
3. Neither is run here. Both are one flag.
