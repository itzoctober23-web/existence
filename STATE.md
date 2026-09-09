# Existence — current state, 2026-09-09

## 🔑 WHY THE PLATEAU EXISTS: the search is BARE alpha-beta, by design

`search.rs` at the horizon returns the static eval with **no capture resolution**:

```rust
if depth == 0 { return ... net.eval(pos, &mut self.scratch) }
```

`MASTER_PLAN.md:38` — *"Seed search program: BARE alpha-beta … No ordering, no hash reuse, no
iterative deepening, no quiescence"*. Line 53 — *"all of these must be DISCOVERED as program edits
that beat the current"*.

**So the ~0.86 plateau is the strength of a bare depth-2 alpha-beta with a width-16 eval, and that
is the expected, pre-registered result.** A search with no quiescence evaluates mid-capture
positions as if they were quiet; no amount of eval training fixes a horizon that cuts through
exchanges.

**This reframes the whole day.** Every candidate I tested — blend, epochs, depth, capacity, draws,
horizon, gate-every, data volume — is a *training-loop* knob. The training loop is at the ceiling of
what a bare search can express. The plan says the next gains come from the **discovery track**
(`evolve`), whose job is to find qsearch / ordering / hash reuse as program edits.

**And the discovery track is the thing that is actually broken:** its population collapsed to
identical members (`pop 8 spread 0.005508–0.005508`), 25 generations with 0 accepts, which is why I
killed it. That — not another hyperparameter — is where the remaining strength is.

## ✅ SETTLED (read this before anything below)

**Every tuning candidate is closed. None moved the plateau. The shipped defaults are correct.**

| candidate | verdict |
|---|---|
| capacity / width | closed — w64 does not beat w16 |
| draw filter, horizon | closed |
| datagen depth | closed — the effect was search PARITY (odd vs even), not depth |
| blend 1.00 | **dead** — advantage is depth-2 only (depth shift z = 4.4) |
| epochs 2 | **dead** — 0.498 / 0.473 / 0.448 once arms are matched; **3 is better** |
| `--gate-every 5` | no depth-4 gain (0.502 ± 0.022) |

**What the loop actually does:** from scratch it reaches ~0.82 in **10 generations** and is flat by
15. `champion_long` sits at **0.864**, the top of that band. Training from `champion_long` produces
no reliable gain because it is *already at the plateau this procedure reaches*.

**What actually improved things today — three fixes, no hyperparameters:**
1. **`batch_base` seeded from the starting champion.** The batch gate never gated its first K
   generations; `ga_d4` lost 0.048 with no baseline to roll back to. Verified live: the repaired
   gate caught a −0.045 block.
2. **`champ_anchor` invalidated on ARCH accept** — found by turning bug 1 into a search over every
   `champion =` site.
3. **`netmatch` prints arm sizes with the bias quantified** — three results today were confounded
   by unequal training (11v5, 96v1, 28v26).

**The measurement wall:** effects of 0.02–0.05 need **~19 seeds** to separate from seed noise
(spread ~0.045). Two seeds cannot; one certainly cannot. Every "resolved" one-seed reading today was
this.

**Open:** `fg_60` (do block increments keep rising as the pool grows?), `dv_4800` (is the plateau
data-limited?), `sg_20 vs bn_075` (does gating help from scratch?), `ep2_10 vs ep2_3` (is epochs 3
too *low*?).

---

> ## ⚠ READ FIRST: the origin metric does NOT invert — it is imprecise, and depth was the confound
>
> **This block previously claimed sign reversals. That claim is WITHDRAWN.** Measured at a MATCHED
> depth with equal total games (896 pairs, depth 2):
>
> | | value |
> |---|---|
> | bn_075 vs origin | 0.846 ± 0.015 |
> | bh_100 vs origin | 0.864 ± 0.014 |
> | **origin-increment** | **+0.018 ± 0.021** — unresolved, **sign CORRECT** |
> | **direct match** | **+0.050 ± 0.016** — resolved |
>
> At matched depth the increment **agrees in sign** with the direct match. The earlier "inversions"
> came from comparing a **depth-4 equal-time control** against a **depth-2 direct match**:
>
> ```
> bn_075 vs origin:  0.847 (d4 control)  0.846 (d2)   unchanged
> bh_100 vs origin:  0.832 (d4 control)  0.864 (d2)   0.032 WORSE at depth 4
> ```
>
> **bh_100 is stronger at depth 2 and not at depth 4.** That is a real depth-dependent difference in
> the nets, not an instrument failing — and it directly threatens the blend candidate, whose whole
> case was built at depth 2.
>
> **What survives: the origin-increment is 3.6× worse signal-to-noise than a direct match** at equal
> games (SNR 1.72 vs 6.12), because it adds two independent estimates in quadrature. The batch gate
> uses the increment. That is the measurement-rate bottleneck, stated precisely.

### The "frozen origin" is NOT frozen across runs

```
main.rs:370   let origin = Net::random(WIDTH_MENU[rung], seed)      <- depends on the RUN SEED
main.rs:413   let anchor = Net::random(champion.n_hidden, 20260907) <- fixed
```

Every run with a different seed faces a **different origin**. Arms at seed 20260907 share one;
the depth replication (424242, 987654) and `blend_seed2` (424242) each face their own.

* **Within a seed the comparison is still clean** — `depth_replicate` pits d2 against d3 at the same
  seed, so both meet the same opponent.
* **Across seeds, origin rates are not comparable.** This is the real explanation for `s2_100`
  reading 0.945 where `bh_100` read 0.832; I attributed that 0.113 swing to seed variance in the
  nets, and it is at least partly a different opponent.

**Consequence of both:** every ceiling arm was scored against the origin. Any gap **under ~0.05**,
any arm scoring **above ~0.95**, and any cross-seed comparison is provisional until re-measured
directly. Detail in `instrument_saturation_RESULT.md`.

Single source of current truth. The `*_RESULT.md` files are the working records and several contain
claims later retracted; **this file supersedes them where they disagree.**

---

## What is MEASURED and stands

### The ceiling investigation — four candidates, three dead

| candidate | verdict | measurement |
|---|---|---|
| capacity / net width | **REFUTED** | w64 loses **0.179 ± 0.021** to w16 at equal TIME |
| **datagen depth (d2 vs d3)** | **CONFOUNDED WITH PARITY** | see below — the effect is even-vs-odd, not shallow-vs-deep |
| draw filter | **REFUTED** | excluding draws better by **+0.086 ± 0.015**, two independent protocols |
| horizon schedule | **REFUTED** | widening beats narrow-fixed by **+0.064 ± 0.034** |

