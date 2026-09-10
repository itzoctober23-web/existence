# The fitness set's discriminating power lives ENTIRELY in its non-mate-in-1 positions

`evolve ttgraft 40 <n1> <n2> <n3> 3` — three runs, the SAME 40 crossover children each time
(crossover is seeded, so the children are identical across runs and this is a paired comparison).
Only the position set changed.

## The measurement

| set | composition | kept all 25 mates | fitter AND correct | degenerate child #30 |
|---|---|---|---|---|
| pure mate-in-1 | 25 / 0 / 0 | 22/40 | **22/40** | 25/25 mates, **2934.933x** |
| mixed (the shipped default) | 15 / 5 / 5 | 0/40 | 0/40 | 19/25 mates, 2274.444x |
| depth-heavy | 5 / 10 / 10 | 1/40 | **1/40** | 11/25 mates, 1306.606x |

`n1` = `mate_set` (mate-in-ONE only), `n2` = `disagreement_set` (depth-requiring), `n3` =
`window_sensitive_set`. All three runs score against the same seed baseline of 25 mates at cost
9,930,290,911.

## What child #30 is, and why it matters

    cost 3,634,500 against the seed's 9,930,290,911  =  0.0366% of the work, a 2,732x reduction
    on pure mate-in-1 : 25/25 mates -> 2934.933x  "a spectacular improvement"
    on mixed          : 19/25 mates -> 2274.444x
    on depth-heavy    : 11/25 mates -> 1306.606x

**It does essentially no search.** A mate-in-one is a one-ply check, so a program that abandons the
search entirely still finds every one of them. This is FITNESS §10's declared degenerate solution —
*"Prune everything / return eval"* — realized exactly, and **the mate-in-1 stratum cannot see it.**
On a mate-1-only set it is the single best program measured in this project by three orders of
magnitude.

## The finding

**The 10 non-mate-in-1 positions in the shipped 25-position set are doing ALL of the discrimination.**
Drop them and 22 of 40 broken programs become "correct and fitter". Add more and the degenerate
child's mate count falls monotonically, 25 -> 19 -> 11.

Read the last column of the table as the real measure of a fitness set: **not how many candidates it
admits, but the best score an obviously-broken program can achieve on it.**

    pure mate-in-1   a null search scores 2934.933x
    mixed            a null search scores 2274.444x
    depth-heavy      a null search scores 1306.606x, and the best CORRECT child scores 1.283x

On the depth-heavy set the only child that passes the correctness guard scores **1.283x** — a
plausible, non-degenerate number in the same range as the hand-built `ab_hash` rung's 1.024x. On the
mate-1 set, 22 children pass with ratios up to 2934x and every one of them is wreckage.

## Why this is a FITNESS result and not a guard result

`ladder_valley_RESULT.md` records that `guard_tolerance` "converts an unreachable rung into a
reachable MATE SALE", and concludes no threshold repairs the ordering because on this fitness the
sale outscores the rung. This says where that bad ordering comes from: **60% of the shipped set is
satisfiable without searching at all.** The surrogate is not mis-weighted; it is being asked to rank
programs using positions that cannot tell a search from a no-op.

FITNESS §3 already specifies the fix and it is not implemented. The spec calls for **500 positions at
each of MATE-1, MATE-2, MATE-3 and MATE-4** — 2,000 stratified, reported per N. `mate_set` builds
**mate-in-ONE only** (it plays random moves and keeps a position where some legal move mates
immediately). So the spec's entire depth ladder is absent, and the only reason the shipped set
discriminates at all is the 10 positions from `disagreement_set` and `window_sensitive_set`, which
are doing the job MATE-2/3/4 was specified to do.

## What this refutes, including my own proposal from earlier tonight

`relaunch_bigset.sh` argued for a bigger set on the grounds that a 23-position set makes
`guard_tolerance` a coarse 17.4% licence, and proposed `n1=48 n2=24 n3=5` — 77 positions, **62%
mate-in-1**. That keeps the dilution almost exactly as it is. The measurement above says set SIZE was
never the axis: **composition is.** A 77-position set that is 62% mate-in-1 buys a finer tolerance
granularity on a surrogate that still cannot see a null search.

The direction is `n1` DOWN and `n2`/`n3` UP, or better, `mate_set` extended to real MATE-2/3/4 as §3
specifies.

## Honest limits

* One depth (3), one net (`Net::random(32, 20260907)`), 40 children drawn from one crossover
  distribution (`ab <- uct`). The MONOTONE trend across three sets is the claim; the specific counts
  are not portable.
