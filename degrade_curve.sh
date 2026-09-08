#!/usr/bin/env bash
# WHERE DOES THE NET START GETTING WORSE? Score against the frozen origin EVERY generation.
#
# THE OBSERVATION. In the batch-gate arm the champion measured 0.831 +/- 0.015 against the origin
# at generation 6, where champion_long itself measures 0.861 +/- 0.010 (1448W-546D-6L, re-measured
# today, not inherited). Difference 0.030 +/- 0.018 -- resolved. Loss counts are louder than the
# rate: champion_long loses SIX games in 2000 to the origin, the trained net loses 265.
#
# WHY THE FIRST REPLICATE WAS THROWN AWAY. It ran ONE generation and called that a replication of a
# GENERATION SIX reading. It is not the same intervention: `horizon = (10 + (g-1)*5)`, so gen 1
# trains only on positions within 10 plies of the end, from an EMPTY pool, while gen 6 trains at
# horizon 35 on a pool of ~200k. Same code path, different data regime. A clean result there would
# have proved nothing about the observation and I would have reported it as if it had.
#
# WHY A CURVE BEATS THREE POINTS. Three seeds at one generation give one condition with error bars.
# A per-generation control gives the SHAPE, which distinguishes the candidate causes directly:
#   * MONOTONIC DECLINE tracking the horizon => the horizon schedule widens faster than the engine's
#     strength justifies, so later generations train on labels that are increasingly noise.
#     datagen.rs:17-20 states this risk outright: "in self-play by a near-random engine the OUTCOME
#     is nearly independent of a position 40 plies earlier -- the players are noise".
#   * A CLIFF AT ONE GENERATION => a specific batch of data, not a schedule.
#   * FLAT AT ~0.861 => the single 0.831 was a bad draw and the plateau needs another cause.
#     Entirely possible off one reading, which is the whole reason this is being run.
#
# The label sign was checked FIRST and is not the bug: datagen stores z white-POV
# (Outcome::Loss -> -1 when White is the mated side to move) and the trainer flips `root` to
# white-POV before blending. The stale module header claiming "MOVER's point of view" contradicts
# the code, not the other way round.
#
# --gate-every 100 with --gens 8 means NOTHING is ever gated and no batch gate fires: every
# generation is accepted unconditionally. That is the intervention -- train, gate nothing, measure.
set -uo pipefail
cd "$(dirname "$0")"
INIT=${INIT:-champion_long.net}
GENS=${GENS:-8}
SEED=${SEED:-20260907}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xtarget/release/learn}

[ -f "$INIT" ] || { echo "no champion at $INIT"; exit 1; }
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }

echo "=== degradation curve: $GENS generations, NOTHING gated, control vs origin EVERY generation ==="
echo "=== baseline champion_long = 0.861 +/- 0.010 (re-measured 2026-09-08) ==="
echo "=== horizon = 10 + (g-1)*5, so g1=10 g2=15 g3=20 g4=25 g5=30 g6=35 g7=40 g8=45 ==="
timeout 3000 taskset -c 15 nice -n 19 ionice -c 3 "$LEARN" \
  --init "$INIT" --gens "$GENS" --games 2400 --threads 1 --depth 2 --epochs 3 \
  --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 1 \
  --seed "$SEED" --out "dc_${SEED}.net" --ledger "dc_${SEED}.jsonl" > "dc_${SEED}.log" 2>&1

echo
echo "=== CONTROL vs ORIGIN BY GENERATION (the curve) ==="
grep -E "control vs origin" "dc_${SEED}.log" | sed 's/^ */  /'
echo
echo "  baseline (untrained champion_long): 0.861 +/- 0.010"
echo "  Monotonic decline tracking the horizon => the schedule outruns the engine's strength."
echo "  A cliff at one generation => a specific batch, not the schedule."
echo "  Flat near 0.861 => the 0.831 was a bad draw; report that and look elsewhere."
