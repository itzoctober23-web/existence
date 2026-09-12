# UNRESOLVED at 600 games a side — and the within-run OLS trend I cited as evidence is REFUTED

**2026-09-12 05:26.** Result of `champion_absolute_PREREG.md`, read against the decision rule fixed
before the measurement returned.

## The measurement

Both champion files against the same anchor (SF-1320 @10k nodes, our engine at fixed depth 4), 600
games each, sequential, pinned, behind a measured capacity guard:

```
NEW (promoted 04:43, c72eea5182d6)   +199 +/- 28    W-D-L 400-110-90   score 0.7583   (1064s)
OLD (shipped champ,  9545a35289e9)   +196 +/- 28    W-D-L 392-123-85   score 0.7558   (1098s)
DIFFERENCE                            +3   95% CI [-37, +43]
```

## The verdict: UNRESOLVED — the pre-registered third branch

The rule fixed in advance:

> interval **contains 0** → UNRESOLVED. This neither confirms nor refutes the promotion. Report it as
> unresolved and state the games needed — do NOT report a direction, and do not let the point
> estimate's sign leak into the writeup.

So: **no direction is reported.** What the interval can and cannot exclude:

```
the netmatch promotion claim   +25.1   INSIDE  [-37,+43]  -> not excluded
no change                        0.0   INSIDE             -> not excluded
the OLS within-run trend       -61.2   EXCLUDED
```

At this power the measurement cannot tell +25 from 0. It is **underpowered, not negative**, and the
two champions are close enough against a third party that 1,200 games could not separate them.

**What would resolve it.** The difference interval must fall under ±25 to exclude zero against a
+25 Elo claim. Variance scales as 1/n, so that is **2.6× the games — 1,536 per side, ~3,072 total,
about 92 minutes** at the observed rate.

**A better design at the same cost, for whoever runs it.** Both sides already play the same opponent
from the same default seed, so their opening sets are shared and the games are **paired** — but the
±28 reported per side is the unpaired binomial interval, and differencing two of them inflates by
√2 and discards the pairing entirely. `sf_ruler.py` emits only aggregate W-D-L, so a paired analysis
is impossible post hoc. Adding per-game output would cut the required games substantially for free,
and is the cheapest improvement available to this instrument.

## The part that is a correction to my own analysis

`champion_absolute_PREREG.md` framed this as "two sound instruments disagree" and cited the
within-run OLS — **−61.2 Elo over gen 291..29386, 95% CI [−99.2, −23.2]** — as one of them. The
direct measurement **excludes that value**. The OLS is refuted as evidence about the promoted net.

The prereg anticipated exactly this and said so before the result landed:

> the within-lineage OLS ... points the same way but has the same defect in miniature: its newest
> readings are at gen 29386, while the promoted net is gen 24941.

That caveat is now the finding. **A regression slope across a lineage's readings is not a
measurement of any particular net in it.** The OLS answers "does this series trend down on average",
fitting a straight line through 38 points with sd 38 spanning 29,000 generations. The promotion asks
"is THIS file better than THAT file". A net at generation 24941 can sit anywhere in that scatter, and
the slope tells you nothing about where. Formal significance (−2.10 ± 0.67 per 1000 gens, 3.2 sd) did
not make it the right quantity — it made a wrong quantity look authoritative, which is worse.

**Standing correction: do not use the ruler's within-run trend as evidence about a specific
checkpoint. Measure the checkpoint.**

## What survives, and what does not

**Survives — `promotion_ladder_decay_FINDING.md` is untouched by this.** Its table is built from
cross-lineage MEANS (n=79, n=67, n=38, se 3.8–6.2), not from a slope, and it does not depend on this
experiment. The absolute step across successive lineages decaying +78, +47, +27, −5, −0 while the
promotion margin holds at ~0.535 stands as measured.

**Does not survive — the claim that the 04:43 promotion specifically made the champion weaker.** It
was never asserted, because the prereg forbade asserting it before this measurement; now it is
explicitly unresolved.

**Not refuted — the promotion itself.** The 896-pair confirmation (0.536 ± 0.015, independent seed,
agreeing with the 224-pair read to 0.001) is untouched. +25.1 sits comfortably inside this interval.
Nothing here says that measurement was wrong, and nothing here ships.

**Not yet answered — the question the prereg was built for.** Whether the promotion criterion tracks
absolute strength is still open at the level of an individual promotion. The cross-lineage evidence
says the ladder has stopped delivering; this measurement cannot say whether any single promotion
does. Answering that needs the 1,536-a-side version, or the paired instrument.

## Instrument validation, which did pass

Two independent checks that the A/B arms are sound, both done before the verdict:

* the CI scaled as designed — the ruler's usual 120 games gives ±66, and 600 games gave **±28**
  against ±29.5 predicted by 66/√5.
* the NEW arm agrees with the live ruler's independent readings of that lineage. Pooling the 12
  readings in gen 20000–30000 gives **+213.2 ± 21.5, CI [+192, +235]**, against the A/B's
  **+199 ± 28, CI [+171, +227]** — overlapping.

So the null here is a statement about power, not about a broken harness.

## Note on timing

`p1_champion.net` was still `c72eea5182d6` when this finished, so the file measured is the file that
was live. `auto_promote` runs on a ~40-minute cycle and may promote again; both arms of this
measurement snapshot their nets at the start, and OLD is a fixed backup file, so neither can be
affected retroactively.
