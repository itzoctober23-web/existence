# EXPERIMENTS — what was tried, and why it failed

The do-not-regress list. Every entry states the design, the result, and — where the
result did not hold up — what was wrong with the EXPERIMENT rather than the idea.

## 2026-09-08 — METHODOLOGICAL: my arms have been n=1, and it shows

**The problem, stated against my own data.** I ran single-seed arms all day and drew
causal conclusions from them. Two runs with IDENTICAL `--epochs 3` disagree:

| run | flags that differ | outcome |
|---|---|---|
| `cap40` | threads 4, arch-every 5 | NO collapse; control 0.694 +/- 0.044 at gen 20 |
| `long`  | threads 3, arch-every 6 | collapse from gen 13; rate 0.398, McNemar z -13.31 |

`--threads` repartitions the RNG stream across workers, so the two saw different games
from the same seed. That is enough to make them different draws, not a controlled
contrast. The `--epochs 1` arm then degraded LESS than `long` and MORE than `cap40`,
which is consistent with epochs mattering and equally consistent with it mattering not
at all.

**What this invalidates.** The commit "Decouple the trainer's step budget from datagen
volume" argues from the `long` run's collapse that raising volume 125x broke the
trainer's step budget. That mechanism is plausible and the arithmetic is real (~200 ->
~110,000 samples per generation at fixed epochs). But the EVIDENCE offered for it is one
run, and another run at the same setting did not collapse. `--steps-per-gen` remains
worth testing; it is not yet supported. Default stays 0.

**The rule, which was already written down and which I did not follow:** >= 3 seeds per
arm and >= 2 checkpoints before believing a difference. Eval power is n = 2/e^2 per arm,
so resolving a 0.05 effect needs ~800 pairs, not 320.

**Design for the re-run, once the box is free** (a 58.6M-row 4PC training holds it):
- 3 seeds x {epochs 3, steps-per-gen 20000}, all other flags IDENTICAL including
  `--threads`, since threads changes the data.
- Report the POOLED control across seeds with its interval, not the best arm.
- Pre-register the prediction before looking: if step budget is the mechanism, the
  fixed-budget arm should hold its McNemar z above zero where the epochs arm goes
  negative. If both go negative, the mechanism is wrong and the cause is elsewhere.

## 2026-09-08 — the horizon filter should be REMOVED, not tuned (at blend 0.75)

Full sweep against the trained champion at blend 0.75, 10 replicates per arm:

| horizon | samples | mean | 95% CI |
|---|---|---|---|
| 10 | 8,854 | 0.5352 | [0.5265, 0.5438] |
| 20 | 16,830 | 0.5539 | [0.5445, 0.5633] |
| 40 | 32,229 | 0.5988 | [0.5846, 0.6130] |
| 80 | 53,775 | 0.6074 | [0.5930, 0.6219] |
| 160 | 58,734 | 0.6215 | [0.6052, 0.6378] |
| 1000 | **58,734** | 0.6242 | [0.6057, 0.6428] |

h160 and h1000 have the SAME sample count, so no decided position lies beyond 160 plies and
those two arms are the same dataset — the filter is inert past 160 and the curve has saturated,
not peaked.

**So the horizon filter is not a knob to tune, it is a restriction to remove.** Monotone
improvement all the way to "use every decided position", 0.5352 -> 0.6242.

The arc of this constant today, which is worth keeping as a caution about one-dimensional
sweeps:
1. shipped cap 40 (n=1, uncapped looked catastrophic)
2. swept at blend 0, found a peak at 20, shipped 20
3. swept at blend 0 against a TRAINED champion, found the whole axis below 0.5 and declared
   the schedule "backwards" and the axis "exhausted"
4. shipped blend 0.75 for unrelated reasons
5. re-swept: the optimum inverted, and the correct setting is no cap at all

Every step was measured. Steps 2 and 3 were measured under a blend that made the answer
meaningless, and nothing in the measurement itself could reveal that — only changing the OTHER
constant did.

## 2026-09-08 — horizon at blend 0.75: monotone up, plateau from ~40

Completed sweep against the trained champion, blend 0.75, 10 replicates per arm:

