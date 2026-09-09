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

Depth is the only surviving lever *among the four named candidates*. Its effect is smaller than the
~0.07 between-run band, so a 2-seed replication with `--horizon-cap 45` on both arms is queued.
Epochs is a fifth candidate, never tested on a working metric, also queued.

### The training target's BLEND outweighs all four

`target = (1 - blend) * z + blend * root` — blend weights the net's own search score against the
game outcome. Measured on the frozen-origin metric, 20 generations, identical seed:

| blend | vs frozen origin | decisive @ gen 20 |
|---|---|---|
| 0.75 (shipped) | **0.847 ± 0.022** | 1170/2400 |
| 0.25 | **0.735 ± 0.025** | 498/2400 |
| 0.00 | **0.691 ± 0.025** | 340/2400 |

**+0.112 ± 0.033** — bigger than draws (+0.086), horizon (+0.064) or depth (+0.025), and clear of
the between-run band. Controlled by an identity check: both arms report `dec 389/2400` at generation
1, before any training, so they diverge only downstream of the target. No improvement is available
(0.75 is already shipped); the high side is queued as `blend_hi.sh`. Detail in `blend_RESULT.md`.

### Every cheap proxy for strength has failed

| proxy | vs | result |
|---|---|---|
| `mcnemar_z` surrogate | 239 gate results | r = **−0.095**, CI [−0.220, +0.032] |
| training loss | control vs origin, n=12 | r = **+0.379**, CI [−0.249, +0.783] — wrong sign |
| candidate-vs-champion gate | fixed anchor | **0.500 ± 0.007** on pairs an anchor separates easily |

A fourth proxy is the first with a useful point estimate, and it is ONE ARM from resolving:

| proxy | r | 95% CI | n |
|---|---|---|---|
| mcnemar_z surrogate | −0.095 | [−0.220, +0.032] | 239 |
| training loss | +0.379 | [−0.249, +0.783] | 12 |
| **decisive-game rate** | **+0.771** | **[−0.108, +0.973]** | **6 arms** |

The CI still includes zero, so it is NOT established, and there is a visible counterexample: the
strongest arm on the board (`wd_r2`, 0.967) has FEWER decisive games than a weaker one (1124 vs
1170). But at r = +0.771 a seventh arm clears zero, and **every arm already logs `dec` for free** —
so this resolves at no extra compute as the queue drains. Measured on 20-generation arms only; the
1-generation depth probes were excluded because training amount drives both terms.

Only games against a **fixed anchor** have resolved anything. They cost ~1,650 pairs to resolve one
generation's real edge, which is why they are not the per-generation metric.

### The champion gate COMPRESSES — it does not invert. This decides which old results survive

The blend arms were measured on both instruments, giving the first direct calibration:

| 0.75 − 0.25 | champion gate rate | vs frozen origin |
|---|---|---|
| | +0.036 ± 0.016 | **+0.112 ± 0.033** |

Same direction, ~3× the magnitude. Therefore:

* a **DIRECTION** from the champion gate is probably safe — it got the blend ordering right;
* a **NULL** from it is worthless, because compression manufactures nulls. It cannot distinguish
  *equal* from *invisible*.

**Every "these arms are indistinguishable" reading taken on the champion gate must be re-asked**,
starting with main.rs's flat 0.75/0.85/0.95/1.00 plateau and its conclusion that "the game outcome
contributes nothing measurable".

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
7. **"Batching the gate does not help — every batch rolls back, so it is a LEARNING failure."**
   `batch_ab.sh` ran at 17:22 on a binary built at 17:13; the anchor-increment batch gate landed at
   19:18 (`bae8c7b`). It measured the OLD head-to-head gate, printing `0.500+/-0.007` — the blind
   gate's signature, not a null. Its arms were also unequal (11 generations vs 5, time-boxed), and
   its negative was pre-registered as expected. Re-queued as `batch_ab2.sh`.

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

### The acceptance rule cannot accept a real step

`resolved_up = pent_rate - ci95 > 0.5` (main.rs:675) — acceptance needs an edge **larger than the
gate's own ci95**. At 224 pairs that is **0.0309**; a real per-generation edge is **~0.0114**. Both
figures reproduce numbers recorded independently elsewhere in the tree.

> The shipped gate demands an edge **2.7× larger** than a generation produces.

main.rs:757 already states the premise — "real steps are far smaller than that" — but concludes it
only about the ANCHOR gate, which by main.rs:752 "can only ever veto" and so cannot rescue anything
the champion gate rejected. **Even at face value with zero compression, the 0.0309 floor exceeds
datagen depth (+0.025) — the only surviving ceiling candidate.** Detail in
`acceptance_floor_RESULT.md`.

## Task list (docs/MASTER_PLAN items 1-6) — verified stale

| item | status, verified by reading |
|---|---|
| 1. incremental NNUE accumulator | **done and correctly OFF.** `Acc` exists, `tests/incremental.rs` checks it against a full refresh, and `search.rs:169` gates it on `n_hidden >= 64`. It is a measured **0.91× LOSS** at width 32 (arch.rs:150), and shipped width is 16. All three clauses of its premise expired: eval is a sparse gather over ~38 active rows, not a dense 256×782 sweep; width is 16, not 256. It pays only at width ≥64, which was refuted at equal time. |
| 3. Zobrist + real TT slots | **done.** Incrementally maintained key (`chess.rs:271`) with a from-scratch `zobrist()` to check against. |
| 6. xcheck + perft as `#[test]`s | **done.** `board/tests/perft.rs`, `board/tests/xcheck.rs`, 13 test files total. |
| 5. register bytecode | deprioritised — the interpreter measured 1.003× hand-written on a quiet core. |

## Queue (core 15, chained by PID)

blend A/B v2 (arm 0.00 running) → ratchet test → depth replication (horizon-capped, pre-flight
verified) → epochs A/B v2 → **blend_hi** (1.00 vs 0.85 vs re-run 0.75 control) → **batch_ab2**

`batch_ab2` is the highest-value item and is LAST only because the chain is pid-linked and cannot be
reordered while it runs. **If a slot frees earlier, run it first** — every arm ahead of it asks what
to feed a loop that may be unable to swallow anything.

`blend_hi` also re-runs 0.75 with the same binary and seed as a determinism check. If it does not
reproduce 0.847 ± 0.022, the ~0.07 band is RUN variance rather than SEED variance and no cross-run
comparison in this investigation is safe — read that before reading its blend answer.

## Open, partially answered

**Is the speedup acceptance path reachable?** ANSWERED, and the instrument fix is what delivered it.
`stepdiff6` died at 50/60 (cause unknown — no OOM evidence either way, and neither journal nor
dmesg was readable) yet still reported: **identical 30, cheaper 0; different 17, guard-ok 0**. The
previous run died and yielded nothing; this one died and yielded its answer, which is exactly what
moving every summary counter into the progress line was for.

Read honestly, this is **bounded, not empty**: 0 in 30 gives a 95% upper bound of 10% by the rule of
three. Combined with 93 prior generations producing no cheaper survivor, it is consistent with
empty, but 30 trials cannot prove a rate below ~10%.

The widened 33-position identity check is committed but deliberately NOT deployed, since restarting
a live search-track run to strengthen a path bounded below 10% would cost real generations.

## Not started

* Rung 7 needs its table contents DECLARED before it can be measured.
* Register bytecode (CRATE 4) — a perf task; the interpreter is at 1.003× hand-written speed, so
  it is not urgent.
