# blend 0.85 clears the promotion rule at FULL LENGTH — on one seed, by less than the between-seed spread

**2026-09-11 09:5x.** First full-length test of the blend candidate. Two arms, **2000 generations
each, exactly matched**, shared start from the current champion `044e754f57ba`, seed 20260917,
lr 0.0002 (the shipped rate), everything identical but `--blend`.

## The three verdicts

    comparison                      score           lower bound
    0.85 vs 0.75   (paired, direct) 0.541 +/- 0.032    0.509   CLEARS
    0.85 vs shared start            0.583 +/- 0.027    0.556   clears
    0.75 vs shared start            0.536 +/- 0.030    0.506   clears

**The head-to-head clears the promotion rule** (`rate - ci95 >= 0.500`).

## The decomposition, which is the part worth keeping

Both arms beat the shared start. **2000 generations from this champion is worth something on its
own**, and an experiment that only measured "0.85 vs start" would have credited all of that to the
blend. Against the same start:

    0.85 gained 0.583,  0.75 gained 0.536
    difference +0.047 +/- 0.040  =  1.2 sigma -- NOT resolved

The direct paired comparison (0.541 +/- 0.032) and the indirect one (+0.047) agree on MAGNITUDE --
roughly +0.04 to +0.05 -- but only the paired one resolves it. That is exactly why the promotion
rule is written against a head-to-head: the same effect is 1.2 sigma one way and clears the bar the
other, purely because pairing removes the variance the shared-start route carries twice.

## Why this is NOT shipped

The margin above 0.5 in the head-to-head is **0.041**. The measured **between-seed sd on this
project is 0.047** — larger than the margin. A single seed clearing by less than the seed-to-seed
spread is the lottery this loop has been burned by, and it is the same standard used hours earlier
to REFUSE shipping lr 0.0001 on a 0.011 gap. A rule applied only when its answer is convenient is
decoration.

`blend_sweep_seed2.sh` (seed 20260918) is running the identical experiment. Both seeds must point
the same way, and the pool must clear, before the default moves.

## What was controlled, and how it was verified rather than assumed

* **The flag binds.** `main.rs:569` echoes `blend={blend}`, and each arm's OWN log header was read
  back: `blend=0.75` and `blend=0.85`. A sweep whose knob is silently ignored runs every arm on the
  default and produces a confident null — that has happened on this box and forced a retraction.
* **The control IS the shipped setting.** The binary's default was probed separately and reported
  `blend=0.75`.
* **The arms are matched.** 2000/2000 generations, checked explicitly. The first low-lr sweep was
  invalidated by arms unmatched at ~650 generations.
* **The verdict instrument is the paired netmatch, never the blind candidate-vs-champion rate.**
  `main.rs:490-509` records a blend sweep where 0.75/0.85/0.95/1.00 all overlap — an apparent null.
  `blend_RESULT.md` proves that instrument COMPRESSES this contrast ~3x (+0.036 against +0.112 on
  the SAME arms) and concludes "a NULL measured on the blind metric is worthless". Reusing it here
  would have reproduced a fake null.

## Prior evidence this replicates

`blend_RESULT.md` found 0.85 > 0.75 on **five independent training seeds**, with a NON-MONOTONIC
response — 0.75 -> 0.85 gains, 0.85 -> 1.00 gives it back — so it is not a "more is better"
artefact. Every one of those arms was a **TWENTY-generation** run. This is the first full-length
measurement, and the lr sweep is the standing proof that 20-generation and 2000-generation answers
differ in magnitude.

## Open

* Seed 2 running.
* `blend_sweep_ruler.sh` will place both arms on the ABSOLUTE ruler, which is what the 1600 stop
  condition is written against. The head-to-head says which is stronger; it cannot say how far
  either is from 1600.
* Cosmetic defect, recorded not fixed: the sweep log prints `blend 0.085` because the arm tag is
  already `085`. The value is right and the label is wrong; it was not corrected while the script
  was running.

## CROSS-INSTRUMENT CHECK 10:2x — the ruler and netmatch agree to 0.4 sigma

The 0.75 arm's four ruler samples completed: **+161 +183 +161 +179**, pooling to
**1490 +/- 30** absolute. That number can be predicted independently from the paired netmatch:

    netmatch  0.75 arm vs shared start = 0.536   ->  +25 Elo over the start
    the shared start IS the champion, which pools to 1481 +/- 19
    predicted ruler reading                       ->  ~1506
    measured                                      ->   1490 +/- 30
    difference  -16 +/- 36  =  0.4 sigma

