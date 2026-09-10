# PATH 1 (behaviour-preserving speedup) is CORRECT and EMPTY

The `evolve` loop has two acceptance routes. PATH 2 is the game gate. **PATH 1 accepts with NO GAME
at all**: a candidate that plays IDENTICALLY to the champion on every guard position and costs less
is a pure speedup, so no games are needed to justify it. That is the route the one known real
improvement — hash reuse, +2.4% — would have to take, because it changes nothing about play.

`evolve.rs` poses the question and leaves it open:

> *"IS THE SPEEDUP PATH REACHABLE? ... But 93 generations produced ZERO survivors cheaper than the
> champion, so the path may be correct and EMPTY — a fourth inert feature. Counting it here instead
> of waiting to find out."*

## The count, and it is the answer

`evolve stepdiff`, single edits from the seed, 125 of 200 attempts so far:

    broken     14
    identical  70   <- plays identically on every position
      of which CHEAPER: 0
    different  41   (guard-ok 0)

**70 behaviour-preserving mutants. Zero cheaper. Not one.**

Three independent measurements now agree that PATH 1 is empty:

| source | measurement |
|---|---|
| `evolve.rs` comment | 93 generations, ZERO survivors cheaper than the champion |
| `stepdiff` (this run) | 70 identical-playing single edits, **0 cheaper** |
| live arms | PATH 1 has never fired in any currently-running arm |

## The path is CORRECT — verified, not assumed

`gate_spec_veto_arm.log` (15:58) contains four `ACCEPT speedup` lines reading
`0.002490 was 0.002490` — identical play at IDENTICAL cost, promoted. That looks like a live defect
and is not one. `evolve.rs` records the cause: the "costs less" half was *"silently guaranteed by the
caller, because the strict filter picks only when `popn[0].2 > best_rate`"*, and
`EXISTENCE_SPEC_FILTER` breaks that unstated invariant by picking on `r >= 0.9 * best_rate`. Fixed in
`92a542a`. **Verified live**: the current condition is

```rust
if same_play && rate > best_rate {
```

a STRICT inequality, so identical-play-at-identical-cost can no longer promote. Those four lines are
pre-fix history from a SPEC_FILTER arm.

## What this means

**The one route that could admit hash reuse without playing a single game is open, correct, and
receives nothing.** Combined with the rest of tonight's chain:

* reachability is SOLVED — 5.6% of `ab<-uct` crossovers carry both TT halves;
* the rung is +104 nodes, so it is not one edit from the seed;
* single edits that preserve behaviour never reduce cost — 0 of 70;
* single edits that change behaviour never survive the guard — 0 of 41 (extends the repo's recorded
  0 of 33 to a larger sample, same result).

So a program can be *carried* to the rung by crossover, but it cannot *walk* there: every single step
is either a no-op that costs the same, or a change that breaks correctness. **There is no gradient in
either currency** — not in cost, not in mates.

## Honest limits

* Single edits **from the seed only**. The live loop applies 1-3 edits from evolved parents, and a
  no-op rate measured on the pristine seed need not hold on a mutated one.
* `cheaper` is measured on the guard set at depth 3 with `cost_cap` 20e9. A speedup that only pays at
  greater depth would be invisible here — and `ab_hash` is exactly such a case in miniature (+2.4% at
  this depth, more where a TT sees repeat traffic).
* 125 of 200 attempts at the time of writing; the ratios have been stable across all five checkpoints
  (0 cheaper at 15, 30, 40, 55 and 70 identical), but the run is not finished.
* This measures the SPEEDUP path only. It says nothing about whether PATH 2 (the game gate) is
  correctly calibrated — that is `gate_bounds_RESULT.md`.
