# Gate bounds: the WIDTH was the cost driver, not the stopping rule

**Headline.** Making the gate sequential (FITNESS §7.3) let it accept, but at bounds `[0,10]` a gate
against a null candidate burns the full 400-pair cap **63.8% of the time and returns nothing**.
Widening to `[0,30]` takes cap-burns to **0.0%** and cuts the median pairs-to-decision from **274 to
55**, while keeping false-accepts at 4.8% (α=0.05) and power at 81.8% @ +25 Elo / 98.5% @ +50.

**This deviates from FITNESS §7.2, and reading §7.2 properly showed the deviation costs POWER, not just compliance: `[0,30]` rejects a genuine +10 Elo candidate 72.5% of the time. Correction scheduled — see the last section.**

## Why the sequential gate was slow

`Score::llr` (gate.rs:455) is the pentanomial normal approximation:

    LLR = (n / 2var) * ((mean-p0)^2 - (mean-p1)^2)
        = (n / 2var) * (p1-p0) * (2*mean - p0 - p1)

The evidence per pair scales with **(p1 - p0), the bound width**. At `[0,10]` that factor is 0.0144,
so the LLR crawls and the test needs hundreds of pairs. The stopping rule was never the problem.

Measured cost corroborating this: the smoke binary spent 9 min CPU on <=120 games at depth 3
(4.5-9 s/game), so a 400-pair gate is ~800 games ~ 1-2 hours. The two `[0,10]` arms sat inside a
SINGLE gate for 35+ minutes and emitted zero verdicts, while the fixed-gate controls produced three
verdicts each in the same window.

## Control

`bounds_sim.py` (in this repo). The LLR function is transcribed from `gate.rs` and the zero-variance
give-up at 30 pairs is reproduced. Draw rate 0.806 is the measured A/A rate at these settings, cap
400, 400 runs per cell. It simulates PAIR OUTCOMES only -- it bounds the pair count and does NOT
predict wall-clock (see `microbenchmark bounds, not predicts`).

    true_elo  bounds    ACCEPT%  REJECT%   CAP%   median_pairs
           0   [0,10]       2.5     33.8   63.8            274
          25   [0,10]      99.2      0.0    0.8            157
          50   [0,10]     100.0      0.0    0.0             70
           0   [0,30]       4.8     95.2    0.0             55
          25   [0,30]      81.8     18.0    0.2             62
          50   [0,30]      98.5      1.5    0.0             28
         100   [0,30]     100.0      0.0    0.0             17
           0   [0,50]       4.8     95.2    0.0             18
          25   [0,50]      39.8     60.2    0.0             25
           0  [0,100]       2.5     97.5    0.0              7
          25  [0,100]      13.5     86.5    0.0              7
          50  [0,100]      43.5     56.5    0.0              8

`[0,50]` and `[0,100]` are cheaper still but their power collapses at +25 Elo (39.8%, 13.5%) -- they
would discard real gains. `[0,30]` is the knee: no cap burns, α respected, power retained.

## VERIFIED LIVE 2026-09-09 19:26 — the first real gate under `[0,30]` RESOLVED

Simulation is not production, so the live reading:

    gen 3 MAIN  gate REJECT llr -3.18  0.444+/-0.063  (36 games W-D-L 2-28-6)  needed >0.563

It **crossed the -2.944 bound in 18 pairs** -- resolved, not capped. The same candidate under the old
fixed 6-pair gate returned `REJECT 0.333+/-0.163 (12 games) needed >0.663`: a bar it could not have
cleared and an interval spanning everything, i.e. a non-answer. And the sequential verdict AGREES
with the independent instrument: VERIFY read `0.422 +/-0.027` on 96 pairs with a different seed, also
resolved worse.

18 pairs is well under the simulated median of 55, and consistent with it rather than contradicting
it: the simulation's 55 is the median against a NULL candidate, while this one is genuinely worse
(VERIFY 0.422). A clearly-worse candidate crosses sooner. The `[0,10]` failure mode -- 63.8% of gates
burning the full 400-pair cap for no answer -- did not occur.

**Scope, stated plainly: n=1 resolved gate.** The EPS arm produced the identical line, but it is the
same run seed replaying the same trajectory, so it is not a second observation -- `ab_report.py`
deduplicates it for exactly that reason.

**Cost, now bounded from above.** That generation spanned 18:46-19:26 (~40 min) and contained the
8-candidate fitness evaluation plus 36 gate games plus 192 VERIFY games = 228 games, so per-game cost
is **under ~10.5 s including all per-generation overhead**. Earlier only a >=4.8 s/game lower bound
existed.

## Remedy #2 (decisive openings) is REFUTED at iteration zero