**Two instruments, neither calibrated against the other, agree.** One is a PAIRED relative match
between two nets; the other is an ABSOLUTE score against a fixed external SF-1320 rung. They share
no machinery beyond the engine itself, so this simultaneously validates the netmatch verdicts, the
champion's 1481 pooled figure, and the ruler pipeline — which matters because that pipeline was
REBUILT this morning after `blend_sweep_ruler.sh` was caught measuring 13-generation nets as though
they were the 2000-generation arms.

**Precision, stated so the check is not over-read:** the ruler pooled at +/-30 cannot resolve a
+25 Elo contrast. This is an ALTITUDE check, not a verdict on 0.85 vs 0.75 — netmatch resolved that
at 0.541 +/- 0.032. Agreement here means the altitudes are trustworthy, not that the ruler could
have found the contrast on its own.

## THE RULER HALF, COMPLETE — 8 samples, 4 per arm, frozen copies

    arm                samples (rel to SF-1320)     POOLED absolute
    blend 0.75         +161 +183 +161 +179          1490 +/- 30
    blend 0.85         +236 +223 +228 +211          1544 +/- 32

    ruler contrast     +54 +/- 44   (1.2 sigma)  -- NOT resolved
    netmatch contrast  0.541 +/- 0.032 = +29 Elo -- RESOLVED (lower bound 0.509)
    the two are 0.6 sigma apart: consistent in SIGN and in MAGNITUDE

**This is the division of labour working as designed.** The paired netmatch resolves the contrast
that the absolute ruler cannot see; the ruler places both arms on a scale the netmatch cannot
reach. Neither is a substitute for the other, and the ruler's 1.2 sigma is NOT a failure to
replicate — it is the expected result of asking a +/-44 instrument about a +29 effect.

### Against his stop condition

    shipped champion   1481 +/- 19    to 1600:  +119   (6.3 SE)
    blend 0.75 arm     1490 +/- 30    to 1600:  +110   (3.7 SE)
    blend 0.85 arm     1544 +/- 32    to 1600:   +56   (1.8 SE)

The 0.85 arm is the closest anything has come to 1600. The gap falls from 119 to 56 — but 1.8 SE
is still a gap, not an arrival, and this is ONE seed.

### What this does NOT license

**Not a ship.** The head-to-head margin above 0.5 is 0.041 and the measured between-seed sd is
0.047, so seed 20260918 is running the identical experiment. Both seeds must point the same way and
the pool must clear before the default moves.

**Not a claim that blend alone moved it +54.** Both arms trained 2000 generations from the same
champion, and the 0.75 arm ALSO improved (0.536 +/- 0.030 against the shared start). The ruler
difference is blend-plus-training against training-alone; the netmatch head-to-head is the clean
attribution, and it says +29 Elo.

## CROSS-SEED VERDICT — both seeds clear, and the pool clears the between-seed spread

Seed 20260918 ran the identical experiment: 2000 generations per arm, exactly matched, shared
start from the same champion, only `--blend` differing, binding re-verified from each arm's own
log header.

    seed 20260917   0.541 +/- 0.032   lower bound 0.509   clears 0.500
    seed 20260918   0.606 +/- 0.026   lower bound 0.580   clears 0.500
    POOLED          0.574 +/- 0.021   lower bound 0.553   clears 0.500

**Both seeds point the same way and the pool clears.** The margin above 0.5 is now **0.074**
against a measured between-seed sd of **0.047** — the quantity that made a single-seed result
insufficient. Seed 1 alone had a margin of 0.041, SMALLER than that spread, which is why it was
refused. Two seeds move the margin above it.

### The staleness guard earned its place

`blend_sweep.sh` writes FIXED filenames, so the unsuffixed logs held SEED 1's numbers until seed
2's netmatch overwrote them — verified at 10:33, both slots reading an identical
`0.541 +/- 0.032`. A reader that simply opened both files would have compared seed 1 against
ITSELF, found perfect agreement, and printed the sentence that licenses a default change.
`blend_crossseed.py` refuses on an mtime check and reported "STALE" until 10:43, when the real
seed-2 verdict landed. The number above is `blend_085_vs_075.log` at mtime 10:43:14 against an
archive at 09:55:27.

### Against the normal promotion rule

    blend_085 (seed 1) vs the shared start, which IS the champion:
        0.583 +/- 0.027   lower bound 0.556   CLEARS (rule needs >= 0.500)

So both halves hold: the CONTRAST is established across seeds, and the candidate clears the normal
rule against the incumbent. Seed 2's own vs-champion match is still running and will add a second
independent measurement of the same thing.

### What is still not claimed

