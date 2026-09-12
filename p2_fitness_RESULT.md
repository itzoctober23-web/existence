# Growing `disagreement_set` made candidates LESS decisive, not more — pre-registered FAIL

**2026-09-11 23:47. Planned N complete** — 40 generations, `evolve` exited on its own. Verdict by
`p2_fitness_verdict.sh`, written at gen 34 and self-tested at gen 37, both before this outcome existed.

## The arms (paired by construction: crossover is seeded, only the position set differs)

```
CONTROL    10 + 4 + 5 = 19 positions, 53% mate-in-1   (prop_gens40, prop_gens40_RESULT.md)
TREATMENT   4 + 8 + 7 = 19 positions, 21% mate-in-1
same: 40 gens, pop 4, depth 3, gate 6 pairs. ONE change: the ratio.
```

## Pre-registered primary — treatment vs control, non-overlapping 95% CIs

```
                 decisions  games  decisive  fraction        CI
CONTROL              79      948      170     0.179   [0.155, 0.204]
TREATMENT            21      252       27     0.107   [0.069, 0.145]

-> FAIL: treatment BELOW control, CIs DISJOINT.    accepts: 0 in both.
```

**The direction is the opposite of the hypothesis.** The amendment argued that `disagreement_set`
requires the plies by construction and so should surface candidates that differ behaviourally. Weighting
it more heavily produced candidates that were **less** decisive.

## Power, quoted with the estimate rather than under it

Treatment 21 decisions / 252 games against the control's 79 / 948 — **3.8x fewer games**, so the
treatment is the weaker measurement. The CIs are disjoint anyway, so the verdict does not rest on the
thin arm being generous to itself; it is the arm that lost.

## The mechanism, measured during the run and not fitted afterwards

```
CONTROL     1 '..none' turn,   0 candidates above threshold,   0 gated-skipped
TREATMENT  56 '..none' turns, 70 candidates above threshold,  70 gated-skipped (100%)
```

A `gated-skip` is a candidate above the incumbent's rate that **has already been through the game gate
and lost** (`evolve.rs:3316`) — remembered, not blocked. So the harder set did not obstruct the gate; it
**exhausted the supply of novel candidates** clearing the incumbent, after which the population recycled
rejects. The control had a fresh candidate on almost every turn.

## What follows — the successor was named in advance

`p2_fitness_PREREG.md` recorded the reading for a non-positive outcome before the arm ran: *if candidates
barely differ behaviourally, changing what we weigh does not change what there is to weigh*, and the
named successor is **GRAMMAR 4 (type checker + mutation operators)** — which `gate_power_RESULT.md` had
already nominated independently. This result strengthens that: the composition lever was the best
remaining fitness-side idea, it was measured in the pre-registered direction, and it moved the metric the
WRONG way.

**Closed by this result:** composition re-weighting as a route to more separable candidates. **Not
closed:** the fitness question generally — `proxies_RESULT.md` ("only games measure strength here") and
`surrogate_inverts_RESULT.md` (the surrogate ranks the strongest program LAST) are untouched by this.
