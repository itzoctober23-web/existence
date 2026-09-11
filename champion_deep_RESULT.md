# The deep-datagen net PASSED THE GATE against the champion — 0.622 ± 0.022, and it is now champion

**2026-09-10.** The chain that started with `datagen_depth_RESULT.md` closes here, confirmed on two
independent instruments.

## The gate

`netmatch`, 448 pairs, **depth 4 — the project's own strength standard** (`gate_depth_cap` default),
seed 20260907, both nets width 16:

```
  deep-datagen net vs p1_champion:  0.622 +/- 0.022   interval [0.599, 0.644]
```

The acceptance rule is `rate − ci95 ≥ 0.5`. **0.599 ≥ 0.500 — PASSED**, and not marginally: the
interval clears the bar by 0.099.

`netmatch`'s own power note reports the effect as **0.122 against a between-seed sd of 0.047**, i.e.
~1 seed for ~80% power. One training seed is defensible for an effect this size, which is not
something this repo has often been able to say — `rl-census-sample-size` and
`blind_gate_reports_a_tight_500` both record the opposite case.

## Two instruments agree, which is the part that matters

| instrument | old champion | new net | difference |
|---|---|---|---|
| absolute ruler (SF-1320 @10k, depth 4) | −104 ± 52 | **−17 ± 46** | **+87** |
| head-to-head, 448 pairs | — | 0.622 ± 0.022 | **≈ +86 Elo** |

Two measurements that share no machinery — one against Stockfish, one against the champion directly —
land within 1 Elo of each other. Every strength claim earlier today rested on a single instrument;
this is the first that does not.

## What produced it

Nothing but the datagen depth. Same architecture (width 16), same trainer, same blend, same seed
family. `main.rs:152` defaulted `--depth` to **1**, and every P1 measurement in this repo — the ~1216
plateau, the deceleration curve, the width/blend/horizon sweeps, the 2,200-generation champion — was
taken on a loop labelling its own positions with a ONE-PLY search. Raising it to 3 is the whole
change.

## Provenance

The previous champion is kept as `p1_champion_pre_deep.net`. The promoted net is the one measured
above, copied from the running loop at generation ~7,600 of the depth-3 run and re-checked after
promotion (−35 ± 85 on 40 games, consistent with −17 ± 46 on 120).

## Not claimed

* **Not that the ceiling moved.** It passed the champion; whether it keeps climbing or flattens near
  ~1300 is what the live ruler is measuring now.
* **Not that depth 5 is better.** That arm reached 34 generations before the cores were consolidated
  and its reading (−374 ± 74) is a barely-trained net, not a verdict on depth 5.
* **Not replicated.** One seed, one run. The between-seed power note says that is defensible for this
  effect size; it is not the same as replicated.

---

## SECOND PROMOTION, ~40 minutes later — and the ruler's "decline" was noise

The live ruler read **−17 → −29 → −41 → −44** across four successive 120-game samples. Four readings
drifting one way looks like degradation, and the honest move was to test it on the instrument that
settled the first promotion rather than to believe or dismiss it.

`netmatch`, 448 pairs at depth 4, current net against the champion promoted 40 minutes earlier:

```
  0.586 +/- 0.020   interval [0.566, 0.606]   -- clear of 0.5, A is stronger
```

**So the run was still improving while the ruler said it was declining.** The net that "looked worse"
beat the shipped champion outright.

### What that says about the instrument, which is the durable part

A 120-game ruler sample carries **±50 Elo**. The apparent slide from −17 to −44 is 27 Elo — well
inside one sample's error, and four correlated samples of a moving target are not four independent
observations. `depth_ruler_PREREG.md` already recorded the same lesson at 60 games (±80), where the
sequence −134 / −101 / −176 was pure noise. **The ruler answers "roughly where is this net"; it
cannot answer "is it moving".** Only the paired head-to-head can, because it removes the opponent as
a variable.

This is the second time today a trend was read off the ruler and had to be withdrawn. The rule is:
**a direction claim needs the paired instrument, never a sequence of ruler samples.**

### Chain so far, all on 448 pairs at depth 4

| promotion | vs | score | verdict |
|---|---|---|---|
| deep-datagen net | original champion (~2,200 gens, depth-1 labels) | 0.622 ± 0.022 | PASSED |
| current net (gen ~1,400) | that champion | **0.586 ± 0.020** | **PASSED** |

Previous champions kept as `p1_champion_pre_deep.net` and `p1_champion_gen1045.net`.
