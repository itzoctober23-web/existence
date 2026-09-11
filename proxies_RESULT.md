# Every cheap proxy for strength has failed. Only games measure strength here.

Zero compute — mined from logs already on disk, 2026-09-08.

## The measurement

Each control-vs-origin reading paired with the training loss from the SAME generation of the SAME
run, plus each run's built-in end-of-run control paired with its final loss:

```
n = 12 (loss, strength) pairs from 12 runs
loss     mean 0.1307   range 0.0166 - 0.4145
strength mean 0.8103   range 0.5000 - 0.9670

CORRELATION training loss vs strength-vs-origin:  r = +0.379   95% CI [-0.249, +0.783]
```

**A useful proxy needs a strongly NEGATIVE r** — lower loss should mean higher strength. The
interval excludes any strong negative relationship, and the point estimate has the WRONG SIGN.
Training loss carries no usable information about playing strength here.

## This is the third independent measurement of the same thing

| proxy | measured against | result |
|---|---|---|
| `mcnemar_z` held-out surrogate | 239 paired gate results | r = **−0.095**, CI [−0.220, +0.032] — no usable signal |
| training loss (width A/B) | equal-time head-to-head | w64 loss **0.0262** vs w16 0.0397, and w64 **loses 0.179 ± 0.021** |
| training loss (draws A/B) | control vs origin | include loss **0.0191** vs exclude 0.0726, and include scores **0.774 vs 0.828** |
| training loss (this, n=12) | control vs origin | r = **+0.379**, CI [−0.249, +0.783] — indistinguishable from zero |

Two anecdotes became a measurement. Every cheap signal tried in this project has failed to predict
strength, and two of them point the wrong way.

## Why this is the day's most consequential finding

It explains the shape of everything else rather than being one more result beside them.

The loop needs a cheap per-generation signal to ratchet on. It has three candidates and all three
are now measured as uninformative: the held-out surrogate, the training loss, and the
candidate-vs-champion gate (near-blind — 0.500 ± 0.007 on nets a fixed anchor separates easily,
plus demonstrated non-transitivity). **The only thing that has resolved anything today is games
against a FIXED opponent.**

And games at the required resolution are unaffordable per generation. The measurement-floor
arithmetic, validated against the harness's own logged bars: pair sd 0.2362, so a 224-pair gate
resolves ±0.031 while the best per-generation edge ever measured here is +0.0114 — 2.7× too coarse,
and 6.4× at 40 pairs. Resolving one generation's real improvement needs ~1,650 pairs.

So the loop is caught between a signal it can afford that means nothing, and a signal that means
something it cannot afford. That is a structural statement about the design, not a bug to fix, and
it reframes the ceiling work: the four candidates were being tested as if the *component* were at
fault, when the *feedback path* is what is broken.

## What survives

* **Fixed-anchor games resolve.** They refuted capacity (0.179 ± 0.021) and the draw filter
  (+0.054 ± 0.034). Both were clean, resolved results.
* **Batching the gate** is the one structural response already built: gate the accumulated change
  over K generations rather than each one, so the signal grows while the floor stays put.

## Honest limits

* **n = 12 is small** and the interval is correspondingly wide. The claim defended here is the
  narrow one — *no usable negative relationship* — not "loss is positively harmful".
* **The pairs mix two protocols.** In-loop `control vs origin` readings and end-of-run built-in
  controls use different pair counts, and the losses come from different generations with
  different horizons. This is an observational sweep over runs that were not designed as a loss
  study.
* A within-run analysis at a fixed protocol would be stronger. It is not free, and the three
  agreeing measurements above already carry the conclusion.


## CONFIRMED AGAIN, 2026-09-10 — training loss ROSE 31% while the net got measurably stronger

The deep-datagen run, mean training loss binned over its own log:

```
  gens     1-  443   0.01643
  gens   444-  886   0.01837
  gens   887- 1329   0.01971
  gens  1330- 1772   0.02034
  gens  1773- 2215   0.02153     <- +31% over the run
```

Over the same span the net passed two head-to-head gates against successive champions
(**0.622 ± 0.022**, then **0.586 ± 0.020**, 448 pairs each at depth 4). So loss rose monotonically
while strength rose.

**The mechanism is a moving target, not divergence.** The label is the net's OWN deeper search, so as
the net improves the labels become more informative and harder to fit, and the replay pool grows more
varied (20,323 positions at the point of measurement). A falling loss here would more likely mean the
labels had stopped carrying new information.

This matters because rising loss is the obvious thing to read as "something is wrong" and act on. It
is not. **Nothing in this file has changed: only games measure strength.** Loss is not merely a weak
proxy here, it points the wrong way.