The original depth claim was **+0.025 ± 0.013** (8 deep generations beating 92 shallow at equal wall
clock). It is smaller than the ~0.07 between-run band, and it is now also known to be an even-vs-odd
comparison — see the parity section. A 2-seed replication with `--horizon-cap 45` is running, and
`depth_parity.sh` (d2 vs d4) is what actually decides it. Epochs is a fifth candidate.

### The depth lever is SEARCH PARITY, not depth

`distill_gap` measures `|tanh(root/scale) − tanh(eval/scale)|` — at blend 1 that is not a proxy for
the training signal, it **is** the training signal. Across two odd/even pairs:

| net | d3 | d4 | d5 | d6 |
|---|---|---|---|---|
| origin(random) | 0.0244 | 0.0137 | 0.0235 | 0.0167 |
| bn_000 | 0.3518 | 0.1524 | 0.3567 | 0.1842 |
| bn_075 (20 gen) | **0.5715** | 0.1976 | **0.5861** | 0.2365 |
| champion_long | **0.6086** | 0.2176 | **0.6220** | 0.2481 |

**Odd depths cluster high, even depths cluster low, and depth barely matters within a class.** For
bn_075, two extra plies inside a parity class moves the gap +2.5% (d3→d5) and +20% (d4→d6); crossing
parity moves it **2.6×**. This is the classic alpha-beta odd-even effect — at odd depth the side to
move gets the last ply and takes material without reply, inflating the root against a quiet eval.

**The depth lever compared d2 with d3 — even against odd.** Its +0.025 is a comparison between two
target distributions that differ 2.6× in how far they sit from the net's own eval, so "deeper search
gives better labels" is not what was measured. `depth_parity.sh` (d2 vs d4, parity held fixed) is
queued and decides it.

Note this does not say depth-3 training is *worse* — it says the mechanism is misattributed. Odd-depth
targets are systematically optimistic about the side to move, and that may genuinely help; it is just
not "deeper search sees more".

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
* **My own fix refuted before shipping.** Champion and base are scored on DIFFERENT opening sets
  (`match_nets` seeds openings from `Rng(seed | 1)`, and the two call sites differ by `^g`), and a
  comment at the anchor gate falsely claimed otherwise. Pairing them looked like free variance
  reduction. **Measured: ratio 1.12×, F(9,9) interval ~[0.28, 4.5] — does not clear 1**, while
  costing 2× games. Cause: openings walk only 4 random plies, so opening difficulty is a small part
  of the variance; it is in the games. `EXISTENCE_PAIRED_BATCH` stays off.
* **`readlink /proc/PID/exe` goes stale the moment you rebuild.** Rebuilding a binary while an
  instance is running makes its exe link read `<path> (deleted)` — the inode survives, so the JOB is
  fine, but any monitor globbing on the exe path silently stops seeing it. I rebuilt `netmatch`
  mid-run, my scan reported the process "GONE", and I was one step from relaunching a job that was
  7 minutes into a depth-4 match. **The brief recommends exe-matching as the safe alternative to
  `pgrep -f`; this is that alternative's own blind spot.** Match on the BASENAME with the suffix
  stripped: `b=$(basename "${e% (deleted)}")`.
* **A verification whose output I never read is not a verification.** I ran
  `grep -A1 'BASENAME with the suffix stripped' STATE.md` to confirm a claim, got NOTHING, did not
  notice, and committed a message asserting the text was intact. The text *was* intact — the phrase
  spans a line break and grep is line-based, so the pattern could not match. Right answer, no
  verification: the brief's own rule, *a grep that finds nothing is usually a broken pattern, not an
  absence*, applied to the check itself. **Print a count or a hit, never rely on silence.**
* **Killing a child does not stop the loop that spawned it.** I killed `depth_parity.sh`'s
  depth-4 arm for seed 424242 after finding the protocol flaw, and reported it handled. The script's
  `for SEED` loop simply started the *next* arm (987654) with the identical flaw, and it ran for
  five more minutes on a core before I noticed the log name had changed. **Kill the parent script
  FIRST, then the child** — otherwise the loop races you and respawns what you just removed.
* **A null from a coarse instrument is not a null.** `du -sb` over a 125G tree reported "0 bytes
  added in 15s" for the 4PC datagen and I nearly recorded it as STALLED. Per-file `stat` over the
  same interval showed +25KB and +67KB and a new file created mid-sample. Earlier the same day a
  `find -name '*.npz'` returned 0 because the job writes `.tsv`. **Two false nulls in one check** —
  match the instrument's resolution to the thing being measured, and confirm a null against a
  second method before believing it.
* **`taskset` on a build does not pin what the build spawns.** `taskset -c 14 cargo test` left
  rustc children with affinity `0-15` — running on HIS cores, which is a hard resource rule. Cargo
  spawns compiler processes through its jobserver and they did not all inherit the mask. Re-pinning
  them individually is a losing race against new spawns. **For builds, pin with an explicit
  `-j` limit and verify the CHILDREN's affinity, not the parent's** — or do not run a parallel build
  while other work is on the box. Killed the run rather than keep racing it; the test suite is worth
  having but not worth taking his cores.
* **A verdict block can spend real compute on a tautology.** `fixed_gate.sh` was 11 minutes into
  `fg_20.net vs champion_long.net` at depth 4 — but I had already md5-verified those files are
  **byte-identical** (4 gates, 0 KEEP, so every rollback restored exactly). It was matching a net
  against itself for an answer that is 0.500 by construction. Meanwhile `gate_align.sh`, relaunched
  after its arms were already complete, was re-running a verdict I had. **Before spending a match,
  check whether the two nets can differ at all** — the md5 that proves a rollback worked also proves
  the comparison is empty.
* **State outliving its run.** A 5-hour-stale `hz_1000.log` about to be read as a current arm; a
  mid-run script edit that killed a verdict block, where **reverting within a minute did not undo
  it**.

---

### Run-to-run is BIT-EXACT; the ~0.07 band is entirely SEED variance

`ratchet_test.sh` and `batch_ab2`'s K=5 arm turned out to be the same configuration (same seed,
init, binary, settings) launched as two independent processes on different cores. They agree on
**every line of all 9 overlapping generations**, batch gates included. So with a fixed seed and
`--threads 1` there is **no run-to-run noise at all**, and the ~0.07 between-run band is entirely
SEED variance. Two arms sharing a seed are directly comparable; only different seeds need the band.

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

### RETRACTED, AND REVERSED: it is a MEASUREMENT failure, and batching is the repair

