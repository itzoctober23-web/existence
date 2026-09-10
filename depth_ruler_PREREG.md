# PRE-REGISTRATION — datagen depth on the ruler, written before the games were read

**2026-09-10 18:2x.** The three arms have trained; the ruler runs have not been read. Recording the
expected outcome first, because the generation counts are already visible and they make one reading
much more likely than the others.

## What is already visible

| arm | generations in 1800s |
|---|---|
| datagen depth 1 | **468+** |
| datagen depth 3 | **5** |
| datagen depth 6 | **0 — no net produced at all** |

## Prediction: depth 1 wins THIS run, and that is mostly a statement about the budget

`untrained_baseline_RESULT.md` measures gen200 at −114 and an untrained net at −366, so **most of the
loop's total gain arrives within the first ~200 generations**. The depth-1 arm has 468 and is
therefore near its plateau. The depth-3 arm has **five**, which is far short of where the depth-1 arm
was when it had gained anything at all.

So I expect the depth-1 arm to read substantially higher, and **that would not refute deeper
datagen.** It would mean 1800 seconds is too small a budget for this comparison — the deep arm never
reached the part of the curve where its labels could matter.

## The design limit, named in advance rather than discovered afterwards

Both arms use `--games 2400` per generation, inherited from `depth_ab.sh`. That fixes the number of
GAMES per generation while the cost per game varies ~10× with depth, so the deep arm spends its
entire budget on a handful of enormous generations. The trade being measured is therefore not
"deep labels vs shallow labels" but "5 training steps vs 468 training steps", which is a different
question and one whose answer is obvious.

**The fix for round 2 is to hold GENERATIONS roughly equal and let the games-per-generation absorb
the depth cost** — e.g. depth 3 at ~240 games/generation instead of 2400. Then both arms take a
comparable number of training steps and the labels are what differs, which is the thing under test.

## What each outcome means, given the above

* **depth 1 ≫ depth 3** — expected, and mostly uninformative about labels. Round 2 with matched
  generation counts is the real test.
* **depth 3 ≥ depth 1 despite 5 generations against 468** — a strong result for label quality, and
  much stronger than it looks, because the deep arm would be winning from ~1% of the training steps.
* **depth 6 produces no net** — already certain. It is a cost measurement, not a failure: at 2400
  games per generation, depth-6 datagen cannot complete one generation in 30 minutes.

## What is NOT at risk here

Whatever this returns, it does not touch `speed_cannot_pay_RESULT.md`. That file's numbers (eval =
25.4% of a leaf, one ply = 9.2× nodes, quantization ≈ 5 Elo) are independent measurements and do not
depend on any datagen arm.