| horizon | samples | mean | 95% CI |
|---|---|---|---|
| 10 | 8,854 | 0.5352 | [0.5265, 0.5438] |
| 20 | 16,830 | 0.5539 | [0.5445, 0.5633] |
| 40 | 32,229 | 0.5988 | [0.5846, 0.6130] |
| 80 | 53,775 | 0.6074 | [0.5930, 0.6219] |

h80 - h40 is 0.0086 +/- 0.0204: not significant. The curve rises steeply to 40 and then flattens.

Put beside the blend-0 sweep, the full picture is that these two constants define a plane and
the loop was sitting in its worst corner:

| | blend 0 | blend 0.75 |
|---|---|---|
| narrow (h10) | 0.4867 | 0.5352 |
| wide (h80) | 0.3977 | **0.6074** |

The shipped configuration was blend 0, horizon 20 -> 0.4648. The measured best corner is
blend 0.75, horizon 40-80 -> ~0.60. Every arm at blend 0 is below 0.5 (training makes the
champion worse); every arm at blend 0.75 is above it.

That also explains why tuning the horizon alone looked hopeless this morning: at blend 0 the
whole axis tops out at "no change", so the knob genuinely had no good setting. It had no good
setting because the OTHER knob was wrong.

h160 and h1000 running to decide whether a cap should exist at all -- if the plateau holds,
the schedule needs no cap and `--horizon-cap` becomes a safety rail rather than a tuning knob.

## 2026-09-08 — RETRACTION: the horizon schedule is NOT backwards. It was disabled by blend=0.

The horizon optimum REVERSES with the blend. Same trained champion, same data, 10 replicates:

| horizon | blend 0.00 | blend 0.75 |
|---|---|---|
| 10 | 0.4867 | 0.5352 |
| 20 | 0.4648 | **0.5539** |
| 40 | 0.4203 | **0.5988** |
| 80 | 0.3977 | (pending) |

At blend 0 every arm is below 0.5 and NARROWER is better. At blend 0.75 every arm is above 0.5
and WIDER is better, monotonically, in the opposite direction.

**This retracts "the horizon schedule is backwards", recorded a few hours ago.** That entry
argued the schedule's widening was "an active harm that grows with generation" and that the cap
was "treating a symptom". The measurement behind it was real; the conclusion drawn from it was
scoped to blend = 0 and I did not say so, because I did not yet know the blend mattered.

The mechanism is now clear and the loop's original design was right:
- with an OUTCOME label, a position 40 plies from the end is labelled by a result that had
  little to do with it. Noise grows with distance, so narrow wins.
- with a SEARCH-SCORE label, distance from the terminal is nearly irrelevant — a depth-2 search
  is about as informative at ply 40 as at ply 10. The extra positions are extra signal.

So `horizon = 10 + 5*(g-1)` widening with generation is CORRECT, and the comment justifying it
("the label becomes informative further back as play improves") was right for a reason it did
not state: it becomes informative once the label is a search score.

CONSEQUENCE: the shipped horizon default of 20 is measured WRONG under the shipped blend of
0.75 — h40 beats it by 0.045, roughly 6 SE. Waiting on h80 before changing it, since the curve
is still rising and I have already shipped two horizon defaults today on incomplete sweeps.

Also invalidates today's datagen-depth runs: both were at horizon 10, now known to be well
below the optimum, and at 1500 games where sample count is itself limiting.

## 2026-09-08 — operator fairness: CONFIRMED live, after three layers of the same bug

The search track's operator draw is now uniform. 249 proposals on a freshly built binary:

| operator | count | share |
|---|---|---|
| ReplaceConst | 41 | 16.5% |
| Dup | 41 | 16.5% |
| WrapIf | 33 | 13.3% |
| Delete | 32 | 12.9% |
| SwapSiblings | 31 | 12.4% |
| Tweak | 27 | 10.8% |
| InsertMax | 24 | 9.6% |
| WrapLoop | 20 | 8.0% |

Expected 12.5% each; at n=249 that is 31 +/- 5.5 per bucket at 1 sigma, so an 8.0-16.5% range
is chance. Before: **InsertMax 38%, WrapIf 3%** — a 12x spread, now 2x.

It took three fixes, and each one revealed the next:
1. **Selection was a race.** A fresh random operator was drawn on every retry and whichever
   applied first was kept, so usage was proportional to how many node types an operator
   accepts. Tweak appeared 0 times in 67 proposals.
