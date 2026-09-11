# The diversity reserve has been run, and the run was far too short to mean anything

2026-09-11. A sizing note, not a result. Written to stop a documented mechanism being written off on
an underpowered run — the exact failure corrected in `hardn_inert_RESULT.md` earlier the same day.

## Two corrections to the current record

`ladder_valley_RESULT.md`'s head (2026-09-10 05:1x) says `EXISTENCE_DIVERSITY_SLOTS` is **"built but
NOT yet tested live"**. That was true when written and is now stale: `gate_diversity_s1.log`
(2026-09-10 **07:30**, two hours later) shows the reserve engaged from generation 7 onward, with
`dsl6` through `dsl9` on the generation lines. Nineteen generations, 24 gate calls, **0 accepts and
0 PATH-1 promotions**, best rate ever seen 1.000000x — a neutral twin.

**That is not evidence against the reserve.** Using the file's OWN published rate (#7,
`0.20 x 0.0108` per generation-trial) and its OWN pricing of the reserve (3.00x on the union rate):

```
 gens |  reserve OFF  |  reserve ON (3.00x)
   15 |      3.2%     |       9.3%
   19 |      4.0%     |      11.6%   <- the run that happened
   31 |      6.5%     |      18.3%   <- reproduces the file's own 6.5% figure, reserve OFF
   50 |     10.2%     |      27.8%
   80 |     15.9%     |      40.6%
  120 |     22.9%     |      54.2%
```

At 19 generations with the reserve ON, **finding nothing was the single most likely outcome (88%)**.
The arithmetic reproducing the file's published 6.5% at 31 generations is the check that this model
is the file's model and not a new one.

## What a real test costs

**107 generations for a 50% chance** with the reserve on. At the ~15 min/generation this box
currently sustains under load, that is roughly 27 hours of one arm.

So the honest framing for any future run:

* A **15-generation** arm has a **9%** chance. Running one and reporting "the reserve did not help"
  would be the hardn_probe failure repeated — a zero read as a refutation when zero was expected.
* The pre-registration must be stated in terms of the PLANNED N and the expected yield at that N,
  and a null must be reported as a BOUND, never as a refutation.
* If the budget will not stretch to ~100 generations, the honest options are to run it and report
  the bound, or not to run it — not to run 15 and conclude.

## Why this matters more than the gate work

The gate starvation measured in `gate_arithmetic_RESULT.md` does NOT block this path. Hash reuse is
exact alpha-beta — it agrees with the seed on every guard position — so a hash-reuse union plays
IDENTICALLY and takes **PATH 1**, which accepts on identity plus lower cost with **no game at all**.
The 6-pair gate never sees it.

So the valley track and the gate track are independent, and the valley track leads to the only
improvement this project has ever measured (the rung at 1.024x).

## Not claimed

That the reserve works. It has one run of 19 generations, which resolves nothing in either
direction, and the 3.00x pricing is a projection from `ttsupply` draws rather than a live
measurement. This note sizes the experiment; it does not pre-judge it.

## CORRECTION to the number above — 107 generations is a point estimate on a very thin base

Having published "~107 generations for 50%", I went and read where its inputs come from. The
projection `0.20 x 0.0108` decomposes as (`ladder_valley_RESULT.md:471-474, 529-531, 811`):

```
0.20    = P(a union is even ATTEMPTED in one generation)
          "measured from composition_s2's gen-6 population" -- ONE population, 2 of 10
0.0108  = P(an attempted union is acceptable)  -- the minimal-halves PATH-1 rate, 13/1200
```

**The 0.20 is a single observation of 2 successes in 10.** Its binomial 95% interval runs roughly
0.03-0.56, so the per-generation union rate spans about an order of magnitude and the 50%-chance run
length spans roughly **40 to 700 generations**. The file itself offers a competing figure at :598 —
`0.20 x 0.015 = 0.30%/generation` — which is a different second factor again.

The table above is therefore a POINT ESTIMATE, not a plan. It is still enough to establish the thing
it was written for: **19 generations could not have resolved anything at any plausible rate.** That
conclusion is robust across the whole interval. The specific "107" is not.

### What this changes about the experiment design

Do not run a fixed 107 generations against a rate known only to within an order of magnitude. The
run should MEASURE ITS OWN POWER, because the binding input is directly observable:

* `P(union attempted per generation)` is countable from the generation lines — it is a property of
  the population composition, which the `dsl{n}` field and the rate histogram already report.
* With that measured over the first N generations, the required run length follows from the run's
  own data rather than from one gen-6 population.

So the design is: run, instrument `P(attempted)` continuously, and report BOTH the union count and
the measured attempt rate. A null then yields a real bound — "over N generations at a measured
attempt rate of X, no acceptable union appeared" — instead of a number that cannot be interpreted
without an estimate the run never checked.

This is the same lesson as `hardn_inert_RESULT.md` one level up: there, a zero was uninterpretable
because the power was never computed. Here the power depends on a quantity the run can measure for
itself, so it should.
