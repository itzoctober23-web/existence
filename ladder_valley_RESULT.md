# The one verified rung is behind a VALLEY, and the valley is structural

`crates/interp/examples/evolve.rs valley 15 5 5 3` — deterministic, re-run reproduces the integers.

## The measurement

25 positions (15 mate-in-1, 5 depth-requiring, 5 window-sensitive) at depth 3, scored with the
search track's OWN `fitness`, on the same set that loop builds. Not a second harness.

| program | nodes | mates | cost | mates/Mcost | vs seed |
|---|---|---|---|---|---|
| bare alpha-beta (seed) | +0 | 25 | 9,930,290,911 | 0.002518 | 1.000x |
| probe only (never stores) | +69 | 25 | 10,023,369,775 | 0.002494 | **0.991x** |
| store only (never probes) | +39 | 25 | 9,962,055,071 | 0.002510 | **0.997x** |
| hash reuse (both halves) | +104 | 25 | 9,698,559,036 | 0.002578 | **1.024x** |

Mates are 25/25 for all four, so `f >= best_found` never discriminates; the entire decision is
cost, which is what makes the ratios exact rather than noisy.

**The control passed.** The full rung reproduces its gain on this set (+2.4%), independently of
the 120-position ladder that reported 0.98x cost. Had it not, the 0.98x would have been the thing
to re-examine and nothing else in the table would have meant anything.

## What it means

`evolve` accepts on `f >= best_found && rate > best_rate` — **strictly** greater. So:

* Neither half can ever be taken. Both are below 1.000x.
* Therefore the pair can never be assembled one edit at a time.
* And the pair is **+104 nodes**, which is not "1-3 mutations" under any reading.

**GRAMMAR 9 requires "a path of single mutations from the seed exists where every step is fitter".
For the ONLY rung ever measured as fitter than the seed, that premise is now measured FALSE.**

## Why this generalises past this set

The valley is not a property of these 25 positions, it is a property of what a transposition table
IS. `Interp::run` clears the table per position, so:

* probe-only searches a table **nothing ever wrote** — every probe is a guaranteed miss, so it is
  pure overhead by construction, not by sampling.
* store-only writes entries **nothing ever reads** — pure overhead by construction.

A TT is two edits whose payoff is *conjunctive*. Neither half pays alone, in this or any set. So a
larger or different position set moves the ratios and cannot move the sign.

## What this KILLS, before the time was spent

The obvious next move was to add mutation operators that can build `Probe`/`Key`/`Field`/`Store`,
since `tests/reachability.rs` proves the current operators cannot. **That work would not have
helped.** Reachability is necessary and not sufficient: the primitives being constructible says
nothing about a monotone path existing, and here it demonstrably does not. The operator work was
the natural thing to do next and it is now refuted for the cost of one 90-second probe.

It also means the reachability test's own note — "this failing means the single known improvement
is now inside the search space, re-run the search track" — was too optimistic, and is corrected.

## What it points AT

The bottleneck is the **search**, not the grammar. The valley is shallow — 0.9% and 0.3% — and
strict hill climbing is the only reason it is impassable. Options, in the order I would try them:

1. **Accept non-improving steps.** Plateau/drift acceptance (`rate >= best_rate * (1 - eps)`) with
   eps ~2% would let both halves be taken. Cheapest change; risks random walk, so it needs the
   `f >= best_found` mate guard to stay strict, which it does.
2. **A population with regression tolerance** rather than a single champion — the standard answer
   to a conjunctive-payoff valley.
3. **NOT** an operator that inserts probe+store as one edit. That is hand-coding the answer;
   MASTER_PLAN line 53 requires these to be DISCOVERED, and a gadget-inserting operator would make
   the discovery vacuous.

Option 3 is the tempting one and it is exactly the thing this project has agreed not to do.

## Honest limits

* One net (`Net::random(32, 20260907)`), one depth (3), one set. The SIGN is structural, per above;
  the MAGNITUDES (0.991 / 0.997 / 1.024) are specific to this configuration.
* This says nothing about whether hash reuse is worth having in the final engine — it is, at
  depths where a TT sees repeat traffic. It says the SEARCH TRACK cannot get there from here.
