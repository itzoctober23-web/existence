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

## THIRD READING, at 400 pairs — the tightest yet, and it does not move the conclusion

`--ancestor-pairs 160 → 400` (run r9). gen 600 vs gen 200: **353W-52D-395L, rate 0.474 ± 0.031**,
interval **[0.443, 0.505]**.

| reading | run | pairs | rate | interval |
|---|---|---|---|---|
| gen600 vs gen200 | r7 | 160 | 0.464 ± 0.053 | [0.411, 0.517] |
| gen800 vs gen400 | r7 | 160 | 0.498 ± 0.052 | [0.446, 0.550] |
| **gen600 vs gen200** | **r9** | **400** | **0.474 ± 0.031** | **[0.443, 0.505]** |

The interval narrowed by 40% and still contains 0.5. So **"no measurable gain over a
400-generation window" is now established at better power**, and the bound tightens: any gain is
under about **0.03** (order 20 Elo) per 400 generations, not 0.05.

**What is NOT established, and I want the restraint on record:** all three point estimates sit at or
below 0.5 (0.464, 0.498, 0.474), which is suggestive of slight DEGRADATION rather than a plateau.
Every one of those intervals contains 0.5, so that is a pattern in three numbers, not a result.

> **⚠ AND THE FOURTH READING BROKE IT, ~40 minutes later.** r9 gen 800 vs gen 400:
> **0.521 ± 0.034**, interval [0.487, 0.555] — *above* 0.5. The four readings are now
> 0.464, 0.498, 0.474, **0.521**: two below, one at, one above, every interval containing 0.5.
> **The "hint of decline" is withdrawn.** It was three numbers on one side of a line, which is what
> four coin flips look like about 12% of the time, and the fourth landed on the other side. Hedging
> it as a pattern rather than a finding is the only reason this costs a paragraph instead of a
> retraction. The conclusion that survives is the flat one: **no gain, and no loss, detectable over
> a 400-generation window.**
Pooling them would be wrong — they come from two different runs over different windows, which is
the pooling error this repo has recorded three times today. The honest position is: no gain
detected, and a hint of decline that is not yet distinguishable from noise.

The accept audit now running is the direct test of the mechanism that would produce exactly this
shape — accepts that are not actually stronger, so the champion random-walks or drifts down.

## What to do with it, stated as next steps and not as findings

1. **Raise `--ancestor-pairs`.** 160 pairs cannot resolve what is plausibly a small per-window gain.
   The cost is linear and the reading is every 200 generations, so this is cheap to tighten.
2. ~~**Shorten `--ancestor-lag`.**~~ **WRONG, corrected before acting on it.** If the loop gains
   *g* per generation, a 400-generation window accumulates 400*g* and a 200-generation window only
   200*g*. Shortening the lag HALVES the effect while leaving the interval unchanged, so it makes a
   small gain harder to detect, not easier. It would answer a different question — the SHAPE of the
   gain — and only once a gain is detectable at all, which it is not yet. For detection the lag
   should if anything be LENGTHENED.
3. Only (1) is worth doing now, and it is what was done: `--ancestor-pairs 160 → 400` at the same
   400-generation lag, taking ci95 from ~0.052 to ~0.037. Same window, better resolution, and the
   cost is one match per 200 generations.
