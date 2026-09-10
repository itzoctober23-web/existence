# Self-play decisiveness FALLS as the champion improves — so it cannot be the horizon's strength signal

**2026-09-10.** Measured while designing the fix MASTER_PLAN asks for: the training horizon
"should widen with strength rather than being fixed. That schedule is a hyperparameter, i.e.
LEARNED, not declared." The obvious cheap strength signal is the decisive-game fraction, which the
loop already logs every generation (`dec 659/2400`). It does not work, and the reason is worth
having on record before someone builds it.

## The measurement

P1's resumed run, 29 generations, first half against last half. Both halves are 33,600 games, and
the champion ACCEPTED 6 times across the window, so it is genuinely improving over the interval.

| | decided / games | fraction |
|---|---|---|
| first half | 10868 / 33600 | **0.3235 ± 0.0050** |
| last half | 9440 / 33600 | **0.2810 ± 0.0048** |
| difference | | **−0.0425 ± 0.0069 — RESOLVED** |

The interval is nowhere near zero. This is not a tie being read as a trend.

## Why it is not a paradox

Datagen is SELF-PLAY: the champion plays itself. A stronger, more consistent net agrees with itself
more often, so the games it produces are more drawn, not less. This is the same draw-saturation that
makes the P2 search gate powerless (measured 87.7% draws there) appearing on the P1 side.

**The cross-run comparison points the other way and is confounded.** The resumed run starts from a
champion scoring 0.798 vs origin and opens at 0.325 decisive, against 0.249 for the from-scratch run
— which looks like "stronger play is more decisive". But the two runs differ in three things at
once: champion strength, an empty vs warm replay pool, and a horizon reset to 10. The WITHIN-run
comparison changes only the champion and resolves in the opposite direction, so it is the one to
believe.

## What this rules out

**Do not couple the horizon to the decisive fraction.** It moves inversely to strength within a run,
so a schedule keyed to it would NARROW the horizon exactly as play improved — the opposite of the
intent. That design looked obvious and cheap; it is wrong, and it cost one query to find out.

## The harder consequence, which is not about the horizon

The loop's LABEL SUPPLY SHRINKS AS IT GETS STRONGER. Decided games per generation fell from ~776 to
~674, about 13%, over 29 generations. Every usable training label comes from a decided game, so the
loop earns less signal per generation the better it gets, at constant cost.

That is a headwind the bootstrap has to survive, and it argues against the CURRENT schedule too:
`horizon = 10 + (g-1)*5` widens on the assumption that improving play makes distant labels
informative, while the measured trend is fewer decided games to draw any labels from at all. The two
assumptions are not compatible and only one of them has been measured.

## What a real strength signal would have to be

Not decisiveness. The origin control IS a real strength measurement, but it costs ~13 min at 1000
pairs (96% of wall clock before it was cut to 400 pairs every 40 generations), so it is far too
sparse to drive a per-generation schedule. Finding a cheap monotone proxy is the open problem; this
file exists to record that the first obvious candidate is refuted, with the number that refutes it.

**Not claimed:** that the current schedule is harmful. That remains unmeasured — see
`STATE.md` on the horizon saturating inert at generation 31, where the accept-rate comparison across
the saturation point was NOT resolved and accept rate is not a strength proxy anyway.
