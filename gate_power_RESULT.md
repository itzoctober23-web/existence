# The P2 game gate is UNDERPOWERED, not mis-ruled and not cost-blind

**Status: the "nothing is ever promoted" symptom is explained. The fix is not yet sized.**

## The measurement

Across **every gate decision in every log in this repo** — 202 decisions:

| quantity | value |
|---|---|
| decisions logged | 202 |
| sample size, every one | **12 games = 6 pairs** (`evolve.rs:1297`, `gate_pairs` default 6) |
| ACCEPTs, ever | **0** |
| mean ci95 | 0.177 |
| smallest ci95 ever seen | 0.082 |
| **max `pent_rate` ever seen** | **0.542** |

The strict acceptance rule is `pent_rate - ci95 > 0.5`, i.e. `pent_rate > 0.5 + ci95`.
At the **smallest error bar ever observed** that bar is **0.582**.
The **highest rate ever observed in 202 decisions is 0.542**.

So the bar has never once been inside the range of outcomes the instrument can produce. The 0/202
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
  Applying the veto criterion to the 202 observed `(rate, ci95)` pairs accepts **145/202 = 72%**.
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
  see the section at the bottom of this file.** It could not have explained 0/202 in any case: a cost
  ceiling changes *which* program scores higher, not whether a ±0.177 instrument can resolve the
  difference. Cost-blindness remains a live, untested hypothesis and is not a cause.

The binding constraint is that **the gate decides on six pairs.**

## What is NOT yet established: the size of the fix

The obvious fix is to raise `gate_pairs`. Sizing it needs the ci95-vs-pairs curve, and **that curve is
not yet measured.** Two anchors exist and they disagree:

| pairs | ci95 | source | n |
|---|---|---|---|
| 6 | 0.177 | 202 gate decisions | 202 |
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
main finding above, which never depended on it: a cost ceiling cannot explain 0/202 accepts, because
it changes which program scores higher, not whether a ±0.177 instrument can resolve the difference.

**Re-running this properly needs two things first:** a pair where the expensive arm is *measurably*
stronger at the gate's settings (verified, not assumed), and enough pairs that the instrument can see
the effect — which is exactly what `ci95_curve.rs` is measuring.
