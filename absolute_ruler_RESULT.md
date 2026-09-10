# The champion is ~1182 Elo and FLAT across 1200 generations — the plateau is real and it is low

**2026-09-10.** The first absolute strength measurement this project has ever had. Every prior
number was relative — candidate vs champion, arm vs origin, rung vs rung — so "0.83 against the
frozen origin" could equally have meant a real ceiling or a saturating instrument. This settles it.

## The measurement

Our engine at **fixed depth 4** (the project's own strength standard, `gate_depth_cap = 4`) against
**Stockfish 18 with `UCI_LimitStrength`, `UCI_Elo 1320`, 10k nodes**, 120 games per rung, colours
alternating on shared openings.

| rung | W-D-L | score | Elo vs SF-1320 |
|---|---|---|---|
| gen200 | 22-38-60 | 0.3417 | **−114 ± 53** |
| gen600 | 15-44-61 | 0.3083 | **−140 ± 51** |
| gen1000 | 15-35-70 | 0.2708 | **−172 ± 56** |
| gen1400 | 19-40-61 | 0.3250 | **−127 ± 52** |

```
trend vs generation:  −17.8 Elo per 1000 gens,  r = −0.368,  t(2) = −0.56   NOT significant
pooled level:         −138.2 ± 24.4
between-rung sd 24.9  ≈  each rung's own se ~27   → consistent with ONE value
```

**Absolute: ~1182 Elo.** FITNESS's P1 milestone is **~2000 vs SF-limited**. Short by ~818.

## What this settles that no relative instrument could

* **The plateau is real, and it is not a ceiling.** 1182 is nowhere near a saturation point of the
  game; the loop is simply not learning. The origin control reading 0.87 is an artefact of a frozen
  random opponent, not evidence of strength.
* **1200 generations bought nothing measurable.** Not a decline either — the slope is
  indistinguishable from zero — but flat at a level far below the milestone.
* **It calibrates every relative number in the repo.** The ancestor control's ±0.03 over a
  400-generation window corresponds to roughly ±20 Elo here, which is why it could not see anything:
  the true per-window change is smaller than its resolution and the total change over 1200
  generations is under its noise floor too.

## The near-miss, recorded because it nearly became the headline

After three rungs the sequence read **−114, −140, −172** — monotonic decline, and I was composing
the finding. gen1400 came back at **−127** and broke it. Three points falling in order is what four
noisy draws do often enough to be unremarkable; the honest statement needed the fourth. **Do not
read a trend out of three rungs whose intervals are ±50 each.**

## Caveat on the scale

`UCI_Elo` is Stockfish's own calibration, not FIDE, and 10k nodes weakens it further below whatever
1320 nominally means. So **~1182 is one consistent scale, not a rating card.** Its value is exactly
that: all five nets measured against one fixed external opponent, so differences between rungs are
comparable and the absolute level is anchored to something outside this repo.

## What it implies for what to do next

Chasing +0.02 effects with 1,344-game audits is the wrong instrument at this scale — an engine 800
Elo below its own milestone does not have a subtle selection problem, it has a large missing one.
The productive directions are the ones that move an engine at 1182: search depth per unit time,
and whether the learning signal produces anything at all.
