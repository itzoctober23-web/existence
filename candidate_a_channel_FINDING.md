# Candidate A's label channel is LIVE, but it is the channel `label_source` measured as null — the arm needs a third cell

**2026-09-12.** Two existing results both bear on Candidate A and appear to disagree. They do not;
they measured different things, and the difference decides how Candidate A must be run.

| result | what varied | outcome |
|---|---|---|
| `datagen_depth_RESULT.md` | `--depth`, each arm generating its OWN data (610 gens at d1 vs **6** at d3) | **+128 ± 72 Elo, RESOLVED** |
| `label_source_RESULT.md` | the label COLUMN only, positions/trainer/seed identical | +49 Elo against ±58/±69, **UNRESOLVED** |

`datagen_depth` moved labels, the position distribution, and sample efficiency together.
`label_source` isolated the label and found nothing that resolves — even though the label it
substituted was **Stockfish @10k nodes** and was demonstrably learned (11x better held-out fit than
untrained). Its own conclusion: *"better labels were absorbed; strength did not follow."*

**Candidate A's registered mechanism is the label channel** — "label quality is worst exactly where
positions are hardest". That is the `label_source` channel. Its empirical motivation is
`datagen_depth`, which is the other one.

## Measured: the label channel is not negligible

Before assuming either way, measure how much the label actually moves. Positions held FIXED — one
trajectory, driven by the depth-3 control, with both labellers scoring every position, so the
comparison cannot smuggle the position distribution back in.

`crates/pipeline/examples/label_delta_budget_vs_depth.rs`, champion net, budget 5,269, 25 games/seed:

| | 20260912 | 777001 | 424242 |
|---|---|---|---|
| positions | 2556 | 2678 | 3144 |
| same best move | 67.7% | 65.2% | 69.4% |
| same sign of eval | 92.3% | 84.2% | 94.4% |
| median \|Δ\| (trainer units) | 0.000 | 0.007 | 0.000 |
| p90 \|Δ\| (trainer units) | 0.220 | 0.422 | 0.303 |
| labels moving < 0.01 | 60.7% | 51.0% | 65.4% |

The distribution is sharply bimodal: **half the labels are identical and the rest move a lot.** That
is exactly what `budget_realised_depth_RESULT.md` predicts — 47-58% of positions reach depth 3 under
the budget and get the identical label; the 31-38% that stop at depth 2 and the 9-15% that reach
depth 4-6 are where the movement is. Two independent measurements agreeing on the same split is the
useful part.

### The control, run before any of the above was believed

Both labellers share one `Searcher`, and `shuffle_children` advances `self.rng` on every visit
(`search.rs:110`), so two searches from the same searcher explore different child orderings. Some of
the disagreement above could therefore be shuffle noise rather than depth. Same probe, same seeds,
with label B replaced by **a second depth-3 search** — identical method, so everything it reports is
noise:

| | 20260912 | 777001 | 424242 |
|---|---|---|---|
| same best move | 95.6% | 96.7% | 96.2% |
| same sign of eval | 100.0% | 100.0% | 100.0% |
| labels moving < 0.01 | **100.0%** | **100.0%** | **100.0%** |

The score is identical on **every single position** — exact alpha-beta returns the same value
whatever order the children are visited, so move ORDER can flip which of two equal-scoring moves is
returned (~4% of the time) but cannot move the LABEL at all.

That makes the treatment signal clean: the control's label-movement floor is 0%, so the 35-49% of
labels that move in the treatment are **entirely** depth-driven. The ~4% move-choice noise should be
subtracted from the move column, leaving roughly 31-35% of positions with a genuinely different best
move.

So the label channel is **live**: about one position in three gets a different best move, and the
p90 label shift is 0.22-0.42 on a target confined to [-1, 1].

*(The centipawn column is reported too but is the wrong unit for this question: its mean is inflated
by mate scores — max |Δ| ~27,000 cp — which `tanh(cp/600)` squashes to nothing. `Sample.root` is the
tanh, so the tanh column is the one that reaches the trainer.)*

## What this changes about the arm

It does **not** say Candidate A will fail. The budget arm regenerates its own data, so it sits in
the `datagen_depth` regime where the resolved +128 lives, not the positions-fixed regime where the
null lives.

It says the result **will not be interpretable as registered**. If the budget arm wins, the PREREG
would credit label quality — and `label_source` is direct evidence that the label channel alone does
not convert to strength at far larger label upgrades than this one. The win would more likely be the
position distribution, which the PREREG does not mention at all.

**Recommended third cell, cheap and already demonstrated.** `label_source` shows how: take the
control arm's positions and RELABEL them with the budget search — "literally one column apart", no
regeneration. Three arms then decompose the effect:

```
A  fixed depth 3, own data                 (control)
B  budget 5269, own data                   (the registered arm: labels + positions)
C  fixed depth 3 POSITIONS, budget labels   (labels ONLY)
```

`B - C` is the position-distribution contribution and `C - A` is the label contribution. Without C,
a win is uninterpretable and a null cannot distinguish "budget does not help" from "the label
channel is null again, as already measured".

## Status

No arm launched. This is a synthesis of two existing results plus one new measurement of the label
delta; the Elo figures quoted are from the files named, not re-measured here.
