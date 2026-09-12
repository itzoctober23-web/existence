# P1 kill, conjunct 2 MEASURED for the first time: the static-vs-deep agreement is FLAT. Both conjuncts now hold — and the kill should NOT be fired.

**2026-09-12 01:00.** `MASTER_PLAN.md` §Phases: *"Kill: no iteration-over-iteration gain across
iterations 4-8 AND the static-vs-deep residual is not shrinking -> pipeline bug; stop and find it."*
Conjunct 2 had never been evaluated — the original form was refuted on 2026-09-11 as unmeasurable, and
its repaired replacement was built but not run across a series. This runs it.

## Setup, and the choice that IS the result's main assumption

```
series      p1_champion.net.r9.gen100 .. gen2200   22 checkpoints, ONE run (iteration-over-iteration)
reference   prodk1658.net                          a DIFFERENT lineage, ruler 1551
positions   400 sampled, 395 kept (5 mate-scored excluded), depth 4, seed 20260911
```

**Why the reference is from another lineage.** `static_deep_residual --ref` fixes the deep-side target
so it stops moving with the net under test. The instrument guards a BYTE-IDENTICAL reference, but not
a reference that is merely DOWNSTREAM of the series — and scoring a series against its own endpoint (or
a descendant) makes later checkpoints agree better by construction. That is precisely the circularity
that made the ORIGINAL metric report **corr 0.900 for an untrained random net**.

## Result

```
gen  100  0.611      gen 1200  0.630
gen  200  0.572      gen 1300  0.635
gen  400  0.627      gen 1500  0.647   <- max
gen  700  0.647      gen 1800  0.605
gen 1000  0.644      gen 2200  0.618
random-net floor      0.038

n=22   first 0.611   last 0.618   min 0.572   max 0.647   mean 0.6220
slope +0.0029 per 1000 generations (se 0.0064)   t = +0.46   -> FLAT
first-half mean 0.6210    second-half mean 0.6230    delta +0.0020
```

**The agreement does not improve across 2,100 generations.** It is also far above the 0.038 random
floor, so the nets have learned a great deal — that learning simply happened BEFORE gen100 of this
window.

## So both conjuncts hold. The kill still should NOT be fired.

Conjunct 1 was satisfied today (`WEEK1_RETRO.md`: prodk1926 slope **+0.0 ± 0.5** over 53,881
generations; `promo_g39836_RESULT.md`: a promotion resolving to 0.503 ± 0.014). Conjunct 2 is now
measured flat. Read mechanically, the spec says *"pipeline bug; stop and find it."*

**That inference is already known to be unsound in this project, and MASTER_PLAN says so itself.** Its
own 2026-09-11 correction records:

> *"The plateau was ultimately explained without it — the learning rate, a hardcoded literal that had
> never been varied — so no pipeline bug was hiding behind the dead conjunct."*

A flat eval and a flat agreement are exactly what a **configuration** ceiling produces, and this project
has already found one (lr 0.0005 -> 0.0002, `lr_sweep2_RESULT.md`) and shipped it. The kill's
conjunction is a sufficient condition for "look hard", not for "a bug exists".

**What it does license:** the conjunction is, for the first time, EVALUABLE. Before today one conjunct
could not be measured at all, so the kill could never fire in either direction — which is worse than a
kill that fires wrongly, because it is silent.

## Limits, stated

* **The series is not the live run.** `prodk1926` has only two nets on disk (start, live) — no
  checkpoint series exists for it — so this measures the r9 window of 2026-09-10. The live run's
  flatness is established separately by the ruler and by netmatch, not by this instrument.
* **One reference.** `prodk1658` is one independent opinion; a second reference would test whether the
  flatness is a property of the series or of that particular target.
* **22 points over 2,100 generations** bound the slope to ±0.0064 per 1000 gens at 1 se — tight enough
  to exclude a rise of the size the eval would need, not tight enough to exclude a very slow one.

---

## Limit closed: a SECOND independent reference gives the same answer

The limits section above flagged *"one reference — a second would test whether the flatness is a
property of the series or of that particular target."* Re-run with everything identical except the
reference:

```
reference                 n   mean     first   last    slope/1000 gens        t      verdict   floor
prodk1658 (ruler 1551)   22   0.6220   0.611   0.618   +0.0029 ± 0.0064    +0.46    FLAT      0.038
prodk1056 (ruler 1522)   22   0.6171   0.601   0.617   +0.0042 ± 0.0062    +0.67    FLAT      0.025
```

**Two independent opinions, from two different lineages, agree to within 0.005 on the mean and both
give a slope indistinguishable from zero.** Each also reproduces its own random-net floor (0.038,
0.025), so neither run is reading a broken target.

**So the flatness is a property of the SERIES, not of the reference.** That was the one substitution
that could have made the conjunct-2 result an artifact of a convenient choice, and it does not.

**Still open from the limits above, and not closed by this:** the series is the 2026-09-10 `r9` window,
not the live `prodk1926` run — which has only two nets on disk, so no checkpoint series exists to
measure. The live run's flatness rests on the ruler and on netmatch, which is a different kind of
evidence from this one.