**The section below is wrong and is kept only to show what the underpowered reading looked like.**

At 448 pairs `b2_5` vs `champion_long` read **0.513 ± 0.022** and I called it indistinguishable. At
**960 pairs** it is **0.529 ± 0.015, interval [0.514, 0.543] — b2_5 is STRONGER, clear of 0.5.**

So 20 generations of **batch-gated** training from `champion_long` produced a **real, resolved gain
of ~0.029**, while the per-generation arm over the same span accepted **nothing at all**. That is
precisely the hopeful branch `batch_ab2.sh` pre-registered:

> "KEEPs, and the arm ends above the baseline => MEASUREMENT failure. The gains were real and
> unmeasurable one at a time, batching is the repair, and the shipped default of `--gate-every 1`
> is the brake."

**The acceptance-floor arithmetic stands after all.** The gate demands an edge larger than its own
ci95 (0.0309 at 224 pairs) while a generation produces ~0.0114. At `--gate-every 1` the floor binds
and nothing is ever accepted; at `--gate-every 5` the accumulated change clears it and gets kept.
My "the floor is not the binding constraint" reading was built on the underpowered null and goes
with it.

**`--gate-every 5` is therefore a shipping candidate** — the default is 1. One training run so far;
a second seed is required before the default moves, same bar as blend.

Third time an underpowered null has misled me today, all from the same cause, which is why the
`netmatch` null threshold moved from ci95 < 0.03 to < 0.015 (~896 pairs). This re-check is the fix
paying for itself immediately.

### Superseded reading (kept for the record): "a LEARNING failure at the plateau"

`batch_ab2` finished both arms from `champion_long`, 20 generations each, on the fixed
anchor-increment gate. **Neither arm produced any change:**

* **gate-every 1** — **0 accepts in 20 generations.** The champion never moved, so the program
  printed *"no candidate was accepted; nothing to control against"* and wrote no net. Its output is
  `champion_long` by construction.
* **gate-every 5** — 4 batch gates, **1 KEEP and 3 ROLL BACK**. Final net vs `champion_long`
  head-to-head: **0.513 ± 0.022, INDISTINGUISHABLE.**

The origin metric read that batch arm as a *decline* (0.824 against the 0.861 baseline). The direct
match says no change — **saturation again**, now caught a third time.

**So `batch_ab.sh`'s original conclusion — "learning failure, the gate is not the lever" — was
RIGHT.** Its evidence was not: a broken instrument, arms of 11 vs 5 generations, and a negative
pre-registered as expected. Withdrawing it was still correct; a right answer reached through a
broken ruler is not a result, and it would have blocked exactly this re-run.

The acceptance floor is real arithmetic but it is **not the binding constraint**: with the floor
removed and the compressing opponent replaced, there is still nothing to keep.

**The loop learns from RANDOM and not from a trained champion.** From `--rung 0` it reaches 0.847
in 20 generations; from `champion_long` it moves nothing in 20. The ceiling is real and it is in the
training signal.

### Earlier partial reading on the fixed batch gate (superseded by the above)

The ratchet test runs `--gate-every 5` on a binary that has the anchor-increment gate, so it is
already producing the measurement `batch_ab2` was queued for:

```
batch gate g5: champ-vs-origin 0.819+/-0.024  base 0.828+/-0.022  increment -0.009+/-0.033  ROLL BACK
```

The acceptance-floor finding predicts that removing the 0.031 floor should reveal accumulated gains.
It does not. Five generations delivered **−0.009 ± 0.033**, CI [−0.042, +0.024], and the **+0.057**
that 5 × 0.0114/gen predicts is **excluded by the measured interval**.

So the floor is real (the arithmetic stands) but "the floor is why nothing is accepted" is now
doubtful: with the floor gone and the compressing opponent replaced, there is still nothing to
accept. **n = 1 batch gate**, at `champion_long`'s plateau (0.828 vs origin), and the 0.0114 figure
came from a different regime and the compressed metric — so this is evidence, not a verdict.
`batch_ab2` yields four more; the ratchet yields more still.

## Non-transitivity is REAL — so the direct match's 3.6× precision is on the wrong quantity

I suspected the codebase's non-transitivity demonstration was the same depth confound that produced
my false "sign reversals", which would have freed the batch gate to use a direct champion-vs-base
match and gain 3.6× signal-to-noise. **Measured at a matched depth 2, all three matches, 448 pairs:**

| | value |
|---|---|
| ep_1 vs champion_long, **direct** | **0.548 ± 0.021** — ep_1 stronger, clear of 0.5 |
| ep_1 vs origin | 0.831 ± 0.016 |
| champion_long vs origin | 0.855 ± 0.016 |
| **origin-increment** | **−0.024 ± 0.023** — champion_long stronger |

**The two metrics disagree in sign at the same depth.** Not a protocol artifact — genuine
non-transitivity. `ep_1` beats `champion_long` head-to-head while being worse against a fixed
reference.

So the anchor-increment design is **right** and stays. The direct match is 3.6× more precise, but it
measures "beats this specific opponent", which is not "stronger" — and a ratchet built on it would
climb a matchup rather than a strength. The 3.6× is not available; it was precision on the wrong
quantity.

Note this cuts *both* ways and neither metric is an oracle: for the blend pair the two agreed in
sign at matched depth, here they do not. Which one is trustworthy is **pair-dependent**, so every
number should name the instrument that produced it.

## The ceiling is not a COMPONENT — it is a measurement RATE

Every named candidate is now measured and closed:

| suspect | verdict |
|---|---|
| capacity / width | refuted — w64 does not beat w16 head-to-head, loses at equal time |
| draw filter | refuted (and the direction survives a direct match) |
| horizon schedule | refuted |
| datagen depth | **was search PARITY** — odd depths sit ~2.7× from the eval, and d2-vs-d3 is even-vs-odd |
| acceptance gate | real: batching produced 2 KEEPs in 5 gates, P = 0.0059 under a null |
| training signal collapses | refuted — the search-minus-eval gap **grows** with strength |
| encoding: net is material-only | refuted — champion_long is R² 0.588 against material |
| encoding: net is a linear PST | refuted — 0 always-on, 0 always-off, 1100–2300 activation regions |

What is left is not a component but an **arithmetic**:

* a generation produces about **0.0114**;
* a 224-pair gate resolves about **0.031**, and the two-sample batch bar is √2 × that;
* so an improvement can only be *banked* as fast as it can be *measured*.

