# There was no decline. The origin control's "collapse" was two runs read as one curve

**2026-09-10.** This file retracts a result I stated as resolved, and records the two explanations I
built for a phenomenon that did not exist, so the next person spends their time better than I spent
mine this morning.

## What I claimed

That the champion was getting WORSE while the loop kept accepting:

> 0.873 → 0.871 → 0.847 → 0.819, **RESOLVED WORSE, −0.054 ± 0.033**

and that this contradicted `netmatch`, which said the later net was STRONGER head-to-head. Two
resolved measurements pointing opposite ways was treated as the project's open question.

## What was actually true

`learn` **appends** to its log and its generation counter **restarts at 1 on every launch**.
`p1_fixedgate.log` contains **five runs** — `gen 1` appears at lines 6, 29, 75, 531 and 616. Every
one of those four "points" was selected by `grep 'control vs origin' | tail -4`, across the whole
file.

Segmented by run, the same seven control lines say:

| run | readings | shape |
|---|---|---|
| earlier run | gen100 0.873, gen200 0.871, gen300 0.847, gen400 **0.870** | **flat** |
| current run | gen400 **0.819**, gen800 **0.837** | **rising** |

The file contains **two different gen-400 rows** — 0.870 and 0.819 — a 0.051 gap at the same label
with ci95 ≈ 0.023. The "decline" was the splice between them. Run A never fell, and run B has been
going up.

## The two explanations I built for it, both refuted by their own pre-declared tests

1. **A moving instrument.** The control derives its node caps per reading from a wall-clock probe
   (`equal_time_caps` → `ns_per_node`), and I had doubled datagen from 4 lanes to 8 mid-window.
   Starve both sides and the rate drifts toward 0.5 — right mechanism, right sign.
   **Refuted:** caps are stable to **1.9%** with the nets held fixed, against a 5% threshold
   written down before the run (`control_caps_RESULT.md`).
2. **Selection at the wrong depth.** The loop gates at depth 1 while strength is judged at depth 4
   — `main.rs:154` already warns it "optimises the game it measures".
   **Not refuted — and it resolves the OPPOSITE way from the worry.** See the correction below.

Both were good hypotheses. Neither was the answer, because there was no question.

## ⚠ CORRECTION, one hour later, to my own "+0.039 at depth 4"

**I quoted a depth-4 number from a DIFFERENT, unmatched run and compared it to a matched depth-1
number.** Exactly the mistake this file is about — comparing rows that do not belong together —
committed while writing the file that documents it. The matched pair, same two files, same 448
pairs, same seed, only depth differing:

| depth | earlier net scores | later net's margin |
|---|---|---|
| 1 — what the gate uses | 0.457 ± 0.018 | **+0.043** |
| 4 — what strength is judged at | **0.361 ± 0.020** | **+0.139** |

**The margin more than TRIPLES at depth 4.** Both intervals are clear of 0.5, and this is the clean
comparison: one script, both arms, nothing varying but the depth argument.

So hypothesis 2 is **not refuted** — it is real, and it points the other way from the worry. The
fear was that a depth-1 gate would select for a shallow game and the gains would evaporate at
depth 4. Instead the gains are **three times larger** at depth 4 than the gate can see. The loop is
improving substantially more than its own accept criterion is able to measure.

**⚠ THIS LEAD IS NOW CLOSED — REFUTED, see `reject_holdout_RESULT.md`.** 34 rejected candidates,
each replayed against the champion it actually lost to at depth 4, pool to **0.4982 [0.4847,
0.5117]** — exactly champion strength. The gate is NOT discarding improvements, so raising the gate
depth would buy nothing for ~200x the cost. The paragraph below is kept as written because it is
what I believed at the time and the pre-registered test that killed it only makes sense against it.

**What that implies, stated as a lead and not a result:** a gate blind to two thirds of the
improvement it is selecting on must be rejecting candidates that are real depth-4 gains. Raising
the gate depth is the obvious move and is NOT free — 224 pairs at depth 4 costs ~7 min/generation
against ~2s today, roughly 200x, which is why depth 1 was chosen. The cheap version to test first
is a *shallow gate plus a deeper tiebreak* on candidates near the accept threshold. Not run yet.

## The measurement that stands

Head-to-head between two saved nets, 448 pairs, same seed: the later net wins by **+0.043 at
depth 1 and +0.139 at depth 4**. That compares two *files* and so is unaffected by the labelling
bug.

**But the labels are still wrong and must not be quoted as "gen200 vs gen400".** The `gen200` rung
was written 11:53 and the `gen400` rung 12:42, by **different runs**. The lineage is continuous —
the current run was launched 12:24:29 with `--init p1_champion_t2.net`, a snapshot written 12:24:27
— so this is a valid ancestor comparison, but the two nets are **~49 minutes of training apart**,
not 200 generations.

## The defect class, which this project has now hit repeatedly

A key that omits a varied dimension **pools incomparable records silently**. Here the key was the
generation number and the missing dimension was the run. Nothing errored. Nothing looked broken.
Every individual number was correct. The aggregation answered a different question than its label
claimed, and it produced a plausible, resolved, wrong answer with a tight confidence interval —
which is far more dangerous than an obvious failure, because a tight interval reads as authority.

**A confidence interval says nothing about whether the rows belong together.**

## Fixed

* `ops/gen_status.sh` (4PC repo, the phone dashboard) now scopes every Existence figure to the
  current run: generations, accepts, last line, control list, and the Elo headline and its age.
  It says which run it is and that the counter restarts. Before this it showed "1489 generations"
  while the run was at gen 920, and aged the headline by subtracting a within-run number from a
  cross-run total.
* `--rung-every` decouples ladder snapshots from the expensive control, so ancestors exist to
  compare against without waiting 400 generations.

## Standing rule

**Before reading any series out of a log, confirm the rows come from one run.** `grep ... | tail -N`
over an appended log is a pooling operation, not a time series.
