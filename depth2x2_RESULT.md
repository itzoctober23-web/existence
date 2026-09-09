# Depth is real; parity does not reach trained strength — 2026-09-09

`depth_2x2.sh`. Four arms at **equal generations** (4 each, asserted after the run, not assumed),
equal games per generation (300), `--horizon-cap 45` on every arm, one seed (424242). Verdicts are
head-to-head `netmatch` at depth 4 — the project's strength standard per `netmatch.rs:23-29` — never
the frozen-origin control, which is recorded as reversing signs on two prior results.

## Why the equal-generation framing was necessary

The equal-WALL-CLOCK comparison cannot answer "are deeper labels better", because it hands the
shallow arm more generations by construction: **48 vs 4**, a 12x disparity in training amount. At
this tree's ~0.0114 per generation that extrapolates to 0.502 of advantage — off the top of the
0.5–1.0 scale, i.e. the linear figure has left its valid range and the comparison is DOMINATED by
training amount. Equal generations removes that and leaves depth and parity as the only differences.

## The 2x2

| contrast | holds fixed | result | verdict |
|---|---|---|---|
| d1 vs d3 | parity (both ODD) | **0.450 ± 0.015** | d3 stronger, interval clear of 0.5 |
| d2 vs d4 | parity (both EVEN) | **0.379 ± 0.016** | d4 stronger, interval clear of 0.5 |
| d1 vs d2 | depth (both shallow) | **0.512 ± 0.014** | **INDISTINGUISHABLE, and precisely so** |
| d3 vs d4 | depth (both deep) | **0.506 ± 0.014** | **INDISTINGUISHABLE, and precisely so** |

**Both depth rows resolve in favour of the deeper arm. Both parity rows are precise nulls ON THIS
SEED — and that phrasing is CORRECTED below: the shallow parity cell reads 0.554 ± 0.014 on seed
987654, so parity is small and unresolved cross-seed, not an absence** — narrow
intervals containing 0.5, which `netmatch` distinguishes from an underpowered "unresolved" by its own
0.015 criterion (ci95 0.014 in each, so both qualify). The 2x2 is complete and it separates cleanly:
**depth moves strength at both parities; parity moves nothing at either depth.**

Pre-registered before the run: *"d3>d1 AND d4>d2 => depth genuinely helps per label"*. That is the
outcome, and the shallow parity row adds that parity does not.

## This CORRECTS a recorded conclusion, and narrowly

`STATE.md:326` heads a section **"The depth lever is SEARCH PARITY, not depth"**, on the strength of
`distill_gap`: crossing parity moves `|tanh(root/scale) − tanh(eval/scale)|` by **2.6x** while two
extra plies inside a class move it 2.5%. That measurement is not in dispute and is not refuted here.

What is refuted is the **inference from it to trained strength**. The parity effect is large in the
TARGET distribution and — at shallow depth — worth 0.512 ± 0.014, i.e. nothing, in the trained net.
Depth, which barely moves the target distribution within a parity class, is what moves strength.
A large difference in the training signal did not produce a difference in the trained result.

The original "+0.025 for depth 3" claim was still confounded, and remains withdrawn: it was
even-vs-odd AND uncapped-horizon AND judged on a saturating metric. It happened to point the right
way for the wrong reasons.

## What this does and does not license

* It does **not** say the loop should switch to depth 3. At equal wall clock the shallow arm still
  wins on 12x more generations; that comparison is unchanged and is the one the loop faces.
* It does say the ceiling work should stop treating parity as the explanation and treat **throughput**
  as the lever, which is what `MASTER_PLAN.md:616-617` already predicts: *"the real unlock is making
  deep search cheap enough that both hold at once."*
