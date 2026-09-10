# The origin control's node budget is STABLE to 1.9% — my "moving instrument" explanation is REFUTED

> ## ⚠ SUPERSEDED, same day, on the premise rather than the finding
>
> **The decline this file sets out to explain does not exist.** See `pooled_runs_RESULT.md`. The
> four points below were selected by `grep ... | tail -4` across a log holding **five runs** whose
> generation counter restarts at 1, so they are not one lineage: the file contains two different
> gen-400 rows, 0.870 and 0.819, and the "decline" is the splice between them. Segmented by run,
> the earlier run is FLAT (0.873/0.871/0.847/0.870) and the current one is RISING (0.819 → 0.837).
>
> **The measurement in this file is still valid and still binding** — the caps really are stable to
> 1.9%, and that was established against a threshold declared before the run. Only its motivation
> was wrong. Keep the "do not re-run `ctrl_fixed` on this idea" instruction; ignore the framing that
> a decline needs explaining.

**2026-09-10.** This file records a hypothesis of mine that failed, and the number that killed it,
so it is not proposed again. The decline it tried to explain is still unexplained.

## The thing being explained

The periodic origin control reported the champion getting steadily worse across gens 100-400:

| gen | control vs origin |
|---|---|
| 100 | 0.873 |
| 200 | 0.871 |
| 300 | 0.847 |
| 400 | **0.819** |

End to end **−0.054 ± 0.033 — resolved**. Over the same window `netmatch` at depth 4, the project's
declared strength standard, says gen400 **beats** gen200 head-to-head at **0.539 ± 0.027**. Both
intervals exclude their nulls. At most one of them measures what I was reading it as.

## The hypothesis, and why it looked strong

The control does **not** play at a fixed node budget. `main.rs` calls
`arch::equal_time_caps(&champion, &origin, budget_ns, ...)` on every invocation, and that derives
its caps from `ns_per_node()` — a **wall-clock timing probe**, run at that moment, on this box,
under whatever else is running.

So the operating point is an *output* of each reading, not a constant of the experiment. And the
load genuinely did change during the window: datagen went from 4 lanes to 8 (commit `5255de0`),
roughly doubling the competing work on the same cores. Starve both sides of nodes and both play
worse, more games draw, and the rate drifts toward 0.5 — **the observed direction**. A real
mechanism, a real load change, and the right sign.

## The measurement that refuted it

`cap_stability.rs` calls `equal_time_caps` on one **fixed** pair of nets, 12 times back to back.
The nets never change, so any spread is instrument noise by construction — there is no strength
signal available for it to be.

| | mean | sd | spread (max−min) |
|---|---|---|---|
| champion caps | 12,140 nodes | 52 (0.4%) | **1.3%** |
| origin caps | 11,323 nodes | 61 (0.5%) | **1.9%** |

**The threshold was declared in the file's header before the run**: under 5% refutes, tens of
percent confirms. It came out at 1.9%. The hypothesis is dead, and it is dead by a rule I wrote
down before I could see which answer it would give.

## What this does and does not license

**Does:** stop attributing the decline to a wandering budget. Do not re-run `ctrl_fixed` on the
strength of this idea — the header said not to, and this is the file that makes that binding.

**Does not:** it is not a general proof that the caps are load-*insensitive*. All 12 reps ran
within a second of each other under one load condition, so this measures short-timescale jitter,
not a 4-lane vs 8-lane shift. Stating that plainly rather than banking the stronger claim: the
strong form (the budget is jittery) is refuted; the across-load form is **untested**, and there is
no historical record of the caps to test it against, because until today they were never logged.

## What was fixed anyway

The control printed a rate and never the caps it played at, so no two readings could be compared —
the check above could not be done from the log at all, only by rebuilding. `main.rs` now prints
`[ca vs cb nodes]` on the control line. Not a fix for a live bug; a fix for an unfalsifiable log.

## Where the contradiction stands

Still open, with the leading explanation now the one already on record rather than mine:
`instrument_saturation_RESULT.md` documents the origin control **reversing sign twice**, at 0.861
and 0.967. The champion entered this window at 0.873 — **inside the band where this instrument has
already been measured wrong**. That is prior evidence, not a new idea, and it predicts exactly what
was seen: a control that misreads above ~0.86 while head-to-head against a recent ancestor keeps
working.

The head-to-head evidence available so far agrees with "not declining": in the 5-rung round robin
at depth 4, gen14 loses to every later rung — 0.312 to gen29, then 0.167 / 0.171 / 0.175 to
gen200 / gen300 / gen400. Those last three sit inside each other's intervals at 60 pairs, so
between gen200 and gen400 the honest reading is **level, not falling** — with `netmatch`'s better
powered 0.539 ± 0.027 the only resolved signal, and it points slightly *up*.
