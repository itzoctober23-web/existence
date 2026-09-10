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
