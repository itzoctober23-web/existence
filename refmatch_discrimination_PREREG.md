# PRE-REGISTRATION — can the gate's games discriminate at all? (written before the results exist)

2026-09-11. Two reference matches launched, 24 pairs (48 games) each at depth 3, budget 16, using
`evolve refmatch` — the SAME `match_progs` the search track's gate calls, so the numbers are
commensurable with in-loop gate results.

## Why

The instrumented gate line reads `identity:12/27` with `W-D-L 0-11-1`: a candidate agreeing with the
champion on only 44% of positions still drew eleven of twelve games. That refuted the near-identity
explanation in `gate_arithmetic_RESULT.md`, and left open whether the games can discriminate ANY
behavioural difference.

## The two arms and what each is for

**ARM 1 — `hash` (hash reuse vs bare alpha-beta).** `evolve.rs` documents this program as playing
IDENTICALLY to the seed: "it agrees with the seed on 40/40 positions at depth 3 and 12/12 at depth 4,
so its game rate is exactly 0.5". This is an effective A/A and is the CONTROL.

**PREDICTION, stated now:** rate exactly **0.500**, with the pentanomial concentrated in the
middle bucket and ZERO observed variance. Note the mechanism, because "identical play" does NOT mean
"all draws": within a pair the colours are swapped, so two identical programs either draw both games
or split them one-all — and both outcomes score the pair at 1.0 of 2. Either way the bucket is the
same and the variance is zero.

That matters because zero variance triggers the rule-of-three interval, `ci95 = 1.5/n`. At the gate's
6 pairs that is **0.250**; at 24 pairs here it is **0.0625**. So the control should reproduce, on
demand, the exact degenerate reading that `gate_arithmetic_RESULT.md` measured in 28.6% of real gate
decisions.

**ARM 2 — `capture` (capture extension vs bare alpha-beta).** A genuinely different program: the
extension exists to see tactics a flat search misses, and `evolve.rs` records it costing 1.679x.

**THE QUESTION:** does a real behavioural difference produce decisive pairs?

## Pre-registered readings

* **Arm 2 shows real variance and decisive pairs** → the games CAN discriminate, and the 85.7% draw
  rate in real gate decisions is about the CANDIDATES the operators produce, not about the game
  conditions. The suspect moves to the mutation operators.
* **Arm 2 also lands at ~0.500 with little variance** → the gate's game conditions cannot resolve
  even a program built specifically to search differently. That would make the 6-pair gate
  unfixable by pair count alone, and the fitness's game half would need rethinking rather than
  resizing.
* **Arm 1 does NOT come out at 0.500 / zero variance** → my model of the pairing is wrong, and every
  inference in `gate_arithmetic_RESULT.md` that rests on it needs re-checking before anything else
  is read.

Arm 1 is the control and is checked FIRST. If it fails, arm 2 is not interpreted.
