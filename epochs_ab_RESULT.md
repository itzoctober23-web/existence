# epochs A/B — RESULT (2026-09-08): under-fitting REFUTED, the labels are the problem

Three arms from the same champion, equal wall-clock, gate-pairs 224. The measurement is the
distribution of `mcnemar_z`, the loop's own paired candidate-vs-champion comparison on held-out
positions — it uses no games, so nothing here can be blamed on gate resolution.

| epochs | generations | train_loss (median) | mcnemar_z (median) | candidate better |
|---|---|---|---|---|
| 3 (current default) | 26 | 0.041705 | **+0.359** | 14/26 (54%) |
| 10 | 27 | 0.031720 | +0.141 | 14/27 (52%) |
| 30 | 27 | **0.025284** | **-0.445** | 9/27 (33%) |

## Against the pre-registration, written before the numbers existed

epochs_ab.sh said:

* *"If the mcnemar_z median moves clearly positive as epochs rise, the training step was
  UNDER-FITTING and epochs is a real lever."* — **Did not happen.**
* *"If the median stays at ~0 at 30 epochs, the optimisation is NOT the constraint and the problem
  is upstream — the labels or the surrogate. Report that as a refutation of the under-fitting
  hypothesis, not as inconclusive."* — **Happened, and worse than stated:** the median went
  NEGATIVE and the sign rate fell from 54% to 33%.

**The under-fitting hypothesis is refuted.** Training loss falls monotonically with epochs
(0.0417 -> 0.0317 -> 0.0253), so the optimiser is working exactly as intended: it fits its data
better every time. Candidate QUALITY moves the opposite way. Fitting these labels harder makes the
net worse at the one thing the loop measures it on.

## What that means

The bottleneck is not selection (fixed today: the 40-pair gate was accepting coin flips and
actively eroding the champion, 0.826 vs 0.859 for the 224-pair gate) and it is not optimisation
(this experiment). It is the TRAINING TARGET. The labels come from self-play at depth 2, and a net
that fits them better plays no better — which is what "the loop cannot learn" has meant all day:
seven scored nets, none above the 0.864 they started from.

## Do not conclude from this

* NOT "3 epochs is optimal". +0.359 at 26 generations is still a coin flip in absolute terms
  (14/26). The right reading is that all three arms are near zero and MORE fitting moves it down.
* NOT that the surrogate is proven sound. `paired_sign` compares only the SIGN of the eval, so it
  is a coarse instrument, and "the labels" and "the surrogate that scores them" are not separated
  by this experiment. Distinguishing those two is the next question, not a settled result.

## Next

Attack the label. Concretely: the depth-2 self-play target, the z used as the training signal, and
whether a deeper or differently-weighted target produces candidates that beat their parent. That
is a different experiment from anything run today.


---

## CORRECTED READING (2026-09-08, later): use the GAMES, not the surrogate

Everything above is scored on `mcnemar_z`, which has since been measured against actual game
results over 239 paired generations: **corr -0.095, 95% CI [-0.220, +0.032]** — it carries no
usable information about strength. So the z-ordering reported above should not be leaned on.

The same ledgers carry a 224-pair GATE RESULT for every generation, which is a strength
measurement. By that:

| epochs | generations | mean gate rate (0.5 = equal to the champion) |
|---|---|---|
| 3 | 26 | **0.5011** |
| 10 | 27 | **0.4715** |
| 30 | 27 | **0.4537** |

**More training makes candidates monotonically WEAKER, measured in games.** That is a stronger and
cleaner result than the surrogate version, and it does not depend on any surrogate being
trustworthy. It also independently confirms the pre-registered refutation of the under-fitting
hypothesis — by the metric that actually matters.

It is consistent with something already written in main.rs:465: *"training on ALL decided positions
makes the eval WORSE (sign acc 0.452 -> 0.441) ... Far-from-terminal labels are anti-signal while
both players are near-random."* Fitting these labels harder is known to hurt; this measures how
much, in games, at three training budgets.

### Two corrections to my own earlier notes

* The ledger's `loss` on NET rows is the **TRAINING** loss (`tr.epoch` over the trained-on subset),
  not the held-out loss. I renamed it `heldout_loss` earlier today; that name belongs to
  `train_from`'s `best_loss`, and `train_from` is called ONLY by the ARCH arm.
* I claimed the loop "keeps the epoch with the lowest held-out loss and therefore selects the
  weakest candidate". That early-stopping rule is real but lives in `train_from` — ARCH only. The
  NET arm runs `for e in 0..epochs` and keeps the LAST epoch, with no selection at all. The
  mechanism I described does not apply to the main path.

Both errors have the same shape: asserting which code path produced a value without following it.
