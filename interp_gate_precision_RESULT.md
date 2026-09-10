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

## The fix that actually worked: BEST-OF-N, not a warning

Reporting a median with a spread warning was honest but not useful -- the box carries four evolve arms
and six datagen lanes and will not be idle for nine days, so "re-measure on a quiet box" is not a plan.

**Contention noise is ONE-SIDED.** Another process stealing a core can only make a run slower, never
faster. So each arm's MINIMUM observed time is its least-contended sample, and the ratio of minima
estimates the true ratio far better than a median of ratios. Standard best-of-N benchmarking, and it
needs no quiet box and no new dependency.

Three independent invocations, depth 4, 7 reps each, on the SAME loaded box:

    BEST-OF-7   0.970x    median 0.962x  [0.942, 0.970]
    BEST-OF-7   0.963x    median 0.956x  [0.951, 0.975]
    BEST-OF-7   0.961x    median 0.957x  [0.940, 0.985]

**Spread of the quoted figure: 1.009x, against the one-shot instrument's 1.67x.** The equivalence
check passed on every run (42,820 evals both arms).

### And it confirms the recorded number

**0.961-0.970x against P0's recorded 0.98x.** The claim was right all along; there was simply no
instrument capable of demonstrating it. A future bytecode comparison against 0.96x is now meaningful
at the ~1% level, which is the whole point of having fixed this before CRATE 4 rather than after.

## Honest limits

The spread stays ~1.5x even at depth 4 because the box is contended, not because the run is short;
a quiet box would tighten it and the guard would fall silent. That is the guard behaving correctly:
it reports the condition, it does not assert a cause. Re-measuring on an idle box is worth doing
before the bytecode comparison, and the guard now makes it obvious when that has not been done.
