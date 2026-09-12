# Two sound instruments disagree about the same net: does the promotion ladder track ABSOLUTE strength?

**2026-09-12 04:50. Pre-registered before the measurement returns.** The decision rule below is
fixed now so the result cannot be read to suit whichever answer arrives.

## The disagreement

At 04:43 `auto_promote.sh` promoted `prodk0127.net` at generation 24941, and for the first time the
896-pair confirmation stage fired:

```
04:43 prodk0127.net gen 24941: CONFIRMED 0.536 +/- 0.015 at 896 pairs seed 911911
04:43 prodk0127.net gen 24941: PROMOTED   0.535 +/- 0.031
```

Two independent reads — 224 pairs on one seed, 896 on another — agreeing to 0.001. That is the
strongest evidence standard this project has. It says the promoted net beats its comparator by
**+25.1 Elo, 95% CI [+14.6, +35.6]**.

The comparator is preserved as `p1_champion_prev_g24941.net`, md5 **9545a35289e9** — the shipped
champion, and the net `prodk0127` was itself started from (`prodk0127_plateau_STATUS.md`: started
01:27 from the shipped champion). So the promotion claims a net beats its own starting point.

The live ruler, over that same lineage and the same span, says the opposite. 38 readings against an
unchanging SF-1320 @10k-node anchor, OLS of Elo on generation:

```
slope            -2.104 Elo per 1000 generations  (se 0.666)
over gen 291..29386   -61.2 Elo   95% CI [-99.2, -23.2]
```

**The intervals do not overlap.**

## Both instruments are sound, which is why this matters

Neither reading can be dismissed as a broken harness:

* the netmatch is **paired** (same openings both sides), runs at **fixed depth 4** so node counts are
  deterministic and the result is load-immune, and it was **confirmed on an independent seed** at 4x
  the pairs.
* the ruler runs at **fixed depth 4 against fixed 10k nodes** — also load-immune — **snapshots the
  net to /tmp before playing** so it cannot read a half-written file, is pinned to 6-11, and every
  one of the 38 readings used the same anchor.

The resume transient cuts the right way too: a resumed arm reads ~95 Elo LOW at first
(`resume_dip_RESULT.md`), which would manufacture a RISING trend. The observed trend is falling, so
the transient makes this estimate conservative rather than explaining it.

## CORRECTION, 05:08 — the hypothesis below is mis-framed, and `nontransitive_walk_RESULT.md` says why

Written before I re-read the index, which the rules require and which I did only after launching.
That file already settles part of this, and it rules OUT the framing used below.

It measured three independent 224-pair matches along one lineage and checked that they COMPOSE:

```text
  gen 2162 -> 4818    +28.6
  gen 4818 -> 6803     -0.7
  sum                 +27.9
  measured 2162->6803 +27.9      difference 0.00 Elo
```

**At the ~2,000-generation scale this walk is TRANSITIVE — gains add, to within 0.01 Elo.** That file
explicitly narrows its own headline to say non-transitivity, if present at all, is a SHORT-RANGE
effect at 5-generation steps, and that long-range comparisons are self-consistent.

A promotion spans ~25,000 generations, which is long-range. So "self-play non-transitivity" does not
explain the disagreement, and the hypothesis below is wrong as stated.

**The corrected hypothesis.** That composition check demonstrates netmatch is *internally* consistent
— it is a reliable RELATIVE instrument, and nothing here impugns it. But internal consistency across
comparisons drawn from one opponent distribution says nothing about agreement with a DIFFERENT
opponent. The disagreement is not transitivity failing; it is that **strength measured against the
net's own self-play family does not transfer to strength against an external opponent.** That is
distribution overfitting, and it is a sharper claim than the one below, because it survives the
composition check instead of contradicting it.

The measurement and the decision rule are unchanged and still correct — they compare the two champion
FILES against the external anchor, which is exactly the quantity in question. Only the mechanism
named in the row headings changes: read "does not track absolute strength" rather than
"non-transitivity".

## The hypothesis

They disagree because they measure different things. One is strength against the net's **own
predecessor**; the other is strength against a **fixed external opponent**. Self-play
non-transitivity produces exactly this signature, and this project has already seen its smaller
cousin — `cell_c_transitivity_FINDING.md` measured a transitivity discrepancy that grew from 7.1 to
13.1 Elo.

If it is real, the consequence is not a detail: **the promotion criterion can be satisfied forever
without absolute progress**, and the plateau is then a SELECTION problem, not a training-rate one.
Consistent with that, absolute strength across the three most recent lineages is flat while the
ladder recorded confirmed promotions throughout:

```
prodk1658   n=24   +232.6 +/- 5.6   [222, 244]
prodk1926   n=79   +227.9 +/- 3.8   [220, 235]
prodk0127   n=38   +227.7 +/- 6.2   [215, 240]
```

Five promotions at ~0.535 each should compound to roughly +125 Elo. The observed change is zero.

## The measurement

`champion_absolute_ab.sh`: both champion FILES against the same anchor, 600 games each (the ruler's
usual 120 gives +/-66, so a difference of two such readings carries +/-93 — too wide to resolve a 61
Elo gap; 600 gives roughly +/-30 a side and +/-42 on the difference). Sequential, pinned, niced,
behind a measured capacity guard.

This compares the exact two files at stake, which the OLS trend does not: the trend runs to gen
29386 while the promoted net is gen 24941.

## Decision rule — fixed in advance

| outcome | reading |
|---|---|
| difference interval lies **wholly below 0** | The promotion made the champion WEAKER in absolute terms. The 896-pair netmatch is then measuring strength against the predecessor only, the promotion criterion does not track absolute strength, and the ladder is unsound as a selection mechanism. Revert `p1_champion.net` to the previous file and change the criterion before any further promotion. |
| difference interval lies **wholly above 0** | The promotion is real and absolute. The ruler's negative within-run trend is then not about the promoted net, `prodk0127_plateau_STATUS.md` must be redone, and the flat cross-lineage table above needs another explanation. |
| interval **contains 0** | UNRESOLVED. This neither confirms nor refutes the promotion. Report it as unresolved and state the games needed — do NOT report a direction, and do not let the point estimate's sign leak into the writeup. |

**Predicted, so it can be wrong:** the difference lands below 0, near the −61 the trend implies,
but with an interval wide enough that "wholly below 0" is not guaranteed at 600 games a side.

## What is NOT being claimed

Nothing here says the netmatch gate is broken. A paired, seed-confirmed, load-immune measurement
that says A beats B is evidence that **A beats B**. The question is only whether "beats its
predecessor" is the right thing to select on — and no Elo figure from this will be quoted as shipped,
because nothing here ships.