2. **The draw itself was skewed.** With the operator chosen before placement, the distribution
   was still Delete 40 / InsertMax 39 / SwapSiblings 2. `Rng::new` was `Rng(seed | 1)` with no
   warmup, and the search seeds a fresh generator per candidate from a small structured value;
   a bare xorshift64's first output correlates with its seed. splitmix64 finalizer fixed it.
3. **The running process had a stale binary.** The ledger still showed InsertMax 38% because
   the isolated build tree's last build was the RNG negative control. Restoring source is not
   deploying it. (run.sh now builds and asserts freshness in the tree it executes.)

TWO SIDE EFFECTS, both good and neither predicted:
- `0 ill-typed` per generation, down from 2. Try-every-position no longer abandons an operator
  that is merely hard to place.
- Oracle rejection fell from ~70% to 9 of 24. The operators that used to dominate were the most
  destructive ones, so a fair draw sends more candidates to the gate — the search got cheaper
  per useful candidate as a consequence of being fair.

## 2026-09-08 — datagen depth is a NULL at blend 0, and the reason is structural

| datagen depth | decisive | samples | mean | 95% CI |
|---|---|---|---|---|
| 2 | 415/1500 (28%) | 3,345 | 0.4703 | [0.4622, 0.4784] |
| 3 | 948/1500 (63%) | 7,733 | 0.4703 | [0.4521, 0.4885] |

Identical means, despite depth 3 producing **more than twice the decisive rate** — much better
play, same training result.

The reason is structural rather than empirical, and I should have seen it before running the
sweep: **at blend = 0 the stored root score is never read.** The training target is the game
outcome alone, so search depth can only change WHICH GAMES ARE PLAYED, never what the label
says about them. `--deepen-at` defaulting to 1,000,000 meant the deepening the code calls
"AlphaZero's engine of improvement" had never run — but running it changes nothing while the
label ignores the search.

So the two knobs are COUPLED and I tested them independently: deeper search is worth more
precisely when the target includes the search score. Re-running depth 2/3/4 at blend 0.75.

This is the same shape as the blend finding itself. The loop had two halves of one mechanism —
a search score worth trusting, and a target that reads it — and both were switched off, each
for a reason that made sense at iteration zero.

## 2026-09-08 — BLEND: training flips from harmful to beneficial. The stall was the LABEL.

Same trained champion, same data, horizon 10, 10 replicates per arm. Only the training TARGET
differs: `(1-blend) * game_outcome + blend * own_root_score`.

| blend | mean | 95% CI | |
|---|---|---|---|
| 0.00 | 0.4805 | [0.4722, 0.4887] | significantly WORSE than the champion |
| 0.25 | 0.4898 | [0.4799, 0.4997] | worse |
| 0.50 | **0.5188** | [0.5114, 0.5261] | **BETTER, excludes 0.5** |
| 0.75 | **0.5258** | [0.5123, 0.5393] | **BETTER, excludes 0.5** |

0.75 vs 0.00 is **+0.0453 +/- 0.0158**, monotone across the sweep.

**This is the acceptance stall, and it was never a horizon problem.** Every horizon arm was
below 0.5 because the LABEL was wrong, not because the wrong positions were selected. I swept
the horizon from 3 to 1000 and the whole axis topped out at "no change"; changing one term in
the target moves it to a measured gain.

The loop hardcodes `blend = 0.0`, and the comment says why: mixing the net's own root score
into its target is self-referential WHEN THE NET IS RANDOM, so it "teaches nothing". True at
iteration zero, and it stopped being true the moment the net was trained — but the constant
never moved, and nothing re-tested it. MASTER_PLAN lists "Objectives: game outcome; agreement
with own deeper search" in the GIVEN column: half of the declared objective was switched off.

The mechanism is AlphaZero's: the search score is a lower-variance target than a single game
outcome, because it summarises a subtree rather than one playout. It only becomes a BETTER
target once the search is worth trusting — which is the same condition the code comment
describes for deepening, and which nothing had checked had arrived.

Peak not yet located: 0.75 is the highest arm tested and the curve is still rising. blend = 1.0
is pure self-reference (train toward what the net already says) and must be degenerate, so
there is a peak between. Refining before changing the default.

