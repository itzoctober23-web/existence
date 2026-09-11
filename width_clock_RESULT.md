# Widening never pays on the CLOCK at this engine's speed — 11 attempts, 0 accepted, sign never flips

**2026-09-10.** The ARCH arm proposes a wider net, trains it, and gates it twice: at equal NODES
(`fixed-cost`) and at equal TIME (`clock`). Across 11 attempts spanning widths 32, 64 and 128 the
two gates disagree the same way every time.

| gate | range over 9 gated attempts | direction |
|---|---|---|
| `fixed-cost` — equal NODES | 0.490 … 0.583 | **8 of 9 ABOVE 0.5** — wider is better per node |
| `clock` — equal TIME | 0.190 … 0.467 | **9 of 9 BELOW 0.5** — wider is worse per second |

Zero accepted. Not one attempt produced a clock score above 0.5, at any width, on any generation.

## The mechanism is node loss, and it scales with width

| proposal | clock score | nodes searched (cand vs champ) |
|---|---|---|
| w16 → w32 | 0.379 – 0.467 | 6560–6872 vs 7154–7491 |
| w16 → w64 | 0.285 – 0.330 | 5674–6030 vs 7200–7423 |
| w16 → w128 | **0.190** | **4814 vs 7167 (−33%)** |

A wider net evaluates better and searches less in the same time, and the second effect wins by a
margin that grows monotonically with width. Nothing here is a near miss.

## What was ruled out before concluding

**The suspected circularity.** The incremental accumulator was once gated behind `n_hidden >= 64`,
which would have made this self-inflicted — wider nets losing on the clock precisely because the
thing that makes them cheap was switched off for them. `pipeline/src/search.rs:233` shows the gate
is gone: incremental is ON at every width unless `EXISTENCE_FULL_REFRESH` is set. **The clock loss
is real WITH the accumulator enabled**, not an artefact of its absence.

## What this closes, and what it points at

**Closed: width is not a lever at the current engine speed.** `--arch-every` was moved from 5 to
100 on this evidence, which took P1 from 81 to 1214 generations/hour — the widening search was
consuming 95% of wall clock to re-derive the same answer.

**Pointed at: the lever is SPEED, and the plan already names it.** The only thing that changes this
verdict is making evaluation cheap enough that the node loss shrinks — MASTER_PLAN's "the real
unlock is making deep search cheap enough that both hold at once: incremental accumulator, then the
bytecode". The accumulator has landed and is measured as a 0.91× LOSS at width 16 (`arch.rs:150`),
which is consistent with everything above: at narrow widths its bookkeeping is not amortised. It
should become a WIN at width 64+, which is exactly the regime ARCH cannot reach.

That is the shape of the remaining problem, stated precisely: **the accumulator pays where the
search cannot go, and the search cannot go there because eval is expensive.** Breaking it needs a
speed change large enough to move a clock score from 0.33 to above 0.5 — `bytecode_headroom_RESULT.md`
bounds the register bytecode at ~4%, which is not it.

**Not claimed:** that wider nets are worse in general. They are measurably BETTER per node, 8 of 9.
The finding is about a time budget, and it expires the moment the time budget buys more nodes.

## Replicated 2026-09-10 22:2x — from a NEW champion, and re-derived at full cost because I did not read this file first

`arch_widen_from_champion.sh` was launched at **`--arch-every 5`** — the exact value this file
records moving away from — against the champion promoted an hour earlier
(`depth5_vs_depth3_RESULT.md`). Three verdicts before it was stopped:

```text
  w 16 -> w 32   held-out 0.1059 vs champ 0.0828   -- surrogate filter, no gate
  w 16 -> w 32   loss 0.0893 vs 0.1316, paired z 4.44   fixed-cost 0.509+/-0.044   clock 0.445+/-0.045
  w 16 -> w 64   loss 0.0968 vs 0.1421, paired z 5.57   fixed-cost 0.464+/-0.045   clock 0.310+/-0.040
```

**Every number falls inside the ranges above** — fixed-cost 0.509 in [0.490, 0.583]; clock 0.445 and
0.310 in [0.190, 0.467]. Both widenings fit strictly better (paired z 4.44 and 5.57) and both lose
on the clock. Zero accepted, for the 12th, 13th and 14th consecutive attempt.

What is genuinely new is thin but real: the pattern **replicates from a different champion**, so it
is a property of the engine's speed rather than of one net.

**The cost, measured, because this file's own warning was the thing I ignored:** 5 generations/min
with ARCH on against 108/min with it off and every other flag identical — **95.4%** of the run, which
matches the "consuming 95% of wall clock" recorded above almost exactly. The source comment claiming
an ARCH step costs *"40% more games"* understates it by ~50× at `arch-every 5`; a proposal runs a
full paired match at two gates and takes ~55 s against ~0.5 s for a generation.

**Production runs now use `--arch-every 0`** (`p1_production.sh`), not the 100 recorded above: at
~108 gens/min a proposal every 100 generations still costs ~50%, because 100 generations and one
proposal take about the same wall clock. 100 remains the right setting for a run that wants a slow
trickle of vigilance on a closed question; a run whose purpose is champion progress should pay
nothing for it. Measured after the switch: **170 generations/min at 303% CPU, 0 ARCH lines.**

**The lesson is not about ARCH.** A closed question was re-opened because the launcher was written
from the plan rather than from the results directory. Reading one headline would have prevented it.
