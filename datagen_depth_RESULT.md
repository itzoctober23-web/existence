# Six generations of depth-3 datagen match a champion built from 2,200 — datagen depth IS the lever

**2026-09-10.** The biggest result this project has produced, and it **refutes the prediction I
pre-registered two hours earlier** in `depth_ruler_PREREG.md`.

## The measurement

Three arms, equal wall clock (1800s), in parallel on separate cores, nothing gated. Only
`--depth` differs. Judged on the **absolute ruler** — depth 4, SF-1320 @10k nodes, 120 games — the
same instrument that reads untrained at −366, gen200 at −114 and the champion at −104.

| arm | generations in 1800s | W-D-L | score | Elo vs SF-1320 | absolute |
|---|---|---|---|---|---|
| **datagen depth 1** (the loop's default) | **610** | 4-41-75 | 0.2042 | **−236 ± 53** | ~1084 |
| **datagen depth 3** | **6** | 17-50-53 | 0.3500 | **−108 ± 48** | **~1212** |
| datagen depth 6 | 0 — no net | — | — | — | — |
| *champion, for reference* | *~2200* | *22-41-57* | *0.3542* | *−104 ± 52* | *~1216* |

**Difference +128 ± 72 Elo. RESOLVED** — the intervals are disjoint ([−289, −183] against
[−156, −60]) and the difference excludes zero.

**Six generations of depth-3 datagen are statistically indistinguishable from the champion**
(−108 ± 48 against −104 ± 52), which took roughly 2,200 generations of depth-1 datagen.

## Why this is the answer to "learned, then stopped"

`untrained_baseline_RESULT.md` established that training gains ~235 Elo before generation 200 and
then nothing across the following 1,200. `label_source_RESULT.md` then showed the trainer is not
broken and that substituting Stockfish's eval as the label buys at most ~113 Elo, unresolved. Both
findings pointed at the data rather than the learner, and neither identified what about it.

This does. **The labels were produced by a one-ply search.** A depth-1 root score is barely a
function of the position, so the target the net is fitting carries almost no information about who
is winning — and no amount of training, capacity, or label-source substitution recovers information
that was never in the signal. Raise the search that produces the label and the same learner, the same
architecture and the same trainer reach the champion's strength in **1% of the generations**.

## The prediction this refutes, kept because being wrong is the point

`depth_ruler_PREREG.md`, written before the games were read:

> I expect the depth-1 arm to read substantially higher, and **that would not refute deeper
> datagen.** It would mean 1800 seconds is too small a budget for this comparison — the deep arm
> never reached the part of the curve where its labels could matter.

The reasoning was that most of the loop's gain arrives inside ~200 generations, so a 6-generation net
has barely started. That is *true of depth-1 generations* and it is exactly what makes the result
what it is: the deep arm did not need 200 generations, because its generations are not the same
thing. Six were enough.

The pre-registration also named the outcome that did occur, and its reading stands:

> **depth 3 ≥ depth 1 despite 5 generations against 468** — a strong result for label quality, and
> much stronger than it looks, because the deep arm would be winning from ~1% of the training steps.

## What it does NOT say

* **Not that depth 6 is better still.** That arm produced no net: at 2,400 games per generation,
  depth-6 datagen cannot complete a single generation in 30 minutes. Its cell is empty, not zero.
* **Not that the ~1216 plateau is broken.** The depth-3 arm *reached* the champion's level far more
  cheaply; it has not been shown to pass it. Whether deeper labels also raise the ceiling is a
  different question and needs a longer run.
* **Not a replication.** One seed, one 1800s budget. `depth_RESULT.md` measured +0.025 ± 0.013 for
  depth 3 over depth 2 against the frozen origin and called it "promising, needs replication",
  correctly — that instrument reads in the 0.85–0.97 band where `instrument_saturation_RESULT.md`
  records it saturating and reversing sign twice. This is a much larger effect on an absolute
  instrument, which is why it resolves where that one could not, but it is still one run.

## What changes immediately

`main.rs:152` sets `--depth` default **1**. Every P1 measurement in this repo — the plateau, the
deceleration curve, the width and blend and horizon sweeps, the 2,200-generation champion — was taken
on a loop generating its own labels at one ply. That default is now the most expensive line in the
project.

The next questions, in order:

1. **Replicate** — two more seeds, since one run is one run and this repo has retracted a
   three-point trend before.
2. **Does it raise the ceiling or just reach it faster?** Run the depth-3 arm long enough to pass
   −104, or establish that it plateaus there too.
3. **Where is the knee?** Depth 2 was never measured here, and depth 6 could not complete a
   generation at 2,400 games. `--datagen-nodes` now expresses this in the natural unit (nodes per
   move) rather than in plies.

## Method note

`depth_ruler2.sh` was written before these numbers were read, to correct a real design limit: both
arms used `--games 2400`, so the comparison was "610 training steps vs 6" rather than a matched one.
That correction is still worth running — it asks whether deep labels win at *equal* training steps,
which is a cleaner question — but it is no longer needed to establish that the lever exists. The
unequal-steps version already answers that, and answers it in the harder direction: the deep arm won
from 1% of the steps.
