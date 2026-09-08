# EXPERIMENTS — what was tried, and why it failed

The do-not-regress list. Every entry states the design, the result, and — where the
result did not hold up — what was wrong with the EXPERIMENT rather than the idea.

## 2026-09-08 — METHODOLOGICAL: my arms have been n=1, and it shows

**The problem, stated against my own data.** I ran single-seed arms all day and drew
causal conclusions from them. Two runs with IDENTICAL `--epochs 3` disagree:

| run | flags that differ | outcome |
|---|---|---|
| `cap40` | threads 4, arch-every 5 | NO collapse; control 0.694 +/- 0.044 at gen 20 |
| `long`  | threads 3, arch-every 6 | collapse from gen 13; rate 0.398, McNemar z -13.31 |

`--threads` repartitions the RNG stream across workers, so the two saw different games
from the same seed. That is enough to make them different draws, not a controlled
contrast. The `--epochs 1` arm then degraded LESS than `long` and MORE than `cap40`,
which is consistent with epochs mattering and equally consistent with it mattering not
at all.

**What this invalidates.** The commit "Decouple the trainer's step budget from datagen
volume" argues from the `long` run's collapse that raising volume 125x broke the
trainer's step budget. That mechanism is plausible and the arithmetic is real (~200 ->
~110,000 samples per generation at fixed epochs). But the EVIDENCE offered for it is one
run, and another run at the same setting did not collapse. `--steps-per-gen` remains
worth testing; it is not yet supported. Default stays 0.

**The rule, which was already written down and which I did not follow:** >= 3 seeds per
arm and >= 2 checkpoints before believing a difference. Eval power is n = 2/e^2 per arm,
so resolving a 0.05 effect needs ~800 pairs, not 320.

**Design for the re-run, once the box is free** (a 58.6M-row 4PC training holds it):
- 3 seeds x {epochs 3, steps-per-gen 20000}, all other flags IDENTICAL including
  `--threads`, since threads changes the data.
- Report the POOLED control across seeds with its interval, not the best arm.
- Pre-register the prediction before looking: if step budget is the mechanism, the
  fixed-budget arm should hold its McNemar z above zero where the epochs arm goes
  negative. If both go negative, the mechanism is wrong and the cause is elsewhere.

## 2026-09-08 — step budget: INTERIM, seed 1 of 3

First arms to complete with the derived per-net gate budget (the earlier attempts aborted on
the coverage guard). Control vs the frozen origin, seed 20260907:

| gen | epochs 3 | steps-per-gen 20000 |
|---|---|---|
| 10 | 0.641 +/- 0.045 | **0.756 +/- 0.043** |
| 20 | 0.809 +/- 0.040 | **0.834 +/- 0.035** |

At generation 10 the intervals do not overlap ([0.596, 0.686] vs [0.713, 0.799]), which is a
real separation favouring the fixed step budget. By generation 20 they overlap and the
difference is not significant.

**NOT A RESULT YET, and the reason is written above in this file.** This is n=1. Two runs at
IDENTICAL settings disagreed earlier today — that is what started this experiment — so a single
seed showing a clean separation is exactly the evidence that has already misled me once. Seeds
424242 and 987654 are running. The claim waits for the pooled three.

What IS established independently of the arms: both now compound strongly (0.809 and 0.834
against the origin at gen 20, against 0.694 in the earlier run), because the derived budget
gives the gate 100% coverage instead of the 33-57% a fixed 4000 nodes happened to produce.

## 2026-09-08 — FIRST SEARCH-TRACK RESULT: 128 mutations, 0 accepted

The search track ran end to end for the first time. 8 generations x 16 candidates against the
bare alpha-beta seed, gated on games at equal cost budget:

    106 well-typed, 22 ill-typed (type checker rejected them before any compute)
     90 failed the correctness oracle
     16 reached the game gate
      0 beat the champion

**This is the expected outcome and it is a measurement, not a failure.** A single random
mutation of a 71-node program that already computes the exact minimax value has almost no way
to improve it; GRAMMAR 6 puts the nearest real milestone (hash reuse) at +104 nodes, which is
not one mutation away. What the run establishes is that the PIPELINE works: candidates are
generated, ill-typed ones are rejected for free, incorrect ones are caught before spending
games, and the survivors are judged by play.

The oracle is doing the heavy lifting -- 90 of 106 well-typed candidates were REJECTED FOR
BEING WRONG, i.e. they returned a move the full-width reference disagreed with. Without that
stage every one of them would have gone to the gate, and the cheap-but-worse ones would have
been indistinguishable from genuine improvements on a cost-based metric.

TWO BUGS THE RUN EXPOSED, both in my harness rather than in the idea:

1. THE INTERPRETER NEVER ENFORCED ITS BUDGET (see above). One mutant looped for four hours.
2. THE SURROGATE WAS INERT. The seed scored 0/40 on the mate-in-1 set, so the filter compared
   0 < 0 and passed everything. Cause: the surrogate ran at the GAMES depth (3), where a
   single position costs ~411M cost units (ladder), overshooting the 2e9 safety cap and
   forfeiting. Mate-in-1 needs one ply. Given its own --surrogate-depth (default 2) it now
   scores 40/40 with 0 forfeits, and an assertion aborts the run if the seed ever fails its
   own surrogate again -- an inert filter that silently passes everything is worse than no
   filter, because it looks like a stage.

