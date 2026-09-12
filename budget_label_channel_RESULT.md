# The node budget's LABEL channel is null — 0.4960 with 47.8% of labels changed, against a −43 Elo full effect

**2026-09-12 05:26.** `run_budget_label_ab.sh` / `budget_label_ab.rs`, the fixed-corpus isolation
designed in `candidate_a_channel_FINDING.md` after the live "cell C" arm was shown unable to do it.

## The construction, and why it is trustworthy

One corpus, scored twice, with **only the `root` column swapped**. The trajectory is driven by the
depth-3 search alone, so both arms see byte-identical positions:

```
corpus 6220 positions, identical FEN/z/plies in both arms (asserted, not assumed)
labels differing: 2974 (47.8%)
split 5560 train / 660 holdout (by FEN hash, not file position)
```

The intervention is **large** — nearly half the labels changed — so a null cannot be dismissed as
"the flag was inert". The assertion is in the binary: it compares every row's FEN, `z` and
`plies_to_end` across arms and aborts if any differ.

**The validity trap is ruled out.** `label_source_RESULT.md` had to eliminate "both arms
undertrained, so the comparison is void rather than negative" before it could report. Held-out MSE
over 660 positions:

```
net              vs depth label    vs budget label
depth                   0.10007            0.10995
budget                  0.10091            0.10425
untrained               0.46916            0.46913
```

Each arm fits **its own** column best (depth wins the depth column 0.10007 vs 0.10091; budget wins
the budget column 0.10425 vs 0.10995), and both are ~4.6× better than untrained. The arms learned,
they learned *different* things, and they learned the thing each was given.

## The result

```
seed 20260907    0.499 +/- 0.026
seed 911911      0.493 +/- 0.026
pooled           0.4960 +/- 0.0184     95% CI [0.4776, 0.5144]
```

**Contains 0.5. NULL — about −2.8 Elo.**

## Reproduced three times, by accident — and what that does and does not buy

A marker bug re-ran this gate on every 5-minute timer fire (the script wrote its completion marker
with `touch`, creating a zero-byte file, while its own guard tested `-s`, which is true only for
non-zero size). Three complete runs executed before it was caught, each retraining both arms from
scratch:

```
run 1  05:13 / 05:16    0.499 +/- 0.026    0.493 +/- 0.026    labels differing 2974 (47.8%)
run 2  05:23 / 05:26    0.499 +/- 0.026    0.493 +/- 0.026    labels differing 2974 (47.8%)
run 3  05:33           0.499 +/- 0.026    (stopped)          labels differing 2974 (47.8%)
```

**Identical to three decimals, including the corpus construction.** That is worth recording because
it shows the whole pipeline — corpus generation, the 47.8% label divergence, training, and the
matches — is deterministic given its seed, so this result is exactly reproducible by re-running the
script.

**It buys no statistical power.** Same seed, same corpus, same nets, same match seeds: these are the
same measurement three times, not three measurements. The interval stays ±0.018 on 448 pairs. Saying
otherwise would be the `dedup-key-too-coarse` error in another costume — pooling readings that are
not independent.

## What it means, against Candidate A

```
full budget, live self-play arm    0.4383  CI [0.4008, 0.4758]   -43.1 Elo   RESOLVED BELOW
label channel alone, fixed corpus  0.4960  CI [0.4776, 0.5144]    -2.8 Elo   NULL
```

`candidate_a_budget_loses_RESULT.md` measured the full node budget costing ~43 Elo. The replication
has since finished (`candidate_a_replication_RESULT.md`) and the picture is more nuanced than
"reproduced": head-to-head B-vs-A came back **UNRESOLVED** at 0.4530, landing exactly on the gate's
0.047 between-seed threshold, where the original at 0.4383 had cleared it. What replicated cleanly
is each arm against the **shared frozen start** — the budget arm finishes BELOW the net it started
from in both independent runs (0.445 and 0.422; joint P = 0.0059 under the null).

**Isolating the label channel recovers none of that harm.** This is not an underpowered null: the
pooled interval is ±0.018 against a full effect of 0.062 below parity, so an effect the size of
Candidate A's would have been resolved here roughly 3× over. The label contribution is bounded at
**at most ~±13 Elo**, and the point estimate is ~3.

`candidate_a_channel_FINDING.md` listed three channels the budget moves at once — the labels, the
position distribution, and the decisive-game rate. This removes the first from suspicion and leaves
the other two.

## The limitation that matters, stated plainly

**This does not rule out a compounding label effect.** The design deliberately removes self-play so
the corpus cannot drift — that is the whole reason it works where cell C could not, because in a
self-play loop the data generator IS the net being trained and the arms diverge at generation 2
(measured: positions identical on 4 of 1180 generations).

But removing compounding also removes the mechanism by which a per-generation label bias too small
to see here could accumulate over 2,000 generations. What is measured is: **in a single training pass
on a fixed corpus, differing labels produce no measurable strength difference.** Extending that to
"labels are harmless in the live loop" is a step this experiment cannot take, and the distinction is
the same one `generation_is_not_a_unit_FINDING.md` records.

A design that could take that step would need the labels to differ while the positions are held
fixed *across generations*, which is the thing the self-play loop structurally forbids.

## Standing

Both nets are **DIAGNOSTIC**. `bla_depth.net` and `bla_budget.net` are trained on 6,220 positions
for 12 epochs and must never be shipped or gated. Nothing here ships; nothing here is quoted as Elo
gained.
