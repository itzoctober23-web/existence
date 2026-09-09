# Excluding draws is BETTER — REFUTED, resolved

`draws_ab.sh`, 2026-09-08. Pre-registered in three directions before the run; this is the branch
labelled REFUTED.

## The measurement

Two arms from scratch, identical seed, games/generation, epochs and generation count, **nothing
gated in either**. They differ in `--include-draws` and nothing else. Each arm's built-in final
control against its own frozen origin, 448 pairs:

| arm | W-D-L | rate | pool at end | train/gen (gen 1) |
|---|---|---|---|---|
| exclude (shipped) | 716W-51D-129L | **0.828 ± 0.023** | 433,900 | 3,124 |
| include (draws) | 650W-87D-159L | **0.774 ± 0.025** | 988,766 | 19,715 |

**Difference +0.054 ± 0.034 — resolved, excludes zero.** Keeping draws OUT is better.

So `.filter(|s| s.z != 0.0)`, which discards ~64% of every self-play batch, is doing its job.
Draws are the uninformative middle. **This lever is closed.**

## The flag was verified before the result was believed

A null from a no-op flag looks exactly like a null from a real intervention, so the intervention
was checked first. Same seed, identical games — `pos 338582`, `dec 389/2400` in both arms — and the
pools diverge exactly as the 16% decisive rate predicts:

* gen-1 training samples: 3,124 → 19,715, a **6.3x** increase, and 1/0.16 = 6.25.
* final pool: 433,900 → 988,766.
* per-generation cost: 26s → 25s, so this was not a compute trade either.

## Two things not to overread

**The losses are not comparable across arms.** Gen-1 loss is 0.0726 (exclude) against 0.0191
(include), and the include arm's is lower only because it is fitting an easier, draw-heavy target
distribution. Lower loss on a different target says nothing about strength — the same trap that
made "heldout_loss falls while the paired statistic collapses" look like a paradox earlier.

**These are CAPPED equal-time controls.** 0.828 is not comparable to `champion_long`'s 0.861, which
was measured fixed-depth-2 uncapped. Different protocols. Three of today's corrections came from
exactly this kind of transfer, so it is stated rather than left for a future reader to trip over.

## Where the ceiling stands

Four candidates were named in `ceiling_ANALYSIS.md`. Two are now refuted with resolved
measurements:

| candidate | status |
|---|---|
| capacity / net width | **REFUTED** — w64 loses 0.179 ± 0.021 to w16 at equal time (`width_RESULT.md`) |
| the draw filter | **REFUTED** — this result |
| datagen depth (depth-2 labels) | queued, chained, at equal COMPUTE |
| the horizon schedule | untested |

A refutation is worth as much as a confirmation. What is worth nothing is a lever left standing as
a suspect because nobody measured it.

## Incidental, and worth keeping

The decisive-game rate rises from **389/2400 (16%) at generation 1 to 1039/2400 (43%) at generation
20**. The engine plays more decisively as it trains. That is independent evidence that training
does something real, even while the strength ceiling holds — and it is the sort of thing that would
have been invisible without reading the per-generation lines.


---

## CONFIRMED on an independent, tighter protocol

The verdict above used each arm's built-in 448-pair capped control. The driver also ran a separate
**1000-pair fixed-depth-2 uncapped** match per arm — a different protocol, more than twice the
games:

| arm | W-D-L | rate |
|---|---|---|
| exclude (shipped) | 1375W-617D-8L | **0.842 ± 0.010** |
| include (draws) | 1036W-954D-10L | **0.756 ± 0.011** |

**Difference +0.086 ± 0.015 — resolved with a large margin**, and in the same direction as the
built-in control's +0.054 ± 0.034. Two protocols, two independent measurements, one conclusion:
excluding draws is better.

**A mechanistic tell worth keeping.** The include-trained net draws **954** games against the
origin where the exclude-trained net draws **617**. Training on drawn positions taught it to value
them, and it now plays more drawishly — which is exactly the causal story the filter was there to
prevent, showing up in the game record rather than being inferred from the loss.
