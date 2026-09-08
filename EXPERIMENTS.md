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