* "Depth-heavy" here means more `disagreement_set` and `window_sensitive_set` positions, which is a
  PROXY for §3's MATE-2/3/4 ladder and not the same thing. Building the real ladder is the work this
  result argues for, and it would also let the per-N thresholds (0.9x on MATE-{1,2}, 0.8x on
  MATE-{3,4}) be applied as specified.
* 1 of 40 passing on the depth-heavy set is a small number to reason from; it says the set is
  strict, not that it is correctly calibrated.

## 2026-09-09 23:2x — FITNESS §3's ladder BUILT, and it does exactly what §3 says it will

`mate_within` / `mate_set_n` / `forcing_move` now build the stratified ladder, and `ttgraft ... 1`
scores against it. Same 40 seeded children as every run above.

| set | best score a CORRECT child reaches | null-search child #30 |
|---|---|---|
| pure mate-in-1 (25/0/0) | **2934.933x** | 25/25 — **passes** |
| mixed, shipped (15/5/5) | — (none passed) | 19/25 |
| depth-heavy proxy (5/10/10) | 1.283x | 11/25 |
| **LADDER 12 x MATE-1 + 12 x MATE-2** | **3.669x** | **12/24 — rejected** |

**Child #30 scores exactly 12 of 24: every MATE-1, zero MATE-2.** It does 0.0366% of the seed's
search, and a mate-in-two needs a real 3-ply search to find. It cannot fake one, and the mate guard
rejects it. That is the whole mechanism §3 was specifying, working.

**And the achievable score collapses to a sane range.** On a mate-1-only set the best *correct*
child scores 2934.933x — three orders of magnitude, all of it degenerate. On the ladder the best
correct child scores **3.669x**, in the same neighbourhood as the hand-built `ab_hash` rung's 1.024x.
A surrogate whose maximum is 3.669x can be reasoned about; one whose maximum is 2934x cannot.

### The bug I reintroduced, and the control that caught it

The first ladder scored the SEED at **12 of 24** — every MATE-1, every MATE-2 impossible. `fitness`
credits positions two different ways:

```rust
Some(best) => { if mv == *best { found += 1; } }   // credit the FORCING move
None       => { /* did this move mate ON THIS PLY? */ }
```

and `mate_set_n` pushed `None` for every stratum. **The first move of a mate-in-two never mates on
its own ply, so a `None`-labelled MATE-2 position scores 0 for every program forever.** The codebase
had already found and documented this exact defect — *"that bug made depth 1, 2 and 3 all read 0
until the control caught it"* — and I walked straight back into it.

Fixed by `forcing_move`, which recovers the proving move; MATE-1 keeps `None`, deeper strata carry
their move. Control after the fix: **seed 24 of 24.**

**Worth stating plainly: a stratum the champion itself cannot score is constant-zero across every
candidate and adds no discrimination at all** — it is pure cost. That is the same failure this
document opens with, in a new place, and the SEED SCORE is what detects it. Any future stratum must
be checked against the seed before it is trusted, exactly as `evolve ttk` checks the tag against
programs known to carry each half.

### Limits, and the open one

* **Ambiguity in the label.** A position may have several mate-forcing moves; `forcing_move` returns
  the first in move order and `fitness` compares by equality. So these strata measure "finds THIS
  forcing move", not "finds A forcing move". That is the existing mate-in-two convention, not
  something new, but it understates any program that finds a different sound mate.
* **Depth 4+ collapses, and it is NOT this bug.** The seed scores 24 at depth 3 and 2 at depth 4,
  with cost pinned near 4.7e10. Cost hits the same ceiling at depths 5 and 6 (4.80e10). The budget
  (16) is the binding constraint, not the depth, and **that means the harness currently cannot
  evaluate any program past depth 3.** MATE-3 needs 5 plies, so §3's third stratum is unreachable
  until that ceiling is understood. Open, and the next thing to measure.

### MATE-4 by random-play mining is measured infeasible — §3's retrograde walk is not optional

    MATE-1   0.001 s/position
    MATE-2   0.050-0.11 s/position
    MATE-3   3.17-5.34 s/position
    MATE-4   >840 s with ZERO of 3 found, then killed

**A lower bound, not a rate.** The probe had not exhausted its 400,000-try budget, so all this
supports is "more than ~157x the MATE-3 cost for even one position". The trend across the three
measured strata is roughly 50-60x per level, and MATE-4 is consistent with that or worse.

FITNESS §3 asks for 500 positions at each of MATE-1..4 and specifies they be **"mined by retrograde
walk from actual game endings in own self-play"**. On these numbers that phrasing is not a stylistic
preference: MATE-1..3 at 500 each is roughly half an hour of random-play mining and is affordable,
while MATE-4 is not reachable this way at all. The retrograde walk is the only route to the fourth
stratum, and it needs self-play games to walk back FROM — which at iteration zero do not exist yet.