* `throughput_RESULT.md` has now measured where that cost actually is, and it inverts the brief's
  ordering. Per leaf node at width 16: `legal_moves()` **406 ns (44%)** (the 451 first measured was inflated 9% by `black_box`), eval **261 ns (26%)**, the
  deliberate child shuffle **257 ns (25%)**, make/unmake **47 ns (5%)**. Eval is third, not first.
  The cheapest quantified win is replacing the shuffle's integer division with a multiply-shift —
  **153 ns/node, ~12% of throughput** — which preserves the move-ordering denial exactly and costs
  only exact seed reproducibility. None of this reaches the ~10x that would let depth 3 win at equal
  wall clock, so the honest reading is that the throughput route is incremental, not a step change.

## Honest limits

* **One seed.** The run replicates on 987654 next; the between-run band in this tree is ~0.07, wider
  than these ci95s, so a second seed is the price of calling the depth rows settled. The parity null
  is the more robust of the two claims here because a null at ±0.014 is harder to produce by chance
  than a direction.
* The fourth cell landed as predicted in this file before it ran: parity nulls at depth too
  (0.506 ± 0.014), so the conclusion is the clean one rather than the "parity matters only when deep"
  variant that would have required amending this write-up.
* 448 pairs per match. `netmatch` prints the pair count its own precision rule would demand.

## Free side-result: the decisive-game rate tracks strength, with the confound removed

`STATE.md:399-409` has the decisive-game rate as the most promising cheap proxy for strength —
**r = +0.771, CI [−0.108, +0.973], n = 6 arms** — one arm short of clearing zero, and notes that the
depth probes were **excluded "because training amount drives both terms"**.

The 2x2 arms do not have that confound: all four ran **exactly 4 generations** on the same games per
generation, so training amount is constant by construction. And strength here is head-to-head at
depth 4 — the strength standard — not the frozen-origin metric that is documented to saturate.

| arm | mean dec/300 | strength index | rank agreement |
|---|---|---|---|
| d4 | 95.0 | +0.115 | 1st / 1st |
| d3 | 92.5 | +0.056 | 2nd / 2nd |
| d1 | 72.8 | −0.038 | 3rd / 3rd |
| d2 | 31.2 | −0.133 | 4th / 4th |

(strength index = sum of pairwise margins over the four head-to-head matches)

**Spearman ρ = +1.000, 6 of 6 pairs concordant.**

### What this is and is not

* It is **not** a seventh arm for the n=6 correlation. Different arm length (4 vs 20 generations) and
  a different strength instrument; pooling them would manufacture a verdict, which this project has
  been bitten by before.
* It **is** independent supporting evidence on the one axis the existing result is weakest on: the
  exclusion note says training amount drives both terms, and here it cannot, because it is held
  constant.
* **n = 4.** Perfect concordance has p = 1/24 ≈ 0.042 under a random-ordering null one-tailed, 0.083
  two-tailed. Suggestive, not established.
* The **within-group** orderings (d4 over d3, d1 over d2) rest on margins smaller than their own
  intervals — both of those matches are precise nulls on seed 424242 (the shallow one does not replicate). The load-bearing part is the **between-group**
  split, {d3, d4} stronger than {d1, d2}, which is resolved in both cross-group matches and matches
  the decisive-rate split exactly.
* The counterexample on record still stands: `wd_r2` is the strongest arm on the board with FEWER
  decisive games than a weaker one. A proxy that fails on the strongest arm is not usable for
  promotion decisions, whatever its correlation.

The seed-987654 replication will produce four more matched arms, giving this the same test again at
no extra compute.

## The d2-vs-d3 verdict is protocol- AND seed-independent (judge_depth, closed)

`judge_depth.sh` asked whether the equal-wall-clock d2-vs-d3 winner is a fact about the nets or about
the depth it is judged at. Five matches, 448 pairs each, answered it unanimously:

| seed | judged d2 | judged d3 | judged d4 |
|---|---|---|---|
| 424242 | 0.612 ± 0.018 | 0.580 ± 0.023 | 0.655 ± 0.020 |
| 987654 | 0.636 ± 0.020 | 0.622 ± 0.023 | — |

**Same winner every time, every interval clear of 0.5, across both parity classes and two training
seeds.** Stopped after five: a sixth unanimous confirmation was not worth holding a core while the
blend question — the largest lever on the board — was unresolved.

