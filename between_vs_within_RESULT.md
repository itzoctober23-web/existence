# The week's central claim, reproduced on a THIRD instrument: agreement rises BETWEEN runs (t = +8.3) and is FLAT WITHIN one (t = +0.46)

**2026-09-12 01:10.** `WEEK1_RETRO.md` rests on *"every production run is FLAT on the absolute ruler;
all 166 Elo came from BETWEEN runs."* That was measured by the ruler, corroborated overnight by
`netmatch`. This is the same claim on `static_deep_residual --ref`, which shares no machinery with
either: it scores a net's STATIC eval against a FIXED external net's DEEP search, and reports a
correlation.

## The contrast, same instrument, same two references, same positions

```
WITHIN one run   p1_champion.net.r9.gen100 .. gen2200   22 checkpoints, 2,100 generations
  ref prodk1658   0.611 -> 0.618    slope +0.0029 ± 0.0064 per 1000 gens    t = +0.46   FLAT
  ref prodk1056   0.601 -> 0.617    slope +0.0042 ± 0.0062                  t = +0.67   FLAT

BETWEEN runs     the champion lineage, 10 promotion points spanning today's config changes
  ref prodk1658   0.681 -> 0.830    slope +0.0188 ± 0.0023 per step         t = +8.30   RISING
  ref prodk1056   0.678 -> 0.828    slope +0.0174 ± 0.0022                  t = +7.75   RISING
```

**A rise of +0.149 / +0.150 across the lineage, against a flat line within a run.** Both references
agree to three decimals on the rise and to within 8% on the slope. Random-net floors reproduce at 0.038
and 0.025, so neither run is reading a broken target.

## What the lineage spans — the config levers the retro named

```
pre_lr002 -> pre_lr0005 -> pre_lr00002    the learning-rate sequence (lr_sweep2_RESULT.md)
pre_blend085                              blend 0.85 (blend_sweep_RESULT.md)
g2052, g23056, g34789, g39836             promotions inside prodk1926
```

The retro's conclusion was that **configuration** levers paid and **structural** ones did not. This
instrument sees the same thing from a different direction: the eval's agreement with an independent
deep opinion climbs across the points where configuration changed, and does not move across 2,100
generations of training at fixed configuration.

## It also corroborates tonight's promotion result, independently

The lineage's last two points are the promotion that `promo_g39836_RESULT.md` resolved as a no-op:

```
ref prodk1658   g39836 0.838  ->  p1_champion 0.830   (−0.008)
ref prodk1056   g39836 0.827  ->  p1_champion 0.828   (+0.001)
```

**The promoted net does not out-agree its predecessor** — the step that the 953-pair netmatch called
`0.503 ± 0.014, INDISTINGUISHABLE` is also the one step in the lineage with no rise, on both references.
Three instruments, one answer.

## Limits, stated

* **The lineage points are NOT equally spaced**, in generations or in wall time, and they mix promotion
  events with configuration changes. The slope "per promotion step" is therefore an ordering statistic,
  not a rate. The claim it supports is the CONTRAST with a flat within-run series, not its magnitude.
* **Confounded by design, and that is the point.** The lineage deliberately includes config changes;
  isolating which lever moved the correlation would need one arm per lever, which `lr_sweep2` and
  `blend_sweep` already did against the game gate.
* **Both references are production nets** from other arms. They are not downstream of the final
  champion (mtimes 18:58 and 16:56 vs 23:04), but they are not unrelated either — a reference from an
  entirely different architecture would be a stronger control and does not exist.
