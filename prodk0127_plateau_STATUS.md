# The fresh lineage reproduces the plateau: 1561 pooled, FLAT, 39 short of the day-7 line

**2026-09-12 03:07.** `prodk0127` is the production run keepalive started at 01:27 from the shipped
champion (`9545a35289e9`) at the shipped lr 0.0002. It has run 15,701 generations and the live ruler
has taken **20 independent readings** against SF-1320 @10k nodes, 120 games each.

```
POOLED absolute   1561    sd 36   se 8    95% CI [1545, 1577]
TREND            -2.89 Elo / 1000 generations   se 1.66   t = -1.74
best single reading   1616 +/- 74
worst single reading  1478
```

## The day-7 condition is still not met, and the newest reading does not change that

`WEEK1_RETRO.md` sets the stop condition as **1600 on the POOLED ruler WITH a rising trend**. Both
halves fail here: pooled is **39 Elo short**, and the trend is very slightly negative (t = -1.74,
i.e. not distinguishable from flat and certainly not rising).

**The reason to write this down is the newest reading.** At 03:03 the ruler printed
`+285 +/- 74`, an implied absolute of ~1605 — it *touches* 1600. Read alone it looks like the
target was reached. It is one sample from a distribution whose readings span **1478 to 1616** with
individual CIs of ±60–74; the pooled estimate over all 20 is 1561 ± 16. Quoting the single 1605
would be the `quote-the-noise-with-the-number` error, and the pooling directive in `WEEK1_RETRO`
exists precisely because individual ruler samples span 45–65 Elo of raw spread.

## Why it matters beyond the number

`WEEK1_RETRO` concluded that configuration levers paid and every structural lever returned null, on
three earlier lineages (`prodk1658`, `prodk1056`, `prodk1926`), all flat. **This is a fourth,
independent lineage reproducing the same picture** at the shipped configuration — 15,701 generations
of lr 0.0002 producing a flat 1561.

That is the standing argument for spending effort on the structural track rather than more
configuration sweeps, and it is why `structural_next_PREREG.md`'s Candidate A is being run tonight
rather than the brief's already-closed low-lr sweep.

## Status

Measured, not inferred: 20 readings from the live ruler's own log, pooled and regressed here. No
action taken on the production run — it continues at the shipped rate. Nothing shipped.
