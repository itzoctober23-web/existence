# Existence — current state, 2026-09-08

Single source of current truth. The `*_RESULT.md` files are the working records and several contain
claims later retracted; **this file supersedes them where they disagree.**

---

## What is MEASURED and stands

### The ceiling investigation — four candidates, three dead

| candidate | verdict | measurement |
|---|---|---|
| capacity / net width | **REFUTED** | w64 loses **0.179 ± 0.021** to w16 at equal TIME |
| draw filter | **REFUTED** | excluding draws better by **+0.086 ± 0.015**, two independent protocols |
| horizon schedule | **REFUTED** | widening beats narrow-fixed by **+0.064 ± 0.034** |
| datagen depth | **survives, unreplicated** | 8 deep generations beat 92 shallow, **+0.025 ± 0.013** |

Depth is the only surviving lever. Its effect is smaller than the ~0.07 between-run band, so a
2-seed replication with `--horizon-cap 45` on both arms is queued. Epochs is a fifth candidate,
never tested on a working metric, also queued.

### Every cheap proxy for strength has failed

| proxy | vs | result |
|---|---|---|
| `mcnemar_z` surrogate | 239 gate results | r = **−0.095**, CI [−0.220, +0.032] |
| training loss | control vs origin, n=12 | r = **+0.379**, CI [−0.249, +0.783] — wrong sign |
| candidate-vs-champion gate | fixed anchor | **0.500 ± 0.007** on pairs an anchor separates easily |

Only games against a **fixed anchor** have resolved anything. They cost ~1,650 pairs to resolve one
generation's real edge, which is why they are not the per-generation metric.

### The search track: why it produced nothing, and what changed

* **Alpha-beta is exact.** All seven AB-family reference programs return the SAME move — 40/40 at
  depth 3, 12/12 at depth 4. Correctness cannot discriminate among correct programs.
* **0 of 33** behaviour-changing single edits passed the all-or-nothing guard. The guard was in
  practice "do not change behaviour".
* **Three declared primitives did not do what GRAMMAR says.** `pred` was a stub returning false;
  rung 6 applied its extension at every depth instead of the horizon (73× cost); `tread` discarded
  its index arguments. All three fixed except rung 7's table CONTENTS, which are an undeclared
  Given (filling them with "reduce later moves more" would be seeding LMR).
* **The pipeline now runs end to end** — guard tolerance 4 on an alpha-sensitive guard admits
  candidates, the surrogate proposes, games decide. First live case: a 1.14× surrogate improvement
  rejected at 0.417 on the board.

---

## RETRACTED — do not rebuild on these

1. **"The training step degrades the net and the gate was hiding it."** Off one reading
   (0.861 → 0.831). 44 readings show champion_long sits at the TOP of the procedure's own
   0.79–0.86 band; further training is regression to its mean, not damage.
2. **"The only class that can improve is inexact variants, and it is empty."** Both members were
   no-ops from missing implementation, not from alpha-beta's exactness. Rung 6 now plays
   differently (8/10) at 1.679× cost.
3. **"35% of single edits are behaviour-changing AND correctness-preserving."** The instrument
   checked "returned a move", not the guard. Correct answer: **0 of 33**.
4. **"Iterative deepening is budget-aware."** True of real ID, false of `ab_id` here — it loops
   depth 1..D and never reads `Budget`. Nine of eleven references are budget-blind.
5. **"The exploits lose 5 guard positions by construction."** Measured: DEPTH loses 9, ALPHA
   loses 2 on the old guard. Both figures were wrong.
6. **"A transposition-table soundness bug."** The depth-4 disagreement was the 2e9 cost cap
   truncating searches, with `MOVE_NONE == MOVE_NONE` scored as agreement.

---

## Recurring failure modes, named because each cost hours

* **Two variables moving, the uncontrolled one flattering the result.** equal-DEPTH vs equal-TIME
  (width gate); equal-BUDGET vs equal-COST (game gate); equal-WALL-CLOCK vs equal-HORIZON (depth
  A/B). **Rule: if arms are matched on time, pin `--horizon-cap` explicitly.**
* **Inert features that look correct in the diff.** Cost ceiling set above where it could bind;
  `catch_unwind` under `panic = "abort"`; guard tolerance that changed only the printout. **Rule:
  verify by program BEHAVIOUR — a number that should move and doesn't.**
* **Instruments whose stated semantics differ from their code.** stepdiff's buckets; ttvalue's
  "UNSOUND" message printed for any challenger; moveagree counting MOVE_NONE as agreement.
* **State outliving its run.** A 5-hour-stale `hz_1000.log` about to be read as a current arm; a
  mid-run script edit that killed a verdict block, where **reverting within a minute did not undo
  it**.

---

## Queue (core 15, chained by PID)

blend A/B v2 → ratchet test → depth replication (horizon-capped, pre-flight verified) → epochs A/B v2

## Not started

* Rung 7 needs its table contents DECLARED before it can be measured.
* Register bytecode (CRATE 4) — a perf task; the interpreter is at 1.003× hand-written speed, so
  it is not urgent.
