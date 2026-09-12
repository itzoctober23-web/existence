# The gen-39836 promotion does NOT replicate — the effect falls from +0.039 to +0.001 at 2x the pairs

**2026-09-11 23:47. Planned N complete** (448-pair extension finished). Read by the method
`promotion_was_sound_RESULT.md` established, applied to a MORE marginal case.

## What happened

```
23:04  auto_promote PROMOTED prodk1926.net gen 39836 on 0.539 +/- 0.030
       rule rate - ci95 >= 0.5  ->  0.509, clears by 0.009
       netmatch's own line: "A leads, but MARGINALLY ... needs more pairs"
                            "effect 0.039 vs between-seed sd 0.047 -> ~11 seeds for ~80% power"
                            "** ONE SEED CANNOT SETTLE THIS **"
```

## The re-test: p1_champion.net (9545a35289e9) vs p1_champion_prev_g39836.net (0097ddc3f5e6)

```
reading                pairs   score            interval          effect
original (promotion)     224   0.539 +/- 0.030  [0.509, 0.569]    +0.039
EXTENSION, same seed     448   0.501 +/- 0.021  [0.480, 0.522]    +0.001
replication, same seed   224   0.494 +/- 0.029  [0.465, 0.524]    -0.006
```

**At double the pairs the effect is +0.001.** The openings are seeded, so the 448-pair run contains the
original 224 as a prefix — which means the SECOND 224 pairs scored roughly 0.463. That is regression to
the mean, and the promotion was measured at the high tail of it.

## What this does NOT say

**It does not say the champion is worse, and netmatch refuses to let it.** Its own verdict on both
re-tests is *"UNRESOLVED — contains 0.5 but is NOT tight enough to call a null; more pairs, not a
verdict"*, with *"Re-run at ~953 pairs before calling it a tie."* So the honest statement is **no
detectable difference at 448 pairs**, not "the promotion was wrong".

**Therefore no rollback.** Reverting a champion on an unresolved reading would repeat the exact error
that produced the promotion — acting on an interval the instrument says cannot support it. The two nets
are indistinguishable, so the promotion is harmless; it simply was not an improvement.

## The contrast with the promotion that DID hold

`promotion_was_sound_RESULT.md` re-tested `d3c` the same two ways and it held: 0.557 -> **0.559** at 448
pairs, interval clear of 0.5. Same method, opposite outcome. The difference is where they started —
effect 0.057 there, 0.039 here, against a between-seed sd of 0.047. **A promotion whose effect is below
the between-seed sd is the one that does not survive extension**, and that is now measured rather than
assumed, from two cases.

## What corroborates the week's main finding

`WEEK1_RETRO.md`'s central measurement is that **every production run is FLAT on the absolute ruler —
all 166 Elo came from BETWEEN runs**. A within-run promotion that evaporates under extension is exactly
what that predicts. This is independent evidence for it from the PAIRED instrument, which is the one
the retro trusts.

## A defect in my own harness, caught and confirmed here

I launched the second arm as `MAS_PAIR_SEED=911911`. **netmatch does not read that variable** — the seed
is the 5th POSITIONAL argument — and the output above proves it: the arm labelled "INDEPENDENT SEED"
printed `seed 20260907`. So it is a same-seed REPLICATION, not a second seed, and is reported as such.
A true independent seed still needs `netmatch A B 224 4 911911`.

---

## RESOLVED at 953 pairs — INDISTINGUISHABLE. The promotion was a no-op, not a gain and not a loss.

`netmatch` asked for a specific power: *"Re-run at ~953 pairs before calling it a tie."* Run at exactly
that count, it changes its own verdict from UNRESOLVED to a result:

```
reading                pairs   score            interval          netmatch's verdict
original (promotion)     224   0.539 +/- 0.030  [0.509, 0.569]    "A leads, but MARGINALLY"
extension                448   0.501 +/- 0.021  [0.480, 0.522]    "UNRESOLVED ... more pairs"
RESOLVE                  953   0.503 +/- 0.014  [0.489, 0.517]    "INDISTINGUISHABLE, and
                                                                   precisely so"
```

**0.503 +/- 0.014.** The champion and the net it replaced are the same strength on this instrument, to
within +/-1.4 points. The 0.539 that triggered promotion was the high tail of a distribution centred at
0.503.

## What this settles, and what it does not