That is exactly why five batched generations (≈0.057) produced a real KEEP where per-generation
gating accepted **nothing in twenty**. The loop is not blocked from learning; it is blocked from
*recognising* what it learned. `compound.sh` tests whether that banking compounds over 40
generations, and larger `--gate-every` is the obvious next dial if it does.

## Killed: `pd_d4` (depth-4 datagen from champion_long)

**Its premise expired and I did not re-price it.** `plateau_depth.sh` asked whether a deeper teacher
could move a champion that depth 2 could not. Depth 2 **can** move it — `b2_5` beat `champion_long`
0.529 ± 0.015 at 960 pairs. I retracted the "learning failure" reading and left the experiment built
on it running.

The cost made that expensive: 30 minutes elapsed, still on **generation 1**. Depth-4 datagen at 2400
games/gen is ~30 min per generation, so 20 generations is **~10 hours of a core** — spent on "is
depth 4 better by enough to matter" while the two matches that decide whether anything ships were
queued behind slower work.

A marker was written into `pd_d4.log` before killing it, because `plateau_depth.sh`'s verdict block
says a missing `pd_d4.net` **is** the tie branch. That reading does not apply: there is no result,
only a stopped run. Logs are gitignored, so this note is the tracked record.

## 8-net POOL RATING — the first non-transitivity-robust ranking (depth 2)

`pool_rating.rs`, 28 matches, 224 pairs each, **zero cyclic triples out of 56** — the pool orders
consistently, so this ranking can be read as one:

| net | rating |
|---|---|
| **b2_5** (gate-every 5) | **0.5544** |
| ep_2 | 0.5537 |
| ep_3 | 0.5371 |
| champion_long | 0.5207 |
| bh_100 (blend 1.00) | 0.4908 |
| s2_100 (blend 1.00, seed 2) | 0.4786 |
| s2_075 (blend 0.75, seed 2) | 0.4354 |
| bn_075 (blend 0.75) | 0.4292 |

All three candidates hold **at depth 2**, on 1568 pairs per net rather than one match:
gate-every 5 is +0.034 over champion_long; blend 1.00 beats 0.75 at **both** seeds (+0.062, +0.043);
epochs 2 beats epochs 3 by +0.017.

**The second-seed blend pair is the case that matters.** Its direct match was unresolved
(0.487 ± 0.015) and I downgraded the candidate over it; against a field it separates cleanly. The
pool did not overturn that reading — it *resolved* it, because a rating over seven opponents carries
seven times the games and no single matchup dominates.

**Still depth 2, which is not the strength standard**, and both candidates already degrade at depth 4
(b2_5 +0.029 → +0.002; bh_100 0.864 → 0.832 vs origin). The same pool is running at depth 4
(112 pairs, ~3h). **Nothing moves before it lands.**

### Ship-candidate attribution (depth 2)

```
sc_c vs s2_075 (shipped defaults)  0.531 ± 0.014   combination BETTER
sc_c vs s2_100 (blend only)        0.472 ± 0.017   combination WORSE than blend alone
```

**Epochs 2 hurts on top of blend 1.00.** Blend-alone is the better change. Running all three matches
instead of only the ship test is what makes that readable rather than a bare win.

## `--gate-match-depth` verified to BIND, and its real cost

The loop learns from depth-2 labels **and selects on depth-2 matches**, while strength is judged at
depth 4 — it optimises the game it measures. The new flag defaults to the datagen depth, so it
changes nothing until set. Verified by behaviour, same seed and training, only the gate depth
differing:

```
gate-match-depth 2:  champ-vs-origin 0.516±0.038  base 0.516±0.021  increment +0.000
gate-match-depth 4:  champ-vs-origin 0.422±0.060  base 0.492±0.056  increment -0.070±0.082
```

Completely different output — **not inert**, unlike the three features that shipped looking correct
in the diff and moved no number.

**Cost, corrected twice.** I first called it "a fraction of the price", then measured a 1.58×
variance penalty at depth 4 and revised to 2.4×. Both were wrong. Re-deriving pair sd from the
*trained-net* matches:

| match | ci95 | implied pair sd |
|---|---|---|
| b2_5 vs champion_long, **depth 2**, 960 pairs | 0.015 | **0.2371** |
| b2_5 vs champion_long, **depth 4**, 448 pairs | 0.022 | **0.2376** |
| established, 15,008 pairs | — | 0.2362 |

**Depth does not change pair variance for trained nets.** The 1.58× came from a 5-generation,
200-game probe whose nets were near-identical and drew heavily — a different variance regime, not a
depth effect. A depth-4 gate needs the **same pairs**; only the ~25× node cost applies, giving
**~1.9× total compute at `--gate-every 5`**.

Note `bn_075 vs origin` implies pair sd **0.162** — matches against a weak opponent are lopsided and
so have *lower* pair variance. That also corrects the "direct match has 3.6× the SNR" claim: the
noise ratio is only **1.3×**, and the remaining **2.8×** is the origin comparison *compressing the
signal* because both nets are near-saturated. I attributed the whole gap to quadrature addition.

## The full 40-generation batch-gate record (`rt_k5`)

8 gates, **2 KEEPs**, and the base moved 0.828 → 0.865 on the origin metric:

* KEEPs at **g10 (+0.033)** and **g25 (+0.039)**
* **g30, g35, g40 all ROLL BACK** — the last 15 generations produced nothing keepable

P(≥2 KEEPs in 8 | true null) = **0.0157**, so the KEEPs remain unlikely to be noise, though weaker
than the 0.0059 the first 5 gates gave. **The gains are front-loaded and then stop**, which is what
`compound.sh` was queued to test — and it now has a partial answer before it even starts: within
this run, batching bought two steps and then stalled.

## RESOLVED: blend 1.00's advantage is depth-2 only — the candidate is DEAD

The direct 448-pair match settles the hint below:

```
bn_075 vs bh_100   depth 2:  0.450 ± 0.016   RESOLVED, blend 1.00 stronger
                   depth 4:  0.511 ± 0.022   UNRESOLVED, no advantage
shift              +0.061 ± 0.027,  z = 4.40 -> the DEPTH EFFECT is resolved
```

Blend 1.00 genuinely beats 0.75 **at depth 2** and shows **no advantage at depth 4**. Depth 4 is the
strength standard, so **the blend candidate is dead for shipping** and the shipped default of 0.75
stands. The pool's common-opponent route pointed the same way independently.

**Consequence for the surviving candidate:** `sc_c` (blend 1.00 + epochs 2) beats the shipped
defaults at depth 4 — and it cannot be the blend doing it. It must be **epochs 2**, which the
depth-4 `sc_c vs s2_100` match tests directly.