No Elo figure is quoted for an unshipped change. The head-to-head score converts to roughly +52 Elo
at the pooled 0.574, but that is a paired self-play number, not a rating — and the absolute ruler,
which is the instrument the 1600 stop condition is written against, puts the 0.85 arm at
**1544 +/- 32** against the 0.75 arm's **1490 +/- 30**, a contrast of `+54 +/- 44` that it cannot
itself resolve.

## SHIPPED — default blend 0.85, and the net promoted

**Both halves of the change are now live.**

### The configuration (committed earlier this tick)

`main.rs` default blend `0.75 -> 0.85`, on the cross-seed contrast:

    seed 20260917   0.541 +/- 0.032   lower bound 0.509
    seed 20260918   0.606 +/- 0.026   lower bound 0.580
    POOLED          0.574 +/- 0.021   lower bound 0.553

### The net

Both seeds' 0.85 arms independently clear the normal promotion rule against the incumbent:

    seed 20260917   0.583 +/- 0.027   lower bound 0.556   CLEARS
    seed 20260918   0.558 +/- 0.029   lower bound 0.529   CLEARS
    POOLED          0.571 +/- 0.020   lower bound 0.551   CLEARS

    champion  044e754f57ba  ->  abbbd0d0c5e0  (blend_085_s1.net, 2000 generations)
    previous kept as p1_champion_prev_pre_blend085.net

**The choice between the two nets is a TIEBREAK, not a measurement.** 0.583 against 0.558 is a
difference of 0.025 with a combined SE of 0.040 — **0.6 sigma, not separable**. Seed 1's net was
taken because it has the higher point estimate AND is the better characterised arm: it also carries
four absolute-ruler samples (**1544 +/- 32**), where seed 2's net has none. Either would have been
defensible; pretending the 0.025 meant something would not be.

### Checks run before the swap, not after

* `blend_start.net` and `p1_champion.net` verified byte-identical (`044e754f57ba`), so the 0.583
  measurement was against the CURRENT incumbent rather than a stale baseline.
* `auto_promote` verified NOT mid-match — swapping the champion file under a running comparison
  would misattribute its verdict.
* Previous champion copied aside before the overwrite, and the new champion verified byte-identical
  to the candidate afterwards.

### Where this leaves the stop condition

    champion (old)  1476 +/- 11   22,141 generations, slope -0.6 +/- 1.6 per 1000 gens (FLAT)
    champion (new)  1544 +/- 32   2,000 generations from that champion at blend 0.85
    target          1600          now +56 away (1.8 SE), from +119 (6.3 SE) this morning

No Elo is quoted for the change itself — it passed its gates. The 1544 is a pooled absolute-ruler
reading of the promoted net, not a claim about the size of the blend effect, which netmatch puts at
0.574 +/- 0.021 head to head.

## THE UNANTICIPATED HALF — blend 0.75 LOSES to its own start on seed 2

The full cross-seed table, all six matches:

    comparison              seed 20260917        seed 20260918
    0.85 vs 0.75            0.541 +/- 0.032      0.606 +/- 0.026     both clear
    0.85 vs shared start    0.583 +/- 0.027      0.558 +/- 0.029     both clear
    0.75 vs shared start    0.536 +/- 0.030      0.438 +/- 0.029     seed 2 does NOT clear

**On seed 2, 2000 generations at the OLD blend made the net WORSE than the champion it started
from.** That is not a small effect: the two 0.75 arms differ by `0.098 +/- 0.042` = **2.3 sigma**,
a real seed-to-seed swing. The two 0.85 arms differ by `0.025 +/- 0.040` = 0.6 sigma — consistent.

### So the ship rests on more than a mean

The argument for 0.85 was "it beats 0.75 on the pooled contrast". The six matches say something
stronger and different: **0.85 improves reliably, 0.75 does not improve reliably.** One arm of the
shipped configuration actively degraded a champion over 2000 generations, and only the seed
decided which.

### This is the SECOND time today the shipped setting lost to its own start

    lr sweep, this morning:  lr 0.0005 (then shipped) scored 0.412 against its own start
    blend sweep, now:        blend 0.75 (then shipped) scored 0.438 on seed 2

Both were the incumbent. Both were measured on matched full-length arms from a shared champion.
The pattern is worth naming: **a shipped default that has never been re-measured at full length can
be actively harmful, not merely suboptimal**, and the single-arm runs that shipped it originally
could not have seen that — an arm that degrades its start looks identical to one that stalls unless
you gate it against the start.

### What it does not change

The promotion decision is unaffected: `blend_085_s1.net` cleared against the incumbent on its own
measurement (0.583 +/- 0.027) and that measurement is unchanged. And the 0.438 is one arm on one
seed — it establishes that 0.75 CAN degrade, not that it usually does.
