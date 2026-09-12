# The cost model charges MCTS and proof-number search ~2.9x more per real microsecond than alpha-beta

**2026-09-12 09:58.** `FITNESS.md:116` declares a revisit trigger for the cost model and it had
never been evaluated. It is evaluated here. **Read the ambiguity in the next section before the
headline** — one reasonable reading of the trigger says it is met and another says it is not, and
which one you take changes the verdict but not the finding underneath it.

## The check, verbatim

`FITNESS.md:116`, defining the cost unit that every fixed-budget comparison in this project is
denominated in:

> *"Wall time on the declared hardware is recorded alongside as the reality check on the cost model
> (revisit trigger: cost-vs-time correlation < 0.95)."*

Neither half was true before today. **Wall time is not recorded alongside anything** — no code in
`crates/pipeline` times a program run — and the phrase "cost-vs-time correlation" appears in no
result file. `RESULTS_INDEX.md` has no entry for it. It was a declared check that had never run, so
the cost model had never been compared against the reality it stands in for.

## THE VERDICT DEPENDS ON A CHOICE THE SPEC DOES NOT MAKE

The spec says "correlation" without saying in what space. Both are defensible and they disagree:

```
Pearson r, LINEAR  (cost vs microseconds)   = 0.7655   -> trigger MET
Pearson r, LOG-LOG                          = 0.9836   -> trigger NOT met
```

**I take the linear reading, and the reason is what the number is used for.** Cost is a *budget*: a
program is stopped when its running sum reaches a ceiling. The property that makes that fair is
linear — spending twice the cost should buy twice the work. Log-log correlation only asserts that
bigger programs take longer, which is true of any monotone pricing whatsoever, including one that
charges a single flat unit per node. That is precisely the "high by construction" failure this
harness was designed to avoid, and it should not be allowed back in through the choice of statistic.

**Stated plainly so it is not buried: a reader who meant log-log would conclude the cost model
passes.** The finding below does not depend on that choice.

## The measurement

`crates/interp/examples/cost_vs_time.rs`, depth 3 (operational), budget 800, 12 MATE-1 positions
mined with `ladder.rs`'s own seed, 7 interleaved repeats, all 13 programs of `reference::all()`.
Cost reproduced **exactly** across all 7 repeats for all 13 programs, which is the instrument's
self-test.

```
program                              cost        time_us   cost/us   rel_sd  capped
depth-one (purity seed)           1,066,296          366     2914     18.0%
bare alpha-beta (main seed)   6,182,222,628    3,622,547     1707      7.8%
alpha-beta + hash reuse       6,084,016,396    3,798,853     1602      7.0%
alpha-beta + iterative deep.  6,770,534,955    3,863,978     1752      7.9%
alpha-beta + hash + ID        6,677,843,803    4,078,025     1638      7.4%
UCT-style MCTS                5,888,660,467    1,263,487     4661      9.1%
UCT-style MCTS (blend)        5,765,959,381    1,191,038     4841      9.2%
capture extension (rung 6)   16,863,544,004   10,186,887     1655      7.8%   YES
extend-by-uncertainty         6,520,636,757    4,403,528     1481      8.1%
mix-backup (yardstick b)      1,813,701,550    1,296,418     1399      5.1%
bound-gap stopping              159,281,171       90,709     1756      9.1%
table reduction (rung 7)      6,234,740,270    4,008,127     1556      8.5%
proof-number search           6,477,283,722    1,429,346     4532      8.5%
```

## The finding, which survives either statistic

**The two paradigm families do not overlap.**

```
alpha-beta family   n=8   cost/us  1399 .. 1756   mean 1611
MCTS + proof-number n=3   cost/us  4532 .. 4841   mean 4678
                                                  ratio 2.90x
```

Per-program run-to-run noise is **5.1–9.2%**. The gap between families is **190%**. There is no
value of one family that comes within a factor of 2.5 of any value of the other. This is not a
marginal difference being read out of a noisy timer.

**What it means operationally.** Cost is the budget. A program charged 2.9x more cost per real
microsecond receives **2.9x less actual compute** for the same budget. So under FITNESS 3
(mates per cost), MCTS and proof-number search are running roughly a third of the search that an
alpha-beta program gets for the same price — and then being compared against it on the result.

That is a thumb on the scale, and it points in a specific direction. `FITNESS.md:110` explains that
the cost unit was chosen over eval-count *precisely* to be paradigm-neutral, singling out
proof-number search as the case eval-count would have mispriced. Proof-number search is measured
here at **4532 cost/us against alpha-beta's ~1611** — the unit chosen to protect it is penalising it
by a factor of 2.8. The purpose is sound and the calibration does not deliver it.

