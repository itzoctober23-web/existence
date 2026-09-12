# The budget arm ends BELOW its own start net in BOTH runs — but B-vs-A is UNRESOLVED, and my pre-registered rule was mis-specified

**2026-09-12 05:45.** Result of `candidate_a_replication_PREREG.md`, read against the decision rule
fixed before either arm was launched.

## The measurement

Arms A2 and B2 trained from the same frozen start (`cand_start.net`, md5 `9545a35289e9`, verified at
launch and re-verified after the 04:43 champion promotion — separate inode, unchanged), identical
flags, **training seed 777777** instead of the original's. Both reached 2000 generations with exit 0
in ~37 minutes, and both passed their own control against their random initialisation (0.864 and
0.887, intervals clear of 0.5).

Judged by the same gate, parameterised rather than copied so the two runs cannot drift apart:

```
                 replication (777777)        original
seed 20260907    0.441 +/- 0.030             0.439 +/- 0.028
seed 911911      0.454 +/- 0.029             0.433 +/- 0.029
seed 424242      0.464 +/- 0.029             0.443 +/- 0.029
pooled (672 pr)  0.4530 +/- 0.0169           0.4383 +/- 0.0166
95% CI           [0.4361, 0.4699]            [0.4218, 0.4549]
Elo              -32.8                       -43.1
```

## The verdict: the GATE says UNRESOLVED, and it is right

The gate's own verdict block, which applies a stricter standard than my pre-registration did:

```
B vs A over 3 seeds: [0.441, 0.454, 0.464]
mean 0.4530   observed between-seed sd 0.0115
|mean - 0.5| = 0.0470 < between-seed sd 0.047
=> UNRESOLVED. Reported as a BOUND, not a refutation: over 2000 generations the node
   budget did not beat fixed depth by more than the seed noise of this instrument.
```

It lands **exactly on the threshold** — `|0.453 − 0.5|` is 0.047 to every decimal that matters, and
only floating point (0.046999999999999986) tipped the comparison. A verdict decided at the 15th
decimal place is a verdict of *no margin*, and should be read as one.

### My pre-registered rule was mis-specified, and the gate caught it

`candidate_a_replication_PREREG.md` said the effect is established if "the B2 vs A2 pooled interval
lies wholly below 0.5". It does — [0.4361, 0.4699]. **That bar was wrong**, and the prereg's own
preamble says why, two paragraphs above the rule it then wrote:

> the spread between **RE-TRAINED** runs, not re-played matches. The three match seeds re-play the
> matches.

The pooled interval over three match seeds measures **match noise only** — re-playing the same two
nets. It cannot speak to whether a different *training* seed would land elsewhere, which is the
entire question a replication asks. I used a within-run interval to answer a between-run question,
having written down the reason not to. The gate's `|mean − 0.5|` vs 0.047 test is the correct
single-run standard and it returns UNRESOLVED.

**So the pre-registered "first branch" is not claimed.** What follows stands on different evidence.

## What DOES replicate: the budget arm goes backwards from its own start

The context matches — each arm against the shared start net, 224 pairs, seed 20260907 — are the
stronger measurement, and they were never the headline until now:

```
arm                     original   replication      Elo vs start
A (fixed depth) vs start   0.535       0.552        +24.4   +36.3     both ABOVE 0.5
B (node budget) vs start   0.445       0.422        -38.4   -54.6     both BELOW 0.5
```

**The budget arm finishes weaker than the net it started from, in both independent training runs.**
Two thousand generations of training that went backwards, twice. Against the null (centred 0.5,
between-seed sd 0.047):

```
A improving in both runs        joint P = 0.0306
B degrading in both runs        joint P = 0.0059
all four readings in direction  joint P = 0.00018
```

This is a cleaner claim than B-vs-A for a structural reason: B-vs-A compares two arms that both
moved, so seed noise enters twice and the difference is the small residue between them. Each arm
against the **shared, frozen** start compares a moved thing to a fixed thing — and `cand_start.net`
is byte-identical for all four readings (md5 `9545a35289e9`, re-verified after the 04:43 champion
promotion: separate inode, unchanged).

The direction is therefore replicated even though the head-to-head margin is not resolved. Those are
compatible: B is clearly below its start, A is above it, and the *gap between them* is the quantity
that sits on the noise floor.

## Combined with the channel isolation, this identifies what is NOT the cause

Run the same day, on a fixed corpus with only the label column swapped
(`budget_label_channel_RESULT.md`):

```
full budget, live self-play arm     0.4383 / 0.4530   RESOLVED BELOW      -43.1 / -32.8 Elo
label channel alone, fixed corpus   0.4960            NULL [0.4776,0.5144]  -2.8 Elo
```

with **47.8% of labels changed** and each arm verified to fit its own held-out label column best. The
label isolation is not underpowered — its interval would have resolved an effect of Candidate A's
size roughly 3× over — so the label contribution is bounded at about ±13 Elo.

**The budget genuinely costs strength, and the labels are not the channel it costs it through.**
`candidate_a_channel_FINDING.md` named three channels the budget moves at once; one is now
eliminated and the position distribution and decisive-game rate remain.

The limitation carried over from that file stands: removing self-play is what makes the isolation
possible, and it also removes the mechanism by which a per-generation label bias too small to see in
one pass could compound over 2000 generations.

## The original's "arm A improved" caution, revisited

`candidate_a_budget_loses_RESULT.md` explicitly **declined** to claim arm A had improved, because
|0.535 − 0.5| = 0.035 is only 0.74× the 0.047 between-seed sd — a number it noted "already fooled us
tonight". That caution was correct for one run and is preserved.

With two runs it moves to *moderately supported* (joint P = 0.031), not to a claim. Note this is the
weaker half of the pair: B degrading replicates at P = 0.0059, five times stronger than A improving.

## What does not follow

* **Nothing ships.** These are diagnostic arms; the budget is not being adopted or rejected as a
  production setting on this basis, and no figure here is quoted as Elo gained.
* **Not a statement about node budgets in general.** It is one budget (5,269 nodes, the measured
  mean of depth-3 search) against fixed depth 3, at 2000 generations, at this width and learning
  rate.
* **The mechanism is still open.** Knowing labels are not the channel narrows it to two candidates;
  it does not identify which.
