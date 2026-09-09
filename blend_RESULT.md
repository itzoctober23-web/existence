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

## NEW — blend 0.85 beats 0.75 at DEPTH 4, where 1.00 does not

The settled block declares **blend 1.00** dead. It says nothing about 0.85, which had never been
directly matched — only scored against the frozen origin (0.809 ± 0.024, unresolved against
everything). Direct, 448 pairs, depth 4, the strength standard:

| pair | depth 4 | verdict |
|---|---|---|
| `bn_075` vs `bh_085` | **0.460 ± 0.022** [0.438, 0.483] | **0.85 stronger, interval clear of 0.5** |
| `bn_075` vs `bh_100` | 0.511 ± 0.022 (recorded, STATE.md:806) | no advantage |

So the response is **non-monotonic**: 0.75 → 0.85 gains, 0.85 → 1.00 gives it back. That is a
plausible shape — some weight on the game outcome anchors the bootstrap, and pure self-distillation
(1.00) has nothing holding it to reality, which is the exact risk argument that made 0.75 the default.
0.85 sits between the two.

**This is NOT a result yet, and the reason is quantified in this tree.** The gap is **+0.040**, and
`STATE.md:247` states that effects of 0.02–0.05 need **~19 seeds** to separate from a seed spread of
~0.045. One training seed cannot settle it, however clear the within-run interval looks — that is the
same trap that produced the withdrawn "+0.025 depth lever" and the withdrawn blend-1.00 case.

**What it does change:** the blend axis is not closed. The settled block's "blend 1.00 dead" is
correct and does not generalise, because the optimum is not at either end tested. Queued next:
* `bh_085` vs `bh_100` — is 0.85 better than 1.00 head to head? (blend_h2h's third match, now running)
* a second-seed 0.85 arm — `blend_seed2.sh` produced 0.75 and 1.00 only, so 0.85 needs one 20-generation
  run (~8 min at the measured 24 s/generation) plus one match. That is the replication that matters.

## ⚠ The "depth-2 only" mechanism does NOT replicate — it reverses on the second seed

`STATE.md:230` kills blend 1.00 with a specific mechanism: *"dead — advantage is depth-2 only (depth
shift z = 4.4)"*. Direct 448-pair matches on the SECOND training seed give the opposite pattern:

| seed | depth 2 | depth 4 (strength standard) |
|---|---|---|
| 20260907 (`bn_075` vs `bh_100`) | 1.00 wins — 0.450 ± 0.016, reproduced here at 0.461 ± 0.022 | **no advantage** — 0.511 ± 0.022 |
| 424242 (`s2_075` vs `s2_100`) | **unresolved** — 0.502 ± 0.022 | **1.00 WINS** — 0.456 ± 0.024, clear of 0.5 |

On seed 20260907 the advantage is depth-2-only. On seed 424242 it is depth-4-only. **The interaction
that justified closing the candidate does not survive a second training seed.**

### What this does and does not establish

* It does **not** revive blend 1.00. One seed says it wins at depth 4, one says it does not; that is
  a null across seeds, not a win.
* It **does** refute the stated MECHANISM. "Advantage is depth-2 only, z = 4.4" was measured on one
  training seed and reads as a property of the setting. It is not — it flips.
* It is exactly what `STATE.md:247` predicts: effects of 0.02–0.05 need **~19 seeds** against a seed
  spread of ~0.045. Every direct blend reading so far (+0.050, +0.039, +0.040, −0.044) sits inside
  that band, and the signs have now pointed both ways on both depths.

**The honest state of the blend axis: nothing about it is established at the strength standard.** Not
that 1.00 is dead, not that 0.85 is better, not that the effect is depth-dependent. Four one-seed
readings disagreeing is what a null with a wide seed band looks like. The z = 4.4 depth-shift figure
should be read as within-seed only, and `STATE.md:230`'s parenthetical dropped or qualified.

This also puts the new 0.85 finding in its place: **it is one seed, in the same band, and it should
be believed exactly as much as the "depth-2 only" claim it sits beside** — which is to say, pending
its replication, now running.

