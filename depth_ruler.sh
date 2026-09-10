#!/usr/bin/env bash
# DATAGEN DEPTH, JUDGED ON THE ABSOLUTE RULER — the replication depth_RESULT.md asked for, on a
# better instrument than the one it used.
#
# WHY RE-RUN SOMETHING ALREADY MEASURED. `depth_RESULT.md` compared datagen depth 2 vs 3 at equal
# wall clock and got +0.025 +/- 0.013 for the deeper arm, calling it "PROMISING, needs replication"
# because the run-to-run band is ~0.07. Two things have changed since, and both matter more than the
# extra seeds it asked for:
#
#  1. THE INSTRUMENT IT USED SATURATES. Those arms scored 0.850 and 0.875 against the frozen origin.
#     `instrument_saturation_RESULT.md` records that metric saturating and REVERSING SIGN twice, at
#     0.861 and 0.967 — i.e. exactly the region both arms were read in. A +0.025 measured there is
#     not safe to believe in either direction.
#  2. THERE IS NOW AN ABSOLUTE RULER. `sf_ruler.py` reads Elo against Stockfish on one scale, and
#     `absolute_ruler_RESULT.md` established the champion at ~1216 and FLAT across 1200 generations.
#     A datagen-depth effect can now be read in Elo instead of in a saturating ratio.
#
# AND THE ARM THAT MATTERS WAS NEVER RUN. Both prior arms were depth 2 and 3. `main.rs:152` sets
# `--depth` default **1**, so the loop that actually stalled generates at ONE PLY. Depth 1 is the
# baseline this comparison needs and it has never been in it.
#
# EQUAL WALL CLOCK, NOT EQUAL GAMES. A deeper search costs multiples of a shallow one, so equal
# games would hand the deep arm several times the compute and call the result "deeper is better" —
# the same error that flattered the wider net at fixed depth. The arms get the same seconds and the
# deep ones simply complete fewer generations. That is the trade a practitioner faces: given a fixed
# budget, label many positions shallowly or fewer deeply?
#
# PRE-REGISTERED READING, written before any arm ran:
#   * If deeper arms read HIGHER Elo on the ruler despite fewer generations, datagen depth is the
#     lever and "learned, then stopped" is a label-information problem. Deeper labels mean outcomes
#     that depend on the position.
#   * If they read the SAME, the stall is not the datagen depth and the remaining lever is inference
#     speed -> search depth.
#   * If they read LOWER, shallow-and-many wins at this strength and the depth_RESULT effect was the
#     saturating instrument, not a gain.
#   * ~90 Elo/ply is the scale to judge against (`elo_per_ply_RESULT.md`). An effect worth chasing
#     should be visible against a +/-60 Elo interval at 120 games; anything smaller needs more games
#     before it is called anything.
set -uo pipefail
cd "$(dirname "$0")"

SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
LEARN=${LEARN:-$SCR/xt_cap/release/learn}
SECS=${SECS:-1800}
SEED=${SEED:-20260910}
TAG=${TAG:-r1}
GAMES=${GAMES:-2400}

[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }

# One core per arm so they run in PARALLEL and share one wall clock. Cores 12-14 only: 15 carries the
# P2 evolve arm and 0-11 carry the 4PC datagen lanes, which must not be disturbed.
declare -A CORE=( [1]=12 [3]=13 [6]=14 )

echo "=== arms: datagen depth 1 / 3 / 6, ${SECS}s each, IN PARALLEL, nothing gated ==="
for D in 1 3 6; do
  timeout "$SECS" taskset -c "${CORE[$D]}" nice -n 19 ionice -c 3 "$LEARN" \
    --rung 0 --gens 1000000 --games "$GAMES" --threads 1 --depth "$D" --epochs 3 \
    --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "dr_${TAG}_d${D}.net" --ledger "dr_${TAG}_d${D}.jsonl" \
    > "dr_${TAG}_d${D}.log" 2>&1 &
done
wait

echo
echo "=== generations completed in the SAME clock (the cost of depth, measured not assumed) ==="
for D in 1 3 6; do
  printf "  depth %s: %s generations\n" "$D" "$(grep -cE '^gen ' "dr_${TAG}_d${D}.log")"
done

echo
echo "=== VERDICT: each arm on the ABSOLUTE RULER (depth 4, SF-1320 @10k nodes, 120 games) ==="
echo "    same instrument as untrained (-366), gen200 (-114) and champion (-104)"
for D in 1 3 6; do
  if [ -s "dr_${TAG}_d${D}.net" ]; then
    nice -n 19 taskset -c 12-14 python3 sf_ruler.py --net "dr_${TAG}_d${D}.net" \
      --depth 4 --sf-elo 1320 --sf-nodes 10000 --games 120 > "dr_${TAG}_d${D}_ruler.log" 2>&1
    printf "  depth %s  %s\n" "$D" "$(grep -E 'Elo vs|BOUND' "dr_${TAG}_d${D}_ruler.log" | head -1)"
  else
    echo "  depth $D produced no net -- read dr_${TAG}_d${D}.log"
  fi
done
echo
echo "DEPTHRULERDONE"
