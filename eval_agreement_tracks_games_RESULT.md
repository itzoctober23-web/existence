# The first proxy in this project to reproduce a game-measured ranking — eval-agreement, ρ = +1.000 on n=4

**2026-09-12 01:15.** `proxies_RESULT.md`'s standing headline is *"Every cheap proxy for strength has
failed. Only games measure strength here."* That prior was tested against arms whose game results are
already published, and on this set it does not hold.

## The test: arms with a KNOWN, game-measured answer

`low_sweep2_RESULT.md` ran three 2000-generation arms from one shared start, `netmatch` vs that start:

```
arm          lr        game score vs start     verdict
low_00001    0.0001    0.546 ± 0.031           clears
low_00002    0.0002    0.535 ± 0.028           clears  (this one SHIPPED)
low_00005    0.0005    0.412 ± 0.026           LOSES to its own start, interval clear of 0.5
```

## The measurement: static-vs-deep agreement, reference is an ANCESTOR

```
net                agreement    game vs start
low_00001.net        0.804        0.546
low_00002.net        0.779        0.535
low_start.net        0.764        (baseline, 0.500 by definition)
low_00005.net        0.732        0.412
origin(random)      -0.005        (floor)

by agreement:  low_00001 > low_00002 > low_start > low_00005
by games:      low_00001 > low_00002 > low_start > low_00005
Spearman ρ = +1.000       P(exact match by chance) = 1/4! = 0.042
```

**The reference choice is the crux and was made on descent, not convenience.**
`p1_champion_prev_pre_lr002.net` (01:24) predates `low_start` (07:02), so it is an ANCESTOR of all
four nets and cannot favour any arm. `prodk1658` was EXCLUDED despite being the obvious choice: it
dates from 18:58, downstream of `low_00002`'s promotion to champion at 07:58, so it would have agreed
with that arm by descent.

## The discriminating case

Rank agreement on three clustered arms could be luck. The informative point is **`low_00005`, the only
arm that LOST**, and both instruments put it below the shared start:

```
agreement   0.732  <  0.764 (start)
games       0.412  <  0.500 (start)   -- "loses to its own start", interval clear of 0.5
```

A proxy that merely tracked "amount of training" would rank `low_00005` with the others — it had the
same 2000 generations. It is ranked last because it is worse, which is the thing being measured.

## What this does NOT establish

* **n = 4.** An exact match has p = 0.042 by chance. That is suggestive, not decisive, and one
  additional set could overturn it.
* **No error bars on the agreement values.** The range is 0.732–0.804; adjacent pairs (0.804 vs 0.779)
  may not be separated at all. Only the start-vs-`low_00005` gap is supported by an independent
  measurement of the same sign.
* **One reference, one position set, one depth.** The between-run result used two references and they
  agreed; this used one, because only one ancestor predating all arms was available.
* **It does NOT overturn `proxies_RESULT.md`.** That file tested different proxies (training loss:
  r = +0.379, CI [−0.249, +0.783] — not significant). This is a proxy it did not test, on a set it did
  not use. The honest claim is narrow: **on these four nets, eval-agreement ordered them exactly as
  games did, including the sign of the arm that lost.**

## Why it matters for tonight's other results

`between_vs_within_RESULT.md` used this instrument to show agreement rising +0.149 across the champion
lineage while flat within a run. That result is only interesting if agreement tracks strength. This is
the first direct evidence that it does — on a small sample, with the caveats above, and it should be
re-tested the next time a set of arms gets a game-measured verdict.

---

## Limit closed, and it produced a USAGE RULE: the ORDER is robust, the VALUES are not portable

The limits above flagged *"no error bars — adjacent pairs may not be separated."* Re-ran the same four
nets and the same ancestor reference on three different position samples.

```
                seed 20260911   seed 424242   seed 911911
low_00001           0.804          0.764         0.724
low_00002           0.779          0.754         0.719
low_start           0.764          0.733         0.719   <- ties low_00002 here
low_00005           0.732          0.713         0.704

ordering vs the game order (00001 > 00002 > start > 00005):  MATCHES in 3 of 3
the discriminating sign (start > 00005):  +0.032, +0.020, +0.015  -- correct in 3 of 3
```

