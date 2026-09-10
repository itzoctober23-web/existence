# The GRAMMAR 8 gate PASSES, and "0.98x" was never a three-digit number

`crates/interp/examples/bench_interp.rs` is the acceptance measurement for GRAMMAR 8 / CRATE 11c:
the interpreted seed must reach >= 50% of a hand-written Rust alpha-beta on the same net. P0 records
it as **"interpreter gate PASSED 0.98x"**, and that figure is quoted as a result.

## What the one-shot instrument actually produced

Six consecutive runs, identical binary, identical data, nothing changed between them. The built-in
equivalence check passed every time (`evals: hand 3320 vs interp 3320 -> same tree`), so these are
not different trees -- they are the same measurement repeated:

    0.779   1.298   1.151   0.965   0.951   0.890        spread 1.67x

**0.98 is indistinguishable from anything in [0.78, 1.30] at this measurement length.** The default
run is depth 3: 3,740 nodes, ~14 ms, on a box carrying four evolve arms and six datagen lanes.

## Why this needed fixing rather than noting

The PASS is robust -- the bound is 0.50 and every reading clears it by a wide margin, so no
conclusion in the project changes. The FIGURE is not, and the figure is what gets compared. CRATE 4
puts a register bytecode next, explicitly as a throughput task; the natural way to judge it is
"the ratio moved from 0.98 to X". Against a 1.67x-noisy baseline that comparison is meaningless, and
it would have been made in good faith against a number sitting in the P0 line.

## The instrument now

* `reps` argument (default 9); the ratio reported is the **median** with `[min, max]` beside it.
* A spread guard: if `max/min > 1.25` it prints that the third digit is not resolvable and must not
  be compared digit-by-digit. It fires on the current contended box, correctly.
* The equivalence check is unchanged and still gates the whole reading.

    depth 3  reps 9   MEDIAN 0.886x  [0.654, 1.101]   spread 1.68x  -> warns
    depth 4  reps 5   MEDIAN 0.952x  [0.871, 1.359]   spread 1.56x  -> warns

**The depth-4 median (0.952x) is consistent with the recorded 0.98x within that spread**, so the
recorded claim is not refuted -- it is re-stated at the precision the instrument supports.

## Honest limits

The spread stays ~1.5x even at depth 4 because the box is contended, not because the run is short;
a quiet box would tighten it and the guard would fall silent. That is the guard behaving correctly:
it reports the condition, it does not assert a cause. Re-measuring on an idle box is worth doing
before the bytecode comparison, and the guard now makes it obvious when that has not been done.
