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

---

## RE-REGISTRATION (2026-09-11, after the control FAILED and before arm 2 finished)

The control did not behave as predicted: hash reuse went **8W-33D-7L, 15 decisive of 48**, when
identical play would have forced every pair into the middle bucket. Full account in
`identity_does_not_predict_games_RESULT.md`. The original arm-2 readings assumed a zero-decisive
baseline and are void.

**The failure supplies a better baseline than the one it destroyed.** A structurally different
program that agrees on 40/40 test positions gives a measurable decisive rate, and the real gate gives
another:

```
hash reuse vs seed (arm 1)        decisive  15/48  = 0.312 +/- 0.067
real gate matches (21 of them)    decisive  36/252 = 0.143 +/- 0.022
difference                        +0.170 +/- 0.070   ->  z = 2.41, SIGNIFICANT
```

**The gate's own candidates diverge in games LESS than hash reuse does** — and hash reuse is the
program the codebase documents as playing identically to the seed. So the mutation operators are
producing candidates that are, in GAME terms, closer to the champion than a transposition table is.

CAVEAT on that comparison, stated because it is not perfectly matched: the 21 gate matches mix MAIN
(budget 16) with MCTS (budget 256-1024), while `refmatch` runs at budget 16 throughout. The z = 2.41
is a first calibration, not a controlled contrast.

### Arm 2 now reads against 31.2%, not against zero

`capture` is a larger behavioural difference than hash reuse — an extension that exists to see
tactics a flat search misses, at 1.679x the cost.

* **decisive rate clearly ABOVE 31.2%** → the games respond in a GRADED way to how different a
  program is. The gate's problem is then that its candidates are too similar to the champion, and
  the lever is the mutation operators, not the gate.
* **decisive rate ~= 31.2%** → the decisive rate SATURATES: once two programs differ at all, games
  diverge at a fixed rate regardless of how much. That would make the gate's draw rate a property of
  the game conditions, and no operator change would fix it.
* **decisive rate near the gate's 14.3%** → capture extension behaves in games like an ordinary
  mutation despite being a hand-built structural change, which would say the position sets and the
  cost model are measuring something games are largely blind to. That is the most uncomfortable
  outcome and the one that would most change the plan.

The strength question (which program is better) is NOT part of this reading. Arm 1 already showed
0.510 +/- 0.020 is unresolvable at this pair count, and the same will be true of arm 2. **This is a
test about VARIANCE and divergence, not about Elo**, and it must not be reported as one.

## A MECHANISM, registered before arm 2 reported, because arm 2 is already its discriminator

The two measurements now on record point at the same thing from opposite sides:

* `identity:12/27` (44% agreement on the position set) with `W-D-L 0-11-1` — a near-total draw.
* hash reuse, ~100% agreement on the position set, **31.2% decisive**.
* real gate candidates, **14.3% decisive**, significantly BELOW hash reuse at z = 2.41.

**HYPOTHESIS: the mate guard selects for game-neutrality, and the game gate then cannot resolve what
the guard let through.**

The chain, each link already established in the tree rather than assumed:

1. `search_track_WHY_NOTHING.md`: the guard is `f >= best_found`, the seed scores 25/25, so **every
   surviving candidate also scores exactly 25**. Mates is a pass/fail filter, not a gradient.
2. Therefore every candidate that reaches the game gate solves the SAME tactical positions as the
   champion. The guard has already filtered out anything that changed tactical behaviour.
3. Tactical equivalence on the mate set implies similar play in the tactical moments that decide
   weak-engine games, so the games draw.
4. Draws collapse the pentanomial variance, which is the condition `gate_arithmetic_RESULT.md` proves
   makes acceptance impossible at 6 pairs.

If this holds, the fitness's two halves are in **direct tension**: the mate guard demands behavioural
PRESERVATION, and the game gate demands behavioural DIFFERENCE. A candidate cannot supply both, and
the search track's zero promotions follow from the fitness's own structure rather than from any bug.

**WHAT WOULD FALSIFY IT, and it is already running.** `capture` breaks tactical preservation by
construction — the extension exists precisely to see tactics a flat search misses. So:

* capture's decisive rate **clearly above the gate's 14.3%** → consistent with the mechanism. A
  program that changes tactics diverges in games; guard-survivors, which cannot change tactics, do
  not.
* capture's decisive rate **at or below 14.3%** → **the mechanism is WRONG.** Breaking tactical
  preservation would then not increase game divergence, and step 3 fails.

Registered now, with the discriminator already in flight, so this is a prediction rather than a
story told afterwards about whatever number arrives. If it survives, the next question is whether
the mate guard can be made a gradient instead of a pass/fail filter — which is exactly what
`hardn_probe` was trying to establish with the HARD set before it turned out to be measuring an
inert knob.

---

## NOTE ON THE FIGURES IN THIS FILE — they are deliberately NOT updated

The gate baseline quoted throughout is **14.3% decisive (36/252 over 21 matches)**, which is what was
measured at the time each reading was registered. The arms have since produced 29 matches and the
figure is **14.7% (51/348)**.

**The numbers above are left as written on purpose.** A pre-registration records what a prediction
was made against; editing it afterwards to match better data destroys the only thing it is for. The
result files (`gate_arithmetic_RESULT.md`, `gate_candidates_are_game_neutral_RESULT.md`) carry the
current figures and are the place to read them.

Effect on the readings registered here: **none.** Every threshold was crossed by a margin far larger
than 0.4 percentage points — capture came in at 41.7% against a 14.3% or 14.7% baseline either way,
and the falsifier ("at or below the gate's rate") is not close to triggering under either. Had the
margin been near the difference, this note would say the registration needed re-running rather than
annotating.