## 2026-09-08 — the cost model was never built, and it inverted a published result

GRAMMAR 8 and CRATE 4 both specify a per-primitive cost table (`configs/cost.toml`).
Neither existed; the interpreter charged a flat `self.cost += 1` per node, so a full NNUE
forward pass cost exactly what `const 3` cost. That is not neutral — it is a thumb on the
scale against any program that spends cheap work to avoid expensive work, which is exactly
a transposition table's trade. MEASURED (examples/cost_calibrate.rs, width 32, min-of-5):

| primitive | cost | |
|---|---|---|
| arith / cmp / const / var | 1 | |
| key (zobrist) | 97 | |
| terminal | 703 | |
| apply | 788 | |
| eval | **1365** | 293 ns |
| moves (legal_moves) | 2232 | |

Re-derived ladder, hash reuse relative to the seed: flat 1.250x (D2) / 1.218x (D3) becomes
**1.04x / 1.01x**. The 25% penalty was the instrument. RETRACTS the magnitude of the
"break-even near depth 5-6" claim and the chicken-and-egg constraint written into
MASTER_PLAN from it; the direction (overhead falls with depth) survives.

## 2026-09-08 — a fixed gate budget is wrong when tree size is NET-DEPENDENT

The 3-seed experiment reported ALL ARMS COMPLETE with 4 of 6 arms at zero generations. The
gate-coverage guard had aborted them: a fixed 4,000-node budget covered 57% of one seed's
depth-4 tree and 33% of another's, because different random nets produce different
alpha-beta cutoffs and therefore different tree sizes. The guard was RIGHT — it refused to
run gates that would return confident-looking 0.500s. `--cost-nodes` now derives from the
measured full tree at the cap depth.

The second half is worse and is a repeat: the wrapper never checked exit codes, so four
aborts printed as success. The 4PC queue runner already carries this exact lesson — "a
failed EXPERIMENT is a result; a failed SCRIPT is a bug."

## 2026-09-08 — the search track exists, and its oracle caught its own bug first

`evolve_search.rs`: mutate -> CORRECTNESS ORACLE (full-width negamax agreement) -> mates-per-cost
surrogate -> GAME GATE (candidate program vs champion program, same net, equal COST budget,
pentanomial). The previous `evolve.rs` had only the surrogate, scored against a random net.

First run printed `seed: bare alpha-beta, 71 nodes; oracle 7/10`. A seed failing its own
oracle is impossible — alpha-beta returns the full-width minimax value by construction. The
reference was a ply shallow: the seed's `choose` expands the ROOT itself then searches D more
plies, and I called the reference with depth-1. That is the SAME off-by-one GRAMMAR 8 records
behind the bogus "0.14x" interpreter reading. Fixed; seed now 10/10.

## 2026-09-07 — the ones that held

- **Self-play VOLUME.** 80 -> 10,000 games/generation. Training samples 33-267 ->
  12,212-106,485; the per-generation gate went from all-drawn 0.500 to 20W-44D-0L
  (0.656 +/- 0.057). This is the change that made the gate able to resolve at all.
- **Horizon cap 40.** Uncapped, the gate collapsed once the horizon passed 60 plies
  (eight straight generations below 0.5, four flagged `regression`). Capped, the champion
  compounded 0.552 -> 0.567 -> 0.570 -> 0.694. CAVEAT: also n=1 per arm; the capped arm's
  own trajectory is solid, the cross-arm claim is suggestive.
- **Gate depth cap 6 -> 4.** A 4,000-node budget bought 1.29% of a depth-6 tree, so the
  search never finished its first root move and both sides played near-randomly. Every
  budgeted gate was returning a confident-looking 0.500 that meant nothing.

## 2026-09-07 — the ones that failed, with the reason

- **`ab_hash` was not a transposition table.** Never called `eval`; returned constant 0 at
  every leaf. Its ladder "confirmation" of a 3.2x gain was the bug. Repaired: 175 nodes,
  and hash reuse measures a LOSS at D2-D4 that shrinks with depth (1.250x -> 1.113x),
  break-even near depth 5-6.
- **"Swap ladder steps 4 and 5."** Reasoned that iterative deepening creates the traffic a
  table needs. REFUTED: ID's cost is flat at ~1.086x across D2-D4 and hash+ID is worse
  than hash alone everywhere. Reverted in GRAMMAR 9 and MASTER_PLAN.
- **"Emission order gives alpha-beta a free ordering heuristic."** REFUTED in the
  informative direction: it is an ANTI-ordering costing +16/+31/+54% nodes at depth 3/4/5.
- **"Deeper random openings will make games decisive."** Null: 8.3% decisive at 4 plies,
  15.0% at 32, inside the noise at n=60. The lever was volume, not opening depth.
