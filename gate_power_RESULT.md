# The P2 game gate: why 0 accepts in 203 decisions, and what replaced it

**STATUS 2026-09-09, end of day — the symptom is explained, the cause is measured, and the fix is
BUILT AND RUNNING. This file was written top-down over a day and later sections CORRECT earlier ones;
read this summary first.**

| question | answer | where |
|---|---|---|
| Why 0 accepts in 203 decisions? | The bar was never inside the achievable range. Max rate ever seen **0.542**; the strict bar needs **> 0.5 + ci95** | §1 |
| Is it cost-blindness? | No — probe returned INCONCLUSIVE and its premise (`capture_extension` is stronger) was false | §"cost-blindness" |
| Is it the acceptance rule? | Partly, but not mainly | §"supersedes both" |
| What is it mainly? | **46.8% of decisions measured NOTHING** — `gate.rs` returns a `1.5/n` placeholder on zero variance | §"CORRECTION TO MY OWN HEADLINE" |
| Why zero variance? | The games are **80.6% draws** on the real population; the no-signal case is `0-12-0`, every game drawn | §"DEFINITIVE" |
| Why so drawish? | The gate plays with `Net::random(32, …)` — an untrained net — from a **balanced** 4-random-ply start | §"ROOT CAUSE" |
| So: more pairs? | **Half right.** Correct for the ~53% that measure something; useless for the drawish half, where more pairs buy more draws | §"DEFINITIVE" |
| Is depth the lever? | No — halves draws but delivers **0.33x the information per CPU-second** | §"depth" |
| What IS the fix? | **Sequential SPRT** (FITNESS 7), which spends evidence where it is ambiguous. Built, wired, running on two arms | §"SIZING" |
| What did sizing cost? | Two configurations that would have guaranteed a non-answer: `max_pairs=100` and the [3,5] band | §"SIZING", §"CAN THE GATE ACCEPT" |

**Superseded claims, left in place with their corrections rather than deleted:** "mean ci95 0.177" (it
averaged placeholders with measurements), "0.250 is the number that matters for sizing" (the A/A
distribution is degenerate), "not one candidate has won a game" (one has, 1 in 60), and "202
decisions" (203). Each is marked where it appears.

## The measurement

Across **every gate decision in every log in this repo** — 203 decisions (snapshot; the count grows while arms run):

| quantity | value |
|---|---|
| decisions logged | **203** (snapshot 2026-09-09; grows while arms run) |
| sample size, every one | **12 games = 6 pairs** (`evolve.rs:1297`, `gate_pairs` default 6) |
| ACCEPTs, ever | **0** |
| mean ci95 | 0.177 |
| smallest ci95 ever seen | 0.082 |
| **max `pent_rate` ever seen** | **0.542** |

The strict acceptance rule is `pent_rate - ci95 > 0.5`, i.e. `pent_rate > 0.5 + ci95`.
At the **smallest error bar ever observed** that bar is **0.582**.
The **highest rate ever observed in 203 decisions is 0.542**.

So the bar has never once been inside the range of outcomes the instrument can produce. The 0/203
accept record is not a coincidence, not a run of bad candidates, and not a cost artifact — at 6 pairs
the rule is arithmetically out of reach.

This is a pure measurement. No extrapolation is involved in anything above.

## Why the quantization confirms the sample size

Observed rates take exactly these values: 0.125, 0.208, 0.25, 0.292, 0.333, 0.375, 0.417, 0.458,
0.500, 0.542 — spacing 0.0417 = **1/24**. Six pairs scored in half-points gives 12 half-points over a
max of 24, i.e. steps of 1/24. The spacing independently confirms `gate_pairs = 6` without reading the
default, which is the kind of cross-check this project requires before believing a parameter.

## This supersedes BOTH candidate fixes

Two fixes were on the table for P2. The measurement retires the framing of both.

* **"Relax the acceptance rule"** (the veto arm, `EXISTENCE_GATE_VETO=1`, `pent_rate + ci95 >= 0.5`).
  Applying the veto criterion to the 203 observed `(rate, ci95)` pairs accepts **145/203 = 71%** (recomputed, not renumbered).
  **This is NOT evidence that the veto is a rubber stamp, and I first wrote that it was.** STATE.md:2633
  already measured the contrary on the two-arm comparison — every *resolved-worse* candidate is
  rejected under both rules — and the veto's documented intent is "a veto on unplayable programs",
  which is *supposed* to admit ties. The 72% is dominated by ties, so it is the rule working as
  documented, not failing. The two populations also differ: 8/17 in the controlled two-arm comparison
  against 72% pooled over every run in the repo, which are not the same candidates.

  What the veto rule *is* exposed to is noise. At gen 3 the veto arm rejected a candidate by **0.004**
  (rate 0.333 against a bar of 0.337) whose true strength, taken independently at 96 pairs, was
  **0.422**. Both rules rejected it and the independent observer confirms the reject was correct — but
  a margin of 0.004 on an instrument with ±0.163 error is not a judgement, it is a coin flip that
  landed right. That is a statement about the *measurement*, not about the rule, which is the point of
  this document.

