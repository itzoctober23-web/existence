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