### The hint this replaced (kept: it called the direction correctly while underpowered)

Depth-4 pool, via the common opponent `champion_long`:

| | depth 2 | depth 4 |
|---|---|---|
| bn_075 (blend 0.75) scores | 0.421 | **0.413** |
| bh_100 (blend 1.00) scores | **0.458** | 0.382 |
| ahead | blend 1.00 by 0.037 | blend 0.75 by 0.031 |
| resolved? | **yes** (448 pairs, diff ci95 0.031) | **no** (112 pairs, diff ci95 0.062) |

**Explicitly not claimed.** The depth-4 rows are underpowered — a 0.031 gap against a 0.062
difference interval. Reading a reversal off two underpowered matches is the error that produced
three retractions today. The direct 448-pair `bn_075 vs bh_100` match is running to settle it.

**If it confirms, two things follow:** the shipped default of blend 0.75 is *right* at the depth that
counts, ending my downgrade-then-partial-rehabilitation of blend 1.00 as a refutation; and `sc_c`,
which beats the shipped defaults at depth 4, must be winning on **epochs 2** rather than on blend —
which the depth-4 `sc_c vs s2_100` match now running shows directly.

## ⚠ SUPERSEDED — "first candidate to survive depth 4"
>
> `sc_c` is blend 1.00 + epochs 2. **Blend 1.00 is dead** (advantage is depth-2 only, z = 4.4) and
> **epochs 2 is dead** (0.498 / 0.473 / 0.448 once arms are matched). Whatever `sc_c` was winning
> on, it was not either component as measured. Kept as the record of a reading that looked solid
> and was not.

```
sc_c vs s2_075 (shipped defaults)   depth 2:  0.531 ± 0.014   clean
                                    depth 4:  0.529 ± 0.022   MARGINAL
```

**The point estimate is stable across depths** — 0.531 → 0.529. Every other candidate collapsed:
b2_5 went 0.529 → 0.502, bh_100's origin score 0.864 → 0.832. The depth-4 reading is flagged
marginal by the tool's own rule (margin 0.007 against half-ci95 0.011) purely because it has **448
pairs against depth-2's 896**. That is a *power* limitation, resolvable by spending pairs — not the
transfer failure that killed the others.

**Caveat that has to travel with it:** at depth 2, `sc_c` **loses** to `s2_100` (blend alone),
0.472 ± 0.017. So the combination beats what ships, but blend-alone may beat the combination —
epochs 2 appears to *hurt* on top of blend 1.00. The depth-4 version of that match is running, and
it decides whether the change to make is "blend only" or "both".

Nothing ships until the marginal reading is resolved at higher pair count **and** the depth-4
`sc_c vs s2_100` lands.

## ⚠ SUPERSEDED — DEPTH-4 POOL (28/28): the `ep_*` rows are CONFOUNDED
>
> **Two of the eight nets (`ep_2`, `ep_3`) were time-boxed with unequal generations (28 vs 26).**
> "epochs 2 wins" below is that confound, not an epochs effect — see the retraction section. The
> other six rows are generation-matched and stand. Kept because the *blend* reading here (last at
> depth 4) was independently confirmed by the direct match.

| net | depth 2 | depth 4 |
|---|---|---|
| **ep_2** | 0.5537 | **0.5999** |
| ep_3 | 0.5371 | 0.5634 |
| b2_5 | 0.5544 | 0.5417 |
| champion_long | 0.5207 | 0.5239 |
| s2_100 | 0.4789 | 0.4953 |
| bn_075 | 0.4290 | 0.4521 |
| bh_100 (blend 1.00) | 0.4909 | **0.4133** |
| s2_075 | 0.4356 | 0.4104 |

**Epochs 2 rises to the top at depth 4; blend 1.00 falls to last.** The direct match agrees and
*strengthens* with depth — `ep_2 vs ep_3` is 0.518 ± 0.014 (marginal) at depth 2 and
**0.544 ± 0.023 (resolved)** at depth 4. That is the opposite of blend, whose advantage vanished.

Blend remains **seed-inconsistent even at depth 4**: seed 1 gives `bn_075 vs bh_100` = 0.511
(0.75 ahead), seed 2 gives `s2_075 vs s2_100` = 0.456 (1.00 ahead). Dead.

## The gate-alignment fix FAILS — the origin anchor saturates at depth 4

```
gate-match-depth 2:  base 0.828, champ 0.819–0.862, increments −0.009..+0.033  ->  1 KEEP
gate-match-depth 4:  base 0.939, champ 0.936–0.948, increments −0.002..+0.009  ->  0 KEEP
```

Same net, same opponent, different depth: champion-vs-origin goes **0.828 → 0.939**. Deeper search
lets a good eval convert its advantage more reliably, so both sides crush the random origin and the
increment compresses to nothing. **The depth-4 gate is blind because its ANCHOR is too weak at depth
4**, not because selection depth is the wrong idea.

The binary check passed first (`xt6 reproduces b2_5's gate lines`), so this is the arms differing,
not the builds.

**The fix this implies:** a depth-4 gate needs a *trained* anchor, not `Net::random`. That is a
different change from `--gate-match-depth` and it is the one worth making.

## BUG FIXED: the batch gate never gated its first K generations

`main.rs:860` read `let base = batch_base.get_or_insert_with(|| champion.clone()).clone();` — and
that line sits **inside** the gate block. So `batch_base` was first set at the *first gate*, to the
champion after K generations of training, not to the net the run started from.

Two measured consequences:

* **The first gate compared a net against itself.** First-gate increments across four runs:
  `rt_k5 −0.009`, `b2_5 −0.009`, `ga_d2 −0.009`, `ga_d4 −0.002`. Noise, by construction.
* **Generations 1..K were never gated**, so damage there was permanent. `ga_d4` ran 20 generations
  with **zero KEEPs** and still finished at **0.813** against champion_long's **0.861** — its first
  5 generations cost 0.048 and there was no baseline to roll back to.

Fixed by seeding `batch_base` from the starting champion. Verified by behaviour on the new binary:
the first gate now reads `champ 0.863 base 0.883 increment −0.020` — a real comparison, and it
catches the early degradation the old code was blind to.

**This reframes every batch run today.** `b2_5`, `rt_k5` and `ga_d2` all had 5 ungated generations
baked in before their first real gate. Their KEEPs are still real (those were later gates against a
genuine base), but their *starting point* was already 5 generations of undone drift.

## Invariants held, and a mirror-image test the day's data implies

