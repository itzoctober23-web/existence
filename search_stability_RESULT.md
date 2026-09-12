# The budget's move choice IS noisier than fixed depth's — 3.2 points less self-consistent, on every seed

**2026-09-12 07:45.** Tests the premise of the only surviving mechanism. It holds, which is the first
positive evidence in this chain rather than another elimination.

## Why this was the right next test

`budget_harm_is_emergent_RESULT.md` showed the harm appears only inside the loop and named three
mechanisms. Two are now gone:

```
drift                    CONTRADICTED  -- the BETTER arm drifted further (trajectory_drift_RESULT)
decisiveness feedback    WEAKENED      -- search and net push in opposite directions
variance amplification   the survivor
```

Variance amplification has a premise that needs no training run: **the budget's move choice must
actually be noisier than the control's.** If it were not, all three would have failed and the chain
would need a new hypothesis rather than a bigger experiment.

## The measurement

One fixed depth-3 trajectory, so both arms score identical positions. At each position each arm is
run **twice with different searcher seeds**, and the question is whether it picks the same move both
times. Both arms get identical treatment — two fresh searchers, two different seeds — so any
difference in self-agreement is the arm's own instability.

This is the quantity `budget_allocation_ab` measured only for its depth-4 reference (94.7–96.7%),
which is what revealed that every arm's ~40% agreement with it was a property of depth rather than an
indictment of the budget. Here it is measured for the two arms that actually differ.

```
BUDGET minus DEPTH-3 self-agreement (points; negative = budget is NOISIER)
width      seed1   seed2   seed3     mean       signs   realised d
<=12         1.4    -5.3    -4.9     -2.9     2/3 neg         3.53
13-20        0.0     3.0    -0.2     +0.9     1/3 neg         3.15
21-28       -3.9    -2.8    -7.7     -4.8     3/3 neg         2.90
29-36       -3.9    -6.5    -6.5     -5.6     3/3 neg         2.39
>36         -1.9    -0.7    -0.9     -1.2     3/3 neg         2.15

OVERALL   [-2.1, -3.1, -4.3]   mean -3.17, sd 1.10   ALL NEGATIVE
```

Absolute levels: depth 3 self-agrees **96.5–97.6%**, the budget **93.3–94.8%**.

## The premise holds

**The budget is less self-consistent than fixed depth on 3 of 3 seeds**, by about 3.2 points overall,
and the effect is 3/3 negative in each of the three widest buckets. Feeding a self-play loop a move
chooser that disagrees with itself ~5% of the time instead of ~3% is exactly the input variance
amplification requires, and the loop trains on its own output 2000 times.

## The shape is NOT what I expected, and that matters

The naive prediction was that instability tracks width monotonically, because
`budget_undersearches_wide_positions_RESULT.md` found the budget's damage concentrated at >36 legal
moves. It does not:

```
strongest instability   29-36  (-5.6)   realised depth 2.39
weakest of the three    >36    (-1.2)   realised depth 2.15
```

**The widest bucket is the most stable of the three.** A plausible reading is that at realised depth
2.15 the budget is so consistently shallow that there is little left to vary — it reliably returns
the best depth-2 move — whereas at 2.39 it sits nearer a boundary where a small perturbation decides
whether another ply completes at all.

**That is an explanation, not a measurement**, and this project has a standing rule about the
difference. The discriminator is cheap and not run here: sweep the budget so a bucket's realised
depth moves across an integer, and see whether instability peaks at the fractional values. If it
does, the mechanism is boundary-crossing rather than shallowness.

## What this does and does not establish

* **Establishes:** the budget's move choice is genuinely noisier, on every seed, concentrated in the
  wide positions where its search goes shallow. The premise of variance amplification is real.
* **Does NOT establish that the noise causes the harm.** A 3.2-point instability gap is a plausible
  seed for compounding, but this measures one generation. Whether it amplifies over 2000 requires the
  loop, which is exactly what `budget_harm_is_emergent_RESULT.md` concluded about every channel here.
* **Does not rule out an unnamed fourth mechanism.** Three were named from what was known then; a
  premise surviving is not proof it is the operative one.
* **Nothing ships**, no games were played, and no figure is quoted as Elo.

## The experiment this points to

Instrument an arm pair to save a net snapshot every N generations, then measure the **variance across
consecutive snapshots** for each arm. Variance amplification predicts the budget arm's
generation-to-generation movement grows relative to the control's; a stable ratio falsifies it. That
is the first experiment in this chain that must be built before the run rather than derived after it,
and the instrumentation is the cheap part.
