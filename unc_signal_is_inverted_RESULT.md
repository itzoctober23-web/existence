# The uncertainty signal is REAL and its SIGN IS BACKWARDS — allocate where the net looks most confident

2026-09-11. `unc_probe.py` on 296 labelled positions (`confident_when_wrong 300 3 3 20260912
p1_champion.net` with `UNC_DUMP`), 207 train / 89 holdout, 38 costly flips held out.

## The question this answers

`uncertainty_target_PREREG.md` registered the rule: settle the head's TARGET before Track B's step 2
rests on it. `flip_cost_concentration_RESULT.md` established the prize is worth chasing — the top
decile of flips carries 38% of all flip cost, 3.8x uniform. What was open is whether the head could
LEARN to find that tail.

**A linear probe is the exact test, not a weakened one.** `Net::spread_from` IS a linear map over the
hidden layer — 17 parameters at width 16. A probe on those same activations is the head's own
hypothesis class, so a null here would mean the head cannot be trained to it, not that training was
done badly.

## The measurement

```
                          AUC   (0.50 = chance)
probe on T2 flip cost    0.363
probe on T1 residual     0.305
rank by RAW residual     0.349
RANDOM control           0.502 median, 90% of 200 trials in [0.394, 0.600]

the SAME rankings, NEGATED
NEG probe on T2          0.637
NEG probe on T1          0.695
NEG raw residual         0.651
```

**Every ranker sits BELOW the random band, and every negation sits ABOVE it.** An AUC reliably below
chance is not an absent signal — it is an inverted one, and an inverted ranker is a usable ranker
with a minus sign.

The strongest is the negated residual probe at **0.695**: rank positions by how LOW the net's
predicted uncertainty is, and the costly flips come to the top.

## Why this is the same fact confident_when_wrong found, now in a usable form

That file measured the engine is MORE confident where its cheap search is wrong — 33.3% low-confidence
on costly flips against a 50% base rate — and called it a failure of FITNESS §8, which it is. What
this adds is that the relationship is strong enough and consistent enough to RANK on. The defect and
the signal are the same phenomenon read in opposite directions.

So the decision-theoretic allocator does not need a better-calibrated head. It needs the head it can
already have, used with the opposite sign: **spend compute where the eval is most sure of itself.**

That is counterintuitive and it is what the data says. The mechanism is not mysterious — a position
the static eval finds obvious is one where it has stopped looking, and "stopped looking" is exactly
where a deeper search overturns the move.

## What is NOT established

**The three rankers are NOT three independent confirmations.** All are derived from the same net on
the same positions; the two probes differ only in their training target and the raw residual is what
one of them is fitted to. Treat this as ONE effect measured three ways, not three effects.

**89 held-out points, 38 costly, one net, one seed, depth 3 against depth 6.** AUC 0.695 is outside
the random band but it is not a large effect, and the band itself is from 200 permutations of this
one holdout. A second net and a second seed would make it a finding rather than an observation.

**No Elo is claimed and none is implied.** This says a ranking exists. Whether spending search by
that ranking WINS GAMES is Track B step 2, which is precisely the measurement this was meant to
protect from being run on an unexamined signal.

**The decile metric from the first pass is withdrawn, not merely superseded.** At 9 held-out
positions the random control returned 0.78x where it must average 1.0x, so the differences between
11.1%, 22.2% and 33.3% were one or two positions. That reading measured the instrument. AUC replaced
it because it uses all 89 points and needs no binning.

## What follows

The registered rule resolves: the head trains on either target — both invert — and the ALLOCATOR
consumes it negated. Recording the sign is the whole point, because a head wired the intuitive way
round would spend its compute in exactly the wrong places and the ladder would report a flat result
that looked like a refutation of the paradigm.
