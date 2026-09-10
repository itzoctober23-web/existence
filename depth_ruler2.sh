#!/usr/bin/env bash
# ROUND 2: datagen depth at equal compute AND EQUAL TRAINING STEPS.
#
# Round 1 (`depth_ruler.sh`) fixed `--games 2400` per generation for every arm, inherited from
# `depth_ab.sh`. Cost per game varies ~94x between depth 1 and depth 3 -- MEASURED by round 1 itself,
# 624 games/s against 6.67 -- so the arms completed 468 and 5 generations. That comparison is
# "468 training steps vs 5", not "shallow labels vs deep labels", and its answer was never in doubt:
# `untrained_baseline_RESULT.md` shows most of the loop's gain arriving inside the first ~200
# generations, so a 5-generation net has barely started.
#
# THIS ROUND HOLDS GENERATIONS EQUAL and lets games-per-generation absorb the depth cost, so both
# arms take the same number of training steps on the same clock and the LABEL is what differs:
#
#   depth 1   ~11,232 games/generation
#   depth 3      ~120 games/generation      both ~100 generations in 1800s
#
# The games-per-generation figures are DERIVED FROM ROUND 1'S MEASURED RATES, not assumed -- that is
# the one thing round 1 did supply cleanly.
#
# WHAT IS BEING TRADED, stated plainly: at a fixed budget and a fixed number of training steps, is it
# better to train each step on 11,232 shallow games or 120 deep ones? That is the practitioner's real
# question and neither round-1 arm answered it.
#
# PRE-REGISTERED:
#   * depth 3 >= depth 1  -> label quality beats label quantity; datagen depth is the lever and the
#     loop's default of depth 1 (main.rs:152) is the thing holding it at ~1216.
#   * depth 1 > depth 3   -> at this strength the loop wants MANY labels more than GOOD ones, and
#     lever 1 is closed. The remaining lever is search depth, which is spec-locked behind P2.
#   * Judged on the ABSOLUTE ruler (depth 4, SF-1320 @10k, 120 games), never against the frozen
#     origin -- that instrument saturates and has reversed sign twice in the 0.85-0.97 band where
#     depth_RESULT.md read it.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
LEARN=${LEARN:-$SCR/xt_cap/release/learn}
SECS=${SECS:-1800}
SEED=${SEED:-20260910}
TAG=${TAG:-r2}
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }

declare -A CORE=( [1]=12 [3]=13 )
declare -A GAMES=( [1]=11232 [3]=120 )

echo "=== equal generations (~100) and equal clock (${SECS}s); games/gen absorbs the depth cost ==="
for D in 1 3; do
  timeout "$SECS" taskset -c "${CORE[$D]}" nice -n 19 ionice -c 3 "$LEARN" \
    --rung 0 --gens 1000000 --games "${GAMES[$D]}" --threads 1 --depth "$D" --epochs 3 \
    --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "dr2_${TAG}_d${D}.net" --ledger "dr2_${TAG}_d${D}.jsonl" \
    > "dr2_${TAG}_d${D}.log" 2>&1 &
done
wait

echo
for D in 1 3; do
  printf "  depth %s: %s generations at %s games each\n" "$D" \
    "$(grep -cE '^gen ' dr2_${TAG}_d${D}.log)" "${GAMES[$D]}"
done
echo
echo "=== VERDICT on the ABSOLUTE ruler ==="
for D in 1 3; do
  if [ -s "dr2_${TAG}_d${D}.net" ]; then
    nice -n 19 taskset -c 12-14 python3 sf_ruler.py --net "dr2_${TAG}_d${D}.net" \
      --depth 4 --sf-elo 1320 --sf-nodes 10000 --games 120 > "dr2_${TAG}_d${D}_ruler.log" 2>&1
    printf "  depth %s  %s\n" "$D" "$(grep -E 'Elo vs|BOUND' "dr2_${TAG}_d${D}_ruler.log" | head -1)"
  else
    echo "  depth $D produced no net"
  fi
done
echo "DEPTH2DONE"