## 2026-09-08 — horizon tuning is EXHAUSTED: the best case is neutral, never a gain

Completing the sweep against the trained champion with narrower arms:

| horizon | samples | mean | 95% CI |
|---|---|---|---|
| 3 | 3,235 | 0.4977 | [0.4851, 0.5102] |
| 5 | 4,845 | 0.4797 | [0.4648, 0.4946] |
| 10 | 8,854 | 0.4867 | [0.4688, 0.5047] |
| 15 | 12,849 | 0.4664 | [0.4531, 0.4797] |
| 20 | 16,830 | 0.4648 | [0.4482, 0.4815] |
| 40 | 32,229 | 0.4203 | [0.4052, 0.4355] |
| 80 | 53,775 | 0.3977 | [0.3904, 0.4049] |

**Across the entire swept range, 3 to 1000, no setting produces a gain.** The best arms (h3,
h10) have intervals touching 0.5 — indistinguishable from not training at all. Everything wider
is significantly worse.

So the horizon is a DAMAGE knob, not a strength knob: it controls how much training on this
data hurts, and its optimum is "hurt least". That closes it as a lever and moves the question
somewhere else entirely — the champion has extracted what this data distribution contains, and
no filter over the same positions recovers more.

The remaining candidates are the ones that change WHAT THE LABEL IS or WHAT THE DATA IS, not
which subset of it is used:
  - the label. blend = 0 hardcodes "train on the game outcome only". The stated reason is that
    mixing the net's own root score is self-referential WHEN THE NET IS RANDOM — a premise that
    expired the moment the net was trained. MASTER_PLAN lists "agreement with own deeper
    search" as a Given objective. Testing now.
  - the data. Self-play by a converged champion revisits what it already knows; the openings
    are 4 random plies.
  - capacity. Width 16, and the ARCH arm walked DOWN to it under a clock gate.

## 2026-09-08 — the horizon SCHEDULE is backwards, and the champion has converged

Same sweep, but against a TRAINED champion (champion_ep3_s20260907, width 16) instead of a
random one. 10 replicates per arm, one shared dataset, 1082/4000 decisive:

| horizon | samples | mean | 95% CI |
|---|---|---|---|
| 10 | 8,854 | 0.4867 | [0.4688, 0.5047] |
| 20 | 16,830 | 0.4648 | [0.4482, 0.4815] |
| 40 | 32,229 | 0.4203 | [0.4052, 0.4355] |
| 80 | 53,775 | 0.3977 | [0.3904, 0.4049] |

**TWO findings, and both matter more than the horizon constant.**

1. **EVERY arm is below 0.5.** Training the trained champion on fresh self-play data makes it
   WORSE at every horizon tested. Only h10 has an interval touching 0.5; the rest are
   significantly worse. This is the acceptance stall, quantified: the champion has converged
   with respect to this data distribution, and more of the same data degrades it.

2. **THE SCHEDULE IS BACKWARDS.** `horizon = 10 + 5*(g-1)` widens with generation, on the
   stated theory that "the label becomes informative further back as play improves". Measured
   against exactly the condition that theory describes — an improved player — wider is
   MONOTONICALLY WORSE, and the gradient is steep (0.4867 -> 0.3977 from h10 to h80).

   The random-champion sweep peaked at 20; the trained-champion sweep peaks at the narrowest
   arm tested. The optimum moved the OPPOSITE direction from the one the schedule assumes.

Consequence: the widening schedule is not a refinement, it is an active harm that grows with
generation — consistent with the eight-generation collapse seen in the very first uncapped run,
which I attributed to the cap being absent rather than to the schedule being wrong.

Narrower arms (3, 5, 10, 15) are running to locate the optimum, and the schedule direction
should be re-derived from that rather than patched.

## 2026-09-08 — horizon SWEPT: 20 is the optimum, and 40 (my pick) was measurably worse

10 replicates per arm, one shared dataset, filter applied per arm:

| horizon | samples | mean | 95% CI |
|---|---|---|---|
| 10 | 5,052 | 0.5527 | [0.5470, 0.5585] |
| **20** | **9,418** | **0.5660** | **[0.5599, 0.5721]** |
| 40 | 17,266 | 0.5371 | [0.5289, 0.5454] |
| 80 | 27,303 | 0.5320 | [0.5208, 0.5433] |
| 1000 | 30,151 | 0.5188 | [0.5084, 0.5291] |

