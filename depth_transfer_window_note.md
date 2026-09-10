# Before reading the depth-4 arm: the two windows are NOT the same size

**2026-09-10, written before the matched depth-4 result exists.**

The comparison I was about to make is between this morning's rung pair and r9's, and it would have
been wrong on units.

| pair | what the labels mean | actual training span |
|---|---|---|
| `gen200` (11:53) → `gen400` (12:42) | **different runs** — labels are per-run counters, not a span | **49 min wall clock ≈ 1,470 generations** at ~1800 gens/hr |
| `r9.gen400` → `r9.gen800` | one run, one counter | **exactly 400 generations** |

This morning measured **+0.139 at depth 4** over the larger window. Scaled linearly to 400
generations that is **+0.038** — an expectation, not a measurement, and linearity is itself an
assumption this project has already been burned by (a pre-registered prediction failed on exactly
that assumption earlier today).

**The conclusion to avoid:** "gains stopped transferring to depth 4". Reaching it by comparing
+0.139 over ~1,470 generations against a smaller number over 400 would be reading window size as a
change in the loop — the same class as the run-pooling error that made a flat control look like a
collapse this morning.

**What the depth-4 arm can legitimately settle:** whether, *within the same 400-generation window*,
the depth-1 gain (+0.033, resolved, interval clear of 0.5) is larger than the depth-4 gain. That is
a within-window comparison with only the depth argument differing, and it needs no scaling at all.
Both arms use netmatch, 448 pairs, seed 20260907, the same two rung files.

The ancestor control's independent reading over the same window is 0.521 ± 0.034 (+0.021,
unresolved), which sits near the scaled expectation rather than obviously below it.