## Seed 20260907: a complete, mutually consistent triangle at depth 4

All three pairs matched directly at the strength standard, 448 pairs each, arms matched at 20
generations:

| pair | rate | effect | verdict |
|---|---|---|---|
| 0.75 vs **0.85** | 0.460 ± 0.022 | +0.040 | 0.85 stronger, clear of 0.5 |
| 0.75 vs 1.00 | 0.511 ± 0.022 | +0.011 | no advantage |
| **0.85** vs 1.00 | 0.567 ± 0.022 | +0.067 | 0.85 stronger, clear of 0.5 |

**They are transitively consistent**, which is a real check and not a formality: from the first two,
0.85 should beat 1.00 by about +0.051; measured +0.067. Three independent 448-pair matches agreeing
on the ordering **0.85 > 0.75 ≈ 1.00** is much harder to produce by chance than any one of them.

That said, the largest effect here (+0.067) is 1.7× the between-seed sd of 0.039, so it needs ~3
seeds. Transitive consistency shows the three matches are measuring the same thing on THIS seed; it
says nothing about whether the ordering survives a different training seed. Both remaining
second-seed matches are running:
* `s2_075` vs `s2_085` — the direct replication of the +0.040
* `s2_085` vs `s2_100` — the third leg, so the second seed gets its own triangle

**Why this matters more than the individual numbers.** The shipped default is 0.75 and the settled
block closed the axis by killing 1.00 — but 1.00 was the wrong end to test. If 0.85 holds up, the
default is on the wrong side of an interior optimum, and the risk argument that chose 0.75 over 1.00
("a pure bootstrap has no anchor to reality") does not apply to it: 0.85 still carries 15% of the
game outcome.

## REPLICATED: blend 0.85 beats the shipped 0.75 on both training seeds

Both second-seed legs landed, and both point the same way as seed 20260907:

| pair | seed 20260907 | seed 424242 | effect (s1, s2) |
|---|---|---|---|
| 0.75 vs **0.85** | 0.460 ± 0.022 | **0.403 ± 0.022** | 0.040, **0.097** |
| **0.85** vs 1.00 | 0.567 ± 0.022 | **0.534 ± 0.025** | 0.067, 0.034 |
| 0.75 vs 1.00 | 0.511 ± 0.022 | 0.456 ± 0.024 | −0.011, 0.044 |

Every one of the four 0.85 matches has its interval clear of 0.5, on two independent trainings, at
depth 4 — the strength standard. Cross-seed, with the pooled between-seed sd (now **0.0418**, six
estimates):

* **0.85 over 0.75: mean effect 0.0685, 95% CI [+0.011, +0.126] — EXCLUDES ZERO.**
* 0.85 over 1.00: mean 0.0505, 95% CI [−0.007, +0.108] — contains zero, not resolved.

**So the shipped default is beaten, cross-seed, by a setting the axis was closed without testing.**
The closure killed blend 1.00 — and 1.00 is the one end that genuinely does not help (0.75-vs-1.00
averages to +0.017 across seeds, well inside noise). 0.85 was never tried.

### The honest limits, stated before anyone changes a default

* **Two seeds, and the power calculation says ~2.9.** This is right at the boundary, not comfortably
  past it. The CI excludes zero by 0.011 — a third seed could move that.
* **The interval uses a POOLED sd** across six different net-pair comparisons. On these two points
  alone, n = 2 gives t(1) = 12.71 and nothing is resolvable. The claim is conditional on between-seed
  variance being comparable across comparisons, which is an assumption that keeps being re-tested and
  has so far held (movements 0.024-0.062, no outliers).
* **The two seeds disagree on magnitude by 2.4×** (0.040 vs 0.097). The direction replicates; the
  size does not.
* This is **"passed the gate" language at best, and not even that** — no gate has been run, no Elo
  measured. It is two head-to-head matches per seed.

**What it justifies:** a proper multi-seed run on 0.85, which is now the best-evidenced open candidate
in the tree. **What it does not justify:** changing the shipped default on two seeds, which is exactly
the error that produced the withdrawn "+0.025 depth lever" and blend 1.00's withdrawn mechanism.
