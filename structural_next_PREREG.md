# PRE-REGISTRATION — what the loop does after the configuration wins run out

**Written 2026-09-11, deliberately BEFORE the 1600 line is crossed**, so that the answer to "what
next" is on disk before the celebration rather than chosen in its afterglow. The champion reads
**1519 +/- 9 and FLAT** (`+0.9 +/- 0.7 Elo/1000 gens, z +1.32`), 81 Elo short of 1600.

The premise is that configuration wins are nearly exhausted. The record supports it: lr is shipped
and settled (`low_sweep2_RESULT.md`), epochs is CLOSED (`epochs_2v3_RESULT.md`, flat in both
directions and on cost), blend clears on one seed by less than the between-seed spread
(`blend_sweep_RESULT.md`), and `ruler_trend_RESULT.md` reports **every production run flat on the
absolute ruler — all 166 Elo came from BETWEEN runs, not within them.** Turning the same crank
longer will not close 81 Elo.

Three structural candidates are on the table. **They are not equally open, and the point of writing
this now is to say so before compute is spent.**

---

## Candidate C — the net-growth ARCH menu. CLOSED. Do not run it as the 12th attempt.

`width_clock_RESULT.md` (2026-09-10) already ran exactly this, on the clock:

```
11 attempts, widths 32 / 64 / 128
fixed-cost (equal NODES):  8 of 9 ABOVE 0.5   — wider IS better per node
clock      (equal TIME) :  9 of 9 BELOW 0.5   — wider is worse per second
accepted: 0.  No clock score above 0.5 at any width, on any generation.
```

The mechanism is measured, not inferred: a wider net evaluates better and searches less, and the
second effect wins. At w128 the candidate searched **4,814 nodes against the champion's 7,167 (-33%)**.

"Now that width can be measured on the clock" is already true — that IS the clock gate, and it is
the gate that says no. So C is not blocked on instrumentation; it is answered.

**What would REOPEN it, stated so the condition is checkable rather than rhetorical:** the loss is
node loss, so C reopens only if evaluation gets cheaper per node. That is its own blocked question —
`speed_cannot_pay_RESULT.md` ("the engine cannot spend a speedup at all") and
`simd_refuted_RESULT.md` (`target-cpu=native` buys nothing). **Precondition: a measured drop in
ns/eval at equal strength. Until that exists, a 12th ARCH attempt re-derives a closed file** — which
`RESULTS_INDEX.md`'s own banner records costing 95% of one arm's wall clock.

## Candidate B — a rolling data window. PARTLY ANSWERED, and the answered part is the discouraging one.

Two readings bear on it:

* `games_per_gen_RESULT.md` — **quadrupling the data per generation changes nothing** (0.4642 against
  0.4684). Data VOLUME is not binding.
* `ancestor_first_readings_RESULT.md` — no measurable gain over a 400-generation window.

Neither kills B, because B is about **staleness**, not volume: whether labels made by a 20,000-
generation-old net should still be trained on. That axis is genuinely unmeasured. But the prior from
volume is not encouraging, and B shares a confound with everything else here — `resume_dip_RESULT.md`
puts the resume transient at **~95 Elo before it pays back**, which is larger than the effect being
hunted.

**Rank: second.** Worth doing, not first.

## Candidate A — datagen at a real node budget instead of a fixed depth. OPEN, and the best-motivated of the three.

This is ranked first on EXPECTED ELO, not on cost or novelty, and the reason is that the existing
measurements already point at it:

* `datagen_depth_RESULT.md` — **datagen depth IS the lever.** Six generations of depth-3 datagen
  matched a champion built from 2,200.
* `depth5_vs_depth3_RESULT.md` — **depth 5 LOSES to its own start while depth 3 WINS from the same
  champion.** Fixed depth is NON-MONOTONIC. Something about holding depth constant is wrong, and
  that is the hypothesis a node budget directly addresses.
* `depth_RESULT.md` — deeper labels beat more labels at equal compute (flagged as needing
  replication).

### The code gap, checked rather than assumed

`--datagen-nodes` already exists and **does not do this.** `pipeline/src/main.rs:185-193` uses the
budget to *select one fixed depth* from a table of measured midgame costs — d3 10,309, d4 72,977,
d5 1,234,802, d6 12,696,968 — taking the deepest whose cost fits, and `datagen::NODE_CAP` is then
only "the safety net". The table is sized on "the expensive case, so this never overspends", so
**every position is searched to the same depth and easy positions underspend their budget by
design.**

A real node budget means the opposite: iterative deepening on each position until the budget is
spent, so a hard position gets more depth and a simple one less. **That is a code change, not a
flag**, and it is small and local.

### Hypothesis, and the mechanism it stands on

Fixed depth spends equal effort on unequal positions. Label quality is therefore worst exactly where
positions are hardest, which is where the eval most needs the help. A node budget equalises effort
per position rather than per ply, which should improve labels where they are currently worst without
raising total datagen cost.

**This mechanism predicts something specific and falsifiable:** at a node budget equal to the mean
cost of the depth-3 arm, the *variance* of realised depth across positions must be > 0 and the
budget-matched arm must beat the fixed-depth arm. If realised depth comes out essentially constant,
the mechanism is absent and the arm is measuring nothing — that check runs FIRST and gates the rest,
the same way `assert_setting_took.py` gates `hardn_probe`.

### Pre-registered design

* **Arms:** fixed depth 3 (the current winner) vs node budget set to the depth-3 arm's *measured mean*
  nodes/move. Both resumed from the SAME shipped champion via `--init`, verified not assumed.
* **Matched on GENERATIONS, not wall clock.** `resume_transient` already invalidated one published
  A/B this way, and a node budget changes per-generation cost, so wall-clock matching would confound
  the thing being tested.
* **Planned N: 2000 generations per arm**, the length `low_sweep2` needed to resolve. No reading is
  taken before then. A truncated LLR is noise (`fourpc_truncated_llr_is_noise`).
* **Decision rule:** the standard promotion rule, `rate - ci95 >= 0.5`, on a 224-pair netmatch
  against the champion. Between-seed sd is **0.047**, so a single-seed margin smaller than that is
  reported as UNRESOLVED, not as a win — the error `blend_sweep_RESULT.md` explicitly flags in its
  own headline.
* **A null is reported as a BOUND**, never as a refutation: "over 2000 generations at a measured
  realised-depth spread of X, the node budget did not beat fixed depth by more than Y". The
  `hardn_inert` failure was publishing a zero whose power was never computed.

### What would falsify the whole framing

If realised depth under the budget is near-constant, or if the budget arm loses while showing real
depth variance, then "equal effort per position" is not the defect in fixed-depth datagen, and the
non-monotonicity in `depth5_vs_depth3` needs a different explanation — most likely a cost/quality
tradeoff at depth 5 rather than a per-position allocation problem.

---

## The ordering this registers

1. **A — node-budget datagen.** Open, mechanism-backed, small code change, and the only one of the
   three whose motivating measurement already exists.
2. **B — rolling data window.** Genuinely unmeasured on staleness; discouraging prior from volume.
3. **C — ARCH width.** Answered at 11 attempts and 0 accepts. Reopens only on a measured ns/eval
   drop, and not before.

**Nothing here is a result.** It is the question, the ranking and the decision rule, fixed in
writing while the champion is still 81 Elo short — so that whatever happens at 1600, the next move
was not chosen to fit the mood of the moment.
