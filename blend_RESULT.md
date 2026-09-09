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

---

## 2026-09-09 — the high side was never settled, and the determinism question it posed is answered

**`blend_hi.sh` did not produce a verdict, and the reason is in its own logs.** The two treatment
arms completed 20 generations (`bh_085` → 0.809 ± 0.024, `bh_100` → 0.832 ± 0.024 on the frozen
origin) but **the 0.75 control stopped at generation 9**, so there was nothing to compare them
against. The two treatment arms differ by +0.023 ± 0.034 — unresolved — and they were measured on the
instrument this tree has since documented as SATURATING, which reversed the sign on this very
comparison and on capacity w16-vs-w64.

**The determinism check it asked for is now done, and it passes.** `blend_hi.sh:41-42` says the
re-run doubles as a free determinism test — *"same binary, same seed, --threads 1. If the re-run
reproduces 0.847 exactly, the ~0.07 between-run band is purely SEED variance."* The first nine
generation lines of `bh_075c.log` are **byte-identical** to `bn_075.log`. So the run-to-run band is
**not** nondeterminism at a fixed seed; it is between-SEED variance, exactly as that line predicted.

That has a second consequence, which is what makes the question answerable at zero training cost:
`bn_075.net` is a completed 20-generation 0.75 arm on the same seed and settings, verified to follow
the identical trajectory. It stands in for the arm that died. `blend_h2h.sh` now runs
0.75 / 0.85 / 1.00 head-to-head at depth 4 — matched at 20 generations, judged on the strength
standard rather than the saturating origin metric.

### ⚠ RETRACTED — this was ALREADY SETTLED and I did not read the settled section first

**`STATE.md:221` is headed "✅ SETTLED (read this before anything below)" and line 230 reads
`blend 1.00 | dead — advantage is depth-2 only (depth shift z = 4.4)`.** STATE.md:800-807 has the
exact experiment I set out to run, as direct 448-pair matches:

```
bn_075 vs bh_100   depth 2:  0.450 +/- 0.016   RESOLVED, blend 1.00 stronger
                   depth 4:  0.511 +/- 0.022   UNRESOLVED, no advantage
shift              +0.061 +/- 0.027,  z = 4.40 -> the DEPTH EFFECT is resolved
```

So the blend candidate was closed before I started, by the same instrument and the same pair count.
My depth-2 cell (0.461 ± 0.022) is a REPRODUCTION of their 0.450 ± 0.016 on a fresh opening seed —
worth having as a replication, worth nothing as news. The standalone depth-4 run was an exact
duplicate and was killed.

**This is the documented failure mode, again: read every RESULT headline and the SETTLED block
BEFORE designing an experiment.** The section even says so in its own heading. What led me in was a
mid-file passage (`STATE.md:277`) framing the depth-4 evidence as coming from the weaker origin
control — true of that passage, but superseded 500 lines later by the direct match.

**What survives as genuinely new**, and is left running:
* `bn_075` vs `bh_085` at depth 4 — blend **0.85 has never been directly matched**, only scored
  against the origin (0.809 ± 0.024). The settled block covers 1.00, not 0.85.
* `s2_075` vs `s2_100` at depth 4 — a **second training seed**. The recorded depth-4 result is an
  UNRESOLVED null (0.511 ± 0.022 contains 0.5), and `STATE.md:247` states that effects of 0.02-0.05
  need ~19 seeds to separate from a seed spread of ~0.045. One seed did not settle it; a second does
  not either, but it is the difference between one null and two.

---

### (superseded) Depth 2 reproduces on the same nets


`bn_075.net` vs `bh_100.net`, 448 pairs, **depth 2**, fresh opening seed 777:

```
bn_075.net scores 0.461 +/- 0.022  (interval [0.439, 0.483])
=> B is stronger, interval clear of 0.5
```

That is **+0.039 for blend 1.00**, against the **+0.050 ± 0.016** on record at depth 2 — consistent,
and now confirmed on a different opening draw. The depth-2 half of the blend case is real and
replicates.

The whole question is therefore the one `STATE.md:277` raises: *"bh_100 is stronger at depth 2 and
not at depth 4."* That claim was based on the depth-4 equal-time **origin control**, not a direct
match, and the origin metric is 3.6× worse signal-to-noise than a direct match at equal games. So it
has never been tested properly. Four cells now settle it, all direct matches, all 448 pairs:

| | depth 2 | depth 4 (strength standard) |
|---|---|---|
| seed 20260907 (`bn_075` vs `bh_100`) | **0.461 ± 0.022 → 1.00 wins** | running |
| seed 424242 (`s2_075` vs `s2_100`) | running | running |

If 1.00 wins at depth 2 and loses at depth 4 **on both seeds**, the depth-dependence is reproduced
and the blend candidate is a depth-2 artifact — the largest lever on the board would be closed by its
own evidence. If it wins at depth 4 too, the shipped default of 0.75 is standing on a risk argument
that the measurements contradict.
