# The uncertainty signal is INVERTED where it exists at all — and it does NOT exist on every net

> **⚠ HEADLINE CORRECTED 2026-09-11, after replication. The original read "The uncertainty signal is
> REAL and its SIGN IS BACKWARDS — allocate where the net looks most confident", written from ONE
> net. Two further nets say that is too strong: the effect is NET-DEPENDENT, present on the champion
> lineage and ABSENT on `epochs_03`. The replication section at the bottom is the current state; read
> the original body as the first sample, not as the conclusion.**

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


## REPLICATION — two more nets, a different seed, and a mixed answer

The file above states its own limit: one net, one seed. `confident_when_wrong 200 3 3 20260913` on
`p1_champion_prev_g34789.net` and `epochs_03.net`, 138 train / 60 holdout each, probed per net
(`unc_probe.py` refuses to pool a multi-net dump, because pooling would average away the very thing
being replicated).

```
                         p1_champion(orig)   prev_g34789        epochs_03
                          n=89, 38 costly    n=60, 16 costly    n=60, 18 costly
probe on T2 flip cost          0.363             0.365             0.521
probe on T1 residual           0.305             0.429             0.485
rank by RAW residual           0.349             0.340             0.457
random band            [0.394, 0.600]    [0.374, 0.629]    [0.354, 0.649]

NEGATED
NEG probe T2                   0.637             0.635             0.479
NEG probe T1                   0.695             0.571             0.515
NEG raw residual               0.651             0.660             0.543
```

**`p1_champion`** — all three rankers outside the band. The original result.

**`p1_champion_prev_g34789`** — PARTIAL. The raw residual replicates cleanly (0.340, negated 0.660,
outside a [0.374, 0.629] band). The T2 probe is marginal, sitting a hair outside on both sides. The
T1 probe is INSIDE the band and does not replicate.

**`epochs_03`** — ABSENT. 0.521, 0.485, 0.457 are not "inside a wide band", they are centred on
0.5. There is no signal here to invert.

## What this changes

**The effect is a property of SOME NETS, not of the architecture.** Two champion-lineage nets carry
it; `epochs_03`, trained under a different configuration, does not. That is the difference between
"the head can be used with a minus sign" and "the head can be used with a minus sign ON THIS NET",
and only the second is supported.

**For Track B this is the important half.** An allocator keyed to the inverted signal would work on
the champion and do nothing on `epochs_03` — and a step-2 ladder that happened to run on the wrong
net would return flat and read as a refutation of the paradigm. Any use of this must name the net it
was calibrated on and re-check after a promotion, because the champion changes.

**The strongest and most portable form is the RAW RESIDUAL, not a probe.** It replicates on both
champion-lineage nets (0.349 / 0.340) where the fitted probes do not, which makes sense: a probe
fitted on 138 rows of one net is fitting that net's quirks, while the raw residual is the same
quantity everywhere. If anything gets built, it should be built on the residual and negated.

## Honest limits of the replication itself

The replication holdouts are 60 points against the original's 89, so their bands are wider and the
power lower. `epochs_03` returning ~0.5 is consistent with a real absence AND with a true effect too
small to see at n=60 — those are not distinguished here. What IS distinguished is the original
headline: it claimed a general property, and one net out of three showing nothing is enough to
withdraw that claim, whatever the reason.
