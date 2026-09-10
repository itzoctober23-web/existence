# Gate bounds: the WIDTH was the cost driver, not the stopping rule

**Headline.** Making the gate sequential (FITNESS §7.3) let it accept, but at bounds `[0,10]` a gate
against a null candidate burns the full 400-pair cap **63.8% of the time and returns nothing**.
Widening to `[0,30]` takes cap-burns to **0.0%** and cuts the median pairs-to-decision from **274 to
55**, while keeping false-accepts at 4.8% (α=0.05) and power at 81.8% @ +25 Elo / 98.5% @ +50.

**This is a deviation from FITNESS §7.2 and is flagged, not settled — see the last section.**

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

## OPEN: this contradicts FITNESS §7.2

§7.2 specifies width `e1-e0 = 2` Elo with a bootstrap of `e1=5` (`[3,5]`) until 20 acceptances.
`[0,30]` is width 30. The measured reason for deviating: `[3,5]` needs ~2,673 pairs (~12h) to accept
a +10 Elo candidate, and `[0,10]` burns the cap 63.8% of the time. Neither is affordable at
iteration zero, where the live question is "did it learn to mate at all", not "is it +10 Elo".

Running `[0,30]` on the two SPRT arms pending a decision on §7.2. The fixed-gate arms on cores 12/14
are untouched as the old-gate baseline.