**What it does NOT say.** These are the EQUAL-WALL-CLOCK arms, 48 generations against 4. The result
is that the *arm which trained 12× more* is stronger, measured robustly. It is not a depth result,
and the equal-generation 2×2 above is what answers that question.

## ⚠ Re-examined under the grounded seed band — only ONE depth cell survives it

`STATE.md`'s measurement wall is now anchored to the right quantity: the between-seed movement of a
**paired difference**, measured twice on the blend data at 0.055 and 0.052, giving sd ≈ 0.047 — since re-derived to **0.043** on four estimates; see STATE.md.
Applying that to the cells above, before anyone else has to:

| cell | rate | effect | verdict |
|---|---|---|---|
| d2 vs d4 (depth, EVEN) | 0.379 | **0.121** | **robust** — ~1 seed suffices; effect is 2.6× the seed sd |
| d1 vs d3 (depth, ODD) | 0.450 | 0.050 | **needs ~7 seeds** — one is not enough |
| d1 vs d2 (parity, shallow) | 0.512 | 0.012 | null, but a ONE-SEED null |
| d3 vs d4 (parity, deep) | 0.506 | 0.006 | null, but a ONE-SEED null |

**So the headline needs narrowing.** "Depth resolves in both parity classes" is solid only in the
EVEN class, where the effect is 2.6× the seed sd. The odd-class cell sits at roughly one sd and is
exactly the size of thing this project has repeatedly withdrawn — the "+0.025 depth lever", blend
1.00's "depth-2 only" mechanism, and (pending) my own blend 0.85 result.

**The parity nulls are one-seed nulls too**, and that is worth stating plainly. Their within-run
intervals (±0.014) are tight, but tightness bounds the effect AT THAT SEED; between-seed movement of
~0.047 means a second seed could read 0.46 or 0.55 without contradicting anything. A precise null on
one seed is not a demonstrated absence.

**What survives unchanged:** the equal-generation design itself, which removes the 12× training-amount
confound that makes the equal-wall-clock comparison uninterpretable. That was the point of the
experiment and it stands regardless of how many cells clear the band.

The seed-987654 replication is running and tests all four cells directly.

## Seed 987654: the odd-class depth cell REPLICATES in direction

| seed | d1 vs d3 (depth, ODD) | verdict |
|---|---|---|
| 424242 | 0.450 ± 0.015 | d3 stronger, clear of 0.5 |
| **987654** | **0.474 ± 0.017** | **d3 stronger, clear of 0.5** |

Both seeds point the same way with within-run intervals clear of 0.5, and all four arms in each seed
ran exactly 4 generations (asserted, not assumed).

**But the magnitude is not resolved across seeds, and that distinction matters here.** The two effects
are 0.050 and 0.026; their mean is 0.038 with a cross-seed standard error of 0.027, giving a 95%
interval of **[−0.016, +0.092]** — which contains zero. Two seeds agreeing in DIRECTION is real
evidence; it is not the same as a resolved effect, and this cell was flagged as needing ~7 seeds
before either was run. Nothing here changes that.

### The band tightens — third independent estimate

This comparison gives a third measurement of how far a paired difference moves between training
seeds, and it is much smaller than the two blend ones:

| comparison | seed A | seed B | movement |
|---|---|---|---|
| blend 0.75 vs 1.00 @ d4 | 0.511 | 0.456 | 0.055 |
| blend 0.75 vs 1.00 @ d2 | 0.450 | 0.502 | 0.052 |
| **depth d1 vs d3 @ d4** | 0.450 | 0.474 | **0.024** |

Mean movement 0.0437, so **between-seed sd ≈ 0.039** (was 0.047 on two estimates). Revised seeds
needed: +0.025 → ~19, +0.040 → ~7, +0.050 → ~5, +0.121 → ~1.

