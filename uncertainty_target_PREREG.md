# PRE-REGISTRATION — the uncertainty head's TARGET, before either track is built on it

2026-09-11. Written before any code, because the decision-theoretic plan has one precondition — *an
eval that reports its own uncertainty* — and this project has already MEASURED the proposed signal
and found it pointing the wrong way. Track B step 2 is named as "the one number that tells you
whether it's worth finding". That number is only worth having if the head it depends on predicts the
right quantity.

## What is already measured, and it is not encouraging

The proposed target is `|static eval − own deeper search score|`. That exact quantity is what
`confident_when_wrong_RESULT.md` calls CONFIDENCE (small residual = confident), and it was measured
against the question decision-theoretic search actually asks — *does deeper search change the root
decision here?*

```
net                          flips   low-conf ON flips   (base rate 50%, spec asks >=80%)
p1_champion                  43.2%          35.3%
p1_champion_prev_pre_lr002   41.5%          30.6%
lrs_00005                    42.4%          32.0%
```

Three nets, two lineages, same direction: the residual is **smallest exactly where the move flips**.
The obvious confound — a flip is a tie-break, not an error — was stated first, then tested and
REFUTED: restricted to flips that genuinely cost material, the champion scores **17.1%**, further
from the 80% threshold, not closer.

**So a head faithfully predicting `|static − deep|` would report LOW uncertainty precisely where more
search changes the decision.** Allocating compute by it would spend least where it matters most —
the exact inversion of "spend compute where it changes the root decision".

`static_deep_residual_RESULT.md` adds two mechanical properties of the same quantity, both of which
constrain a head trained on it:

1. **The two sides are the same function.** The search evaluates leaves with the net being measured,
   so static and deep agree by construction — corr **0.900 for a net with no training at all**. Only
   the unexplained remainder carries information. (This is not a defect for THIS use: self-reference
   is exactly what the plan specifies. It is a defect for ranking nets, which is what that file was
   about.)
2. **The residual scales with the net's output range** (sd 237.9 -> 388.3 across training). The
   target distribution therefore DRIFTS as the net learns, so a head trained early is calibrated to a
   scale the net later leaves behind.

## Registered decision, before any number exists

**The head is built as specified, AND the target is validated before Track B step 2 runs.** Two
candidate targets, and the choice is pre-committed to a measurement rather than to taste:

* **T1 — score residual**, `|static − root|`, exactly as directed.
* **T2 — flip cost**, what the flip actually cost: play the cheap move, let the opponent search at
  rich depth, negate; `rich_score − cheap_value`. This is the quantity
  `confident_when_wrong.rs` ALREADY COMPUTES, and it is the one that tracks "deeper search changes
  the root decision at a cost".

**Criterion, fixed now:** on a held-out position set, rank positions by each predicted target and
measure enrichment for costly flips (>=10cp) in the top decile against the 50% base rate. A target
that does not beat base rate cannot drive allocation, and shipping allocation on it would be
optimising a signal already measured inverted.

* T1 beats base rate -> proceed exactly as directed; the prior measurement was about a different use.
* T1 at or below base rate and T2 above -> the head trains on T2. Same architecture, same gate, same
  self-referential property (no external judgment: the engine's own rich search is the referee).
* Neither beats base rate -> STOP, write `unc_target_RESULT.md`, and do not start Track B step 2.
  Track B's ladder rests on this head; running it on a signal that cannot rank is how a flat result
  gets attributed to the paradigm instead of to its input.

## Two implementation facts, checked rather than assumed

* **No datagen change is needed for T1.** `datagen.rs`'s `Sample` already logs `root`, "the engine's
  own root search score at this position", so the DEEP side is stored; the static side is
  `net.eval(fen)` at train time. "The residual already logged" is effectively correct.
* **T1's target drifts with the output scale** (defect 2 above). `confident_when_wrong.rs` already
  solves this by thresholding at each net's OWN MEDIAN, which is self-calibrating across nets whose
  scales span sd 237-420. The head should predict a scale-normalised residual for the same reason.
* The net has ONE output today (`nnue::Net::output`). A second head is contained but real.

## What is NOT claimed

That the plan is wrong. The measurement above is about the residual as a CONFIDENCE signal on
move-flips; a trained head is not identical to the raw residual, and 10% of the variance is
genuinely unexplained. This registers the check that decides it, so the answer is on disk before
Track B spends a ladder — and so a null is read as "the target was wrong", not "the paradigm was".
