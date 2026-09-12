# The promotion confirmation stage is ARMED but has never fired — status, not a success claim

**2026-09-12 04:14.** At 01:04 I added a confirmation stage to `auto_promote.sh`: a PROMOTE verdict
must survive a second match at 896 pairs on seed 911911 before it lands. Three hours later, the
honest status is that **it has not been exercised**.

```
post-fix readings on prodk0127 (the only arm auto_promote sees):
  01:40  gen  1317   hold  0.518 +/- 0.029
  02:18  gen  8898   hold  0.510 +/- 0.029
  02:56  gen 14330   hold  0.501 +/- 0.027
  03:35  gen 19084   hold  0.520 +/- 0.029

readings clearing the OLD single-read bar (rate - ci95 >= 0.5):  0 of 4
'CONFIRMED' lines in auto_promote.out:      0
'NOT CONFIRMED' lines in auto_promote.out:  0
```

**It is reachable** — auto_promote sees exactly one arm (`prodk0127.net`; the `learn_cand` experiment
arms are invisible to it by design, since it matches `*/release/learn`), so `n -eq 1` holds and the
PROMOTE branch is live. Nothing has simply hit the bar.

## Why this needs saying

The fix was justified by a real failure: the 23:04 promotion at 0.539 resolved to 0.501 ± 0.021 and
0.503 ± 0.014 at higher power (`promo_g39836_RESULT.md`), and the promoter takes ~7 looks a day at
p≈0.083 each. That justification stands. But "I fixed the spurious-promotion problem" and "the fix
has demonstrably prevented one" are different claims, and only the first is supported.

**First firing will be the first real test**, exactly as `tune_hybrid`'s confirm gate was — and that
one turned out to be broken on its first firing after months of looking fine.

## Two readings that were NOT blocked by this fix, recorded so they are not miscredited

```
01:08  lrB.net       gen 1392  PASSES but 2 arms training -- NOT promoting  0.593 +/- 0.029
01:44  lrs_00005.net gen  723  PASSES but 4 arms training -- NOT promoting  0.625 +/- 0.029
```

Both were stopped by the **pre-existing** `n -eq 1` guard, not by the confirmation stage. They are
also worth noting on their own: 0.593 and 0.625 are far above the 0.539 that fooled the promoter, so
the reading distribution has a heavier tail than the 23:04 incident alone suggested.

## Method note

My first pass at this filtered the log with `awk '$1>="01:04"'`. The log carries `HH:MM` with no
date, so that comparison is lexicographic across days and pulled in `13:47` and `15:01` from the
previous afternoon, inflating the count to 6. Anchoring on `prodk0127` instead — an arm that did not
exist before 01:27 today — makes the filter unambiguous.
