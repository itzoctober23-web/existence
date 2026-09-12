# The promotion margin is constant at ~0.535 while the absolute step has decayed to zero

**2026-09-12 05:00.** Measured from 354 live-ruler readings already on disk, all against the same
unchanging anchor (SF-1320 @10k nodes, our engine at fixed depth 4). No new compute was spent to
produce this table, and it does not depend on the experiment running in
`champion_absolute_PREREG.md`.

## The table

Production lineages in the order `auto_promote` worked through them:

```
lineage        n   abs Elo    se      step
prod1         19      -5.1   6.6        --
prod3         17     +68.2   8.7       +73
prod4         11    +146.3  10.8       +78
prod5          8    +100.2  16.1       -46
prodk0633     10    +119.9  12.3       +20
prodk0759     33    +159.1   5.7       +39
prodk1056     67    +205.7   4.7       +47
prodk1658     24    +232.6   5.6       +27
prodk1926     79    +227.9   3.8        -5
prodk0127     38    +227.7   6.2        -0
```

Against that, every promotion the ladder has ever made:

```
03:48  prod4.net      gen  2052: PROMOTED  0.531 +/- 0.027
13:47  prodk1056.net  gen 23056: PROMOTED  0.544 +/- 0.029
15:01  prodk1056.net  gen 34789: PROMOTED  0.535 +/- 0.033
23:04  prodk1926.net  gen 39836: PROMOTED  0.539 +/- 0.030
04:43  prodk0127.net  gen 24941: PROMOTED  0.535 +/- 0.031
                                 CONFIRMED 0.536 +/- 0.015 at 896 pairs, seed 911911
```

## The finding

**The relative margin is flat at ~0.535 (+25 Elo) across all five promotions. The absolute step has
decayed from +78 to zero.** The last two transitions are −5 and −0 against standard errors of about
5–6, across which the ladder recorded promotions — including the most recent, which is the
best-evidenced measurement this project has ever taken (896 pairs, confirmed on an independent seed,
agreeing with the 224-pair read to 0.001).

So the gate has not become noisier or less reliable. It keeps reporting the same margin, and that
margin has stopped corresponding to anything an external opponent can feel.

This reframes the plateau. `prodk0127_plateau_STATUS.md` records the pooled ruler sitting at 1561,
flat, 39 short of the day-7 line, and reads that as the training run failing to improve. The table
above says something sharper: **the run keeps producing nets that genuinely beat their predecessors,
and those wins stop converting into absolute strength.** That is a SELECTION problem, not a
training-rate problem, and it will not be fixed by more generations, a different learning rate, or a
wider net — the three levers the structural track has been working through.

## Why this is not just noise

The steps that matter are supported by the largest samples in the table: `prodk1926` has n=79
(se 3.8) and `prodk1056` n=67 (se 4.7). The decay is not one odd reading — it is monotone across the
last four transitions (+47, +27, −5, −0) with the two smallest steps carrying the tightest intervals.

Both instruments involved are load-immune by construction, so contention cannot have produced this:
the ruler is fixed-depth against fixed nodes and snapshots the net to /tmp before playing; the
promotion netmatch is paired at fixed depth 4.

## The mechanism is distribution overfitting, NOT non-transitivity

`nontransitive_walk_RESULT.md` forecloses the obvious explanation. It measured three 224-pair
matches along one lineage and showed they COMPOSE to within 0.01 Elo (+28.6, −0.7, sum +27.9 against
a measured +27.9 over the full span). **At the ~2,000-generation scale this walk is transitive**, and
that file narrows its own headline to say non-transitivity is at most a short-range, 5-generation
effect. A promotion spans ~25,000 generations.

So netmatch is not an unreliable instrument, and the table above is not evidence that it is. It is
internally consistent — demonstrably so. What the composition check cannot establish is agreement
with a DIFFERENT opponent, because every match in it was drawn from the same self-play family.

The correct statement is therefore: **strength against the net's own lineage is real, measurable,
transitive and reproducible — and it has stopped transferring to strength against an external
opponent.** The ladder is not being fooled about who beats whom. It is optimising a quantity that has
decoupled from the one we care about.

## The limitation, stated plainly

Each lineage figure is the **mean over that lineage's whole run**, not the net at the moment it was
promoted — `prodk1926` spans generations 132 to 69928. So this table compares *typical* strength of
successive lineages, which is the right unit for "is the ladder going anywhere" but the wrong unit
for "did promotion X help".

That second question needs the two specific files, and it is exactly what
`champion_absolute_PREREG.md` is measuring now, with its decision rule fixed in advance. The
within-lineage OLS for `prodk0127` (−61.2 Elo over gen 291..29386, 95% CI [−99.2, −23.2]) points the
same way but has the same defect in miniature: its newest readings are at gen 29386, while the
promoted net is gen 24941.

**Nothing here is quoted as shipped Elo.** These are ruler readings against one anchor; the
promotions passed their gate.

## What follows

If the pre-registered file-vs-file measurement lands below zero, the criterion itself has to change
before the ladder is allowed to promote again — "beats the current champion at 224 pairs" is then
demonstrably satisfiable without absolute progress. The obvious candidates, in the order this
project's own evidence supports them:

The corrected mechanism changes which fix can work, so this list is ordered by that rather than by
cost:

1. **Keep the cheap relative gate as a FILTER, and require an EXTERNAL confirmation before the
   champion file is overwritten.** The two-stage shape already exists — the 896-pair confirmation
   built today — and the only change is that stage 2 measures a different opponent rather than more
   pairs of the same one. Stage 1 stays cheap and rejects most candidates; stage 2 runs rarely.
   This is the cheapest change that addresses the mechanism.
2. **Gate against the external anchor directly.** Measures exactly the right quantity, and is the
   most expensive: the ruler needs ~27x its current 120 games to resolve a single +25 Elo step, so
   ~3,200 games per decision. Viable as stage 2 above, not as the primary gate.
3. **A POOL of past champions is NOT a fix here, and was listed as one in this file's first
   version.** Under non-transitivity it would have been the natural answer. Under distribution
   overfitting it is not: every past champion is drawn from the SAME self-play family, which is
   precisely the distribution that has decoupled from external strength. Broadening within a
   distribution that is itself the problem buys little. It would only help to the extent that older
   champions are far enough back to behave like different opponents — an assumption this project has
   not measured, and one `nontransitive_walk_RESULT.md` gives reason to doubt, since long-range
   comparisons within the family compose cleanly.

**The cheap diagnostic that separates 1 from 3**, if it is wanted before committing: match the new
champion against the OLDEST distinct champion on disk (11 are preserved, all distinct hashes). If
the ladder's wins compose along the whole chain — as `nontransitive_walk_RESULT.md` found at the
2,000-generation scale — the family is internally consistent end to end, and a pool gate is confirmed
useless. That is four netmatch runs, not 3,200 games.