Caveat kept explicit: these are three different net pairs, and seed variance need not be identical
across comparisons — pooling them assumes it roughly is. Three estimates is still few, and the 0.024
is half the other two, which is itself a hint that the variance depends on what is being compared.
`netmatch`'s hardcoded 0.047 is now conservative rather than wrong; it is left alone until a fourth
estimate arrives rather than re-tuned on every new data point.

## The EVEN-class depth cell replicates and is the first CROSS-SEED resolved result

| seed | d2 vs d4 | effect |
|---|---|---|
| 424242 | 0.379 ± 0.016 | 0.121 |
| **987654** | **0.441 ± 0.015** | **0.059** |

Both clear of 0.5 within-run, same direction, arms matched at exactly 4 generations in each seed.

Across the two seeds the mean effect is **0.090**. With the pooled between-seed sd (0.043, now from
four independent estimates), the standard error is 0.030 and the 95% interval is
**[+0.031, +0.149] — excluding zero.** That makes deeper datagen, at equal generations with parity
held fixed, the first result today that survives a cross-seed test rather than a within-run one.

**The assumption this rests on, stated rather than buried.** The interval uses a POOLED sd measured
across four different net-pair comparisons, not the spread of these two points. Using only the two
points, n = 2 gives t(1) = 12.71 and an interval of ±0.494 — useless. So the claim is: *given that
between-seed variance is roughly comparable across comparisons, this effect is resolved.* That
assumption is testable and is being tested every time another replication lands; the fourth estimate
(0.062) already widened the pooled sd from 0.039 to 0.043, so it is not a fixed number being defended.

**The odd-class cell still is not resolved**: effects 0.050 and 0.026, mean 0.038, which the same
method puts at [−0.021, +0.097] — contains zero. Two cells, same experiment, and only the larger one
survives. That is what an honest power boundary looks like rather than a uniform verdict.

Remaining when this was written: the seed-987654 parity cells. The shallow one has since landed and did NOT replicate — see the correction below.

## ⚠ CORRECTED — "parity is a precise null" does NOT replicate

Seed 987654's parity-at-shallow cell resolves where seed 424242's did not:

| seed | d1 (odd) vs d2 (even) | reading |
|---|---|---|
| 424242 | 0.512 ± 0.014 | odd ahead by +0.012 — called a PRECISE NULL |
| **987654** | **0.554 ± 0.014** | **odd ahead by +0.054 — RESOLVED, clear of 0.5** |

The between-seed movement is **0.042 against a pooled sd of 0.042** — entirely ordinary seed noise.
And this file's own caveat, written when the first cell landed, said exactly this would happen:
*"between-seed movement of ~0.047 means a second seed could read 0.46 or 0.55 without contradicting
anything. A precise null on one seed is not a demonstrated absence."* It read 0.554.

**The corrected statement.** Cross-seed, parity is mean +0.033 with 95% CI **[−0.025, +0.091]** —
contains zero, ~13 seeds needed. But **both seeds point the same way** (odd ahead). So parity is
**small, unresolved, and directionally favouring odd** — it is NOT a demonstrated absence, and I
should not have written it as one on a single seed however tight ±0.014 looked.

**What this does and does not disturb.**
* The DEPTH result is untouched and is the stronger claim: d2-vs-d4 replicates and is cross-seed
  resolved at [+0.031, +0.149], and d1-vs-d3 replicates in direction. Depth ≫ parity remains the
  finding — the even-class depth effect (0.090) is nearly 3× the parity effect (0.033).
* What is withdrawn is the sharper phrasing "parity is a precise null", and with it the neatness of
  the original 2×2 story. The measured picture is messier: depth is real and large, parity is real
  and small, and one seed could not tell the second from zero.
* `STATE.md`'s settled row and session summary both said "PARITY is a precise null". Both corrected,
  as is this file's own headline at the top — the correction must sit where the reader lands, not
  200 lines below it. That failure mode cost a turn earlier today.

**The tightness of ±0.014 is what made this seductive**, and it is worth naming: a narrow interval
bounds the effect AT THAT SEED and says nothing about the next one. `netmatch` now prints the
seeds-needed figure precisely so this stops being a judgement call.
