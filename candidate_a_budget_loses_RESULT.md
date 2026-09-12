# Candidate A: the node-budget arm LOSES to fixed depth — 0.4383 over 672 pairs, and the registered mechanism was backwards

**2026-09-12 03:23.** `structural_next_PREREG.md` ranked Candidate A first on expected Elo: replace
fixed-depth datagen with a real per-move node budget. Both arms ran to completion tonight and the
budget arm is **behind**.

## The measurement

Both arms from the SAME frozen champion (`cand_start.net`, md5 `9545a35289e9`), same seed 20260912,
same binary, `--gens 2000 --games 8 --depth 3`, matched on GENERATIONS as the PREREG requires.
Only `--datagen-budget 5269` differs — the budget being the measured mean cost of a depth-3 move
(`datagen_node_census_RESULT.md`). netmatch at fixed depth 4, 224 pairs per seed, so the instrument
is load-immune.

| match seed | budget arm vs fixed-depth arm |
|---|---|
| 20260907 | 0.439 ± 0.028 |
| 911911 | 0.433 ± 0.029 |
| 424242 | 0.443 ± 0.029 |

```
pooled over 672 pairs   0.4383   95% CI [0.4008, 0.4758]
between-match-seed sd   0.0050
in Elo                  -43.1    [-69.8, -16.8]
```

Every individual interval lies wholly below 0.5, and the pooled interval excludes 0.5. **For these
two nets the result is resolved: the budget arm is weaker.**

## The budget arm is worse than the net it STARTED from

The gate's context matches, both at 224 pairs, seed 20260907:

```
candA_fixed vs cand_start   0.535 +/- 0.030   [0.505, 0.564]
candB_budget vs cand_start  0.445 +/- 0.027   [0.418, 0.472]
```

**B's interval lies entirely below 0.5**, and `|0.445 - 0.5| = 0.055` is **1.17x** the 0.047
between-seed sd — beyond the band, unlike everything else here. Two thousand generations of
budget-allocated datagen did not merely underperform fixed depth; they left the net **weaker than the
champion it resumed from**.

That is the strongest single statement this experiment supports, and it is consistent with the
mechanism: the budget starves wide positions, and training on labels that are worse exactly where
evaluation is hardest moves the net backwards.

## Context: arm A is NOT shown to have improved — and 0.535 is a number that already fooled us tonight

The gate's context match reads **`candA_fixed vs cand_start` = 0.535 ± 0.030**, interval
[0.505, 0.565]. Taken alone that says the fixed-depth arm gained on the champion it resumed from,
which would make the budget arm's loss a regression against a working baseline.

**It does not survive its own standard.** `|0.535 - 0.5| = 0.035` is **0.74x** the 0.047
between-training-seed sd — inside the band, on one training run.

And there is a sharper check available. `auto_promote` has taken 20 readings of live production arms
against the champion tonight:

```
min 0.450   max 0.539   mean 0.502   sd 0.021
readings >= 0.535:  2 of 20
```

That distribution is centred on **no improvement**. Arm A's 0.535 is in its top tenth, not outside
it. Decisively: the single reading at the top of that range — **0.539 at 23:04** — is the SPURIOUS
PROMOTION diagnosed in `promo_g39836_RESULT.md`, which resolved to 0.501 ± 0.021 and 0.503 ± 0.014 at
448 and 953 pairs. A 0.535 single reading is exactly the magnitude this project has already been
fooled by once today.

So the honest reading is that **neither arm is shown to have improved on the start net**, and the
well-powered statement is the one between the arms: B loses to A over 672 pairs and three match
seeds. The vs-start readings are one seed and 224 pairs each and are reported as context only.

## The limitation that keeps this from being a general claim

`|mean - 0.5| = 0.0617`, which is only **1.31x** the project's between-seed sd of **0.047** — and
that 0.047 is the spread between **re-trained runs**, not between re-played matches. The three seeds
above re-play the *match* on the *same two nets*, which is why their spread is a tiny 0.005. There is
**one training run per arm**.

So this resolves "net B is weaker than net A" and only *suggests* "a node budget is weaker than
fixed depth". Establishing the general claim needs a second training seed per arm, and at 1.31x the
relevant noise that is not a formality. Reported as a **bound**, in the PREREG's own words: *over
2000 generations, at a measured realised-depth spread of 2-6, the node budget did not beat fixed
depth, and lost by 0.0617 ± 0.0375 of score.*

## The mechanism, and why the PREREG had it backwards

Three measurements taken tonight, each on 3 seeds, make the loss coherent rather than surprising:

1. **The budget gives NARROW positions more depth and WIDE positions less**
   (`budget_realised_depth_RESULT.md`). Branching against realised depth is monotone decreasing on
   every seed: ~38 legal moves at depth 2, ~21 at depth 3, ~9-13 at depth 4, ~3-4 at depth 5. The
   PREREG stated the opposite — *"a hard position gets more depth and a simple one less"* — which is
   not what equal-NODE accounting does.

2. **The budget arm gets MORE data, not less** (`budget_makes_games_decisive_RESULT.md`): self-play
   is 14.5 points more decisive (55.4% -> 69.9%), games are 9.6% shorter, and usable training rows
   rise **26.7%** with rows-per-decisive-game unchanged.

3. **And it still loses.** More depth where positions are forcing, more decisive games, 27% more
   training data — and 0.4383.

The reading that fits: **equalising NODES is worse than equalising DEPTH, because a wide position
needs more nodes, not fewer.** A budget starves exactly the positions whose evaluation is hardest,
and buys depth in forcing lines where the search was already adequate. That is the inverse of the
registered rationale, and it is what the data says.

## Status

**Nothing shipped, no action on production.** The arms' nets are diagnostic artefacts. Candidate A as
registered is answered in the negative for these nets; `structural_next_PREREG.md` ranks
B (rolling data window) second and C (ARCH width) closed.

The label channel remains unseparated — `candidate_a_channel_FINDING.md` records why the live cell C
arm cannot isolate it, and `budget_label_ab.rs` is the corrected fixed-corpus design, built and
smoke-tested but not yet run at size.