**Settled:** the promotion was not an improvement. It was also **not a regression** — the interval is
narrow and contains 0.5, which `netmatch` distinguishes explicitly from a wide one (*"a narrow interval
around 0.5 is a RESULT; a wide one is IGNORANCE"*). **So no rollback**: the two nets are equivalent, and
reverting would be as unjustified as promoting was.

**Not settled:** whether a different TRAINING seed would have produced a better net. netmatch still
prints *"effect 0.003 against a between-seed sd of 0.047 -> ~2080 seeds for ~80% power"*. This result is
about these two nets on this opening set, which is the question the promotion rule actually asks.

## The general lesson, now measured from two cases

`promotion_was_sound_RESULT.md` re-tested a promotion whose effect (0.057) EXCEEDED the between-seed sd
(0.047): it held, 0.557 -> 0.559. This one's effect (0.039) was BELOW that sd: it collapsed,
0.539 -> 0.503. **A promotion whose effect is under the between-seed sd is the one that does not
survive extension** — two cases, opposite outcomes, and the discriminator is visible at promotion time
because `netmatch` prints it.

`auto_promote.sh` does not read that line. Its rule is `rate - ci95 >= 0.5` on the WITHIN-run interval,
which cleared by 0.009 here. The fix is not to change the bar — that would suppress the case that held
— but to treat "effect < between-seed sd" as a flag for re-test before the promotion is trusted
downstream. Recorded as an observation; `auto_promote.sh` is not modified by this result.

## Corroborates the week's central finding

`WEEK1_RETRO.md` measures that **every production run is FLAT on the absolute ruler; all 166 Elo came
from BETWEEN runs.** A within-run promotion that resolves to 0.503 at high power is that same finding,
reached independently by the PAIRED instrument — the one the retro trusts precisely because the ruler
is noisy.

**Arm 2 (448 pairs, seed 911911, passed POSITIONALLY) is still running** and is reported separately. Its
header confirms the earlier harness defect is fixed: it prints `seed 911911`, where the `MAS_PAIR_SEED`
version printed `seed 20260907`.

---

## The mechanism, from `auto_promote`'s OWN log: repeated testing with no correction

The resolve says the promoted net and its predecessor are indistinguishable. `auto_promote.out`
shows independently WHY the rule fired anyway — seven readings of the same trainer against the same
champion across 42,200 generations:

```
19:59  gen  4510  hold      0.501 +/- 0.029
20:36  gen 11559  hold      0.481 +/- 0.030
21:13  gen 18773  hold      0.515 +/- 0.027
21:50  gen 25856  hold      0.491 +/- 0.031
22:28  gen 32723  hold      0.528 +/- 0.029
23:04  gen 39836  PROMOTED  0.539 +/- 0.030   <- the MAXIMUM of the series
23:41  gen 46710  hold      0.483 +/- 0.029

n=7   mean 0.5054   sd 0.0209   min 0.481   max 0.539
```

**The series mean (0.5054) is the 953-pair resolve (0.503 +/- 0.014).** The promotion was the maximum
of a random walk centred where the high-power measurement later landed.

### The false-positive rate, computed from the observed dispersion

The rule is `rate - ci95 >= 0.5`, and `auto_promote` reports `ci95 ~ 0.029`, so a reading must reach
**0.529**. Under H0 (equal nets) readings centre on 0.500 with the observed sd of 0.0209:

```
z = (0.529 - 0.500) / 0.0209 = 1.39   ->   p(single look) = 0.083

n =   7  (tonight)                P(>=1 spurious promotion) = 0.45
n =  48  (one day at EVERY=1800)  P(>=1 spurious promotion) = 0.98
n = 336  (one week)               P(>=1 spurious promotion) = 1.00
```

**`auto_promote` looks every 30 minutes and promotes on any single crossing.** Each look is an
independent test at p ~ 0.083; nothing corrects for the number of looks. Under the null a spurious
promotion is ~98% likely within a day. Exactly one occurred in seven looks, and it was the maximum.

### What this does and does not justify

**It does NOT justify raising the bar.** `promotion_was_sound_RESULT.md` records a promotion that
HELD on extension (0.557 -> 0.559); a higher bar would have suppressed it. The defect is not the
threshold, it is that the threshold is applied to an unbounded sequence of looks.

**It does NOT mean the champion is wrong.** The promoted net measures 0.503 +/- 0.014 against its
predecessor — equivalent. A noise-driven promotion between equivalent nets costs nothing directly; the
cost is that every measurement afterwards is referenced to a champion selected by its luckiest reading.

**`auto_promote.sh` is NOT modified by this result.** The observation is recorded because the
discriminator is already printed at promotion time — `netmatch` says *"effect 0.039 against a
between-seed sd of 0.047 -> ONE SEED CANNOT SETTLE THIS"* — and the rule does not read it. Two cases
now agree on which promotions survive extension: effect ABOVE the between-seed sd held, effect BELOW
it collapsed.

**Provenance checked, not assumed:** `p1_champion.net` md5 `9545a35289e9`, mtime 23:04:53 — unchanged
throughout the re-test, so the comparison measured the net that was promoted. The concurrent second
`netmatch` was `existence-auto-promote.service`'s own 23:41 check (`hold 0.483`), attributed by cgroup,
not a second arm of mine.