**Consequence, stated plainly:** the ladder is buildable to MATE-3 today, and MATE-3 additionally
needs the cost ceiling raised (5 plies, ~90e9 per position extrapolated). The fourth stratum waits on
self-play, which is a dependency the spec implies and nothing in this repo had made explicit.

## ⚠ 2026-09-09 23:3x — CORRECTION: the codebase already ran this experiment, and already refuted the fix

Before launching an arm on "n1 down, MATE-2/3 up" I went looking for where to wire it in and found
**`forced_mate_set` (`evolve.rs:200`) — an existing mate-in-2 builder**, excluding mate-in-1
positions and labelled with the forcing move. That is `mate_set_n(_, 2)` with a different name. Its
doc comment states this document's opening finding, with a better measurement:

> *"On a mate-in-ONE-only set no amount of shallowness can lose a mate, so the 'must not lose mates'
> guard could never bite and the optimiser was free to drive cost to zero. It did: the first
> restarted run went 0.03 -> 8.73 mates/Mcost in ONE type-preserving edit, **333x cheaper** at an
> unchanged node count. So the repair is the SET, not the rule: the rule was always right and had
> nothing to enforce."*

**That is exactly the 2934.933x result above, found earlier and stated better.** My contribution here
is a replication, not a discovery, and the honest framing is that I re-derived a documented finding
because I designed the measurement before reading the function I was proposing to change.

**And the next comment down refutes the fix I was about to deploy** (`disagreement_set`, `:259`):

> *"The forced-mate-in-2 set was added to stop a candidate from simply searching less, and it worked
> at fitness depth 2. At fitness depth 3 it has no teeth: that set is solved 40/40 AT DEPTH 2
> (measured -- the forcing move is also the eval-best move), so cutting 3 -> 2 costs nothing on it.
> The search track promptly found exactly that: `Const(0)` -> `Const(1)` in the horizon guard, one
> ply shallower, 11x cheaper, all 20 mates intact. A depth guard must require the FULL fitness
> depth, and a mate-in-N does not imply N plies of search. **Disagreement does, by construction.**"*

### Reconciling the two results, because BOTH are true

| candidate | what it does | MATE-2 stratum | `disagreement_set` |
|---|---|---|---|
| child #30 | null search, 2732x cheaper | **CAUGHT** (12/24) | caught |
| `Const(0)`->`Const(1)` | ONE ply shallower, 11x cheaper | **BLIND** (40/40 at depth 2) | caught by construction |

A MATE-N stratum catches **gross** truncation and is blind to a **one-ply** cut, because mate-in-N
does not require N plies of search — the forcing move is often also the eval-best move. My ladder
measurement used a candidate so broken it fails any depth test at all, which made the stratum look
decisive when it is not.

**`disagreement_set` is strictly better for the failure mode that actually occurs**, and is
self-calibrating: it selects positions where the SEED answers differently at D-1 and D, so it
follows the fitness depth automatically instead of needing a new stratum each time depth changes.

### What survives, and what I am NOT doing

**Survives:** the mate-in-1 majority is the weakness (measured twice now, independently), and
`n2`/`n3` UP is the right direction — which is what the depth-heavy 5/10/10 arm tested and what the
codebase already endorses by having built `disagreement_set` at all.

**NOT doing:** wiring a MATE-2/3 ladder into the live loop, and not relaunching an arm on it. It
duplicates `forced_mate_set`, and at fitness depth 3 it is measured toothless against the candidate
class the search track actually produces. **`mate_within`/`mate_set_n`/`ttgraft --ladder` stay as
instruments** — they are how the above table was produced, and `mate_within` is exact-N and
disjoint where `forced_mate_set` is mate-in-2 only — but they are not the fitness set.

**The process lesson, which is the expensive part:** I measured before reading. `git grep mate_set`
would have shown `forced_mate_set` in one command, and its comment answers the question the
experiment was designed to ask. Two of tonight's other corrections have the same shape — the answer
was already in the repo, written down, by me or by an earlier pass.

## 2026-09-10 00:0x — LIVE PAIRED RESULT: the composition set rejects the mate-seller BEFORE the gate

`gate_composition_s1.log` (n1=4 n2=10 n3=10, 17% mate-in-1) against `gate_sprt30_s1.log`
(n1=12 n2=6 n3=5, 52% mate-in-1). Same `EXISTENCE_EVOLVE_SEED=1`, same gate settings.

**The trajectories are the same programs.** At gens 1-2 the two arms report
`rates 0.999-0.998700x` and `0.999-0.998708x`, and `0.497-0.496934x` against `0.497-0.496943x` —
differing in the fifth decimal only, which is what the SAME program scored on a DIFFERENT set. That
is the evidence the comparison is paired rather than two unrelated runs.

