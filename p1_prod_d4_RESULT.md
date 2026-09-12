# RESULT — P1 production, datagen depth 4, 300 generations (planned N reached 2026-09-12 11:34:34)

Prereg: p1_prod_d4_PREREG.md. Start net (control): prod_d4_09121047_start.net md5 bc0b165b28a79c9d6ed80b8dfd67c19f = p1_champion.net at launch. Binary bin/learn_prod md5 bb7ad37d93a9379a0469af0d7c08d39f. Ruler engine bin/engine md5 4227a46564bbe71539ecd410820b2bab, SF-1320 @10k nodes, our depth 4, 120 games per rung, one sample per rung. Final net prod_d4_09121047.net md5 d641d93267f5eb66579eae725e09a1b0.

```
10:52 prod_d4_09121047 gen 0 Elo vs this opponent: +199 +/- 62
10:59 prod_d4_09121047 gen 50 Elo vs this opponent: +165 +/- 62
11:06 prod_d4_09121047 gen 100 Elo vs this opponent: +236 +/- 67
11:12 prod_d4_09121047 gen 150 Elo vs this opponent: +165 +/- 62
11:18 prod_d4_09121047 gen 200 Elo vs this opponent: +228 +/- 64
11:24 prod_d4_09121047 gen 250 Elo vs this opponent: +245 +/- 69
11:34 prod_d4_09121047 gen 300 Elo vs this opponent: +223 +/- 66
```

Pooled per rung / trend (ruler_pool.py, filtered to this run):
```
prod_d4_09121047       0   1         1519    62   -              
prod_d4_09121047      50   1         1485    62   -              
prod_d4_09121047     100   1         1556    67   -              
prod_d4_09121047     150   1         1485    62   -              
prod_d4_09121047     200   1         1548    64   -              
prod_d4_09121047     250   1         1565    69   -              
prod_d4_09121047     300   1         1543    66   -              
Pooled per rung, inverse-variance. Individual samples remain in live_ruler.out (the ledger).
A difference between two pooled rungs smaller than their combined SE is NOT a trend.
```

Reading rule (fixed in the prereg): a rung differing from the control by less than the combined SE is not movement; FLAT unless the slope over rungs is significant at z >= 2. Verdict to be written by the session from the numbers above, not by this script.

## Verdict (session, 2026-09-12 11:36, from the numbers above and the prereg's reading rule)

**FLAT.** Pooled over the six rungs at gens 50–300: +208 ± 52 (95%) over SF-1320, i.e. ~1528 absolute, against the
start net's +199 ± 62 (~1519). Difference +9 with a combined SE of 41; weighted slope +23 ± 61 Elo per 100 generations
(z = +0.73, below the pre-registered z ≥ 2). Datagen depth 4 from the current champion, 300 generations at 8 games per
generation on one core, produced no measurable movement on the ruler — the same shape as every within-run reading in
`WEEK1_RETRO.md`. Not called as a decline either: every rung is inside its own interval of the control. Nothing here
changes the PARKED status; the Existence week-stop condition (pooled > 1600 with a significant trend) is not met by
this run.