Unimodal, peak at 20. **20 vs 40 = 0.0289 +/- 0.0102, excluding zero.** The default I shipped
this morning was measurably worse than an untested neighbour I had named in the comment as
untested and then not tested.

h20 beats h40 on 45% FEWER samples, so again quality over quantity — the same shape as the
capped-vs-uncapped result, now with the peak located rather than just bounded.

DECLARED LIMIT: measured against a RANDOM champion, i.e. early in a run. The schedule widens
the horizon with generation *because* the label becomes informative further back as play
improves, so this optimum should MOVE. It sets the cap early generations run into; it is not a
claim about a strong champion, and re-measuring against a trained champion is the obvious
follow-up.

## 2026-09-08 — horizon cap 40: CONFIRMED on a controlled A/B (and it was shipped on n=1)

Same champion, ONE raw generation (4000 games, 40,202 decided positions), the horizon filter
applied per arm so both see the same games, 10 replicates each:

| arm | samples | mean | sd | 95% CI |
|---|---|---|---|---|
| horizon 40 | 17,266 | **0.5371** | 0.0133 | [0.5289, 0.5454] |
| horizon 1000 (uncapped) | 30,151 | 0.5188 | 0.0167 | [0.5084, 0.5291] |

Difference **0.0183 +/- 0.0133** at 95% -> [0.0050, 0.0316], excludes zero. Significant.

**The capped arm wins on 43% FEWER samples.** So this is not data quantity, it is data QUALITY:
labels far from the terminal are anti-signal while play is weak. The code comment beside the
constant asserted exactly that and had never demonstrated it.

This was shipped as a default this morning on n=1 per arm, before the run-to-run variance
(0.151) was known -- i.e. on evidence I spent the afternoon refusing from --steps-per-gen. It
now has evidence that survives the standard. The n=1 caveat in main.rs is superseded and
`run_horizon_experiment.sh` (3 loop seeds) is no longer needed: the loop is the wrong instrument
for this question, at 25x worse resolution than the A/B.

Worth noting against the entry above: the SAME harness refuted the step budget and confirmed
the horizon. It is discriminating, not merely returning nulls.

## 2026-09-08 — mate-in-2 surrogate: NULL, the set is too rare to build

The mate-in-1 surrogate is saturated: the seed's terminal guard fires before its depth guard,
so every program finds all of them without searching and the count filter compares 0 < 0
forever. Mate-in-2 needs real lookahead, so it should discriminate — a program that prunes
unsoundly misses it, which is the failure FITNESS 3 exists to catch before games are spent.

Built it (forward search: a move such that for every reply, some follow-up mates; sparse
positions so depth 3 stays cheap). MEASURED: **400,000 random walks produced 2 positions.**
Mate-in-1 needs ONE winning move to exist; mate-in-2 needs EVERY reply to lose, which is orders
of magnitude rarer on positions reached by random play.

A 2-position surrogate carries no signal, so the default reverts to mate-in-1 plus the
mates-per-COST rate check, which does discriminate — on waste rather than on correctness. The
builder is kept behind `--mate2` because it works; what failed is finding enough instances
cheaply, not the idea. A retrograde construction (start from a mated position, unwind three
plies) would produce them by the thousand and is the obvious next attempt if this surrogate
ever needs to bite.

## 2026-09-08 — step budget: REFUTED on a controlled A/B. The loop signal was trajectory noise.

Same champion, ONE shared dataset (4000 games, 17,266 training samples), 8 replicates per arm
differing only in the hyperparameter and the training seed, each gated against that champion:

| arm | updates | mean | sd | 95% CI |
|---|---|---|---|---|
| epochs 3 | ~51,798 | **0.5444** | 0.0168 | [0.5328, 0.5561] |
| steps-per-gen 20000 | 20,000 | 0.5352 | 0.0167 | [0.5236, 0.5467] |

Difference 0.0092 +/- 0.0163 at 95%. **Not significant, and the sign is REVERSED** from the
loop, where steps led on 2 of 2 seeds.