**Then they diverge, and the divergence is the whole point:**

| gen | control, 52% mate-in-1 | composition, 17% mate-in-1 |
|---|---|---|
| 3 | **gated** -> VERIFY 0.422, REJECT llr −3.18, **mates 19** of 23 | **mate-ok 0** — no candidate passed the guard |
| 4 | **gated** -> VERIFY 0.430, REJECT llr −3.03, **mates 20** of 23 | mate-ok 1, rates 0.999, `..none` |
| 5 | **gated** -> VERIFY 0.398, REJECT llr −2.97, **mates 20** of 23 | (not yet reached) |

The control's gen-3 candidate **sold 4 of 23 mates** for a cost cut, cleared the tolerance-4 guard,
consumed a 96-pair VERIFY and a 36-game gate, and resolved at **0.422 — decisively worse**. On the
composition set the same generation yields `mate-ok 0`: **the set refuses it outright.**

**What this does and does not claim.**

* **Claimed:** the depth-heavy composition rejects, at the guard, a candidate class that the shipped
  composition admits and that then measurably loses. That is the mechanism `fitness_set_composition
  _RESULT.md` predicted, now observed in the live loop rather than on grafted children.
* **Claimed:** it is cheaper. Each avoided gate saves 96 VERIFY pairs plus ~30 games, spent to learn
  something the set could have said for free.
* **NOT claimed: that this is progress toward a stronger program.** The pre-registration in
  `relaunch_composition.sh` says exactly this — "FEWER accepts than the control is EXPECTED and is
  NOT failure... the question is whether what it admits is better, which only VERIFY answers." The
  composition arm has admitted NOTHING so far. A set that rejects everything is trivially free of
  bad accepts.
* **NOT claimed:** that the gens-3/4/5 candidates are byte-identical across arms. The near-identical
  rates make it near-certain at gens 1-2; by gen 3 the populations may have diverged, since retention
  depends on the rates the set produces.

**The reading that matters comes later.** `search_has_no_choice_RESULT.md` measures that 94% of
generations offer ≤1 distinct fitness, so a stricter set risks pushing the choice count from ~1 to 0
— which is what `mate-ok 0` at gen 3 looks like. Whether this composition eventually admits something
that VERIFIES ABOVE 0.5, or simply starves the search, is the open question, and gen 6 is where the
earlier `tt` work says the interesting structure appears.

## 2026-09-10 — the composition effect REPLICATES at a second seed (p = 0.045)

`gate_composition_s2` runs the identical configuration at `EXISTENCE_EVOLVE_SEED=2`, same binary
(`xt_comp`), same set sizes, same gate settings — only the mutation trajectory differs. Launched
because the original result was **n = 1 seed**, and one sample is a lottery.

| arm | mate-in-1 % | generations reached the gate |
|---|---|---|
| control `gate_sprt30_s1` | 52% | **3 of 5** (gens 3, 4, 5) |
| composition seed 1 | 17% | 0 of 4 |
| composition seed 2 | 17% | 0 of 3 |
| **composition, both seeds** | 17% | **0 of 7** |

Fisher exact, two-sided: **p = 0.045**.

**What that does and does not license.**

* It says the depth-heavy composition is a **stricter filter**, and that the strictness is a property
  of the SET rather than of one lucky trajectory — which is the only thing the second seed was run to
  establish.
* It says **nothing about strength.** Every control gate RESOLVED WORSE (`VERIFY 0.422`, `0.430`,
  `0.398`), so the contrast is "reached a gate that then failed" against "never reached one". A set
  that admits nothing is trivially free of bad accepts, and the pre-registration in
  `relaunch_composition.sh` said exactly this before either arm ran: *"FEWER accepts than the control
  is EXPECTED and is NOT failure... the question is whether what it admits is better, which only
  VERIFY answers."*
* **p = 0.045 on n = 12 generations is weak** and should not be quoted as though it were a gate
  result. Two of the three "successes" in the control arm are consecutive generations of one
  trajectory, so they are not independent, which the test assumes.

**One thing the replication added that seed 1 never showed:** seed 2's gen 2 reports `distinct:2` —
TWO distinct fitness values among the guard-passers, both `>=.98`. Across all arms only 6% of
generations offer more than one distinct rate (`search_has_no_choice_RESULT.md`), so this is the rare
case where selection actually had a choice to make. It still did not accept, because both candidates
scored BELOW the seed.

**Standing question, unchanged:** whether this set ever admits something that VERIFIES above 0.5. If
both seeds reach gen 6 with 0 accepts, composition is not the constraint either, and the remaining
suspect is the correctness oracle under graft — 0 of 40, measured.
