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

---

## THE RUN REGRESSED, and the automation caught it: 0.444 ± 0.030 — 2026-09-10 21:27

`auto_promote`'s first automatic verdict, 224 pairs at depth 4 against the champion it had promoted
~40 minutes earlier:

```
  gen 2905:  hold   0.444 +/- 0.030   interval [0.415, 0.474]
```

**The interval is entirely below 0.5.** The run had not plateaued — it had got WORSE than the net it
came from. Training past that point was actively costing strength.

### Why this matters more than the promotions

Both earlier promotions were real (0.622, then 0.586). But between the second promotion and this
check, ~1,500 generations of training made the net worse, and **nothing would have noticed**. The
ruler read −14 to −20 across the same span, which is inside its ±50 noise and looks like "fine". The
training loss rose, which `proxies_RESULT.md` explains as a moving target and is not a fault signal.
The only instrument that could see it is the paired match against the banked champion, run
automatically.

That is the argument for `auto_promote` existing: not to bank wins, but to **notice losses**. The
champion is safe because it was copied at promotion time; the regressed weights are simply discarded.

### What it triggered, as pre-registered

A hold was the stated signal to switch levers. The depth-3 run is stopped and
`depth5_from_champion.sh` is running from the banked champion — matched start via `--init`, judged by
`netmatch` against that same start, with a written prediction that depth 5 **loses** (35× the cost
per label, on labels already informative).

Not yet known: whether the regression is specific to this run's later generations, or whether the
depth-3 configuration has a ceiling around here. One hold does not distinguish those.