* **"Make `COST_PER_MOVE` finite"** (`cost_blind.rs`). **It has now run and returned INCONCLUSIVE —
  see the section at the bottom of this file.** It could not have explained 0/203 in any case: a cost
  ceiling changes *which* program scores higher, not whether a ±0.177 instrument can resolve the
  difference. Cost-blindness remains a live, untested hypothesis and is not a cause.

The binding constraint is that **the gate decides on six pairs.**

## What is NOT yet established: the size of the fix

The obvious fix is to raise `gate_pairs`. Sizing it needs the ci95-vs-pairs curve, and **that curve is
not yet measured.** Two anchors exist and they disagree:

| pairs | ci95 | source | n |
|---|---|---|---|
| 6 | 0.177 | 203 gate decisions — **but see the CORRECTION below: this averages placeholders with measurements** | 203 |
| 96 | 0.027 | the VERIFY observer | **1** |

Pure `1/sqrt(n)` scaling from the 6-pair anchor predicts **0.044** at 96 pairs; the single observed
value is **0.027**, a factor of 1.6 apart. Either anchor could be the misleading one:

* At n=6 the ci95 uses a *sample* sd estimated from six numbers and multiplies by 1.96 rather than a
  t-quantile, so the 6-pair figure is unstable and probably inflated.
* The 96-pair figure is **a single observation**. This project has withdrawn several claims built on
  one sample.

So the honest statement is: the bar `0.5 + ci95` needs to fall below ~0.542 for the rule to be
satisfiable at all, the 96-pair anchor would put it at 0.527 (satisfiable), and the conservative
extrapolation would not reach that until ~192 pairs. **Do not pick a number from this table.** The
next step is a direct ci95-vs-pairs probe running `match_progs` on one fixed pair at 6/12/24/48/96/192
pairs, which measures the curve instead of assuming its exponent.

## Pre-registered reading of that probe

* **ci95 falls as 1/sqrt(n) from the 6-pair anchor** ⇒ the 96-pair VERIFY reading was a lucky low
  sample, and the gate needs ~192 pairs (32×) to make the strict rule reachable. That is a real cost
  and would justify reconsidering the budget rather than paying it blindly.
* **ci95 tracks the 96-pair anchor** ⇒ the 6-pair ci95 is inflated by small-sample sd estimation, ~96
  pairs suffices, and the VERIFY observer is already running at exactly the right size — the fix is to
  make the gate itself as large as the observer it was given.

Either way the acceptance rule stays strict. A strict rule on a resolved measurement is the thing this
gate was supposed to be; it has simply never been given a resolved measurement to judge.

## How this was found

The VERIFY observer was added to answer a different question — *are the veto rule's ACCEPTs correct?*
It has not answered that one yet (no divergent ACCEPT has occurred). Instead its first output, sitting
next to the gate line that made the actual decision, showed the same candidate measured at ±0.027 and
±0.163. That 6× gap is what prompted counting the accepts, and the count is what settled P2.

---

## The cost-blindness probe returned INCONCLUSIVE — and its premise was wrong

`cost_blind.rs` completed. Pre-registration: *"a score that MOVES as the ceiling tightens means
`cost_per_move` decides gate outcomes."*