**The ranking is robust. The values are not.** A single net swings by up to **0.080** across samples —
larger than the 0.025 gap between adjacent arms — and the whole spread compresses from 0.072 to 0.020
between the best and worst sample. The seed-to-seed movement is **common mode**: every net moves
together, which is why the order survives while the numbers do not.

### The usage rule this establishes

> **`static_deep_residual --ref` values are comparable WITHIN one position sample and meaningless
> ACROSS samples.** Any comparison must hold `n_pos`, `depth` and `seed` fixed for every net in it.

**Tonight's other results satisfy this, checked rather than assumed:**

* `between_vs_within_RESULT.md` — the r9 series and the champion lineage were both measured at
  `500/400 positions, depth 4, seed 20260911`. Same sample, so the FLAT-vs-RISING contrast is a
  within-sample comparison and stands.
* `p1_kill_conjunct2_RESULT.md` — both reference runs used `400 positions, depth 4, seed 20260911`;
  only the reference changed.

Had either mixed seeds, a 0.080 common-mode swing could have manufactured or erased the contrast — the
r9 slope it was testing is +0.003 per 1000 generations, twenty times smaller than that swing.

### What this does to the headline claim

It **strengthens** it on rank and **weakens** it on magnitude. Spearman ρ = +1.000 now holds on three
independent position samples rather than one, so the exact-match probability of 0.042 is no longer the
whole argument. But no statement about HOW MUCH better one net is may be read off these numbers, and
the tie at seed 911911 (low_00002 = low_start = 0.719) shows adjacent arms genuinely are not always
separated — exactly what the original limit warned.

---

## OUT-OF-SAMPLE test on a second arm set: 2 of 3 pairwise signs. The headline must SHRINK.

The section above asked for re-testing "the next time a set of arms gets a game-measured verdict".
`blend_sweep_RESULT.md` is that set, and its game verdicts were read and stated as a PREDICTION before
the instrument was run.

```
predicted from games:   0.85 > 0.75 > start
                        (0.85 beats 0.75 head-to-head 0.541 ± 0.032; both beat the start)

measured agreement:     blend_085  0.808     blend_start 0.779     blend_075  0.776
                        => 0.85 > start > 0.75
second seed (_s1):      blend_085  0.801     blend_start 0.779     blend_075  0.775   -- same order
```

### Pairwise, against the game sign

```
0.85 vs 0.75    agreement +0.032   game margin +0.041 (0.541 ± 0.032)    MATCH
0.85 vs start   agreement +0.029   game margin +0.083 (0.583 ± 0.027)    MATCH
0.75 vs start   agreement −0.003   game margin +0.036 (0.536 ± 0.030)    MISMATCH
```

**2 of 3.** Both seeds agree with each other and both disagree with games on the same pair, so this is
not sampling luck — it is a real limit of the instrument.

### Where the limit is, stated precisely

The failure is on the **smallest game margin**, and the instrument's gap there is **0.003** — inside
its own measured noise (common-mode swing up to 0.080 across samples; adjacent arms tied outright at
one seed). The set where it scored 4/4 had far larger separations:

```
lr arms    game scores vs start   0.546 / 0.535 / 0.412     spread 0.134
blend arms game scores vs start   0.583 / 0.536             spread 0.047
```

**Revised claim, replacing the headline above:** eval-agreement reproduces the game ORDER when the
strength differences are LARGE, and cannot resolve differences of the size the blend arms differ by. It
is a coarse screen, not a substitute for the game gate — which is much closer to
`proxies_RESULT.md`'s standing position than my first reading suggested.

### What this does to tonight's other results

`between_vs_within_RESULT.md` compares a **+0.149** rise against a flat line. That is 4x the largest
gap the instrument just failed on and 50x the failing gap itself, so it sits well inside the regime
where the instrument was right 6 of 7 times. The conclusion stands, but it now rests on an instrument
with a known resolution floor of roughly **0.03**, not on one that ranks anything correctly.

**The method note worth keeping:** the first set was chosen because it was convenient — it was the
sweep I already had open. The second was chosen because it was a TEST. Only the second was informative
about the instrument's limits, and it took one run.
