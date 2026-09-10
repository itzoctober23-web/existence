# The horizon cap is OBSOLETE past bootstrap: uncapped beats capped-at-10 by +0.064 ± 0.034

**The measurement already existed and had no result file.** It sat in `hz_10.log` and
`hz_1000.log` from 2026-09-08, two lines at the bottom of two logs, and was re-asked as an open
question twice since — including by me on 2026-09-10, until I checked. This file exists so the
next person finds the answer instead of the experiment.

## The measurement

`horizon_ab2.sh`, 2026-09-08. Two arms, identical but for `--horizon-cap`, both resumed from
`champion_long.net`, 20 generations each, 2400 games/generation, depth 2, epochs 3, blend 0.75.
Verdict is the built-in final control against the FROZEN ORIGIN — deliberately, because
`horizon_ab.sh` v1 had used mean gate rate and settled nothing (two similar nets at depth 2 draw
almost everything, so the gate returns 0.500 ± 0.007 on pairs a fixed anchor separates easily).

| arm | control vs origin | Elo |
|---|---|---|
| `--horizon-cap 10` (narrow) | 637W-113D-146L, **0.774 ± 0.025** | +214 [+190..+240] |
| `--horizon-cap 1000` (uncapped, = today) | 722W-57D-117L, **0.838 ± 0.023** | +285 [+258..+317] |
| difference | **+0.064 ± 0.034 — RESOLVED** | **+72** |

Both arms accepted 20 of 20 generations, so this is not one arm stalling.

## What it settles

**The bootstrap finding has expired, exactly as its own explanation predicted.** MASTER_PLAN's P1
log measured that at ITERATION ZERO, training on all decided positions made the eval WORSE (sign
accuracy 0.452 → 0.441) while ≤10 plies made it BETTER (→ 0.543): "when both players are near-random,
the game result is nearly independent of a position 40 plies earlier". It then predicted the fix:
"the label becomes informative further back as play improves".

Twenty generations from a real champion, it has. Capping at 10 now throws away usable labels and
loses by 72 Elo against the origin.

## Which corrects something I wrote earlier the same day

`STATE.md` records that `horizon = 10 + (g-1)*5` crosses `MAX_PLIES` (160) at **generation 31**, after
which the filter admits every position and the "widening schedule" stops being a schedule. That
arithmetic is right and I framed it as a possible defect — the loop sitting in the configuration
measured as harmful at bootstrap.

**That framing was wrong.** Uncapped is the BETTER configuration past bootstrap, so saturating into
it is the loop arriving at the right answer by an accident of arithmetic. The schedule is still
badly built — it is declared rather than learned, it cannot survive a resume, and its printed value
climbs to 500+ against a ceiling of 160 — but what it saturates INTO is correct, and no strength is
being lost to it.

## What is still open, narrowly

Whether a horizon between 10 and 160 beats no horizon at all. Both measured points are extremes:
10 (loses) and ≥160 (wins, = no filtering). Nothing has tested, say, 45 or 80. The gap is worth one
run *only if* something cheap suggests it; on current evidence the schedule's job is done by
`min(..., MAX_PLIES)` and the honest simplification is to delete the ramp rather than tune it.

**Do NOT re-run the 10-vs-uncapped comparison.** It is resolved, at +0.064 ± 0.034, and re-running a
resolved question is how three attempts became four.
