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

## Clarification I owe on my own reporting: production promotions are NOT game-gated

While checking a champion change at 13:47 (generation 23056) I found the production run reports
**24,400 ACCEPT lines in 24,400 generations**, every one with `gate 0W-0D-0L 0.000+/-1.000`.

That is configuration, not a defect. `p1_production.sh:93` passes `--gate-every 1000000`, and the
run's own header confirms `gate-every=1000000` — the game gate effectively never fires in
production. Acceptance therefore rests on the mcnemar surrogate (`dec N/8` on the generation line),
with the ABSOLUTE RULER as the external check on whether that is working.

`main.rs:978-998` shows the decision is properly ordered where games ARE available:

1. games resolved the sign -> they outrank the surrogate, and REJECT on `resolved_down`;
2. else `ci95 < 0.05` -> precisely measured and straddling 0.5 -> reject;
3. else -> the surrogate decides.

In production only branch 3 is reachable, because with no games `ci95()` returns 1.0.

**What I owe:** I have been saying "the champion" and quoting its ruler level without noting that
production promotions are surrogate-based rather than game-gated. The ruler number (1515 +/- 12,
flat, `STATE.md`) is unaffected — it is an external measurement and does not care how the champion
was chosen. But "promoted" in the NET track means something weaker than "promoted" in the search
track, and I should not have used the word for both without distinguishing them.

**It also completes this morning's ledger correction.** I recorded 22 of 24 ledger accepts as an
"artifact" of a zero-game match. Half right: zero-game entries are EXPECTED for an ungated track, so
their existence is by design. The defect `gate.rs:72-83` documents was narrower — writing
`resolved = true` for a match that played nothing — and it is fixed, which the live log confirms by
printing `+/-1.000` rather than `+/-0.000`.

**And a coincidence worth noting.** `main.rs:990` gives its example of a precisely-measured null as
"0.510 +/- 0.020". That is, to three decimals, the hash-reuse control measured above. The reading
landed exactly on the shape the codebase already names as the canonical "no meaningful difference",
which is independent support for treating arm 1's strength question as unresolved rather than as
evidence of equality.

## CORRECTION to the clarification above — champion promotions ARE game-gated, by a separate daemon

I wrote above that "production promotions are surrogate-based rather than game-gated", reasoning
from `--gate-every 1000000` in `p1_production.sh`. That is true of the TRAINER's in-loop ACCEPT and
false of the thing I called a promotion.

`auto_promote.sh` is a separate daemon. Every 30 minutes it finds the live trainer's net, runs a
**224-pair netmatch against the champion**, and promotes only on the standard rule. Its record is in
`auto_promote.out` — which I failed to find earlier because I looked for `auto_promote.log`, a
broken probe rather than an absent log. Today's entries:

```
03:48 prod4.net      gen  2052: PROMOTED   0.531 +/- 0.027
13:47 prodk1056.net  gen 23056: PROMOTED   0.544 +/- 0.029
10:02 prodk0759.net  gen 19670: REGRESSION 0.454 +/- 0.027  (interval entirely below 0.5)
11:17 prodk1056.net  gen  1769: REGRESSION 0.459 +/- 0.029  (interval entirely below 0.5, PAST the resume transient)
01:08 lrB.net        gen  1392: PASSES but 2 arms training -- NOT promoting  0.593 +/- 0.029
14:23 prodk1056.net  gen 28785: hold       0.501 +/- 0.030
```

**The 13:47 champion change was a gated promotion at 0.544 +/- 0.029** — lower bound 0.515, clearing
`rate - ci95 >= 0.5` on 224 pairs. Not an unconditional checkpoint, which is what I implied.

The daemon is also doing more than promote. It REJECTS regressions with the interval entirely below
0.5, it annotates whether a reading is past the resume transient (the ~95-Elo effect
`resume_transient` records), and it refuses to promote at all while multiple arms are training —
`lrB` PASSED at 0.593 and was held because two arms were live, which is precisely the confound a
promotion mid-experiment would create.

**So the two-track picture is:** the trainer accepts every generation on the surrogate (by
configuration), and a separate 224-pair game gate decides what becomes CHAMPION. My earlier
paragraph conflated them and understated the rigour. The ruler figure (1515 +/- 12) is unaffected
either way, being an external measurement.

**What this cost:** nothing yet, because the claim had not been used for anything. What it shows is
that "a required daemon writes no log" should have been "I could not find its log" — the standing
rule about an empty result being a broken probe applies to files as well as greps.