`sg_20` finished 2 KEEP / 1 ROLL BACK, and the md5 check agrees: **it differs from `bn_075`**, as a
rollback requires. Both rollback invariants now pass —

* `fg_20` (4 gates, 0 KEEP) → **identical** to `champion_long`
* `sg_20` (3 gates, 1 rollback) → **differs** from `bn_075`

— so the gate restores exactly when it should and only when it should.

**The mirror test.** At depth 4, `ep2_2 vs ep2_3` = 0.448: *fewer* epochs **loses**. The gradient
therefore points toward **more**, not fewer — the opposite of what I chased all day. `ep2_10` exists
from the same sweep at matched 20 generations, so `ep2_10 vs ep2_3` at depth 4 costs one match and
asks whether the shipped default of 3 is too **low**.

This matters because the distillation gap **grows** with net strength (0.186 at `champion_long`):
the search keeps finding things the eval does not know, so "the net is not being fitted hard enough
to close it" is a live explanation for the plateau that nothing has yet tested.

## The from-scratch learning curve, measured by the gate in 5-generation blocks

`sg_20` (rung 0, gate-every 5) gives the curve directly:

```
g5   base 0.500 -> champ 0.742   +0.242  KEEP
g10  base 0.731 -> champ 0.819   +0.088  KEEP
g15  base 0.825 -> champ 0.811   -0.013  ROLL BACK
```

**Flat by generation 15.** The loop gains almost everything in its first ~10 generations, then
stops. `champion_long`, with far more training, sits at **0.864** — the top of the same band.

That reconciles the day's two halves. Training from `champion_long` shows no reliable gain because
`champion_long` is already at the plateau this procedure reaches; and a from-scratch run reaches
that plateau in about ten generations. The gate is measuring a real ceiling, not failing to see
progress.

## The gate is not over-conservative: from scratch it KEEPs everything

`sg_20` — identical to `bn_075` (rung 0, 20 generations, blend 0.75, seed 20260907) except
`--gate-every 5` instead of 100, so `bn_075` had no gating at all:

```
g5   champ 0.742  base 0.500 ± 0.007  increment +0.242  KEEP
g10  champ 0.819  base 0.731          increment +0.088  KEEP
```

**`base = 0.500 ± 0.007` at g5 is the `batch_base` fix proving itself** — the baseline is the random
starting net matched against itself, which is 0.500 by construction. Under the old code the baseline
would have been the 5-generation champion and that first gate would have been a no-op.

So where there is real progress the gate keeps all of it. The rollbacks on `champion_long` are the
gate working, not the gate being too strict.

**Invariant to check when it finishes:** if every block KEEPs, the champion is never rolled back, so
`sg_20.net` should be **byte-identical to `bn_075.net`** — same seed, same training, and the gate
matches use their own seeded RNG. If it differs, the gate is perturbing the training stream, which
would be a defect worth finding.

## DATA COMPOUNDS: more games raises the decisive-game rate, which yields more data again

`dv_4800` (2× games/generation) against `bn_075`, same seed, same 20 generations:

| gen | bn_075 decisive | dv_4800 decisive | pool ratio |
|---|---|---|---|
| 1 | 389/2400 = **16.2%** | 734/4800 = **15.3%** | — (control) |
| 2 | 372/2400 = 15.5% | 821/4800 = 17.1% | |
| 4 | 600/2400 = 25.0% | 2061/4800 = **42.9%** | |
| 6 | 829/2400 = 34.5% | 2599/4800 = **54.1%** | **3.0×** |

**Generation 1 is the built-in control**: both nets are still random and the fractions match, so the
later divergence is caused by *training*, not by the game count. The pool at generation 6 is **3.0×**
where the game count is only 2× — the extra factor is the rising decisive fraction.

**This is a compounding loop, not a scale-up:** more data → stronger net → more decisive games →
more usable positions → more data. It is the mechanism the data hypothesis required, and it explains
why `champion_long` (far more accumulated data) sits above a fresh 20-generation run.

**AND THE ADVANTAGE IS CLOSING.** By generation 9 the decisive fractions have converged —
48.6% vs 51.1%, a gap of 2.5pp where it was 19.6pp at generation 6 — and the pool ratio has fallen
from 3.0× toward 2.48×, heading for the 2× the game count alone gives.

So this is a **faster climb to the same plateau**, which is the alternative flagged when the arm was
launched and the one the plateau-height criterion says is worth nothing. The 20-generation
head-to-head still decides it, but the trajectory has already answered.

## Testing the last structural lever: is the plateau DATA-limited?

No hyperparameter moved the plateau, and the plateau — not the climb rate — is the shipping
criterion. Three independent observations point at data volume:

* `fg_20`'s block increments rose monotonically as the pool grew: **−0.045, −0.033, +0.003, +0.007**
  with pool 88k → 432k, each block starting from the *same* champion.
* `champion_long`, carrying far more accumulated data, sits at **0.864** where a fresh 20-generation
  run reaches **~0.82**.
* The pool survives rollback, so data is the one thing that monotonically accumulates while the
  champion is repeatedly reset.

`dv_4800` is `bn_075`'s exact configuration with **`--games 4800` instead of 2400** — same 20
generations, so the arm-size guard reads "matched" and the variable under test is data *per*
generation.

**This is not an efficiency claim.** At equal generations the arm burns 2× the compute. The question
is strictly whether the plateau HEIGHT is data-limited; whether that data is worth its cost is a
different question and would need equal wall clock, which is the confound that has bitten three
experiments today.

## THE ONE LIVE LEAD: the data pool survives rollback, and the increments are climbing

`fg_20` finished 20/20 with 4 gates and **0 KEEPs**, and the invariant holds exactly —
`fg_20.net` is **md5-identical to `champion_long.net`**, so all four rollbacks restored cleanly.
That verifies the `batch_base` fix end-to-end.

The interesting part is the sequence:

```
g5   -0.045      g10  -0.033      g15  +0.003      g20  +0.007
pool 88k         pool 250k        pool 368k        pool 432k
```

**Monotone increasing, crossing zero at block 3.** Each block starts from the *same* champion — the
rollback restores the net — but **the data pool is not rolled back**. It grows 9,747 → 432,237
across the run. So the blocks are not independent samples: each trains the same starting net on a
strictly larger pool, and the trend is the pool growing.

Mean over four blocks is −0.017 ± 0.025 (no reliable change), but the mean is the wrong statistic
for a monotone series.

