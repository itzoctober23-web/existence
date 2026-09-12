# Candidate A REPLICATES on a second training seed — 0.4530, and the label channel is not the cause

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

## The verdict: the pre-registered FIRST branch

> B2 vs A2 pooled interval lies **wholly below 0.5** → the effect reproduces across training seeds.
> The claim upgrades from a BOUND to **established**: a node budget is weaker than fixed depth at
> matched generations.

**[0.4361, 0.4699] is wholly below 0.5. The effect reproduces.**

## How strong is "reproduces", honestly

The original was deliberately scoped as a BOUND because one training seed cannot separate an effect
from the project's measured between-seed sd of 0.047. Two seeds is better but it is still two, and
the naive test is worthless — with n=2 the t-interval is

```
mean 0.4457, t(1)=12.71  ->  [0.3525, 0.5389]     contains 0.5, and would contain almost anything
```

So the useful question is the other direction: **under the null — no effect, readings centred on 0.5
with sd 0.047 — how likely are two independent training seeds both landing this low?**

```
seed 1 at 0.4383   z = -1.31   P(<= | null) = 0.0948
seed 2 at 0.4530   z = -1.00   P(<= | null) = 0.1587
joint                                        0.0150
```

**~1.5% under the null.** That is real evidence and it is not overwhelming; it is the honest size of
what two training seeds can buy. The claim is established in the pre-registered sense and would be
strengthened further by a third seed, which costs ~37 minutes of two cores.

Note the replication's effect is **smaller** than the original (−32.8 vs −43.1 Elo). Both are
comfortably below parity and the difference between them (0.0147) is well inside the 0.047
between-seed sd, so this is regression toward the mean rather than a contradiction — and it is a
reminder that the original's point estimate was the high end of what this effect looks like.

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

## Context: the original's "arm A improved" caution is now better supported

The gate's context matches, arm against the shared start net, 224 pairs, seed 20260907:

```
                       original        replication
A (fixed depth) vs start   0.535 +/- 0.030     0.552 +/- 0.031
B (budget)      vs start   0.445 +/- 0.027     (pending)
```

`candidate_a_budget_loses_RESULT.md` explicitly **declined** to claim arm A had improved, because
|0.535 − 0.5| = 0.035 is only 0.74× the 0.047 between-seed sd — a number it noted "already fooled us
tonight". That caution was correct and is preserved here.

The replication's 0.552 is 1.11× that sd — still not decisive alone. But two independent training
seeds both landing above 0.5 is a different situation from one: under the same null, P(≥0.535) =
0.228 and P(≥0.552) = 0.134, joint **0.031**. So the fixed-depth arm improving on its start net moves
from *unresolved* to *moderately supported* — stated as a probability, not upgraded to a claim.

## What does not follow

* **Nothing ships.** These are diagnostic arms; the budget is not being adopted or rejected as a
  production setting on this basis, and no figure here is quoted as Elo gained.
* **Not a statement about node budgets in general.** It is one budget (5,269 nodes, the measured
  mean of depth-3 search) against fixed depth 3, at 2000 generations, at this width and learning
  rate.
* **The mechanism is still open.** Knowing labels are not the channel narrows it to two candidates;
  it does not identify which.
