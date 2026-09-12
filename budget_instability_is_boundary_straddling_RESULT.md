# The budget's instability is BOUNDARY STRADDLING — r = +0.98, and it predicts a larger budget is safer

**2026-09-12 08:00.** `search_stability_RESULT.md` established that a node budget's move choice is
~3.2 points less self-consistent than fixed depth's, and offered an explanation for the shape it
found. This tests that explanation mechanically. It holds, and it yields the first *actionable*
prediction in this chain rather than another elimination.

## The shape: instability is an INVERTED U in budget size

Same instrument, sweeping the budget, two seeds:

```
budget   mean realised depth   seed1   seed2    mean
  2000          2.27           -0.1    +0.3   +0.10
  3500          2.44           -1.7    -2.3   -2.00
  5269          2.63           -2.1    -3.1   -2.60   <- the operational budget
  8000          2.85           -4.0    -4.5   -4.25   <- peak, both seeds
 12000          3.13           -4.4    -2.1   -3.25
 18000          3.34           -1.9    -1.2   -1.55
```

At **budget 2000 the budget is as self-consistent as fixed depth** (+0.10, i.e. no instability at
all). Instability rises to a peak around 8000–12000 and falls again by 18000.

That falsifies the obvious reading of the earlier result. Instability is **not** caused by searching
shallowly: the shallowest budget in the sweep is the most stable one.

## The mechanism, measured rather than argued

`search_stability_RESULT.md` offered boundary-straddling as an explanation and explicitly flagged it
as "an explanation, not a measurement". So it is now measured directly: **the fraction of positions
that reach a DIFFERENT realised depth on the two runs.** If a position sits on a depth boundary, a
small rng perturbation decides whether another ply completes, and the two runs disagree.

```
             seed 20260912              seed 777001
budget    straddle%  instability    straddle%  instability
  2000       4.0%       -0.1           4.1%       +0.3
  5269       6.9%       -2.1           9.0%       -3.1
  8000       9.4%       -4.0           9.7%       -4.5
 18000       5.8%       -1.9           5.6%       -1.2
```

**Rank order is IDENTICAL between straddle and instability on both seeds, independently.** Pooled
over all eight points, **Pearson r = +0.981**.

So the inverted U is not a curiosity about depth: it is the straddle fraction, which is itself
non-monotone because at a small budget almost everything is pinned at depth 2, at a large budget
almost everything reaches 3 or more, and in between a large share of positions sit on the boundary.

## The prediction this makes, and it is testable

```
budget  5269 (in use)   straddle ~8.0%   instability -2.6   mean depth 2.63
budget 18000            straddle ~5.7%   instability -1.6   mean depth 3.34
```

**A larger budget is both DEEPER and MORE STABLE than the one in use.** Those usually trade off;
here they do not, because stability is governed by straddling rather than by depth.

If variance amplification is the operative mechanism — and its premise is the only one of three still
standing (`trajectory_drift_RESULT.md` contradicted drift, weakened decisiveness feedback) — then an
**18000-budget arm should be measurably less harmed than the 5269 arm**, while also searching deeper.
That is a single arm pair against the existing control, and it is the first forward prediction this
chain has produced.

**It is a prediction, not a recommendation.** `budget_harm_is_emergent_RESULT.md` showed every
single-pass channel is null and the harm needs 2000 generations to appear, so nothing here says a
larger budget helps in training — only that it is the arm the mechanism says to try, and that the
mechanism is falsifiable by running it.

## What is NOT claimed

* **Not that instability causes the harm.** The causal step is still unmeasured; this establishes the
  premise's structure, not its consequence. The measurement that would close it is the snapshot-variance
  arm pair named in `search_stability_RESULT.md`.
* **Not that 18000 is optimal.** The sweep has six points and the minimum of the right-hand limb is
  not bracketed — instability may keep falling past 18000, or rise again at the 3/4 boundary.
* **Not a strength claim.** No games were played. This is a search diagnostic.
* **Nothing ships**, and no figure is quoted as Elo.

## Why the peak sits where it does

Worth recording because it is the one number that looked wrong. The peak is at mean realised depth
**2.85–3.13**, which brackets the control's own fixed depth 3. That is not a coincidence and it is not
significant either: a budget tuned to buy *about* three plies is precisely the budget for which the
largest share of positions can go either way, because the control's depth is where the cost
distribution is centred (`datagen_node_census_RESULT.md` measured the depth-3 mean cost at 5,269 —
the operational budget was set to it). The operational budget was chosen to match depth 3 in
expectation, which unavoidably places it on the rising limb of the straddle curve.
