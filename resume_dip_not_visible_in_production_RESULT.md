# The resume dip is NOT observable in the production ruler — so there is no measured case for changing the 6-hour restart

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

## The decision

**Do not change `SECS=21600`.** The case for extending it rested on an overhead that is not
measurable here, and the script's comment gives the restart a purpose the ruler cannot see: it resets
the replay pool and re-anchors the run to the current champion. Changing a six-hour cadence to chase
an unmeasurable gain is exactly the trade this project has recorded going wrong before.

**What would justify revisiting it:** a paired measurement of the production arm against its own
pre-restart state, taken across a restart boundary. That is the quantity `resume_dip_RESULT.md`
measured, it is the one that would actually show the cost, and the ruler cannot substitute for it.

## What this does NOT say

* **Not that `resume_dip_RESULT.md` is wrong.** Its measurement stands on its own instrument; this
  says only that the effect does not transfer to the absolute ruler at n=5–7 early readings.
* **Not that there is no cost** — only that it is below this instrument's resolution. The power here
  is poor: ±33 on an early mean against a predicted ~95 Elo dip would have been detectable, but the
  dip's *duration* means only the first few readings are affected and they are the scarcest.
* **Nothing ships**, and no figure is quoted as Elo.
