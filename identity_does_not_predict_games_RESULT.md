# Position-set identity does NOT imply identical games — measured, and PATH 1 rests on the inference

2026-09-11. A pre-registered control that FAILED, which is what makes it worth reporting.

## What was predicted, in writing, before the run

`refmatch_discrimination_PREREG.md` registered hash reuse vs the seed as the CONTROL, on the strength
of `evolve.rs:3283`:

> Hash reuse is exact alpha-beta: it agrees with the seed on 40/40 positions at depth 3 and 12/12 at
> depth 4, **so its game rate is exactly 0.5** and the gate rejects it permanently.

From that plus the pairing code (`gate.rs:191-203` plays the SAME opening from both sides and scores
it as a pair), I predicted: **rate exactly 0.500, every pair in the middle bucket, ZERO observed
variance, and therefore `ci95 = 1.5/n` = 0.0625 at 24 pairs.**

## What happened

```
hash reuse vs bare alpha-beta, 24 pairs at depth 3
  8W-33D-7L    rate 0.510 +/- 0.020    forfeits 0
```

**Fifteen decisive games out of 48.** Two programs that agree on 40/40 test positions won and lost
real games against each other. The predicted zero-variance reading did not appear: `ci95` is 0.020,
not 0.0625.

## What is and is not refuted

**RESOLVED, and it needs no more games: they do NOT play identically.** Identical programs cannot
produce a decisive pair — the same opening with colours swapped gives mirror games that cancel. Eight
wins and seven losses is a direct existence proof of divergence. No sample-size argument applies.

**NOT resolved: which is stronger.** 0.510 +/- 0.020 spans 0.5, and the run's own output says so:
"UNRESOLVED at this pair count -- needs more games, not a conclusion". Hash reuse may well be equal
in strength. That is a different question and this match does not answer it.

**The failure is in the INFERENCE, not in the pairing model.** The model says: *if* two programs play
identically, *then* pairs cancel. The control tested a conjunction, and the premise is what broke —
agreement on a fixed position set does not make two programs play the same games, because games visit
positions far outside that set, and a transposition table changes move ORDERING, which changes which
move is chosen among equals.

## Why this matters — PATH 1 accepts with NO GAME on exactly this inference

`evolve.rs:3291` grants acceptance when a candidate returns the same move as the champion on every
guard and hard position: *"identical play, fewer resources. No game is needed because there is
nothing to decide -- the two would play the same games."*

**"The two would play the same games" is the sentence this measurement refutes.** The file already
hedges it — "identity across 25 guard positions is strong evidence but not proof: a candidate can
match there and differ elsewhere" — and that hedge is now quantified rather than hypothetical: the
canonical identical-by-construction program produced decisive games in **31% of its games (15/48)**.

So PATH 1 can promote, with no game played, a program whose behaviour genuinely differs. That is the
exact false positive its own comment identifies as the risk of the path.

## What this changes in the day's other findings

**Stands unchanged:** the enumeration in `gate_arithmetic_RESULT.md` (all 210 six-pair outcomes, the
0.48 ceiling at >=4 drawn pairs) is arithmetic on the pentanomial formula and depends on none of
this. The measured 28.6% of gate decisions with zero observed variance is a count from logs, also
independent.

**Withdrawn:** my EXPLANATION of that 28.6%. I attributed those degenerate readings to behaviourally
identical candidates. The canonical identical candidate does not produce them, so whatever causes the
zero-variance gates, it is not this. Recorded as unexplained rather than explained wrongly.

**Second instance of the same decoupling today.** The instrumented gate line read `identity:12/27` —
44% agreement — with `W-D-L 0-11-1`, a near-total draw. Now the reverse: ~100% agreement on the test
set with 31% decisive games. **The position sets and the games are decoupled in BOTH directions.**
That is a property of the fitness worth its own investigation, and it undermines using either as a
proxy for the other.

## Arm 2 is not interpreted

Per the pre-registration: "If [arm 1] fails, arm 2 is not interpreted." The capture-extension arm is
running and its number will be recorded, but the question it was built to answer — can the games
discriminate a real behavioural difference — now has a partial answer from the control itself: the
games detected a difference the position set called identical. Arm 2 will be read only after the
control's failure is accounted for, and its reading re-registered.
