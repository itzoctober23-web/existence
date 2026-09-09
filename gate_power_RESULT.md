# The P2 game gate is UNDERPOWERED, not mis-ruled and not cost-blind

**Status: the "nothing is ever promoted" symptom is explained. The fix is not yet sized.**

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
