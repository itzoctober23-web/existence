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
