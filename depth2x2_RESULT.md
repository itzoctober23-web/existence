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
| d3 vs d4 | depth (both deep) | *running* | pending |

**Both depth rows resolve in favour of the deeper arm. The parity row that has finished is a precise
null** — a narrow interval containing 0.5, which `netmatch` distinguishes from an underpowered
"unresolved" by its own 0.015 criterion (ci95 0.014 here, so it qualifies).

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
* `throughput_RESULT.md` measured where that cost actually is: at the champion's width 16, eval is
  only ~14% of a 1.26 µs node, so the eval-side optimisations the brief ranks first cannot deliver
  the ~10x needed. The remaining ~86% is movegen and make/unmake, unprofiled.

## Honest limits

* **One seed.** The run replicates on 987654 next; the between-run band in this tree is ~0.07, wider
  than these ci95s, so a second seed is the price of calling the depth rows settled. The parity null
  is the more robust of the two claims here because a null at ±0.014 is harder to produce by chance
  than a direction.
* The fourth cell (parity at DEPTH) is still running. If it also nulls, parity is null at both depths
  and the conclusion is clean; if it resolves, parity matters only when deep and this write-up needs
  amending rather than extending.
* 448 pairs per match. `netmatch` prints the pair count its own precision rule would demand.