## Robustness, including the checks that made it look worse

```
r linear, all 12 in the fit            0.7655
r linear, excluding depth-one          0.6918    <- the 18%-sd outlier was HELPING the fit
r linear, only the 10 large programs   0.5149    <- drops the near-origin points
r log-log                              0.9836
```

Removing the noisiest point *lowers* linear r rather than rescuing it, and restricting to the
programs whose timings are most reliable lowers it further. The linear result is not an artefact of
one bad measurement.

`capture extension` is excluded from the fit: at least one single-position run hit
`Interp::cost_cap` (2e9), so its cost is clamped while its time is not. Its cost/us of 1655 sits
inside the alpha-beta band, so including it would not change the story — but a clamped point does
not belong in a correlation and it is reported rather than quietly kept.

## A bug in this harness that inverted the verdict, recorded because it was self-consistent

The first version flagged truncation by comparing each program's **total** cost across all positions
against `cost_cap`. The cap is **per run** — `Interp::run` sets `self.cost = 0` on entry — so a sweep
total legitimately exceeds it. The check therefore excluded both MCTS variants and proof-number
search as "capped", leaving a fit over nine alpha-beta programs that all share one primitive mix. It
reported **r = 0.9837 and a PASS**.

That number was plausible, agreed with the spec, and was wrong. The code also contradicted its own
comment, which stated the per-run rule explicitly while the line below it compared the total. The
guard that would have caught it faster is the one this file's design section already names: a
correlation over a single primitive mix is high by construction, so **an excluded-program count is
never cosmetic** — 4 of 13 excluded should have been read as "the test has lost its contrast", not
as a footnote.

## What this does NOT establish

* **Not which primitive is mispriced.** This measures the aggregate per-program rate. It does not
  attribute the 2.9x to `Eval` vs `Moves` vs `Apply`. The discriminator is
  `crates/interp/examples/cost_calibrate.rs`, which already exists and takes a width argument:
  re-run it and compare the measured ratios against the hardcoded `cost_of` table. **Not run here**
  (it would have contended with these timings and biased them).
* **Not a claim about production strength.** No games were played. Nothing ships. No figure here is
  Elo.
* **Not a validated cause, but a named suspect.** `cost_of(n: &Node)` takes only the node, so it
  **cannot** be width-dependent, yet its own comment says `Node::Eval(_) => 165` reflects an
  "incremental output layer at width >= 64 (from-scratch below it)" and notes the value "was 1365"
  when evals were from-scratch. This harness runs width 32. If eval is underpriced ~8x at the width
  actually in use, an eval-bound family would be charged too little per microsecond — which is the
  direction observed. **That is an explanation, not a measurement**, and `cost_calibrate` is the
  thing that would settle it.
* **The box was not quiet.** A trainer and a 4PC gate were running throughout. Mitigated by
  interleaving rep-major and reporting medians, so drift lands on all arms; the per-program sd of
  5–9% bounds what is left. A quiet-box re-run is the confirmation and was not available.

## Also found, and worth separating from the above

`configs/cost.toml` is **never read at runtime**. Every reference to it in the tree is a comment or
the generator's own `println`; the authoritative table is the hardcoded `match` in
`crates/interp/src/lib.rs:416`. CRATE 4 names the file as the cost table's home, so the file exists
and is described as the source while having no causal role — which is how its `eval = 1356` has
drifted from the code's `165` without anything failing. That is a documentation/architecture gap
rather than a behaviour bug, since the compiled values govern and ratios are what the budget needs.

## FOLLOW-UP, same session: the suspect is confirmed, and the attribution is 7.9x on `eval`

The section above named `cost_calibrate` as the discriminator and declined to run it while the
timing sweep was live. It has now been run at **width 32**, the width actually in use. The
prediction stated above — "if eval is underpriced ~8x at the width actually in use" — was written
from the source comment alone, before this measurement existed.

```
primitive   measured_ns   charged_units   units_per_ns
eval            284.1            165          0.581
moves           149.8           2232         14.900
apply           183.0           1959         10.705
terminal        152.3            703          4.616
```

A faithful model charges the **same units per nanosecond** for every primitive. These differ by
**25.7x**, with `moves` overcharged 25.7x relative to `eval`.

**Normalised to `terminal`, which avoids the unreliable baseline** (see the caveat below):

