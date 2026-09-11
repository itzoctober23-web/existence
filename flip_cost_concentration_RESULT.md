# The prize is concentrated — 38% of all flip cost sits in the top decile — and the proposed key does not fit it

2026-09-11. Measured with `confident_when_wrong 80 3 3 20260911 p1_champion.net`, which gained a
concentration statistic today. The numbers already existed inside that tool once `costs` was
computed; nothing new was searched to get them.

## Why this was worth asking

Decision-theoretic search spends compute where it changes the ROOT DECISION. That only pays if the
opportunity is **concentrated**. If every position is about equally likely to flip, and flips cost
about the same everywhere, then a PERFECT allocator has nothing to allocate toward and the ceiling
on the whole paradigm is zero — no uncertainty head, however well calibrated, can beat a flat
distribution.

That bound is independent of whether the head's target is right, which is the separate question
`uncertainty_target_PREREG.md` registers. It is worth knowing first because it is cheap and it
bounds everything downstream.

## The measurement

```
p1_champion.net   80 positions   37 flips (46.2%)   median cost ratio 815x   verdict NO SIGNAL
  of those, 33 cost >= 10cp (a real error, not a tie-break); low-conf on THOSE: 33.3%
  COST CONCENTRATION: top decile (4 of 37 flips) holds 38% of all flip cost;
                      7 flips hold half.        Flat would be 10% / 18.
```

**The prize is real and it is concentrated.** The top decile of flips carries **38%** of all the cost
against **10%** if it were uniform — a 3.8x enrichment. Half of all the cost sits in **7** flips
where a flat distribution would need 18.

So an allocator that could identify those few positions has something substantial to win. The
precondition for the paradigm holds.

## And the proposed key does not fit that lock

The same run re-measures what `confident_when_wrong_RESULT.md` established: confidence, defined as a
SMALL static-vs-deep residual, is **anti**-correlated with costly flips. Low-confidence on flips is
29.7%, and on the flips that genuinely cost material it is **33.3%** — against a 50% base rate.

Put together, the picture is sharper than either half alone:

* the opportunity is **concentrated** (3.8x over flat), so allocation is worth doing;
* `|static - deep|` **does not find it** — it is slightly enriched in the wrong direction.

That is not an argument against decision-theoretic search. It is an argument that the head must be
trained on something other than the score residual, which is exactly what the registered decision
rule says to settle before Track B's step 2 spends a ladder on it.

## What is NOT established

**This is 80 positions, 37 flips, one net, one seed, depth 3 against depth 6.** The concentration
figure is a single sample from a small denominator: 4 flips in the top decile means the 38% moves
several points if any one of them lands differently. It is enough to say the distribution is not
flat — 3.8x is a long way from 1.0x — and not enough to quote a precise multiple.

**Nothing here says a head could learn to find those positions.** Concentration says the target is
worth hitting. Learnability is a different measurement and needs the head trained.

**"NO SIGNAL" in the verdict column is about the FITNESS §8 check**, not about this file's finding.
That check asks whether confidence is low on >=80% of flips and the answer is no, decisively.

## What follows

The registered rule in `uncertainty_target_PREREG.md` stands and is now better motivated: T1 is
refuted as a ranker, so the head should be trained on flip cost (T2) and evaluated on whether it can
rank the concentrated tail. The prize is measured; the key is not yet cut.
