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