```
eval      should be 1.87x terminal, is charged 0.23x  ->  UNDERpriced  7.9x
moves     should be 0.98x terminal, is charged 3.17x  ->  OVERpriced   3.2x
apply     should be 1.20x terminal, is charged 2.79x  ->  OVERpriced   2.3x
```

**The predicted 8x and the measured 7.9x agree**, and the direction is exactly the one the
program-level result required: alpha-beta is eval-bound and eval is the underpriced primitive, so
alpha-beta programs are charged too little per real microsecond (1611); MCTS and proof-number search
lean on `moves`/`apply`/`terminal`, all overpriced, so they are charged too much (4678).

**The root cause, in one line from the calibrator's own output:** *"eval from-scratch was 258.5 ns;
incremental output layer is 284.1 ns (1x)"*. At width 32 the incremental path is **not faster — it
is slightly slower**. `cost_of`'s comment justifies the 165 as "incremental output layer at
width >= 64 (from-scratch below it)", and `cost_of(n: &Node)` cannot see the width, so a constant
chosen for the wide case is applied at a width where the optimisation does not pay.

**The caveat that keeps this honest.** `cost_calibrate` reports an integer op at **0.1 ns** — about
0.3 cycles — which is not a credible measurement of an integer op; it is at or below timer
resolution, and probably partly optimised away. Every ratio expressed "relative to one integer op"
inherits that, which is also why this run prints `eval = 2265` where `configs/cost.toml` records
1356 while the *absolute* eval time barely moved (284.1 ns vs 291.4 ns). **The denominator changed,
not eval.** The analysis above therefore normalises to `terminal` instead, and uses only primitives
in the 150–290 ns range where the timer is trustworthy.

**Why I believe this despite it being a microbenchmark.** An isolated timing bounds rather than
predicts, and regime can invert a ranking. The defence here is that two independent instruments
agree: a per-primitive microbenchmark says eval is underpriced ~8x and the moves/apply family
overpriced ~2-3x, and a *program-level* measurement that never touches those numbers finds
eval-bound programs charged 2.9x less per microsecond than moves-bound ones. The microbenchmark
predicts the sign and rough size of an effect measured a different way.

## CORRECTION, same session — the per-primitive multipliers above are WITHDRAWN

Immediately after publishing the section above I applied this project's own rule to it: *validate the
instrument against the code path it describes.* I had applied that to `key` (excluding it because the
calibrator times `zobrist()` while the interpreter does a field read `x.key`). **I had not applied it
to the primitives I actually used.** Doing so now:

| primitive | what `cost_calibrate` times | what the interpreter does | match? |
|---|---|---|---|
| `eval` | `score_with(&net, &mut scratch)` | output layer only, own scratch buffer | **yes** — the bench carries a comment saying it was written to match |
| `moves` | `legal_moves().len()` | `legal_moves()` **+ `as_slice().to_vec()`** — a heap allocation and copy | **no** |
| `apply` | `legal_moves()` + `child(..)` | **+ a `contains()` legality scan over the move list, + `Rc::new`** | **no** |
| `terminal` | `legal_moves().is_empty()` | `x.outcome()` | **not verified** |
| `key` | `zobrist()` | field read `x.key` | no — already excluded |

Every mismatch runs the same way: **the bench does LESS work than the interpreter**, so the true
per-primitive times are higher than measured, and by different amounts per primitive. A ratio built
from them is a bound, not a measurement.

**So these figures are withdrawn:** `eval` underpriced 7.9x, `moves` overpriced 3.2x, `apply`
overpriced 2.3x, and the 25.7x units-per-nanosecond spread. They were computed against `terminal` as
the denominator, and `terminal` is one of the unvalidated benches. The "prediction confirmed at 7.9x"
claim is withdrawn with them — a prediction matched against a mis-specified instrument is not a
confirmation, and the agreement with my predicted ~8x made it *more* persuasive rather than less,
which is exactly when this check matters most.

**What survives, and why it does not depend on any of the above:**

1. **The program-level finding — alpha-beta 1611 vs MCTS/PN 4678 cost/us, 2.90x, non-overlapping.**
   That was measured end-to-end on whole programs through the real interpreter. No calibrator bench
   enters it. It is unaffected.
2. **`cost_of` cannot be width-dependent** — it takes only `&Node`. That is a fact about the
   signature.
