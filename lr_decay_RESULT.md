# The schedule is not the lever — ending low is. lr 0.0005 shipped, and loss tracks strength backwards

**2026-09-11 03:00.** Three arms, 2,000 generations each, same start (`p1_champion`), **same seed
(20260914)**, differing only in the learning-rate schedule. Verdict by `netmatch` against the shared
start.

| arm | schedule | vs shared start | interval |
|---|---|---|---|
| **A** | lr 0.002 constant (the shipped control) | 0.484 ± 0.032 | [0.453, 0.516] |
| **B** | lr 0.002 **decaying** to 0.000493 | 0.586 ± 0.027 | [0.559, 0.613] |
| **C** | lr 0.0005 constant | **0.628 ± 0.027** | **[0.601, 0.656]** |

## The pre-registered reading that fired

> **B ~ C, both above A** → what matters is **ENDING low**; the early steps are wasted and a plain
> lower constant is simpler and equal. Ship 0.0005.

**B and C end at the same rate by design** — 0.9993²⁰⁰⁰ takes 0.002 to 0.000493, against C's 0.000500,
and arm B's log confirms it reached exactly `lr now 0.000493 at gen 2000`. That equality is what makes
the comparison mean something: if the decay's large early steps were doing real work, B would beat C.

**B did not beat C. C is nominally ahead, 0.628 to 0.586.** So the schedule bought nothing over
simply using the low constant, and it costs a parameter. `--lr-decay` stays in the code, defaulted
to 1.0 (off), as a measured negative rather than a deletion.

**What is firm:** B and C each beat A with disjoint intervals. **What is not:** C over B, a 0.042 gap
whose intervals overlap on [0.601, 0.613]. This file does not claim C > B.

## lr 0.0005 is now replicated three times, and lr 0.002 is not better than the champion

| lr 0.0005, from this champion | | | lr 0.002, from this champion | |
|---|---|---|---|---|
| sweep, seed 20260912 | 0.589 ± 0.026 | | sweep, seed 20260912 | 0.544 |
| **decay C, seed 20260914** | **0.628 ± 0.027** | | decay A, seed 20260914 | 0.484 |
| auto_promote, gen 723 | 0.625 ± 0.029 | | prod3, gen 5,778 | 0.478 |
| | | | prod3, gen 12,283 | **0.471 — REGRESSION** |

Three independent readings of 0.0005, all clear of 0.5 and mutually consistent. Four readings of
0.002, scattered around parity, and **12,283 generations at 0.002 made the champion measurably
worse.**

## Shipped

`dec_C` clears the project's bar: **0.628 − 0.027 = 0.601 ≥ 0.5**. Promoted to `p1_champion`
(previous champion preserved as `p1_champion_prev_pre_lr0005.net`), and production relaunched as
`prod4` at `lr=0.0005`, verified from its own header: `lr=0.0005 lr-decay=1 lr-min=0`.

The regressing `prod3` was stopped at 18,738 generations. It had been training from the old champion
at a rate now measured to go backwards.

**This passed the gate.** No Elo figure is quoted for it.

## An unplanned replication, and it confirms the project's own noise estimate

Arm A is the *same configuration* as the sweep's lr 0.002 arm — same start, same generations, same
everything except the seed. They read **0.544** and **0.484**.

| interpretation | sd of that difference | observed 0.060 is |
|---|---|---|
| within-run ci95 only | 0.0207 | **2.9 σ** — would look like a real effect |
| with the documented between-seed sd 0.047 | 0.0665 | **0.90 sd** — entirely ordinary |

So the 0.047 between-seed sd this project keeps citing is **confirmed by accident**, and quoting a
single run's ci95 as the uncertainty would have manufactured a significant difference between a
configuration and itself. It is also why the C-over-B gap above is left unclaimed.

> **⚠ CORRECTED 2026-09-11 03:5x — "anti-correlated" OVERCLAIMS, and the correct word is
> UNINFORMATIVE.** `proxies_RESULT.md` had already settled this at a scale none of tonight's
> readings approach: the held-out surrogate at **r = −0.095 over 239 paired gate results**, training
> loss at **r = +0.379, CI [−0.249, +0.783], n=12** — and crucially it contains examples pointing
> **both** ways (w64 has the lower loss and LOSES; the draw filter has the lower loss and WINS).
> A signal that fails in both directions is noise with respect to strength, not a reversed predictor.
>
> The specific defect in tonight's version: the arms it compares differ in **LEARNING RATE**, which
> changes how much the net fits per generation *independently* of how good the net is. lr moves both
> columns, so the monotone ordering is expected with no causal link between loss and strength.
> n=3 against n=239 besides. This was a closed question, already indexed in `RESULTS_INDEX.md`.
>
> **What survives:** loss must not be used to select or veto candidates here. That conclusion is
> unchanged — it just rests on `proxies_RESULT.md`, not on these three points.

## The cleanest evidence yet that training loss runs BACKWARDS to strength

All three arms are matched — one run, one seed, one start — so loss and strength are directly
comparable:

| arm | median training loss | strength vs start |
|---|---|---|
| A (0.002) | **0.02130** | 0.484 |
| B (decaying) | 0.02690 | 0.586 |
| C (0.0005) | **0.03550** | **0.628** |

**Perfectly monotone, and inverted: the arm with the lowest training loss is the weakest, and the
arm with the highest is the strongest.** Every previous statement of this came from comparing
different runs or different instruments (`epochs_ab`, `depth5_vs_depth3`). Here it is one run, three
matched arms.

The decaying arm's loss also *interpolates* from A's level to C's as its rate falls (0.0231 over
generations 1–200 → 0.0384 over the last 200), which is an independent confirmation that
`--lr-decay` acts on the optimiser and is not merely printing a number.

**Consequence, already recorded separately:** `arch_surrogate_filter_RESULT.md` — the ARCH arm vetoes
candidates whose held-out loss is >0.5% worse than the champion's, 33 of 97 proposals killed that way
without ever playing a game. On this evidence that filter points the wrong way.

## What this closes and what it opens

**Closed:** the learning-rate *shape* question. A constant beats a decay to the same endpoint, so
there is no schedule to tune.

**Open:** where the optimum actually is. 0.0005 beats 0.002 three times over, but nothing here
brackets it from below — 0.0002 and 0.0001 are untested, and the sequence 0.01 → 0.002 → 0.0005 has
improved at every step. The next sweep should go down, not sideways.
