# Eval is NOT the throughput bottleneck at the champion's width — measured 2026-09-09

The loop brief has said, for as long as it has existed, that *"eval is a dense 256x782 forward pass
at every leaf and caps the engine at ~10k nps"* and calls fixing it the **biggest single win**. Both
halves are wrong for the engine that actually runs.

## Why this was worth measuring now

The depth 2x2 (`depth_2x2.sh`, equal generations, parity held fixed) resolved both of its depth rows
in favour of the DEEPER arm:

| contrast | result |
|---|---|
| d1 vs d3, within ODD | 0.450 ± 0.015 — d3 stronger, interval clear of 0.5 |
| d2 vs d4, within EVEN | 0.379 ± 0.016 — d4 stronger, interval clear of 0.5 |

So deeper labels ARE better per generation. But at equal WALL CLOCK the shallow arm wins, because it
completes ~12x more generations (48 vs 4). Both facts together say the lever is **throughput**, which
is exactly what MASTER_PLAN.md:616-617 predicts: *"the real unlock is making deep search cheap enough
that both hold at once: incremental accumulator, then the bytecode."*

That makes "what is actually slow?" the question that decides where the effort goes. It had never
been measured at the width the champion uses.

## The measurement

`search_bench`, depth 4, fixed position set. Node counts differ across widths (a different net is a
different tree), but **nps is normalised per node**, so its scaling with width attributes eval's
share directly. If eval dominated, 16x the width would cost ~16x the nps.

| hidden | nodes | nps | µs/node |
|---|---|---|---|
| 16 | 114634 | 796145 | 1.256 |
| 32 | 125263 | 676978 | 1.477 |
| 64 | 111011 | 502968 | 1.988 |
| 128 | 86521 | 408931 | 2.445 |
| 256 | 99320 | 260334 | 3.841 |

**16x the width costs 3.06x the nps, not 16x.**

Fitting `µs/node = fixed + k·width` on the endpoints gives k = 0.0108 µs per width unit and
fixed = 1.084 µs. The fit is checked against the points it was not fitted on: it predicts 699,790 nps
at width 32 (measured 676,978) and 406,150 at width 128 (measured 408,931).

**So at width 16, eval is ~0.17 µs of a 1.26 µs node — about 14%.** The other ~86% is movegen,
make/unmake and search overhead.

## What follows

* **Eliminating eval entirely at width 16 would buy at most ~14%**, nowhere near the ~10x that would
  be needed to make depth-3 datagen win at equal wall clock. Optimising eval is not the throughput
  answer at this width.
* This is consistent with something already measured and recorded in `pipeline/src/search.rs`: the
  incremental accumulator is a LOSS below width 64 (0.91x at hidden 32) and is switched off there.
  A component worth 14% cannot pay for its own bookkeeping — which is the same fact from the other
  side.
* The `~10k nps` figure is stale by roughly **80x**: the measured rate at width 16 is 796k nps.
* The **bytecode (CRATE 4) is also not the lever for this**, for a separate reason: datagen runs the
  native `pipeline/src/search.rs`, not the interpreter. Bytecode speeds up the EVOLVE loop, which
  runs programs. And `bench-interp` already passes at 0.98x of hand-written Rust against a 0.50 bar,
  so there is at most 2% there anyway.

## The honest open question

If eval is 14% and the interpreter is not in the path, the remaining 86% is movegen and
make/unmake. **This does not yet say that is optimisable** — it says that is where the time is, and
that the next throughput measurement should be a profile of that path rather than another eval
change.

One candidate is visible but OFF LIMITS by the plan's own rules: the seed search shuffles children
(`pipeline/src/search.rs`), deliberately, so alpha-beta gets no undeclared move-ordering prior. The
measured cost ladder is ~10x per ply (300 games: d1 1s, d2 3s, d3 31s) where well-ordered alpha-beta
would be nearer 6x. Move ordering is on the DISCOVERY list — it must be found by the loop, not added
by hand — so the cost of denying it is a deliberate, paid-for price, not a bug to fix.
