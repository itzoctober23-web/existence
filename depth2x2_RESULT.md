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

**Both depth rows resolve in favour of the deeper arm. BOTH parity rows are precise nulls** — narrow
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
  ordering. Per leaf node at width 16: `legal_moves()` **451 ns (44%)**, eval **261 ns (26%)**, the
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
  intervals — both of those matches are precise nulls. The load-bearing part is the **between-group**
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
