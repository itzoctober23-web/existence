# The search track's pipeline runs end to end for the first time

2026-09-08. Not an improvement — a precondition for one, which the loop did not previously have.

## Before

**93 generations, 0 accepts, 0 game-gate invocations.** Nothing ever passed the correctness guard,
so the surrogate never proposed anything, so the games never had a candidate to judge. Three
components, none of which had ever run against a real input.

Measured cause (`search_track_WHY_NOTHING.md`): **0 of 33 behaviour-changing single edits pass an
all-or-nothing guard.** Everything that changes play fails `f >= best_found`; everything that
passes plays identically. The guard was in practice "do not change behaviour".

## After

```
gen 1 MAIN  gate REJECT 0.417+/-0.103 (12 games)  surrogate 0.002794
gen 1 MCTS  gate REJECT 0.500+/-0.250 (12 games)  surrogate 0.003431
```

Every stage fired on the first generation:

1. **Guard tolerance (4)** admitted a behaviour-changing candidate — one losing ≤4 of the 25
   alpha-sensitive guard positions.
2. **Surrogate** rated it **0.002794 against the champion's 0.002443 — 1.14×**, so it proposed it.
3. **Game gate** played 12 games and returned **0.417 ± 0.103**.
4. **Rejected.** Not resolved *below* 0.5 either (0.417 + 0.103 = 0.520), so the correct reading is
   "not proven better", which is the right default.

**This is the first live case of the surrogate and the games disagreeing about a real candidate.**
A 14% surrogate improvement that loses on the board is precisely the failure mode the game gate was
added to catch, and until now it had never been handed anything to catch.

## What it took, and every piece was itself a measured fix

| fix | why it was needed |
|---|---|
| `pred` implemented | was a stub returning false; capture extension had a byte-identical eval count to the seed |
| rung 6 moved to the horizon | applied at every depth, costing 73× and truncating |
| alpha-sensitive guard | the window guard varied INF (symmetric) while the exploit raised alpha (asymmetric); exploit loss 2 → 5 |
| guard tolerance 4 | measured to admit 6/33 genuine while rejecting both known exploits |
| absolute floor | `best_found - tolerance` would let the champion ratchet down k×4 over k accepts |
| selection filter actually using the floor | the first version changed only the printout |

## What this does NOT claim

* **No improvement has been found.** Both first-generation candidates were rejected. The loop can
  now take a step; it has not taken a good one.
* **The exploit margin is one.** Genuine candidates admitted at 4, nearest known exploit at 5, on a
  sample of two adversaries. A third exploit losing 4 walks through.
* 12-game gates resolve ±0.10 at best. They are a veto on bad candidates, not a strength measure.