**PREDICTION REFUTED AT BLOCK 5.** The sequence is now **−0.045, −0.033, +0.003, +0.007, −0.010** —
block 5 went back negative with a *larger* pool than block 4 had. Five blocks give
**−0.016 ± 0.020**, no reliable change.

I predicted blocks would turn positive and start KEEPing. They did not. Four rising points was a
pattern fitted to a series short enough that one more point could break it, and one more point broke
it. `fg_60` has 7 blocks left and the mean may still move, but **the monotone reading is dead** and
pool growth alone does not drive the increment.

This is the only mechanism found today that predicts *improvement* rather than explaining absence.

## CLOSED: the loop cannot improve champion_long, and epochs 3 (shipped) is RIGHT

**The fixed gate rolls back everything.** `fg_20`, 15 generations from champion_long with the
repaired `batch_base`:

```
g5   champ 0.819  base 0.864  increment -0.045  ROLL BACK
g10  champ 0.830  base 0.864  increment -0.033  ROLL BACK
g15  champ 0.867  base 0.864  increment +0.003  ROLL BACK
```

**These are three INDEPENDENT 5-generation runs, not a trajectory.** Each block rolls back, so the
champion resets to `champion_long` before the next begins. Read that way:

```
mean -0.025 +/- 0.028   ->  no reliable change
spread 0.048            ->  dwarfs the mean
```

So 5 generations from `champion_long` neither reliably helps nor hurts — and the run-to-run spread
is the **same seed-noise wall** that killed every hyperparameter comparison today, showing up now
inside a single run.

Every block negative or neutral. **This also explains `b2_5`'s apparent gain.** Its "+0.033 KEEP" at
g10 was measured against a base that was already the *degraded* 5-generation net (0.819) — so it was
**recovering toward champion_long's 0.864, not improving past it.** The broken gate turned a
recovery into an accept.

**Epochs reverses direction once arms are matched:**

| comparison | result |
|---|---|
| seed 20260907, depth 2 | 0.498 ± 0.021 — no effect |
| seed 424242, depth 2 | 0.473 ± 0.020 — epochs 3 ahead |
| **seed 20260907, depth 4** | **0.448 ± 0.022 — epochs 3 ahead, resolved** |

The shipped default of 3 is not merely safe, it is **better**. The confounded evidence pointed the
wrong way, and correcting the 2-generation imbalance flipped the sign.

**Net position: nothing about the engine's settings was wrong.** Every candidate promoted today died,
and two of them (blend, epochs) died pointing back at the shipped value. What was wrong was the
gate, and that is fixed.

## A second bug of the same class, found by systematic audit

The `batch_base` defect was **a lazy cache whose subject changed underneath it**. So I enumerated
every `champion = ` assignment and checked each for a nearby `champ_anchor = None`:

```
line  849  champion = cand        cleared OK
line  952  champion = base        NO invalidation
line 1078  champion = acand       NO invalidation   <-- real bug
line 1130  champion = best_net    cleared OK
```

* **1078 (ARCH accept) is a genuine bug.** With `--anchor-pairs > 0`, an architecture step would
  leave the *old* champion's cached anchor score in place and judge the *new* champion's candidates
  against it. Latent, not live — `anchor-pairs` defaults to 0 — but `arch-every` defaults to 5, so
  it would fire the moment anyone enabled the anchor gate. Fixed.
* **952 (batch rollback) is safe**, because the only read site is guarded by `!batch_mode`. Now
  documented in place, so the next audit doesn't have to re-derive that it was checked rather than
  missed.

Two bugs of one shape in one file. **The generalisable move was turning the specific bug into a
search pattern** — "find every mutation of the thing a cache depends on" — rather than fixing the
one instance and moving on.

## CLOSED: epochs has no effect once the arms are matched — and the gate fix catches real damage

**Epochs, measured properly** (20 generations both arms, depth 2):

| seed | result |
|---|---|
| 20260907 | **0.498 ± 0.021** — no effect |
| 424242 | **0.473 ± 0.020** — epochs 3 slightly ahead |

The 0.518/0.544 that made epochs look like the surviving candidate was entirely the 2-generation
confound. **The shipped default of 3 stands.** No candidate remains.

**The `batch_base` fix is verified on live data.** `fg_20`'s first gate:

```
champ-vs-origin 0.819   base 0.864   increment −0.045   ->  ROLL BACK
```

Base 0.864 is champion_long's true origin score. The first 5 generations **cost 0.045**, and the
repaired gate detected and reverted it. Under the old code `base` would have been set to 0.819 — the
already-degraded net — and that loss would have been permanent and invisible. This is the same
0.048 that `ga_d4` lost with zero KEEPs to undo it.

**So the one thing that improved the engine today is a bug fix, not a hyperparameter.** Every tuning
candidate died; the gate now protects generations 1..K that were previously ungated.

## ⚠ RETRACTED: every epochs claim today used UNEQUAL ARMS

`ep_2` and `ep_3` — the nets behind every epochs number I reported — were run with `gens=1000000`,
i.e. **time-boxed, not generation-matched**. Epochs 2 trains faster per generation, so it got more:

```
ep_2   28 generations
ep_3   26 generations
```

Two extra generations at the measured ~0.0114 per generation is **~0.023 of advantage from training
amount alone** — the same size as the entire observed effect (0.018 at depth 2). The epochs effect
cannot be separated from the generation count.

**What this retracts:**
* `ep_2 vs ep_3` = 0.518 ± 0.014 (d2) and **0.544 ± 0.023 (d4)** — the "strengthens with depth"
  result, and the reason I called epochs the surviving candidate.
* **Both pool rankings**, where `ep_2`/`ep_3` are two of the eight nets — including "ep_2 tops the
  depth-4 pool". The other six rows are unaffected; those arms were generation-matched.

**The clean comparisons** are `ep2_*` (20 generations each, seed 20260907) and `es2_*` (20 each,
seed 424242). Only `es2` has reported: **0.473 ± 0.020 — epochs 3 ahead.** `ep2_2 vs ep2_3` is
running.

**This is the third time today the same defect has appeared**: `batch_ab` (11 vs 5 generations),
`depth_parity` (96 vs 1), and now the epochs sweep (28 vs 26). Time-boxed arms and a per-generation
cost that varies with the setting under test produce unequal training every time. The rule that
would have caught all three: **when the knob changes per-generation cost, match on generations, never
on wall clock.**

## ⚠ THE CAMPAIGN'S REAL LIMIT: seed variance swamps every effect being chased

**Both** candidates fail seed replication, the same way, with the heterogeneity itself resolved:

| comparison | seed 20260907 | seed 424242 | difference | z |
|---|---|---|---|---|
| blend 0.75 vs 1.00 | 0.450 | 0.487 | +0.037 | **+3.31** |
| epochs 2 vs 3 | 0.518 | 0.473 | −0.045 | **−3.61** |

Both **flip sign** between seeds. This is not two unlucky draws — the seed-to-seed differences are
significant, so the effects genuinely differ by seed.

**What that costs, quantified.** With a seed-to-seed spread of ~0.045 on the estimate:

| effect size | seeds needed | 20-generation arms |
|---|---|---|
| 0.05 | ~3 | 6 |
| 0.03 | ~9 | 17 |
| **0.02** | **~19** | **39** |

Every candidate this campaign has chased sits in the 0.02–0.05 band. **Two seeds cannot separate
them from seed noise**, and one seed certainly cannot — which is exactly what every "resolved"
depth-2 reading earlier today was.

**This is the honest closing state:** the measurement protocol (train 20 generations at two
settings, match the results) is under-powered for the effects it is being pointed at, by roughly an
order of magnitude in seed count. Nothing in the shipping table below survives that, and the fix is
not another candidate — it is either many more seeds per comparison, or longer runs where the effect
grows relative to seed noise.

## Shipping candidates, with evidence strength stated per item

All head-to-head at 960 pairs. **Nothing here has shipped**; none of it is an Elo number.

> **⚠ ALL OF THESE WERE MEASURED AT DEPTH 2.** This project's own standard for strength is **depth
> 4** — the gate derives its node budget as "7061 nodes = 100% coverage of a full depth-4 search"
> (`gate_depth_cap` default 4), and the built-in control plays at that cap with equal-time budgets.
> A depth-2 result is not automatically a depth-4 result. The depth-4 cross-check is running; until
> it lands, every row below is a claim about depth-2 play.

| candidate | shipped | measured | strength |
|---|---|---|---|
| ~~blend 1.00~~ | 0.75 | d2 **0.450 ± 0.016** / d4 **0.511 ± 0.022** | **DEAD — advantage is depth-2 only (z = 4.4)** |
| **`--gate-every 5`** | 1 | d2 **0.529 ± 0.015** / **d4 0.502 ± 0.022** | **FAILS AT DEPTH 4** — gain vanishes |
| epochs 2 | 3 | 0.518 ± 0.014 (ep_2 vs ep_3) | **MARGINAL** — margin 0.004 vs ci95 0.014 |

**Neither candidate survives the depth change, and the pattern is general.** `b2_5` beats
`champion_long` by +0.029 at depth 2 (resolved) and by +0.002 at depth 4 (unresolved) — the point
estimate collapses, not merely the interval widening. `bh_100` likewise scores 0.864 against the
origin at depth 2 and 0.832 at depth 4.

**Why this is expected in hindsight:** datagen runs at `--depth 2`, and the batch gate scores with
`match_nets(..., depth, ...)` — the *same* depth. So the loop both learns from and selects on
depth-2 play, while the project judges strength at depth 4. It is optimising the game it measures.
Aligning the **gate** to depth 4 is far cheaper than moving datagen there (224 pairs vs 2400
games/generation) and is the obvious next experiment.

**Blend no longer clears the bar either.** Seed 20260907 resolved (0.450, 1.00 stronger); seed
424242 did not (0.487, interval [0.471, 0.502] contains 0.5). Both point estimates favour 1.00, but
the two seeds differ by **0.037 ± 0.011, z = 3.3** — genuinely heterogeneous, so they cannot be
pooled into a win. `blend_seed2.sh` pre-registered exactly this: *"UNRESOLVED => needs a third
seed"*. And both readings are at depth 2, which is not the strength standard.

**Epochs is marginal, not clear.** I earlier called it "clear of 0.5" off a 448-pair reading; at 960
pairs the lower bound is 0.503. The direction is consistent across both readings and both sit above
0.5, but this is not yet a result and must not be shipped on.

Every one of these needs a **second training seed** before a default moves — a match seed re-rolls
openings and nothing else. `blend_seed2` (running), then `ship_candidate` for the combination.

## Task list (docs/MASTER_PLAN items 1-6) — verified stale

| item | status, verified by reading |
|---|---|
| 1. incremental NNUE accumulator | **done and correctly OFF.** `Acc` exists, `tests/incremental.rs` checks it against a full refresh, and `search.rs:169` gates it on `n_hidden >= 64`. It is a measured **0.91× LOSS** at width 32 (arch.rs:150), and shipped width is 16. All three clauses of its premise expired: eval is a sparse gather over ~38 active rows, not a dense 256×782 sweep; width is 16, not 256. It pays only at width ≥64, which was refuted at equal time. |
| 3. Zobrist + real TT slots | **done.** Incrementally maintained key (`chess.rs:271`) with a from-scratch `zobrist()` to check against. |
| 6. xcheck + perft as `#[test]`s | **done AND green.** Ran them: `canonical_perft_suite`, `movegen_agrees_with_an_external_engine`, `incremental_zobrist_matches_from_scratch_everywhere`, `make_unmake_restores_the_position`, plus 2 more — **6 passed, 0 failed**. Previously recorded as done on the strength of the files existing; now actually executed. |
| 5. register bytecode | deprioritised — the interpreter measured 1.003× hand-written on a quiet core. |

## Running now (one job per core, no chains)

| core | job | question |
|---|---|---|
| 13 | `fg_60` | do block increments keep rising as the pool grows past 432k? |
| 14 | `dv_4800` | is the plateau DATA-limited? (2× games/generation, same 20 generations) |
| 12 | `sg_20 vs bn_075` @ d4 | does gating help from scratch? |
| 15 | `ep2_10 vs ep2_3` @ d4 | is the shipped epochs default too **low**? |

The old PID-chained queue is gone — every arm in it either completed or was killed for a measured
defect (`depth_parity` for unequal arms, `pd_d4` for an expired premise, two verdict matches for
being tautological). Chains are not being rebuilt: they made reordering impossible while running,
which repeatedly left the highest-value item last.

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

<!-- Commit 82d7f6a's message body was mangled: it was written with `git commit -m "..."` in double
     quotes, so the backticked spans inside it were executed as command substitution and vanished.
     The line that disappeared was the fix itself, `b=$(basename "${e% (deleted)}")`, leaving the
     message reading `b=` with nothing after it. Every other commit today used a heredoc (-F -) for
     exactly this reason. Not force-pushed: the commit is public and the substantive record above
     is correct, so a rewrite of shared history buys nothing a follow-up note cannot. -->
