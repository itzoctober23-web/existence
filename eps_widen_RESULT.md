# EPS 0.020 → 0.100: the knob is LIVE, and it retains TT-carrying members

**Headline.** Widening the population-retention band from EPS 0.020 to 0.100 changes the search on
its first opportunity: at seed 1 / gen 4 the arm keeps **pop 4** where the otherwise-identical arm
keeps **pop 2**, and the retained set carries a member with **three TT primitives**.

**Scope: one generation, one seed.** This shows the knob is live and what it does. It does NOT show
it helps — nothing has been accepted by either arm.

## The measurement

Two arms, identical in every respect except `EXISTENCE_EPS`, same run seed 1, same `[0,30]` SPRT
gate, same binary. Through gen 3 they are byte-identical, including the gate verdict
(`REJECT llr -3.18`). They first diverge at gen 4:

    EPS=0.020 (control)  -- no gen-4 MAIN line at the same wall-clock; pop 2 through gen 3
    EPS=0.100 (treatment):
      gen 4 MAIN ..none (8 cand, 0 ill, mate-ok 2,
                 rates 0.947-0.961908x [>=.98:0 .90-.98:2 .50-.90:0 <.50:0 distinct:2],
                 hard 0-0)  pop 4 spread 0.002770-0.002924  tt[0, 0, 3, 0]

## Why this is the knob working, arithmetically

`evolve.rs:1730` retains on `x.2 >= top * (1.0 - EPS)`. The retained spread is
**0.002770 to 0.002924**, and `0.002770 / 0.002924 = 0.947` — the weakest kept member is **5.3%
below the top**.

- At EPS 0.020 the bar is 98.0% of top. That member is **discarded**.
- At EPS 0.100 the bar is 90.0% of top. That member is **kept**.

The bucket histogram says the same thing directly: `[>=.98:0  .90-.98:2 ...]`. **Zero** candidates
this generation are within the old 2% band, and **two** sit in the 90-98% band — exactly the range
that EPS 0.020 throws away and EPS 0.100 keeps. Under the old setting this generation contributes
nothing to the population; under the new one it contributes two distinct members.

## Why the `tt` field matters

The retained population reads `tt[0, 0, 3, 0]` — one member carries **three** TT primitives. That is
the structure the standing note pointed at ("the MAIN population shows `tt[2,2,...3]` — TT primitives
present in every member — but rates are flat at the seed's level"). Hash reuse is the ONLY rung ever
measured as fitter than the seed (0.98x its cost at D=3, `GRAMMAR.md:473`), so keeping TT-carrying
members in the population is precisely the diversity worth having.

Note what did NOT happen: no candidate exceeded `best_rate` this generation (every rate is < 1.000x),
so gen 4 produced **no gate call**. A wider band retains more parents; it does not manufacture an
improvement.

## What would settle it

Whether the extra diversity converts into an ACCEPT that the narrow band would have missed. That
needs the arms to run, and it is the same standard applied to the HARD_FITNESS A/B: the count of
accepts, not the appearance of activity.
