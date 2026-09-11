# Depth 5 datagen LOSES to its own start; depth 3 WINS from the same champion

**2026-09-10.** The A/B pre-registered in `depth5_vs_depth3_PREREG.md` is resolved, and it lands on
that file's **first** row — the outcome it predicted.

## The measurement

Both arms resumed from the **same** shipped champion via `--init`, verified rather than assumed:
`d3_start.net`, `d5_start.net` and `p1_champion.net` are byte-identical (md5 `7e25daeafb37`). Equal
wall clock (2400 s), separate cores, only `--depth` differs. Each judged by `netmatch`, 224 pairs at
depth 4 — the paired instrument — **against its own start**, never against each other.

| arm | generations in 2400 s | vs its own start | interval | verdict |
|---|---|---|---|---|
| **d5** (datagen depth 5) | **45** | **0.397 ± 0.026** | [0.371, 0.423] | **LOSES** — entirely below parity |
| **d3c** (datagen depth 3) | **4,327** | **0.557 ± 0.032** | [0.525, 0.589] | **WINS** — clears the acceptance bar |

96× more generations for the depth-3 arm in the same seconds. That is the cost of depth 5, measured.

## What the control buys, which is the whole reason it exists

The PREREG named this exact combination:

> | loses | **wins** | depth 5 is the wrong trade at this budget — the prediction holds, and the
> control proves the budget was survivable |

Without d3c, "d5 lost" is unreadable: it could mean depth 5 is a bad trade, or that 2400 seconds
from this champion was a losing proposition at *any* label depth. d3c winning from the identical
start rules out the second. **The loss is attributable to depth 5.**

The control very nearly did not exist. `depth5_from_champion.sh` stays alive to run its own
`netmatch`; the d3c arm was launched as a bare `timeout 2400 taskset ... learn` with no surrounding
script, so its trainer would simply have stopped and nothing would have measured it. It was given a
verdict watcher with minutes to spare.

## The prediction, and it held

> depth 1 → 3 cost 59× and won decisively because depth-1 labels are nearly information-free.
> Depth-3 labels are already informative, so **depth 5 must beat 35× fewer training steps on labels
> only somewhat better. I expect it to lose or be unresolved.**

It lost. The datagen-depth lever is **not** monotone: `datagen_depth_RESULT.md` measured +128 ± 72
Elo going 1 → 3, and going 3 → 5 is a regression at equal wall clock. The knee is at or below 3.

## Confidence, stated honestly — the two results are NOT equally strong

`netmatch` prints its own power note, and it separates these cleanly:

* **d5**: effect **0.103** against a between-seed sd of 0.047 → *"~2 seeds for ~80% power"*. The
  loss is ~2.2× the between-seed noise. Solid.
* **d3c**: effect **0.057** against the same sd → *"~5 seeds for ~80% power"*. It clears the
  project's acceptance rule (`rate − ci95 ≥ 0.5` → 0.525 ≥ 0.5) but the effect is only marginally
  above seed-to-seed variation. **One seed is one seed.**

So the headline — *depth 5 is the wrong trade* — rests on the stronger of the two. The d3c win is
real enough to promote under the standing rule and should not be quoted as a large gain.

## THE RULER SAID THE OPPOSITE, for the third time

The live ruler read d3c across the run as **+50 → +32 → −29 → −86**, each ±~50. Read as a trend that
is a 136-Elo collapse, and I was within one step of reporting it as one.

It is noise, and the arithmetic says so precisely:

* the **sequence** suggested collapse;
* the **final level**, −86 against the champion's −104 ± 52, is consistent with *"slightly stronger
  than the champion"* — which is exactly what `netmatch` resolved at 0.557.

**The trend was noise; the level was roughly right.** That is the ruler's documented job description
(`champion_deep_RESULT.md`: *"the ruler answers 'roughly where is this net'"*), and this is the third
time a ruler trend has been refuted by the paired instrument — after the −17/−29/−41/−44 sequence
that ran while the net was genuinely stronger, and the −134/−101/−176 sequence at 60 games.

The rule stands, unchanged and now paid for a third time: **a direction claim needs the paired
instrument, never a sequence of ruler samples.**

## Shipped

`d3c.net` promoted to `p1_champion.net` (previous banked as
`p1_champion_prev_g4327_abtest.net`) under the project's own rule, with no trainer running so the
candidate was unambiguous. **No Elo is claimed** — it passed the gate.

## What this makes next

Datagen depth is now measured at 1 (bad), 3 (good), 5 (worse). The lever is **spent**: there is no
remaining direction in label depth to push. The PREREG's own reading of this row is that the next
levers are **capacity or search**, not labels.

⚠ The obvious capacity experiment is currently **mis-specified and would have silently measured
nothing** — see `w64_misspecified_RESULT.md`.


## ⚠ CORRECTED 2026-09-10 23:1x — the two arms sat at different depths in a RESUME TRANSIENT

`resume_dip_RESULT.md` measured what a **depth-3** run does after resuming from this same champion,
with the same flags, judged on the same instrument:

| generations | score vs the champion it resumed from |
|---|---|
| 5 | 0.492 |
| 25 | 0.411 |
| 100 | **0.366** |
| 4,327 | 0.557 |

A resumed run gets **~95 Elo worse before it recovers**, and the dip has nothing to do with label
depth. Interpolating, a depth-3 arm at **45** generations sits at **~0.39–0.40**.

**The d5 arm ran 45 generations and scored 0.397.** That is indistinguishable from where a depth-3
arm sits at the same generation count. So this experiment **cannot separate** *"depth-5 labels are
worse"* from *"depth 5 completed too few generations to escape the transient"*. The control existed
and was still not enough, because it ran 4,327 generations against d5's 45 — it did not match on the
axis carrying the confound.

**What still stands:** the practitioner's question — *given 2,400 seconds from this champion, which
depth do I pick?* At the end of that budget depth 3 is at +39.8 Elo and depth 5 at −79.8, so **do
not use depth 5 at this budget**. That conclusion is unaffected.

**What is WITHDRAWN:** every claim about label QUALITY. "The datagen-depth lever is not monotone",
"3 → 5 is a regression", and "the knee is at or below 3" are **not established**. Depth-5 labels
have not been shown to be worse — only to be too slow, at this budget, to reach the part of the
curve where they could show anything. Testing them properly needs matched GENERATION counts, which
means giving the depth-5 arm roughly 96× the wall clock.

The section above headed "The prediction, and it held" should be read as: the prediction about the
BUDGET held; the reasoning offered for it (label information) was not tested by this design.
