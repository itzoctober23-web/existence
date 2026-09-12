# P1 compounding did NOT pass — and the result does NOT refute the compounding shape

Planned N complete: both arms ran their full 2,000 generations, so this is a result rather than
state. Read by `p1_compounding_verdict.sh`, which transcribes `p1_compounding_PREREG.md` and was
written **before** either arm finished.

## Exact engines compared

```
common start   p1c_start.net       md5 0097ddc3f5e6   (= p1_champion.net at 18:58)
CONTROL        p1c_control.net     md5 47358603787f   2000 gens, 748s
COMPOUND       p1c_compound.net    md5 cd04e334e2fa   2000 gens
```

Both arms from ONE snapshotted start, sequential (the box rule is one trainer), matched on
GENERATION count. Control = production verbatim. Compound = `--datagen-nodes 10309
--steps-per-gen 777 --replay-gens 8 --gate-every 100 --gate-pairs 224`.

## Both pre-registered halves: NULL

**RULER** — `sf_ruler.py`, depth 4 vs SF-1320 at 10,000 nodes, 120 games, 3 seeds per arm.

| seed | CONTROL | COMPOUND |
|---|---|---|
| 20260911 | +203 ± 61 | +187 ± 60 |
| 20260912 | +176 ± 62 | +165 ± 62 |
| 20260913 | +245 ± 70 | +245 ± 70 |
| mean | **+208** | **+199** |

The registered rule requires **disjoint** CIs for a win. These overlap almost completely. **No ruler
difference.**

**GATE** — `netmatch` COMPOUND vs CONTROL, 224 pairs, promotion rule `rate − ci95 ≥ 0.5`:

```
rate 0.481   ci95 0.028   ->   rate - ci95 = 0.453   NOT a pass
```

But the interval is **[0.453, 0.509], which CONTAINS 0.5**. So compound is not significantly *worse*
either. The honest statement is **no difference detected**, not "compound lost".

## The control that makes this readable

Control's own end-of-run check: `691W-49D-156L, rate 0.799 ± 0.025` against its ORIGINAL random
initialisation — interval clear of 0.5. **Both arms learned.** This was not two broken runs tying at
zero; it was two working runs that finished level.

## Why this does NOT refute the compounding shape

**The arms were not gated the same way, and the asymmetry cuts against compound.** Control ran with
the game gate disabled (`--gate-every 1000000`) and accepted every candidate on the held-out
surrogate. Compound ran a real batch gate, and over its 20 calls:

```
3 KEEP, 17 ROLL BACK — and 17 of the 20 increments were POSITIVE
typical increment ~+0.008 against ci95 ~±0.015
```

The gate behaved exactly as specified: the 3 KEEPs are precisely the 3 increments clear of their own
interval. It is simply **underpowered for the effect it faces** — halving that interval needs ~3.5×
the gate games (ci ∝ 1/√n). So compound spent 2,000 generations having most of its improvement
thrown away by an instrument that could not see it.

**Therefore a compound loss has an available explanation that is not "the shape is wrong".** A
compound WIN would have carried no such caveat, having won despite the handicap. This asymmetry was
recorded in STATE and written into the verdict harness *while the arm was still running*, so it
could not be invented afterwards to explain a disappointing number.

## What is NOT established

**This does not say compounding fails.** It says this configuration, with this gate, at 2,000
generations, is indistinguishable from production. The three structural changes moved together by
design (the PREREG says the arm is the shape, not any one knob), so nothing here attributes the null
to the window, the budget, or the gate individually.

**2,000 generations may be short.** Both arms `RESUMED` from the same champion, and a resume costs
~95 Elo before it pays back; both paid it equally, but a compounding effect that needs longer than
2,000 generations to appear would look exactly like this.

**The ruler rung is not saturated** (`ruler_trend.py` reports FLAT, not SATURATED), so the null is
not an artifact of a ruler that ran out of range.

**One oddity, checked rather than waved through:** seed 20260913 returned byte-identical W-D-L
`89-15-16` for both arms. The nets are genuinely different files (distinct md5, `cmp` differs) and
each ruler run loaded its own net per the output header. So it is a real measurement, and it
indicates the two nets are behaviourally very close — consistent with the near-identical means.

## What follows

The next question is whether the compound arm's gate, given adequate power, keeps the improvements
it is currently discarding. That is a change to the GATE (more pairs per batch call), not to the
compounding shape, and it is the one variable this run identified as binding.