3. **At width 32 the incremental eval path is not faster than from-scratch** (258.5 ns from-scratch
   vs 284.1 ns incremental). This is a *within-bench* comparison of the same operation measured two
   ways, so the mismatch above cancels: whatever the bench omits, it omits from both arms. The
   comment justifying `Node::Eval(_) => 165` as "incremental output layer at width >= 64
   (from-scratch below it)" therefore rests on a crossover that does not pay at the width in use.

**So the direction is still supported and the magnitude is not.** Eval being underpriced remains the
live hypothesis; "by 7.9x" is not established. Settling it needs a calibrator whose benches invoke
the interpreter's own node paths rather than approximations of them — which is a fix to
`cost_calibrate.rs`, and is the real prerequisite for any re-pricing.

## RE-ESTABLISHED on a corrected instrument — eval is underpriced 7.8x

The correction above named the fix: a calibrator whose benches invoke the interpreter's own node
paths. That fix is now made (`crates/interp/examples/cost_calibrate.rs`) and re-run at width 32.

**The corrections behaved exactly as the diagnosis predicted, which is itself the check that they
were the right corrections:**

```
primitive   old bench   corrected   what changed
key            19.5 ns      0.8 ns  zobrist() recompute -> the field read `x.key`   (-96%)
apply         183.0 ns    217.3 ns  + contains() legality scan + Rc::new            (+19%)
moves         149.8 ns    174.0 ns  + as_slice().to_vec() heap allocation and copy  (+16%)
eval          284.1 ns    279.5 ns  unchanged -- this bench already matched          (-2%)
terminal      152.3 ns    153.2 ns  legal_moves().is_empty() -> outcome()            (+1%)
```

`eval` barely moving is the control: it was the one bench carrying a comment saying it had been
written to match the interpreter, and correcting the others did not disturb it. `key` collapsing by
96% confirms the specific diagnosis that `zobrist()` was pricing a from-scratch recompute the
interpreter never performs.

**The attribution, now on four benches that all match their nodes** (normalised to `terminal`,
because the 0.1 ns integer-op baseline remains untrustworthy and the absolute unit figures still
inherit it):

```
primitive   should be (x terminal)   is charged   verdict
eval                  1.824              0.235    UNDERpriced 7.77x
moves                 1.136              3.175    OVERpriced  2.80x
apply                 1.418              2.787    OVERpriced  1.96x
```

Units-per-nanosecond spread across the validated primitives: **21.7x** (`moves` vs `eval`).

**The withdrawn conclusion returns; the magnitudes moved 10–15%.** eval underpriced 7.9x -> 7.77x,
moves overpriced 3.2x -> 2.80x, apply 2.3x -> 1.96x, spread 25.7x -> 21.7x. The direction, the
ordering, and the identity of the mispriced primitive are unchanged. This is now supported by two
independent instruments — a per-primitive bench validated against the code it prices, and a
program-level sweep that uses none of it.

`key` is listed at "underpriced 3.67x" by the same arithmetic and that figure is **not worth
acting on**: at 0.8 ns it costs 0.5% of a `terminal`, so a 3.67x error on it cannot move any
program's total. It is reported only so the table is complete.

**What is still not established.** Absolute unit values (`eval = 2170`) remain unreliable while the
integer-op baseline reads 0.1 ns. Any re-pricing should be expressed as ratios against a primitive
that is robustly measurable — `terminal` at ~153 ns is the natural choice — rather than against the
nominal integer op the current table is defined in terms of.

**Not fixed here, deliberately.** Re-pricing `cost_of` changes the denominator of FITNESS 3 and so
changes every mates-per-cost number this project has recorded — including the GRAMMAR 9 ladder,
whose hash-reuse conclusion the existing comments already flag as needing re-derivation. That is a
pre-registered change with a declared re-derivation list, not an edit to make while a candidate arm
is mid-training. Nothing was changed.

## Consequence for GRAMMAR 4

`grammar4_addfn_unpark_blocker.md` records the AddFn unpark as blocked on the cost clause:
*"once something can diverge the lifted body from its origin, or the cost model stops charging a
bare call."* `Node::Call` is confirmed absent from `cost_of` and falls through to `_ => 2`.

**This result does not unblock that, and should not be read as doing so.** It answers the prior
question: the table one would be re-pricing is already failing FITNESS's own fidelity check on the
linear reading, and mis-ranks whole paradigms by ~2.9x. Re-pricing `Node::Call` in isolation would
be tuning one entry of a table whose calibration is in question. The order that follows is
recalibrate first (`cost_calibrate` at the operational width), then revisit the call price.
