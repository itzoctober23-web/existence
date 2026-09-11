# epochs 2 vs the shipped 3 — FLAT at full length, in both directions and on cost

2026-09-11. Two arms, 2000 generations each, exactly matched, shared start `abbbd0d0c5e0`,
seed 20260919. Verdict by paired `netmatch` at 224 pairs.

## Why ask, given epochs was already studied

`epochs_ab_RESULT.md` (2026-09-08) tested **3 → 10 → 30** and found monotonic degradation: training
loss fell every step (0.0417 → 0.0317 → 0.0253) while candidate quality moved the opposite way
(mcnemar_z median +0.359 → +0.141 → −0.445, sign rate 54% → 33%). Its conclusion — "fitting these
labels harder makes the net worse" — has stood.

**Nobody tested downward.** Every arm in that study was at or above the shipped 3, so its own trend
made a prediction it had not checked: if more is worse, less should be better. That is this
experiment, and it is not a re-derivation of the closed question.

## Result

| contrast | score | interval | rule (`rate − ci95 ≥ 0.5`) |
|---|---|---|---|
| **epochs 2 vs epochs 3** | **0.513 ± 0.026** | [0.487, 0.539] | **fails — spans 0.5** |
| epochs 2 vs shared start | 0.550 ± 0.031 | [0.519, 0.581] | clears |
| epochs 3 vs shared start | 0.511 ± 0.027 | [0.484, 0.538] | fails |

**Head to head is FLAT.** Against the pre-registration — "neither clears → epochs is flat between 2
and 3 at full length; record the bound and stop" — the bound is **±0.026 at 224 pairs**.

The two control readings look like they separate the arms, but they do not: `0.550 − 0.511 = 0.039`
against a combined `±0.041`, which is **0.95σ**. The head-to-head is the direct paired test and it is
the one that answers the question.

## The prior study's prediction is not confirmed

More epochs is worse (measured, 3→30). Fewer epochs is **not** correspondingly better. The response
is asymmetric: it degrades upward and is flat downward, so the optimum sits at or just below 3 with
no gain available by moving to 2.

## Not a throughput lever either

If quality is flat, cost would be the remaining argument. It is not there:

```
epochs 2   2000 generations   2,186s total    99.5% of epochs 3
epochs 3   2000 generations   2,196s total
```

Per-generation time is quantised at 1s with ~90% of generations in a single bucket, so the timer
cannot resolve a difference this small — but it also bounds it: whatever the saving is, it is below
the resolution at which it could matter. Per-generation cost is dominated by the 8 games and the
gate, not the training passes. (An earlier read of a SINGLE generation line showed `train 140` vs
`train 254` and looked like a large saving; pooled over 2000 generations those fields are 254 vs 248.
One sample was a lottery, and `train` is a position count, not a cost.)

## Status

**epochs is CLOSED as a lever.** Degrades above 3, flat below it, no cost advantage. Shipped default
stays 3. Do not buy a third arm for an effect this size.

## Footnote — the comparison nearly never ran

`epochs_sweep.sh` is a half-converted copy of the blend sweep. Its arms were correct (its own log
confirms `header reports epochs=2` and `default epochs is 3, so the control arm IS the shipped
setting`), but its verdict loop matched `blend_02.net` against `blend_03.net` — files that do not
exist — so all three pairs would have hit `skip: missing net` and 2 × 2000 generations would have
reported nothing, signed off with a reminder about blend 0.75/0.85 and `BLENDSWEEPDONE`.

It was found while still running, and bash reads a script by byte offset while executing it, so it
could not be edited in place and a rename-swap would not have reached the running shell. The
comparison was done by `epochs_compare.sh`, which waits on the CONDITION — no learn process holding
`--out epochs_*.net` — rather than a unit name or a sleep, because the nets were written at 11:42
while both arms were still at 88% CPU and a match started then could have read a net about to be
rewritten. `epochs_sweep.sh` itself was corrected by rename for future runs.
