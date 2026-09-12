# WEEK 1 RETRO — which lever failed, and why

> **STATUS: DRAFT, written 2026-09-11 evening.** The stop condition is "1600 pooled with a rising
> trend by day 7", and day 7 is tomorrow. Today's reading is **1535 ± 12 (prodk1926) and 1551 ± 13
> (prodk1658), both FLAT** (z = −0.22 and +0.22). Unless tomorrow's reading moves ~50 Elo and turns
> a trend positive, the condition is not met and this file is the required output. It is drafted now
> because the evidence is fresh, not to pre-empt the reading.

## The one-line answer

**Every lever that paid this week was a CONFIGURATION change, and every STRUCTURAL lever returned
null.** The engine is not stuck because a knob is mis-set; it is stuck because nothing in the loop
improves a net *within* a run.

## The evidence, in the order it accumulated

**Configuration levers PAID:**

```
learning rate       the plateau WAS the lr -- 0.692 vs 0.499 on a knob never once varied;
                    replicated on a fresh seed; optimum lower still; 0.0002 shipped
blend               0.85 cleared the promotion rule at full length
horizon             the cap is obsolete past bootstrap, uncapped +0.064 ± 0.034
```

**Structural levers returned NULL:**

```
capacity/width      11 attempts, 0 accepted, sign never flips -- widening never pays on the CLOCK
data volume         quadrupling data per generation: 0.4642 vs 0.4684
epochs              2 vs 3 flat at full length, both directions and on cost
datagen depth       knee at or below 3; depth 5 LOSES to its own start
P1 compounding      ruler CIs overlap (+208 vs +199), netmatch 0.481 ± 0.028 contains 0.5
P2 program search   0 accepts in 203 decisions
```

## The measurement that explains all of it

`ruler_trend_RESULT.md`: **every production run is FLAT on the absolute ruler — all 166 Elo came
from BETWEEN runs, not within them.** Today's readings say the same thing three months of
generations later: prodk1658 +0.5 ± 2.2 per 1000 generations, prodk1926 −0.4 ± 1.7. Both
indistinguishable from zero over ~20,000 generations each.

So the loop's *within-run* dynamics contribute nothing measurable. Progress has come entirely from
restarting with a better configuration — which is why configuration levers looked like they worked
and structural ones did not. **There was no within-run improvement for a structural change to
improve.**

## Why P1 compounding — the week's flagship — could not settle it

It was the right experiment and it came back null, but with a confound recorded BEFORE the numbers
existed: the two arms were not gated the same way. Control ran with the game gate disabled and
accepted everything on the surrogate; compound ran a batch gate that **kept 3 of 20 calls while 17
of 20 increments were positive**, because a typical +0.008 increment cannot resolve against
ci95 ±0.015. So compound spent 2,000 generations having most of its improvement discarded by an
instrument that could not see it.

**That is a power failure, not a refutation.** The compounding shape has not been tested at adequate
gate power, and the run that was supposed to test it could not.

## What the P2 track established instead

`gate_power_RESULT.md`: the gate is settled — 0 accepts in 203 decisions, and raising `gate_pairs`,
swapping in a trained net, and every further acceptance rule are all MEASURED wrong. The binding
constraint is upstream: **the generator has no gradient.** The current 40-generation arm is at 32
generations with **0 accepts and 62 rejects**, which is consistent and adds no new information.

## What I would change, stated as the lever rather than the activity

1. **Give the batch gate enough power to see a +0.008 increment** (~3.5x the games, since ci ∝ 1/√n).
   Until it can, every structural experiment reports null regardless of merit — which is exactly
   what this week looks like.
2. **Stop testing structural changes against a within-run signal that is measurably zero.** A
   between-run comparison at matched generations is the only thing that has ever resolved anything
   here.
3. **The uncertainty head is now fitted and parity-proven but UNTRAINED in production** — it returns
   a constant, and a constant row cannot be selected on. That is a prerequisite, not a result.

## What NOT to conclude

* Not that compounding fails — it was measured at inadequate gate power and the confound was
  recorded in advance.
* Not that the configuration levers are exhausted — lr moved 0.499 → 0.692 and the optimum was still
  falling when the sweep ended.
* Not that the ruler is broken. `ruler_trend.py` carries a saturation control and reports FLAT, not
  SATURATED, so the flatness is real and not an instrument running out of range.
