# Pre-registration: what the game-free instrument predicts for the lr sweep

**Written 2026-09-11 ~02:05, BEFORE `lr_sweep.sh`'s netmatch verdicts existed.** The three arms had
finished training and their nets were final; the three 224-pair matches had not yet reported. Git's
timestamp on this commit is the evidence that this was written first.

## The reading

`static_deep_residual --ref p1_champion_prev_pre_lr002.net`, 600 positions, depth 4, seed 20260912.
All three arms start from `lrs_start` (= `p1_champion` = `lrB`), so the pre-lr champion is
**symmetric** among them — no arm is closer to the reference by ancestry. Using `lrs_start` itself
as the reference would instead have rewarded whichever arm moved *least*, which is 0.0005.

| net | corr | sign agree |
|---|---|---|
| origin(random) | −0.011 | 52.1% |
| **lrs_start** | **0.746** | **70.3%** |
| lrs_001 (lr 0.01) | 0.637 | 67.1% |
| lrs_0002 (lr 0.002) | 0.722 | 65.1% |
| lrs_00005 (lr 0.0005) | **0.769** | 70.0% |

## What is firm, and what is not

**THE TWO COLUMNS DISAGREE, and the honest version says so rather than quoting the one that reads
better.** `corr` ranks 0.0005 > start > 0.002 > 0.01. Sign agreement ranks start ≈ 0.0005 > 0.01 >
0.002 — it puts **0.002 last**. They do not agree on where 0.002 sits against 0.01, which is the
sweep's central comparison.

**Firm on both columns:**
* **lr 0.01 is not the best rate.** Below the start on both measures.
* **lr 0.0005 is at or above the start**, and is the top or joint-top arm on both.

**Not resolved by this instrument:** 0.002 versus 0.01. Predicted direction is 0.002 ahead (that is
what `corr` says, and it is what the first A/B measured at 0.692 vs 0.499), but sign agreement
contradicts it, so this is recorded as a *weak* prediction that could be wrong.

**Predicted pre-registered row from `lr_sweep.sh`:** *"0.0005 beats 0.002 → the optimum is lower
still; sweep again before settling."*

## The mechanism this would support

No arm clearly beat its own start, and the start was **already trained at 0.002**. Only the arm that
lowered the rate *again* (0.0005) held or improved. That is the signature of a rate that buys a
burst of progress and then saturates — which is what `learning_rate_is_the_plateau_RESULT.md`
already argued the constant-step mechanism implies:

> The obvious follow-ups are a sweep … and a **decay schedule**, which is what the mechanism above
> actually argues for — a large step early and a small one late.

If netmatch agrees, the next move is a decay schedule, not a fourth fixed rate.

## How this gets scored

Honestly, including the miss. If netmatch puts 0.002 above 0.0005, the `corr` column was misleading
here and this file says so. An instrument validated on one comparison (the lrA/lrB pair, which it
reproduced under an adversarial reference) is not thereby validated on every comparison.