**This refutes the mechanism, not just the effect.** The morning diagnosis was that raising
self-play volume 125x pushed gradient steps per generation from ~200 to ~110,000 and was
overwriting the champion each cycle. If that were right, the arm doing 2.6x MORE updates should
be worse. It is nominally better.

**And it shows why the loop could not answer this.** The standard error here is 0.0059 against
the loop's 0.151 run-to-run spread -- a 25x improvement in resolution, from removing path
dependence rather than from more compute. The loop's apparent effect was the trajectory, which
is exactly what the variance measurement predicted.

`--steps-per-gen` stays defaulted OFF and is now off for a measured reason. Kept in the code
because the harness that tests it is worth more than the flag.

WHAT THIS DOES NOT SHOW: whether a step budget matters over MANY generations. A single-step A/B
cannot see a compounding effect. But the burden has moved -- there is no longer a measured
single-step benefit to compound.

## 2026-09-08 — mate-in-2 surrogate: NULL, the set is too rare to build

The mate-in-1 surrogate is saturated: the seed's terminal guard fires before its depth guard,
so every program finds all of them without searching and the count filter compares 0 < 0
forever. Mate-in-2 needs real lookahead, so it should discriminate — a program that prunes
unsoundly misses it, which is the failure FITNESS 3 exists to catch before games are spent.

Built it (forward search: a move such that for every reply, some follow-up mates; sparse
positions so depth 3 stays cheap). MEASURED: **400,000 random walks produced 2 positions.**
Mate-in-1 needs ONE winning move to exist; mate-in-2 needs EVERY reply to lose, which is orders
of magnitude rarer on positions reached by random play.

A 2-position surrogate carries no signal, so the default reverts to mate-in-1 plus the
mates-per-COST rate check, which does discriminate — on waste rather than on correctness. The
builder is kept behind `--mate2` because it works; what failed is finding enough instances
cheaply, not the idea. A retrograde construction (start from a mated position, unwind three
plies) would produce them by the thousand and is the obvious next attempt if this surrogate
ever needs to bite.

## 2026-09-08 — step budget: INTERIM, 2 of 3 seeds, and the SEED VARIANCE dominates

Control vs the frozen origin at generation 10:

| seed | epochs 3 | steps-per-gen 20000 | diff |
|---|---|---|---|
| 20260907 | 0.641 +/- 0.045 | 0.756 +/- 0.043 | +0.115 (intervals separated) |
| 424242 | 0.792 +/- 0.032 | 0.822 +/- 0.032 | +0.030 (intervals overlap) |

At generation 20 both arms land in the low 0.8s on both seeds and nothing separates.

**Direction is consistent — steps ahead 2 of 2 — but the effect is not the interesting number.
THIS is:** epochs-3 scored **0.641 on one seed and 0.792 on the other, at identical settings**.
A 0.151 spread between runs that differ only in seed, against a treatment effect of 0.030 to
0.115.

The between-run variance is LARGER than the thing being measured. That has a direct
consequence for the design: three seeds is not enough. To resolve a 0.03 effect against a 0.15
run-to-run spread needs roughly (0.15/0.03)^2 = 25 runs per arm, not 3. The n>=3 rule was
written to stop me believing single runs; it does not by itself make an effect of this size
measurable.

So the honest statement when seed 3 lands will be about DIRECTION (2 or 3 of 3 favouring the
fixed step budget) and not about magnitude — and even the direction is weak evidence from three
paired samples. `--steps-per-gen` stays defaulted OFF until either the effect is bigger or the
sample is.

This also retroactively explains the two identical-setting runs that disagreed earlier today
and started this whole experiment. They were not evidence of a step-budget effect at all; they
were two draws from a distribution this wide.

## 2026-09-08 — mate-in-2 surrogate: NULL, the set is too rare to build

The mate-in-1 surrogate is saturated: the seed's terminal guard fires before its depth guard,
so every program finds all of them without searching and the count filter compares 0 < 0
forever. Mate-in-2 needs real lookahead, so it should discriminate — a program that prunes
unsoundly misses it, which is the failure FITNESS 3 exists to catch before games are spent.

