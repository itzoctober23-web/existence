# The resume dip is invisible to the absolute ruler but REAL on the paired instrument — 0.362 at generation 108

**2026-09-12 08:55.** Measured from 285 live-ruler readings already on disk across six production
lineages. **No new compute.** It closes a change I was about to make.

## What prompted it

The production trainer is wrapped in `timeout 21600` (`p1_production.sh:90`) — exactly six hours — and
`keepalive.sh` relaunches it from the champion each time. That is by design, not a crash, and
`p1_production.sh:87` copies `p1_champion.net` to the start net so banked progress carries across.

`resume_dip_RESULT.md` measures a resumed arm at **~95 Elo below** the champion it resumed from,
recovering by ~4,300 generations. At ~55,700 generations per six-hour window that is 7.7% of each
window spent sub-parity, paid four times a day, during which promotion is impossible. Doubling the
timeout would halve that overhead — an apparently free throughput gain.

**The premise is not supported in production.**

## The measurement

Every lineage's ruler readings split at generation 4,300 — the recovery point that file reports —
against the same unchanging SF-1320 @10k anchor:

```
lineage        early (<4300)      late (>=4300)     early - late
prodk0759      157.2 (n=5)        159.4 (n=28)          -2.2
prodk1056      187.1 (n=7)        207.9 (n=60)         -20.7
prodk1658      224.0 (n=5)        234.9 (n=19)         -10.9
prodk1926      240.2 (n=6)        226.9 (n=73)         +13.2
prodk0127      258.2 (n=5)        223.0 (n=65)         +35.2
prodk0729      229.2 (n=5)        217.3 (n=7)          +11.9
```

A dip would show as a consistently NEGATIVE final column. It does not:

* **Sign is split 3–3.**
* **Five of six differences are inside the noise** of a 5–7 reading mean (readings have sd ≈ 38, so
  a 5-reading mean carries ±33 at 95%).
* **Pooled over six lineages: +4.4, 95% CI [−11.5, +20.4] — contains zero**, and the point estimate
  has the wrong sign for a dip.

## Why the two measurements disagree, and which to believe for this question

They measure different quantities, and that is the whole explanation:

* `resume_dip_RESULT.md` matched the resumed arm against **the champion it resumed from** — a paired
  comparison between two nets one resume apart.
* The ruler matches the arm against a **fixed external opponent**.

A net can be temporarily worse than its own immediate predecessor while being indistinguishable
against a third party at this resolution. Both can be true; only the second bears on "does the
restart cost production throughput", because promotion is decided against the champion but *strength*
is what the anchor sees.

**My 7.7%-of-window figure was a mis-application** — it took a paired-comparison result and treated it
as an absolute-strength deficit. Recorded so the arithmetic is not repeated.

## FOLLOW-UP, same session: the PAIRED instrument was already on disk, and it DOES see the dip

The section above closes by naming what would settle this: "a paired measurement of the production
arm against its own pre-restart state, taken across a restart boundary." That measurement already
exists — **`auto_promote.sh` performs it every cycle**, matching the live arm against the champion it
resumed from. 46 readings are in `auto_promote.out`.

Earliest reading per lineage, where the arm has barely moved off the champion:

```
lineage         gen    rate
prod1           108   0.362   <-- deep
prod4          1100   0.481
prodk0127      1317   0.518
prodk1056      1769   0.459
prodk0729      1988   0.523
prod2          3127   0.365   <-- deep
prodk1658      4380   0.483
prodk1926      4510   0.501
prod3          5778   0.478
prodk0759      7479   0.517
```

**The cleanest evidence needs no statistics.** `prod1` at generation **108** scores **0.362 against
the net it started from**. After 108 generations the arm is still essentially the champion; absent a
dip it should read ~0.500. `resume_dip_RESULT.md` independently measured **0.366 at generation 100** —
agreement to 0.004, on a different run, by a different harness.

**So the dip is real in production.** What the section above establishes stands unchanged: the
ABSOLUTE ruler cannot see it. Both are true, and together they say precisely what the dip is — a
paired-comparison phenomenon, invisible to a fixed external opponent.

**The confound in the pooled split, stated rather than buried.** Pooling gens <4327 (mean 0.4627,
n=7) against >=4327 (mean 0.5025, n=39) looks like a clean dip measurement and is not: later
generations are also *further into training*, so improvement and recovery are conflated. That
comparison is reported for completeness and carries no weight. The gen-108 reading does, because at
108 generations there is no improvement to confound with.

### What this changes about the decision

**It does not reverse it, and it sharpens the cost.** The dip blocks PROMOTION, because
`auto_promote` decides on exactly this paired comparison — an arm below 0.5 against the champion
cannot be promoted no matter how the anchor rates it. So the restart's real cost is measured in lost
promotion opportunities, not in Elo.

How large: the trainer reaches ~9,000 generations per 30-minute `auto_promote` cycle, so the FIRST
reading after a restart already lands at generation 1,300–2,000 — past the deepest part of the dip.
Of the five earliest readings under gen 2,000, two are sub-parity and three are above it. That is
roughly **one promotion opportunity lost per restart**, four times a day, out of ~9–12 per window.

That is a real cost and still not a case for changing `SECS=21600` on its own, because the restart
buys a replay-pool reset whose value is unmeasured. **The honest state is that both sides of the
trade are now quantified on one side only**, and the experiment that would close it is a single
window run at 12 hours with promotion counts compared — cheap, and not run here.

## The decision, after both instruments

**Do not change `SECS=21600` — but the reason is not the one this file opened with.**

The first section concluded "no measurable overhead". That was wrong as stated: it was true of the
absolute ruler and false of the paired instrument, which the follow-up above then found sitting in
`auto_promote.out` the whole time. The dip is real, it is measured, and it costs roughly **one
promotion opportunity per restart** out of 9–12 per window.

The decision survives anyway, for a different reason: **the other side of the trade is unmeasured.**
The restart resets the replay pool and re-anchors the run to the current champion, and nothing on
file says what that is worth. Trading a quantified cost against an unquantified benefit is not a
decision, it is a coin flip with extra steps.

**What would settle it:** one window run at `SECS=43200` (12 h) with promotion counts and ruler
readings compared against a 6 h window. Cheap — it is one flag and one day — and not run here.


## What this does NOT say

* **Not that `resume_dip_RESULT.md` is wrong.** Its measurement stands on its own instrument; this
  says only that the effect does not transfer to the absolute ruler at n=5–7 early readings.
* **Not that there is no cost** — only that it is below this instrument's resolution. The power here
  is poor: ±33 on an early mean against a predicted ~95 Elo dip would have been detectable, but the
  dip's *duration* means only the first few readings are affected and they are the scarcest.
* **Nothing ships**, and no figure is quoted as Elo.
