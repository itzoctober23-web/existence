# The accept/reject surrogate does not predict strength — measured (2026-09-08)

Every generation records BOTH the paired surrogate (`mcnemar_z`, candidate vs champion on held-out
positions) and a 224-pair gate result (`gates[0].rate`, the candidate playing the champion, ci95
~0.031). Those are a surrogate value and a strength measurement on the SAME candidate. Nothing had
ever correlated them.

| run | n | corr(mcnemar_z, gate rate) | mean gate rate |
|---|---|---|---|
| gp_224 | 35 | -0.226 | 0.5021 |
| gp_40 | 41 | -0.276 | 0.5116 |
| ep_3 | 26 | +0.033 | 0.5011 |
| ep_10 | 27 | -0.089 | 0.4715 |
| ep_30 | 27 | -0.441 | 0.4537 |
| rp_8 | 42 | -0.171 | 0.5275 |
| rp_999 | 41 | -0.273 | 0.5212 |

**Pooled: n = 239, corr = -0.095, 95% CI [-0.220, +0.032].**

## Two conclusions, both measured

**1. The surrogate carries no usable information about strength.** The interval includes zero, so
this is not "significantly anti-correlated" — but it decisively excludes the strong positive
correlation a decision metric needs. Six of the seven runs are negative. This is the statistic the
loop falls back on whenever the games do not resolve, and it has never been validated against the
games until now.

**2. The training step produces candidates equal to their parent — by GAMES, not by surrogate.**
Mean gate rate over 239 candidates is 0.5024, where 0.5 is "identical strength to the champion".
That is the cleanest statement of "the loop does not learn" available, and it does not depend on
any surrogate being trustworthy.

## Why this supersedes today's earlier reasoning

The epochs A/B showed held-out loss improving while the paired statistic degraded, and I read that
first as "the labels are bad", then as "draws pollute the surrogate". The second was RETRACTED —
draws are filtered upstream at main.rs:471 and never reach the metric. The correct reading needed
no new experiment: the surrogate simply does not track strength, so its movement in any direction
carries little information about anything.

A contributing mechanism is visible in the code and is consistent with all of it: `Trainer::loss`
scores MSE against `target = (1 - blend) * z + blend * root` at the shipped blend of **0.75**, so
the training objective is three-quarters the net's own root search score, while `paired_sign`
scores agreement with the game result `z` alone. Loss and sign-agreement track different targets,
and neither has been shown to track strength.

## What must NOT be concluded

* NOT "remove the surrogate". When the gate does not resolve, something has to decide, and
  accepting nothing is itself a policy with a cost. What is established is that the CURRENT
  surrogate should not be trusted to override games.
* NOT that the gate rate is a perfect metric. Each is 224 pairs at ci95 ~0.031, so a single
  generation's rate is noisy; the pooled correlation is what carries the weight here, not any one
  row.
* NOT that blend 0.75 is wrong. It was gated (hyper_ab.rs: 0.4805 / 0.4898 / 0.5188 / 0.5258 at
  blends 0.00 / 0.25 / 0.50 / 0.75) and won. It explains the divergence; it is not thereby a bug.

## Next

A surrogate must be selected by its correlation with the gate, not assumed. The candidates already
on disk carry both numbers, so alternative surrogates — held-out loss, loss against `z` alone,
magnitude-weighted agreement — can be scored against the same 239 paired observations WITHOUT
running a single new game.