Built it (forward search: a move such that for every reply, some follow-up mates; sparse
positions so depth 3 stays cheap). MEASURED: **400,000 random walks produced 2 positions.**
Mate-in-1 needs ONE winning move to exist; mate-in-2 needs EVERY reply to lose, which is orders
of magnitude rarer on positions reached by random play.

A 2-position surrogate carries no signal, so the default reverts to mate-in-1 plus the
mates-per-COST rate check, which does discriminate — on waste rather than on correctness. The
builder is kept behind `--mate2` because it works; what failed is finding enough instances
cheaply, not the idea. A retrograde construction (start from a mated position, unwind three
plies) would produce them by the thousand and is the obvious next attempt if this surrogate
ever needs to bite.

## 2026-09-08 — step budget: INTERIM, seed 1 of 3

First arms to complete with the derived per-net gate budget (the earlier attempts aborted on
the coverage guard). Control vs the frozen origin, seed 20260907:

| gen | epochs 3 | steps-per-gen 20000 |
|---|---|---|
| 10 | 0.641 +/- 0.045 | **0.756 +/- 0.043** |
| 20 | 0.809 +/- 0.040 | **0.834 +/- 0.035** |

At generation 10 the intervals do not overlap ([0.596, 0.686] vs [0.713, 0.799]), which is a
real separation favouring the fixed step budget. By generation 20 they overlap and the
difference is not significant.

**NOT A RESULT YET, and the reason is written above in this file.** This is n=1. Two runs at
IDENTICAL settings disagreed earlier today — that is what started this experiment — so a single
seed showing a clean separation is exactly the evidence that has already misled me once. Seeds
424242 and 987654 are running. The claim waits for the pooled three.

What IS established independently of the arms: both now compound strongly (0.809 and 0.834
against the origin at gen 20, against 0.694 in the earlier run), because the derived budget
gives the gate 100% coverage instead of the 33-57% a fixed 4000 nodes happened to produce.

## 2026-09-08 — FIRST SEARCH-TRACK RESULT: 128 mutations, 0 accepted

The search track ran end to end for the first time. 8 generations x 16 candidates against the
bare alpha-beta seed, gated on games at equal cost budget:

    106 well-typed, 22 ill-typed (type checker rejected them before any compute)
     90 failed the correctness oracle
     16 reached the game gate
      0 beat the champion

**This is the expected outcome and it is a measurement, not a failure.** A single random
mutation of a 71-node program that already computes the exact minimax value has almost no way
to improve it; GRAMMAR 6 puts the nearest real milestone (hash reuse) at +104 nodes, which is
not one mutation away. What the run establishes is that the PIPELINE works: candidates are
generated, ill-typed ones are rejected for free, incorrect ones are caught before spending
games, and the survivors are judged by play.

The oracle is doing the heavy lifting -- 90 of 106 well-typed candidates were REJECTED FOR
BEING WRONG, i.e. they returned a move the full-width reference disagreed with. Without that
stage every one of them would have gone to the gate, and the cheap-but-worse ones would have
been indistinguishable from genuine improvements on a cost-based metric.

TWO BUGS THE RUN EXPOSED, both in my harness rather than in the idea:

1. THE INTERPRETER NEVER ENFORCED ITS BUDGET (see above). One mutant looped for four hours.
2. THE SURROGATE WAS INERT. The seed scored 0/40 on the mate-in-1 set, so the filter compared
   0 < 0 and passed everything. Cause: the surrogate ran at the GAMES depth (3), where a
   single position costs ~411M cost units (ladder), overshooting the 2e9 safety cap and
   forfeiting. Mate-in-1 needs one ply. Given its own --surrogate-depth (default 2) it now
   scores 40/40 with 0 forfeits, and an assertion aborts the run if the seed ever fails its
   own surrogate again -- an inert filter that silently passes everything is worse than no
   filter, because it looks like a stage.

## 2026-09-08 — the cost model was never built, and it inverted a published result

GRAMMAR 8 and CRATE 4 both specify a per-primitive cost table (`configs/cost.toml`).
Neither existed; the interpreter charged a flat `self.cost += 1` per node, so a full NNUE
forward pass cost exactly what `const 3` cost. That is not neutral — it is a thumb on the
scale against any program that spends cheap work to avoid expensive work, which is exactly
a transposition table's trade. MEASURED (examples/cost_calibrate.rs, width 32, min-of-5):

