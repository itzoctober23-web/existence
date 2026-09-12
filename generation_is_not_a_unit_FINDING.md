# "Generation" means 8 games in one result and 2,400 in another — a 300x unit gap that made Candidate A look infeasible

**2026-09-12.** Before launching Candidate A I checked whether its planned N was affordable, and the
two numbers on disk disagreed by a factor of ~1000.

```
structural_next_PREREG.md:105   "Planned N: 2000 generations per arm"
datagen_depth_RESULT.md:12      depth-3 datagen managed  6 generations in 1800s
```

Taken together that reads as ~7 days per arm, which would rule the experiment out. Measured on the
live trainer instead:

```
prodk0127  (running, --depth 3)   7,092 generations in 1,988s   = 3.57 gen/s
prodk1926  (completed 6h run)    70,366 generations in 21,600s  = 3.26 gen/s
```

Three orders of magnitude apart, for the same `--depth 3`.

## The cause: `--games` differs 300x and "generation" silently absorbs it

`datagen_depth_RESULT.md:89` already records it — *"arms used `--games 2400`"* — against the loop's
production setting of `--games 8`, confirmed from the trainer's own banner on the 2000-generation
sweep arms (`blend_075.log`, `blend_085.log`: `games/gen=8 depth=3`).

So a "generation" is **8 games** in production, `low_sweep2` and every arm the PREREG's planned N was
copied from, and **2,400 games** in `datagen_depth`. The word is the same; the unit is 300x apart.

| | generations | games/gen | **games** |
|---|---|---|---|
| PREREG planned N | 2000 | 8 | **16,000** |
| `datagen_depth` depth-3 arm | 6 | 2400 | **14,400** |
| champion it was compared against | ~2,200 | 8 | **17,600** |

## Two consequences

**1. Candidate A is cheap, not a 7-day commitment.** 2000 generations at `--games 8` is ~16,000
games — roughly 10 minutes at the production rate of 3.3 gen/s on 4 threads, or ~35 minutes on one.
The capacity objection to running it was an artefact of the unit, and the arms are now running.

**2. `datagen_depth`'s champion comparison is roughly game-for-game even.** Its headline —
"six generations of depth-3 datagen match a champion built from 2,200" — and its "1% of the
generations" framing are true *in generations*, but 14,400 games against 17,600 games is near
parity in data, not 1%.

**This does not weaken that result; the strong comparison is the one inside the experiment.** Its own
depth-1 arm ran 610 generations at `--games 2400` = **1,464,000 games** and reached −236, while the
depth-3 arm reached −108 on 14,400. That is ~100x fewer games for a large, resolved gain, and it is
unaffected by any champion-comparison framing. The file already flags the mismatch at line 89
("610 training steps vs 6 rather than a matched one"); what was missing is that the *headline* number
inherits it.

## What to change

Any planned N in this repo has to carry `--games`, because the repo's own results use the word
"generation" for two quantities 300x apart. `structural_next_PREREG.md` has been annotated
accordingly. This is the `count-the-right-unit` failure: a ratio that looks like a physical
difference and is a unit error.