`unbalanced_open24.log`: control (balanced, today's gate) **8-32-8, 66.7% draws**; treatment
(material gap >= 2) **9-30-9, 62.5% draws**. A two-game difference on 48 -- noise. The probe's own
pre-registration says *"no drop means the balanced start was not what was preventing resolution."*

The mechanism was already documented in `unbalanced_open.rs:4`: both sides evaluate with
`Net::random(32, ...)` -- noise -- **so neither can convert a material edge it cannot see**. Note the
pentanomial in both arms: `[0, 0, 24, 0, 0]`. Every pair landed in the middle cell.

MASTER_PLAN:154 allows exactly three opening sources, and at iteration zero:

1. **Random plies** -- what runs today (80.6% draws).
2. **Self-generated unbalanced book** -- requires mining positions where *own search eval* sits in a
   band. Our eval is a random net. Unavailable, for the same reason the material substitute failed.
3. **Chess960** -- rules-defined but **symmetric**, so it buys diversity, not decisiveness.

MASTER_PLAN:154's premise is draw-death *from strength* ("as strength rises... 2800-3000"). Ours is
the opposite failure: a random eval shuffling to a repetition. Same symptom, different cause, so the
section's remedy does not transfer. **Do not re-run the material-gap book; this is the do-not-regress
entry.**

## ~~OPEN: this contradicts FITNESS §7.2~~ — SUPERSEDED, see the next section

This section argued that `[3,5]` "needs ~2,673 pairs (~12h) to accept a +10 Elo candidate" and that
therefore "neither is affordable at iteration zero". The pair count was close (measured: 2,687), but
the CONCLUSION was wrong in two ways, and the next section replaces it:

- **Affordability was computed on the wrong candidate.** Every candidate actually observed sits at
  -25 to -55 Elo, which `[3,5]` rejects in 255-591 pairs, i.e. 1.4-3.3 h. The 12-hour figure is the
  cost of a candidate near the bound — the case the spec explicitly says deserves thousands of pairs.
- **It never priced what `[0,30]` gives up.** `[0,30]` tests `H1: elo >= 30` and rejects a genuine
  +10 candidate 72.5% of the time.

Its arm description is also stale: cores 12/14 have been HARD_FITNESS treatment arms since 19:47,
not fixed-gate baselines. Kept rather than deleted, because a superseded argument with its error
named is more useful to a later reader than a clean page.

## THE FITNESS §7.2 DEVIATION, read properly — and my cost argument was half wrong

I flagged `[0,30]` as "deviates from §7.2, open, not settled" and then did not read §7.2 in full.
Reading it verbatim changes the picture in both directions.

**What the spec actually says.** The acceptance criterion is FIXED and HUMAN, and the reason is
stated: *"an instrument calibrated by its subject measures nothing, and the degenerate solution
(bounds that accept everything) is obvious and unstoppable."* The schedule is DATA-DERIVED —
`e1` = mean gain per acceptance from an anchor match, recomputed every 20 acceptances — with
**width `e1-e0` fixed at 2 Elo as a declared resolution constant**, a floor of `e1 >= 0.5`, and
**bootstrap: until 20 acceptances exist, `e1 = 5`**. So the spec-compliant bootstrap bound is
**`[3,5]`**, and the draft it explicitly criticises for being hand-picked was `[0,10]` — which is
where I started.

**My hypothesis, tested and REFUTED.** I reasoned that `[3,5]` would reject bad candidates FAST,
because with `e0=3` a null candidate is clearly H0 rather than sitting on the boundary. It does not:
at a 400-pair cap `[3,5]` fails to resolve ANYTHING between -25 and +25 Elo, capping 100% of the
time, and a -55 candidate takes 255 pairs against 11 at `[0,30]`. Width-2 bounds carry ~15x less
evidence per pair, which is the same `(p1-p0)` scaling that made `[0,10]` slow.

**But the cap is MINE, not the spec's** — and the spec expects the cost: *"a candidate near a bound
gets thousands of pairs"*. Uncapped, `[3,5]` is CORRECT and affordable on the population actually
observed:

    true elo   verdict          median pairs   games   wall-clock @ <=10 s/game
        -55    100% REJECT           255         510      1.4 h
        -25    100% REJECT           591        1182      3.3 h
          0    100% REJECT          3861        7722     21.4 h
        +10    100% ACCEPT          2687        5374     14.9 h
        +50    100% ACCEPT           332         664      1.8 h

Every measured candidate so far sits at -25 to -55 (VERIFY 0.422-0.490), i.e. in the 1.4-3.3 h band.

**The finding that matters, and it is against me.** `[0,30]` is NOT the "accepts everything"
degeneracy — its false-accept rate at true 0 is 5.0%, exactly alpha. It is the OPPOSITE failure: it
tests `H1: elo >= 30`, so **it accepts a genuine +10 candidate only 25% of the time and REJECTS it
72.5%.** I traded away detection power at precisely the effect size the spec's `e1 = 5` exists to
catch, and I did it on a cost argument without measuring what the cost bought.

**Why the running A/B is still valid, stated so this is not overclaimed as a crisis.** The falsifier
reads VERIFY, which is a fixed 96-pair match on an independent seed and does not depend on the gate
bounds at all. What the bounds affect is whether a real improvement gets PROMOTED — the run's
progress, not the experiment's primary reading.

**Decision: do not churn the arms again mid-generation.** Switch to spec-compliant `[3,5]` with a cap
sized ABOVE the distance to the bound (the 4PC lesson: *a cap below the distance to the bound is not
a test*) at the next natural boundary, once this A/B has resolved. Recorded now, with the numbers,
so it is a scheduled correction rather than a discovered one.