| primitive | cost | |
|---|---|---|
| arith / cmp / const / var | 1 | |
| key (zobrist) | 97 | |
| terminal | 703 | |
| apply | 788 | |
| eval | **1365** | 293 ns |
| moves (legal_moves) | 2232 | |

Re-derived ladder, hash reuse relative to the seed: flat 1.250x (D2) / 1.218x (D3) becomes
**1.04x / 1.01x**. The 25% penalty was the instrument. RETRACTS the magnitude of the
"break-even near depth 5-6" claim and the chicken-and-egg constraint written into
MASTER_PLAN from it; the direction (overhead falls with depth) survives.

## 2026-09-08 — a fixed gate budget is wrong when tree size is NET-DEPENDENT

The 3-seed experiment reported ALL ARMS COMPLETE with 4 of 6 arms at zero generations. The
gate-coverage guard had aborted them: a fixed 4,000-node budget covered 57% of one seed's
depth-4 tree and 33% of another's, because different random nets produce different
alpha-beta cutoffs and therefore different tree sizes. The guard was RIGHT — it refused to
run gates that would return confident-looking 0.500s. `--cost-nodes` now derives from the
measured full tree at the cap depth.

The second half is worse and is a repeat: the wrapper never checked exit codes, so four
aborts printed as success. The 4PC queue runner already carries this exact lesson — "a
failed EXPERIMENT is a result; a failed SCRIPT is a bug."

## 2026-09-08 — the search track exists, and its oracle caught its own bug first

`evolve_search.rs`: mutate -> CORRECTNESS ORACLE (full-width negamax agreement) -> mates-per-cost
surrogate -> GAME GATE (candidate program vs champion program, same net, equal COST budget,
pentanomial). The previous `evolve.rs` had only the surrogate, scored against a random net.

First run printed `seed: bare alpha-beta, 71 nodes; oracle 7/10`. A seed failing its own
oracle is impossible — alpha-beta returns the full-width minimax value by construction. The
reference was a ply shallow: the seed's `choose` expands the ROOT itself then searches D more
plies, and I called the reference with depth-1. That is the SAME off-by-one GRAMMAR 8 records
behind the bogus "0.14x" interpreter reading. Fixed; seed now 10/10.

## 2026-09-07 — the ones that held

- **Self-play VOLUME.** 80 -> 10,000 games/generation. Training samples 33-267 ->
  12,212-106,485; the per-generation gate went from all-drawn 0.500 to 20W-44D-0L
  (0.656 +/- 0.057). This is the change that made the gate able to resolve at all.
- **Horizon cap 40.** Uncapped, the gate collapsed once the horizon passed 60 plies
  (eight straight generations below 0.5, four flagged `regression`). Capped, the champion
  compounded 0.552 -> 0.567 -> 0.570 -> 0.694. CAVEAT: also n=1 per arm; the capped arm's
  own trajectory is solid, the cross-arm claim is suggestive.
- **Gate depth cap 6 -> 4.** A 4,000-node budget bought 1.29% of a depth-6 tree, so the
  search never finished its first root move and both sides played near-randomly. Every
  budgeted gate was returning a confident-looking 0.500 that meant nothing.

## 2026-09-07 — the ones that failed, with the reason

- **`ab_hash` was not a transposition table.** Never called `eval`; returned constant 0 at
  every leaf. Its ladder "confirmation" of a 3.2x gain was the bug. Repaired: 175 nodes,
  and hash reuse measures a LOSS at D2-D4 that shrinks with depth (1.250x -> 1.113x),
  break-even near depth 5-6.
- **"Swap ladder steps 4 and 5."** Reasoned that iterative deepening creates the traffic a
  table needs. REFUTED: ID's cost is flat at ~1.086x across D2-D4 and hash+ID is worse
  than hash alone everywhere. Reverted in GRAMMAR 9 and MASTER_PLAN.
- **"Emission order gives alpha-beta a free ordering heuristic."** REFUTED in the
  informative direction: it is an ANTI-ordering costing +16/+31/+54% nodes at depth 3/4/5.
- **"Deeper random openings will make games decisive."** Null: 8.3% decisive at 4 plies,
  15.0% at 32, inside the noise at n=60. The lever was volume, not opening depth.
