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

---

## The profile: `legal_moves()` is the biggest cost, and the deliberate shuffle costs as much as eval

`node_profile`, width 16, 200 positions, **best-of-9**. The minimum over repeats is used rather than
the mean because all four background cores were busy with time-boxed experiments; the fastest repeat
is the one that suffered least interference. Same technique `alloc_probe.rs` used to resolve 13.9 ns.

| primitive | ns/op | share of a leaf node |
|---|---|---|
| `legal_moves()` | **451.2** | **44.4%** |
| eval | 260.7 | 25.6% |
| shuffle + buffer copy | 257.4 | 25.3% |
| make + unmake (pair) | 47.1 | 4.6% |
| attributed total | **1016.4** | vs **1256** measured by `search_bench` — **81%** |

**The two independent methods agree on eval.** The width-scaling fit put eval at ~172 ns per average
node; the direct measurement is 260.7 ns per CALL, and eval is called only at leaves. Those reconcile
at a ~66% leaf fraction, which is what a depth-4 alpha-beta frontier looks like. Neither measurement
was derived from the other.

### This inverts the brief's ordering

The loop brief ranks the accumulator first. Eval is **third**, behind movegen and the shuffle, and
its share is ~26% of a LEAF and less of an average node. Meanwhile `legal_moves()` alone is 1.7x
eval, and `make/unmake` — the thing an incremental accumulator would piggyback on — is 4.6%, which is
why the incremental path measured a LOSS below width 64 and is switched off there.

### The shuffle is a real, unmeasured, and *deliberate* cost

`pipeline/src/search.rs` shuffles children at every node so alpha-beta cannot inherit an undeclared
"try pawn moves first" prior from movegen emission order — MASTER_PLAN puts move ordering on the
DISCOVERY list, so it must be found by the loop, not handed over. That is correct and is not in
question here.

**What is new is the price: 257 ns per node, 25.3% of a leaf, the same as the entire eval.** The
denial is principled; paying a quarter of throughput for it was never a measured decision.

And it is likely reducible *without* weakening the denial. The shuffle does one integer `%` per
element — `rng % (i + 1)` — which is a division, ~20-40 cycles, roughly 30 times per node. A
modulo-free unbiased mapping is still a uniform shuffle and still denies the prior; it just draws a
different permutation. That changes exact reproducibility from a given seed, so it is a deliberate
change with a determinism cost, not a free win — FITNESS 10 requires a determinism check, and the
seeds would need re-baselining.

### Honest limits

* Microbenchmarks are cache-hot; a real search touches scattered positions. That biases these
  numbers DOWN and is the most likely home of the missing 19%, along with recursion and bounds
  checks. So treat the shares as a ranking, not a budget.
* **The first version of this profile double-counted.** It called `p.legal_moves()` inside the timed
  shuffle loop, so the shuffle came out at 647 ns — larger than movegen itself, which is impossible
  for a Fisher-Yates over ~30 elements. The move lists are now hoisted out of the timed region. Same
  class as the eval-count equivalence check: an arm doing extra work reports a cost that is not its
  own, and the tell was a number that could not physically be right.
