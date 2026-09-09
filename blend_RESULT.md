# The training target's blend is the highest-leverage knob measured, and the blind metric compressed it 3×

2026-09-08. Two of three arms complete; the third is running. The headline is not the blend number.

## The measurement

20 generations, seed 20260907, `--threads 1`, everything identical but `--blend`. Verdict is the
built-in final control against the **FROZEN ORIGIN** — the metric that resolved capacity, draws and
horizon — never the candidate-vs-champion gate rate.

| arm | vs frozen origin | decisive games, gen 20 |
|---|---|---|
| blend 0.75 (shipped) | **0.847 ± 0.022** | 1170/2400 (48.8%) |
| blend 0.25 | **0.735 ± 0.025** | 498/2400 (20.8%) |
| blend 0.00 | *running* | *pending* |

**Difference +0.112 ± 0.033.** Resolved, and larger than the ~0.07 between-run band, so it is not a
seed artefact.

`target = (1 - blend) * z + blend * root` (main.rs:352). Blend is the weight on the net's **own
search score**; `1 - blend` is the weight on **who actually won**. The arm that leans 75% on its own
search beats the arm that leans 75% on ground truth by a wider margin than any lever in the ceiling
investigation.

### The arms are controlled, and there is a check that proves it

Both arms report **dec 389/2400 at generation 1** — byte-identical. Generation 1's self-play happens
before any training, so an identical count confirms the seeding is deterministic and the arms
diverge *only* downstream of the training target. Without that check "blend changed the outcome"
would be indistinguishable from "the two runs were seeded differently".

## The part that outlives the blend question

The original blend sweep (main.rs:307-345) was measured **against a trained champion** — the
candidate-vs-champion gate rate, since measured near-blind here: 0.500 ± 0.007 on pairs a frozen
anchor separates easily, with non-transitivity shown directly.

Same pair of arms, two instruments:

| | blind metric (vs champion) | working metric (vs frozen origin) |
|---|---|---|
| 0.75 − 0.25 | +0.036 ± 0.016 | **+0.112 ± 0.033** |

**Same direction, ~3× the magnitude.** So the blind metric *compresses* but does not *invert*.

That asymmetry is worth more than the blend answer, because it says exactly which old conclusions
survive:

* **A DIRECTION measured on the blind metric is probably safe.** It got the blend ordering right.
* **A NULL measured on the blind metric is worthless.** Compression manufactures nulls. Any "these
  arms are indistinguishable" reading on that instrument cannot separate *equal* from *invisible*.

## The null this immediately invalidates

main.rs states, on the blind metric: "once the search score is in the target, the game outcome
contributes nothing measurable. blend = 1.0 (no outcome at all) matches the best mix." The evidence
is a flat plateau — 0.75/0.85/0.95/1.00 at 0.5258/0.5234/0.5293/0.5281, all overlapping.

That is a null on a compressing instrument, and it is now unsupported. Queued as `blend_hi.sh`
(1.00 vs 0.85 vs a re-run 0.75 control) on the frozen-origin metric.

The shipped default of 0.75 was chosen *over* the equal-measuring 1.00 on a stated risk argument —
a pure bootstrap off the current net has no anchor to reality and can drift. If 1.00 now measures
worse, that hunch earns real support; if better, it is a shippable gain on the highest-leverage knob
known.

## Pre-registered, testable in ~35 minutes

If the decisive-game rate tracks strength, **blend 0.00 should finish below both 0.735 and 498/2400
decisive.** Written down before the arm lands so it cannot be reinterpreted afterwards. Two arms is
not enough to claim decisiveness is a proxy — three that line up monotonically would be the first
cheap proxy in this project to survive contact, after `mcnemar_z` (r = −0.095) and training loss
(r = +0.379, wrong sign) both failed.

## What this does NOT claim

* **No improvement is available here.** 0.75 is already the shipped default. This is confirmation
  that a past decision was right — and it was right for a reason its own evidence could not
  support, since that evidence came from the compressing metric.
* **One seed, one 20-generation run per arm.** The difference clears the between-run band; the
  *shape* of the curve does not yet.
* Decisiveness is an **observation**, not a validated proxy. The 0.00 arm is its first real test.