| `cost_per_move` | a's score | ci95 | CI vs 0.5 |
|---|---|---|---|
| `u64::MAX` (the gate's) | 0.469 | 0.090 | [0.379, 0.559] — **includes** 0.5 |
| 100_000_000 | 0.396 | 0.083 | [0.313, 0.479] — excludes 0.5 |
| 10_000_000 | 0.500 | 0.062 | includes 0.5 |
| 1_000_000 | 0.500 | 0.062 | includes 0.5 |

**The response is non-monotonic: 0.469 → 0.396 → 0.500 → 0.500.** A ceiling that tightens
monotonically should hurt the expensive program monotonically. Instead the score dips and then returns
to *exactly* parity twice. Only one row (1e8) excludes 0.5 at all, and it is the middle one. That shape
is what noise looks like, not what a mechanism looks like, so the pre-registered "score MOVES" reading
is **not** satisfied — a non-monotonic wobble inside overlapping intervals is not movement.

**The probe's premise was also unverified and is not supported.** `cost_blind.rs` asserts in its own
header that `capture_extension` is "strictly more work per move, and stronger for it". Measured at the
gate's own setting it scores **0.469** — indistinguishable from the seed, not stronger. The design
needed an arm that is *expensive AND stronger*, so that truncation would have something to take away.
Whatever this pair is, it is not that, and the contrast cannot answer the question.

**And the probe has the disease it was investigating.** At 24 pairs its own ci95 is 0.062–0.090 — the
same order as the gate's 0.177 and far too wide to resolve the few-percent effect it was looking for.
I built an underpowered instrument to investigate an underpowered instrument.

### Verdict

Cost-blindness is **neither confirmed nor refuted**; it is untested, because the contrast chosen
cannot test it. It stays a live hypothesis and is *not* promoted to a cause. This does not change the
main finding above, which never depended on it: a cost ceiling cannot explain 0/203 accepts, because
it changes which program scores higher, not whether a ±0.177 instrument can resolve the difference.

**Re-running this properly needs two things first:** a pair where the expensive arm is *measurably*
stronger at the gate's settings (verified, not assumed), and enough pairs that the instrument can see
the effect — which is exactly what `ci95_curve.rs` is measuring.

---

## ⚠ SUPERSEDED BELOW — A/A row 1: the harness is VALID (the 0.250 reading is REFUTED)

> **Read the two sections after this one before believing anything here.** The A/A's 0.250 is
> not a measured error bar at all: it is `gate.rs`'s zero-variance placeholder `1.5/n`. The
> VALIDITY result (`mean_rate` exactly 0.500) stands; the sizing claim does not.

`ci95_curve.rs` first row, `bare_alpha_beta` against itself, 8 replicates of 6 pairs:

```
  pairs  reps  mean_rate  mean_ci95     bar    vs max-ever 0.542
      6     8      0.500      0.250    0.750   out of reach
```

**The validity check passes exactly.** `mean_rate = 0.500` over 8 replicates is what an unbiased
harness must produce when a program plays itself, so `match_progs` is not tilted and the measurements
taken with it stand. That check was worth its cost: had it come back at, say, 0.55, every number in
this file and in the gate logs would have been suspect.

**The half-width at 6 pairs is 0.250 here, against 0.177 averaged over 203 real gate decisions.** Both
are correct, and the difference is not a contradiction:

* Pentanomial variance is **maximised near a rate of 0.5**. An A/A sits exactly there by construction.
* Real gate decisions are frequently lopsided — the observed rates cluster at 0.333, 0.375, 0.417 —
  and a lopsided pair generates less variance, pulling the average half-width down.

**~~For sizing the gate, 0.250 is the number that matters~~ — REFUTED two sections below, and this was the reason to run an A/A.**
The gate's job is to resolve a candidate that is *close to* its champion; a candidate that is
obviously worse needs no statistical help. So the regime the bar has to work in is precisely the
near-parity regime the A/A measures. Using the 0.177 average would size the gate for the easy cases.

That makes the headline finding **stronger, not weaker**: at 6 pairs the strict bar in the regime that
matters is **0.750** — a candidate must win three pairs in four — against a maximum rate of 0.542 ever
observed in 203 decisions. Earlier in this file the bar was computed as 0.582–0.677 from the averaged
half-width. The correct near-parity figure is worse than both.

The remaining rows (12, 24, 48, 96 pairs) are still running, and **the fix is still not sized**: the
whole point of the curve is to measure the exponent rather than assume it.

### ⚠ CAVEAT ON THE A/A, against my own claim above: its scaling is ANOMALOUS

Row 2 landed and the curve is not behaving like a confidence interval:

```
  pairs  reps  mean_rate  mean_ci95     bar
      6     8      0.500      0.250   0.750
     12     4      0.500      0.125   0.625
```

Doubling the pairs **halved** ci95. A confidence interval scales as `1/sqrt(n)`, which predicts
0.177 at 12 pairs, not 0.125. Observed is `1/n`. The values are also exact dyadics (1/4, then 1/8)
with `mean_rate` exactly 0.500 both times.

**The most likely explanation is that the A/A distribution is degenerate.** Two *identical*
deterministic programs playing a colour-swapped pair should split it exactly, so nearly every pair
scores a dead tie and the variance comes from a small, roughly fixed number of pairs that differ. If
the number of differing pairs does not grow with `n`, the sample variance falls as `1/n`, the sd as
`1/sqrt(n)`, and ci95 as `1/n` — which is exactly the shape observed.

**If that is right, the A/A does NOT model the near-parity variance of two DIFFERENT programs**, and
the claim I made in the section above — *"0.250 is the number that matters for sizing"* — is not
supported. Two similar-but-distinct programs disagree on many pairs, not a handful, so their variance
structure is not this one. The A/A remains valid for what it was primarily for: the **bias** check,
`mean_rate = 0.500`, which is unaffected by the variance anomaly.

**Not resolved either way yet.** Row 3 (24 pairs) discriminates: `1/n` continuing predicts 0.0625,
`1/sqrt(n)` from row 2 predicts 0.088. Until it lands, **no pair count should be chosen from this
curve**, and the sizing question stays open exactly as this file has said throughout. The headline
finding is untouched — it rests on 203 real gate decisions, 0 accepts, and a max rate of 0.542, none
of which involve the A/A.

---

## ⚠ CORRECTION TO MY OWN HEADLINE: 47% of gate decisions measured NOTHING AT ALL

The `1/n` anomaly flagged above has an exact mechanism, and it is in the source, not in statistics.
`gate.rs:65-74`:

```rust
if var <= 0.0 {
    // ZERO OBSERVED VARIANCE IS NOT ZERO UNCERTAINTY. ...
    // Rule of three: ... So the interval is 1.5/n, and it correctly says "no idea"
    // at small n instead of "certain".
    return 1.5 / n;
}
```

**When every pair lands in the same bucket, `ci95` is not a measured half-width — it is the constant
`1.5/n`.** That is why the A/A curve halved for a doubling: 1.5/6 = 0.250, 1.5/12 = 0.125, matching
both observed rows to four decimals. `ci95_curve` was therefore measuring the fallback formula, not the
instrument, and it has been stopped; every remaining row was predictable (24 → 0.0625, 48 → 0.0312,
96 → 0.0156).

### The same fallback fires in HALF of all real gate decisions

Counting the 203 logged decisions by whether `ci95` equals the placeholder exactly:

| | count | share | mean ci95 |
|---|---|---|---|
| `ci95 == 0.250` (= 1.5/6, **zero observed variance**) | **95** | **46.8%** | — (placeholder) |
| genuine measured intervals | 108 | 53.2% | **0.1122** |

**Every one of the 95 placeholder rows has `pent_rate` exactly 0.500.** So in nearly half of all gate
decisions, every pair scored identically and the gate observed *no signal whatsoever*. The code's own
comment names the likely cause: *"what a match between two near-random nets looks like, 24 games all
drawn"*.

### This splits P2 into two different problems, and I had merged them

My earlier section here reported "mean ci95 0.177" over all 203 decisions. That number **averages a
measurement with a placeholder** and should not have been used as one quantity. Corrected:

* **~47% of decisions: no signal.** Zero variance, rate exactly 0.500. **More pairs cannot fix this** —
  if the games are all drawn, a larger sample of drawn games is still drawn. The problem is upstream of
  the gate: the match setup is not producing decisive games at these settings.
* **~53% of decisions: genuinely underpowered.** Mean ci95 0.1122 puts the strict bar at **0.612**,
  still above the **0.542** maximum rate ever observed. **For this half the original finding stands**,
  and raising `gate_pairs` is the right lever.

**What does NOT change:** 0 accepts in 203 decisions, and a max rate of 0.542 against a bar that has
never been reachable. Those are counts of real outcomes and do not depend on which interval formula
produced them.

**What this costs me:** the fix is not the single knob I implied. Half the decisions need a gate that
can produce decisive games at all; only the other half needs more pairs. Sizing `gate_pairs` on the
mixed average would have been sizing against a number that is half placeholder.

---

## ✅ RESOLVED: the zero-variance case is BOTH mechanisms — and drawishness dominates

The A/A at the gate's own sample size (6 pairs), with W-D-L:

```
  A/A  seed vs ITSELF   pent [0, 0, 6, 0, 0]  middle 6/6 (100%)  rate 0.500 +/- 0.250
                        W-D-L 2-8-2
```

**8 of 12 games drawn (66.7%); the 4 decisive games split 2-2, perfectly mirrored.** That is exactly
what two identical deterministic programs must produce — the same game twice with colours swapped —
so it doubles as a validity check on the harness. `ci95` is 0.250 = 1.5/6, the zero-variance
placeholder, confirming the path.

### It is not MIRRORED *or* ALL-DRAWN. It is both, and the draws dominate

Every pair lands in bucket 2 by one of two routes: a **drawn pair** (DD) or a **mirrored pair**
(WL/LW). Here two thirds arrive by the first route and one third by the second. Both mechanisms are
real; the question posed as an either/or had a false premise.

### Replicated on the real gate population

The first W-D-L ever logged on an actual gate decision — candidate against champion, not a self-match —
read `0-8-4`: **also 8 draws of 12, also 66.7%**. Two independent measurements on different program
pairs agree on the draw rate to the game.

| measurement | W-D-L | draws |
|---|---|---|
| A/A, seed vs itself, 6 pairs | 2-8-2 | **66.7%** |
| real gate decision, candidate vs champion | 0-8-4 | **66.7%** |

### What this settles about the fix

At `p(draw) ≈ 2/3`, a 6-pair sample has a substantial chance that every pair ties — which is precisely
the 46.8% of decisions carrying the placeholder. Raising `gate_pairs` does reduce that probability, so
it is not useless. But **two thirds of the games carry no signal at all**, and adding pairs buys more
of the same mixture. The efficient lever is making games decisive — more depth, sharper openings, a net
that separates — because it attacks the 2/3, where extra pairs only chip at the sampling noise around
it.

**The earlier framing in this file — "the fix is to raise `gate_pairs`" — is half right.** For the ~53%
of decisions that measure something, more pairs is the correct and sufficient lever. For the drawish
half it is the expensive one.

---

## ✅ DEFINITIVE, on the REAL gate population: the no-signal case is 100% DRAWS

Every W-D-L now logged on an actual candidate-vs-champion gate decision:

| decision | W-D-L | draws | gate rate | VERIFY (96 pairs) |
|---|---|---|---|---|
| s2-control gen 1 MCTS | **0-12-0** | **100%** | 0.500 ± 0.250 | **0.505 ± 0.019** |
| s1-veto gen 3 MAIN | 0-8-4 | 67% | 0.333 ± 0.163 | 0.422 ± 0.027 |
| s1-veto gen 4 MAIN | 0-9-3 | 75% | 0.375 ± 0.110 | 0.430 ± 0.030 |
| **pooled** | **0-29-7** | **80.6%** | | |

**The zero-variance decision is `0-12-0` — every single game drawn.** `ci95` is 0.250 = 1.5/6, the
placeholder, and the strict rule demanded `> 0.750`. This is the case that carries 46.8% of all 203
logged decisions, and on the real population it is **pure ALL-DRAWN**, not mirrored.

**The gate was substantively right and informationally empty.** The independent 96-pair observer puts
that candidate at **0.505 ± 0.019** — genuinely equal to the champion. So "no signal" was the correct
conclusion, reached from twelve games that contained none. It was right by luck, not by measurement.

### This settles the sizing question against more pairs

At 100% draws, **every additional pair is also a draw.** Doubling `gate_pairs` on that decision buys
twelve more draws and the same placeholder. Raising pairs helps only where games are already
sometimes decisive — the ~53% of decisions that measure something — and does nothing for the half
that cannot.

Pooled across the three real decisions the draw rate is **80.6%**, higher than the A/A's 67%, and
not one candidate had won a single game (0 wins in 36). The instrument is not underpowered so much
as it is playing a drawn game.

**UPDATED as arms accumulated (2026-09-09):** five real decisions now, **W-D-L 1-49-10 over 60 games —
81.7% draws**. The draw rate held; the "zero wins" claim did not. **One candidate has now won a game**
(`1-8-3`), so the correct statement is 1 win in 60 (1.7%), not zero. Two of the five decisions are
`0-12-0` — every game drawn — and both carry the `1.5/n` placeholder. The conclusion is unchanged and
slightly sharpened: wins are possible but vanishingly rare, which is why a 6-pair sample so often
sees none at all.

**The lever is decisiveness, not sample size:** more depth, sharper openings, or a net that separates.
`gate_power_RESULT.md` opened by framing this as "raise `gate_pairs`". That framing is now retired for
the drawish half on direct evidence, and retained only for the half where games do resolve.

---

## ROOT CAUSE of the drawishness — and MASTER_PLAN already prescribes the fix

The gate's games are played with **`Net::random(32, 20260907)`** (`evolve.rs:1286`) — He-initialised,
untrained weights. Both programs evaluate positions with noise. That is deliberate and correct for
this track: the search track evolves PROGRAMS, so the net is held fixed and identical on both sides,
"so this measures the PROGRAM and nothing else" (`gate.rs:142`).

**But it makes the games structurally unable to resolve.** With no positional signal, neither side can
convert an advantage, so games run to the length limit and draw. Measured: **80.6% draws over three
real gate decisions, and zero wins in 36 games.**

### MASTER_PLAN diagnosed this in advance, under a different cause

`docs/MASTER_PLAN.md:154` — *"Openings and draw death"*:

> *"Draws label every position ~0 and starve the SPRT gate of information."*

It attributes draw-death to **strength** (">70% around 2800-3000"). Here the mechanism is inverted —
draw death from **weakness**, because a random eval cannot steer toward a win — but the symptom and
the remedy are the same, and the plan lists three legal sources of opening diversity:

1. **Random opening plies** — *"Effective early; weakens as the engine strengthens."*
2. **Self-generated unbalanced book** — mine own games for positions where own search eval sits in a
   band (e.g. +0.6 to +1.5); start there, each played from both sides.
3. **Chess960 start positions.**

**The gate currently uses #1 and only #1**: `match_progs(..., open_plies = 4, ...)` plays four random
plies from the start position (`gate.rs:154-156`). Four random plies from a balanced start, judged by a
random net, is the configuration least likely to produce a decisive game.

**The prescribed fix is #2**, and it applies for the opposite reason to the one the plan anticipated:
starting from positions that are ALREADY unbalanced gives the game a determinate outcome for a better
program to find, without requiring the net to supply one.

### What this does NOT justify

Swapping in a trained net. Holding the net fixed and random is what makes the gate measure the program
rather than the evaluation, and that is the whole point of the search track. The fix is the OPENINGS,
not the eval — which is exactly what the plan says, and it is why "raise `gate_pairs`" was the wrong
first instinct: more pairs from the same balanced start yield more draws.

### MEASURED: depth halves the draws but is NET WORSE per unit compute

The obvious first lever for decisiveness is search depth. Tested it as an A/A at the gate's own 6-pair
size, changing nothing but depth:

| depth | W-D-L | draws | decisive games | CPU | decisive per CPU-second |
|---|---|---|---|---|---|
| 3 | 2-8-2 | 67% | 4 | 102s | **0.0392** |
| 4 | 4-4-4 | **33%** | 8 | 614s | **0.0130** |

**Depth 4 halves the draw rate and delivers 0.33x the information per CPU-second.** Doubling the
decisive-game yield costs ~6x the compute, so on a fixed budget depth 3 produces more usable games
than depth 4 does. Depth is a real lever on drawishness and the wrong one to pull.

Note the pentanomial is `[0,0,6,0,0]` at BOTH depths, and `ci95` is 0.250 either way. That is correct
and not a failure: in an A/A the decisive games are perfectly mirrored (4 wins, 4 losses), so every
pair still lands in the middle bucket. Depth changes how many games *resolve*, not whether two
identical programs are equal — which is exactly why this had to be measured on draw COUNT rather than
on the gate's own output.

**This strengthens the case for MASTER_PLAN's remedy #2.** An unbalanced opening book changes the
STARTING POSITION, so it costs nothing per game — unlike depth, which pays for decisiveness with
compute at a losing exchange rate. Same information gain, no per-game cost.

### My own probe reproduced the gate's defect — 6 pairs cannot validate its own control

The unbalanced-opening probe pre-registered that its CONTROL arm must reproduce the independently
measured 67% A/A draw rate, "or this harness is not the gate and arm 2 means nothing". It came back
**0-12-0, 100% draws**.

Before blaming the harness, the arithmetic:

| sample (identical settings) | W-D-L | draws | n |
|---|---|---|---|
| `pent_shape` A/A | 2-8-2 | 66.7% | 12 |
| probe CONTROL | 0-12-0 | 100.0% | 12 |
| **pooled** | **2-20-2** | **83.3%** | **24** |
| real gate population | 0-29-7 | 80.6% | 36 |

**The pooled value matches the real population.** P(all 12 drawn | p = 0.67) = 0.008 — unlikely, not
excluded. Two 6-pair samples of the same quantity straddle it, and neither alone can pin it.

**This is the same defect the investigation is about.** I built a probe at the gate's own sample size
to model the gate, and inherited its inability to resolve anything — then nearly read a treatment
effect off it. The control requirement is what caught it, which is the only reason it was written
before the numbers were seen.

Re-running at **24 pairs (48 games per arm)**. The treatment reading stands or falls on the control
reproducing ~80% there; at 12 games neither arm carries information.

### Control VALIDATED at 24 pairs: 8-32-8, 66.7% draws

The 6-pair control read `0-12-0` (100% draws) against a pre-registered requirement of ~67%, and was
diagnosed as sampling noise rather than a harness fault. At 24 pairs (48 games) it reads:

```
  CONTROL balanced (gate today)   W-D-L 8-32-8   66.7% draws   pent [0,0,24,0,0]
```

**Exactly the independently measured A/A rate.** The diagnosis holds: 12 games could not pin a
two-thirds draw rate, 48 games can. The pentanomial is all-middle, as an A/A must be — 8 wins and 8
losses perfectly mirrored, plus 32 draws.

The pre-registered control requirement is therefore SATISFIED, and the treatment arm's reading is
legitimate. Without it I would have read a treatment effect off a control that was 33 points from its
own expected value.

### SIZING: `max_pairs = 100` would have returned INCONCLUSIVE every time

Simulated 800 sequential decisions per configuration, drawing pair outcomes from the MEASURED real
gate distribution (W-D-L 1-49-10 over 60 games, 81.7% draws) rather than from an assumed one:

| bounds | max_pairs | reject | accept | inconclusive | median pairs to decide |
|---|---|---|---|---|---|
| **[3, 5]** (FITNESS bootstrap) | **100** | 0 | 0 | **800** | capped |
| [3, 5] | 400 | 800 | 0 | 0 | **254** |
| [0, 10] (sprt.py default, used by 4PC) | 100 | 799 | 0 | 1 | **50** |
| [-5, 0] (non-regression) | 400 | 800 | 0 | 0 | 115 |

**The cap I chose guaranteed the answer.** A parity candidate under the spec's 2-Elo-wide [3, 5] band
needs a median of **254 pairs** to reject; capping at 100 makes every decision INCONCLUSIVE — which is
not a wrong answer, it is *no* answer, dressed as one. Raised to 400 and the arms relaunched (they had
made zero gate calls, so nothing was lost).

**Why so many pairs.** At 81.7% draws the pentanomial piles into the middle bucket, so the variance is
tiny — and `LLR ∝ n/var` means a *low* variance makes each pair informative, but the [3, 5] band is
only 2 Elo wide and sits entirely above parity, so `2mu - s0 - s1` is a very small negative number. The
band width, not the draw rate, is what costs the pairs.

**The cost is real and worth stating:** 254 pairs is 508 games at ~8s each, roughly **an hour per gate
decision**. The [0, 10] bounds the 4PC side uses resolve the same candidate in **50 pairs** — 5x
cheaper — and have precedent in this codebase. That is a bounds question for whoever revisits FITNESS
7.2's schedule, not something to change unilaterally here; recorded so the trade is visible.

**This was found by simulation before the arms spent hours producing it.** The measured draw rate made
the prediction possible, which is the payoff for having measured it.

### CAN THE GATE ACCEPT? Under the spec's bootstrap bounds at this draw rate — effectively no

The whole point of going sequential is a gate that *can* accept. Simulated at the measured 81.7% draw
rate, holding everything but the candidate's true strength fixed:

| true Elo | bounds | accept | reject | inconclusive | median pairs | ~hours/decision |
|---|---|---|---|---|---|---|
| 0 | [3,5] | 0 | 0 | **500/500** | capped | — |
| +10 | [3,5] | — | — | — | **2,673** | **11.9** |
| +20 | [3,5] | — | — | — | 1,002 | 4.5 |
| +50 | [3,5] | 488/500 | 0 | 12 | 319 | 1.4 |
| **+10** | **[0,10]** | — | — | — | **464** | **2.1** |
| **+20** | **[0,10]** | 200/200 | 0 | 0 | **207** | **0.9** |
| +5 | [0,10] | 94/200 | 101/200 | 5 | 808 | 3.6 |

**Under FITNESS 7.2's bootstrap [3,5], the gate accepts only at ~+50 Elo.** Everything from 0 to +20
returns INCONCLUSIVE at any affordable pair count — a +10 candidate needs a median **2,673 pairs**,
about **12 hours per decision**. That gate rejects bad candidates efficiently and can never promote a
modest good one, which is the same failure as before wearing a better statistic.

**Both SPRT arms therefore run [0, 10]**, and the deviation is deliberate and evidenced rather than
convenient:
* it is **5.8x cheaper** at +10 (464 pairs against 2,673) and resolves +20 in 207;
* it has **precedent in this codebase** — `tools/sprt.py` defaults to `ELO0=0, ELO1=10` and every 4PC
  gate this project has run uses it;
* at +5, exactly the midpoint, it splits 94 accept / 101 reject, which is the correct behaviour for a
  candidate sitting on the bound rather than a bias either way.

**What FITNESS actually pins down.** 7.2 fixes the width at 2 Elo "for STC, LTC, and the
fixed-cost-budget gate". The fixed-cost-budget gate is §6, which is *NET/ARCH/FEATURE only*. The
search-track PROGRAM gate is not named there, so the width rule's application to it is an
interpretation, not a quotation — and the measurement above is what an interpretation should be
decided on. **Recorded as an open bounds question for FITNESS 7.2 rather than settled here.**

The error rates (alpha = beta = 0.05) and the pentanomial statistic are untouched: those the spec
fixes unambiguously, and 7.2 part 1 is explicit that the human declares the threshold semantics.

### The sequential gate would have REGRESSED on the no-signal half — fixed before it ran

`Score::llr` returns **0.0 when every pair lands in the same bucket**, which is correct: a dead heat
carries no evidence. But 0.0 never reaches ±2.944, so `match_progs_sprt` would have played to
`max_pairs` — **400 pairs, ~800 games, roughly 1.8 hours** — on a decision that was settled at pair 2.

The old FIXED gate spent **6 pairs** on that same case. And it is not a rare case: **46.8% of the 203
logged decisions carry zero observed variance.** Without a guard, going sequential would have been a
~67x cost regression on nearly half of all decisions, while producing the identical non-answer.

**Guard added: give up as INCONCLUSIVE if all pairs are still in one bucket after 30.** Not after 2 —
at the measured 80.6% draw rate a genuinely different pair of programs can tie its opening several
pairs by luck, and quitting on that discards a real candidate. Reaching 30 with zero spread means they
do not differ on these openings, and `Inconclusive` is the honest label: the evidence never separated
them, which is a different statement from "they are equal".

**Found by reading the code, not by waiting.** The zero-variance branch is explicit in `gate.rs`, so
the consequence was derivable without spending the 1.8 hours to observe it — and the arms had not yet
reached a gate call, so the fix landed before it could cost anything.

**The threshold of 30 is measured, not chosen.** From the real gate distribution (W-D-L 1-49-10), a
pair lands in the middle bucket with probability **0.6725**, so the chance a genuinely DIFFERENT pair
of programs produces N straight single-bucket pairs is:

| N | P(all in one bucket) | verdict |
|---|---|---|
| 10 | **1.9e-2** | would discard ~2% of real candidates |
| 20 | 3.6e-4 | borderline |
| **30** | **6.8e-6** | negligible — the chosen value |
| 60 | 4.6e-11 | wasteful |

The intuitive "give up after a few pairs" would have been **10**, and it would have thrown away one
real candidate in fifty. An identical pair hits the guard with probability 1 at any N, so the only
cost of raising it is pairs; the only cost of lowering it is discarded candidates. 30 buys a 1-in-150,000
false-stop rate for 24 extra pairs, and saves **370 pairs (~1.6 h)** every time it fires.

---

# 2026-09-10 — THE SPRT FIX HAS RUN. It resolves, and it says the candidates are genuinely worse.

Yesterday this file ended with "the fix is BUILT AND RUNNING". It has now run to completion on
`gate_sprt30_s1`, so the open question — *does spending evidence sequentially actually resolve
anything?* — has a measured answer. It does.

| | 12-game fixed-n gate | sequential SPRT gate |
|---|---|---|
| decisions | 80 | 16 |
| games spent | 960 | 592 (mean **37**/decision, range 20–82) |
| reached a verdict | n/a (fixed n) | **16/16, zero INCONCLUSIVE** |
| ACCEPTs | 0 | 0 |

**Pooled over all 592 SPRT games: W-D-L 19-498-75, score 0.4527, 95% CI [0.4371, 0.4683].**
The interval excludes 0.5, so this is *resolved*, not another non-answer: the candidates are worse,
by roughly **−33 Elo**, winning only **19 of 94 decisive games (20.2%)**.

## What this changes

The whole P2 story so far has been "the gate cannot accept". That is true and this file measures it.
But it invited an inference I made and now retract: *that good candidates were being blocked.* They
were not. The one instrument in this project with enough power to resolve the question says the
rejections were **correct**. The generator, not the gate, is what has no gradient.

## Correction to `EXISTENCE_GATE_VETO`, including a pre-registration I wrote and lost

I pre-registered that `GATE_VETO=1` would accept at gen 2 MCTS where its control rejected. **It did**,
and the two logs are byte-identical through the preceding line, so the mechanism is confirmed:

    control    gen 2 MCTS  gate REJECT 0.417+/-0.103 (12 games W-D-L 0-10-2) ... needed >0.603
    GATE_VETO  gen 2 MCTS  ACCEPT 8 mates 0.000822 (142 nodes, was 0.000737)  gate 0.417

Confirming the mechanism is not vindicating the fix. Exact power of the 6-pair rule, enumerating all
5^6 pair-outcome sequences (no net-wins shortcut — see the caveat below):

| candidate's true strength | P(ACCEPT) shipped | P(ACCEPT) under GATE_VETO |
|---|---|---|
| exactly equal (0 Elo) | 1.45% | **98.55%** |
| +34 Elo | 9.90% | 99.99% |
| wins *every* decisive game | **14.04%** | 100% |

The veto accepts **98.55% of exactly-equal candidates**. Combined with the pooled −33 Elo above, a
veto arm does not admit ties pending better evidence — it promotes measurably worse programs, which is
what `evolved_MCTS_gen2.prog` becoming dirty in the worktree was. The arm was stopped and the artifact
reverted. **`GATE_VETO` is not the P2 fix and must not ship.**

This does not contradict the 2026-09-09 note above ("the veto is *supposed* to admit ties"). That
defence is sound on its own terms and is exactly the problem: at 6 pairs almost nothing is ever
*resolved* worse, so "admit everything unresolved" is very nearly "admit everything".

## Two errors of my own, recorded because both were nearly published

1. **"The rule accepts iff net wins >= +3."** Clean, and **false**. Acceptance depends on the realised
   *pair multiset*, not the margin: 3-9-0 with the wins in distinct pairs gives ci 0.110 → ACCEPT,
   while the same 3-9-0 clustered gives ci 0.167 → REJECT. Net +3 is **necessary, not sufficient**
   (the minimum accepting rate is 0.625 = 0.5 + 3/24). The 0-of-80 record stands, since necessity is
   all that record needs.
2. **"50% power is unreachable at any strength."** Also false. It assumed the 87.7% draw rate is
   fixed, but a genuinely stronger engine *converts* draws, which raises decisive count and power:

   | draws | implied Elo | P(ACCEPT) shipped |
   |---|---|---|
   | 0.877 | +34 | 9.90% |
   | 0.750 | +70 | 36.9% |
   | 0.600 | **+115** | **61.2%** |
   | 0.400 | +182 | 76.5% |

   The correct statement is narrower and still decisive: **the 12-game gate needs roughly +115 Elo in
   a single generation to reach coin-flip power.** Evolutionary steps are not that size.

## What this makes next

Not another acceptance rule. Both rules are now measured, and neither is the binding constraint:
the SPRT gate resolves in 37 games and finds nothing to accept because there is nothing to accept.
The open question moves upstream to what produces candidates at all — GRAMMAR 4's mutation operators
and the type checker, which is where the plan already puts it.
