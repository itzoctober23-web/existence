# Candidate B is NOT "genuinely unmeasured" — a plateau-regime replay sweep ran on 2026-09-08 and returned a null

**2026-09-12 03:26.** `structural_next_PREREG.md` ranks Candidate B (a rolling data window) second
and says of the staleness axis: *"That axis is genuinely unmeasured."* It is not. `replay_ab2.log`,
dated 2026-09-08, contains a plateau-regime sweep that was never written up as a `_RESULT.md` and so
never reached `RESULTS_INDEX.md`.

```
=== replay window, PLATEAU regime: 1200s per arm, all resuming from champion_long.net ===
  replay-gens 1     reached gen 42 ( 6 accepted)   0.855 +/- 0.042
  replay-gens 8     reached gen 42 (11 accepted)   0.852 +/- 0.046
  replay-gens 999   reached gen 41 ( 8 accepted)   0.863 +/- 0.038
```

Scored against the frozen origin with an identical match for every arm. The resumed champion itself
measured ~0.83-0.85, which is the baseline all three sit on.

## What it shows, stated at its real strength

**A null, and an underpowered one.** The three arms span **0.011** against confidence intervals of
**±0.038 to ±0.046** — the spread is a quarter of a single arm's interval. Windows of 1, 8 and
effectively-unlimited are indistinguishable here.

It is the right regime. `replay_ab_CAVEAT.md` is explicit that the EARLIER 420s run started from
scratch and therefore tested generations 1-19, where the champion improves fast and the staleness
argument is at its strongest. This run resumes from `champion_long.net`, which is the plateau regime
the question is actually about, and where the caveat predicts discarding should be *pure loss*. The
unlimited arm is nominally highest (0.863) — consistent with that prediction — but by a margin four
times smaller than its own interval.

## Why it still does not close Candidate B

Three scope limits, all checkable from the log:

1. **42 generations per arm**, not 2000. `low_sweep2` needed 2000 to resolve a configuration effect.
2. **`--games 2400 --depth 2`**, which is not the production configuration (`--games 8 --depth 3`).
   `generation_is_not_a_unit_FINDING.md` records how badly "generation" compares across differing
   `--games`.
3. **Underpowered by its own numbers** — nothing separating 0.011 could resolve against ±0.04.

So the honest status is **"measured once in the right regime, null, underpowered, at a
non-production configuration"** — not "genuinely unmeasured". That distinction matters for ranking:
a re-run is a *power* upgrade on a null, not a first look, and it inherits a prior that the effect is
small.

## Also noted: the 2026-09-08 arms ran on HIS core

`replay_ab2.sh:50` pins the arms with `taskset -c 15`. Core 15 is inside the 12-15 range reserved for
his desktop. Not acted on — the run is four days finished — but any re-run must use 6-11, and the
script should not be reused as-is.

## Status

No experiment launched. This corrects a PREREG claim against what is on disk, which is the same class
of error as `standing-brief-existence-next-is-closed`: a plan is a hypothesis about what is open,
never an authority on it.
