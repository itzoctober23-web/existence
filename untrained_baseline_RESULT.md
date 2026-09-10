# Training DID work — ~235 Elo, all of it before generation 200, then flat for 1200 generations

**2026-09-10.** The control that makes the "not learning" finding actionable, suggested precisely
because the absolute ruler alone could not separate two very different failures.

## The measurement

An **untrained** width-16 net — `Net::random(16, …)`, the same construction and architecture as the
champion — on the same ruler: depth 4, SF-1320 @10k nodes, 120 games.

| net | W-D-L | score | Elo vs SF-1320 | absolute |
|---|---|---|---|---|
| **untrained w16** | 0-26-94 | 0.1083 | **−366 ± 67** | ~954 |
| gen200 | 22-38-60 | 0.3417 | −114 ± 53 | ~1206 |
| champion (gen ~2200) | 22-41-57 | 0.3542 | −104 ± 52 | ~1216 |

**Training is worth ~235–260 Elo over the untrained net, and the intervals do not overlap** (−366±67
against −104±52). This is the one comparison today that is unambiguously resolved.

## Which of the two failures this rules out

The ruler said "flat across 1200 generations". That was consistent with two very different problems:

* **the trainer, feature encoding, or loss is broken** → no labels would help, and the fix is the
  learner
* **the learner is fine and the LABELS stop carrying signal** → the fix is datagen, not the trainer

**It is the second.** A broken learner could not have produced 235 Elo. The pipeline demonstrably
learns; it stops learning.

## Where the learning happened, and where it stopped

All of it lands **before generation 200** — gen200 already reads −114, statistically identical to the
champion 2000 generations later. So the curve is a step, not a slope:

```
untrained  −366
gen200     −114     <- essentially all the gain, in under 200 generations
gen600     −140
gen1000    −172
gen1400    −127
champion   −104
```

## What that implies

The labels are informative while the net is near-random and stop being informative once it is not.
That is the mechanism MASTER_PLAN already predicts for the horizon — *"when both players are
near-random, the game result is nearly independent of a position 40 plies earlier"* — arriving from
the other side: once the net is no longer near-random, **its own depth-1 self-play at 2400 games a
generation is not producing positions whose outcomes teach it anything further.**

The remaining question, which control 2 answers, is whether better LABELS on those same positions
would restart the climb. If they would, the fix is datagen depth and game length. If they would not,
something subtler is saturated.
