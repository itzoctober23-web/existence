#!/usr/bin/env bash
# EQUAL WALL-CLOCK: how many games per generation actually buys the most strength?
#
# WHY. I concluded from a cost measurement that generations should be small -- datagen is
# 3.3ms/game and therefore ~all of a generation's cost, so 150 games/gen makes generations
# 27x cheaper than 4000. That is true and it is not the question. MEASURED on the two runs
# already on disk:
#
#     4000 games/gen, generation  5:  decisive 2066/4000 (52%), gate 0.703
#      150 games/gen, generation 49:  decisive   19/150 (13%), gate 0.479-0.521
#
# The cheap shape bought 12x more generations and stayed at its STARTING decisive rate,
# training on ~800 samples per generation against ~46,000. Cheaper generations are not the
# goal; strength per second is, and generation count is only a means to it. Counting
# generations would have scored the worse arm as the winner by a factor of twelve.
#
# METHOD. Each arm gets the SAME wall-clock, not the same generation count -- that is the
# whole point. Then every arm's champion is scored against the SAME frozen origin with an
# identical match, so the arms are comparable to each other and not just to themselves.
#
# --seed 20260907 is not arbitrary: examples/control.rs reconstructs the origin as
# Net::random(hidden, 20260907), so an arm trained under a different seed would be scored
# against a DIFFERENT origin and the comparison would be silently meaningless.
set -uo pipefail
cd "$(dirname "$0")"
SECS=${SECS:-300}
PAIRS=${PAIRS:-64}
SEED=20260907

echo "=== equal wall-clock ${SECS}s per arm, scored vs the same origin (seed $SEED) ==="
for G in 150 600 2400; do
  echo "--- arm: $G games/gen ---"
  timeout "$SECS" taskset -c 15 nice -n 19 ionice -c 3 ./target/release/learn \
    --gens 1000000 --games "$G" --threads 1 --depth 2 --epochs 3 \
    --gate-pairs 24 --arch-every 0 --control-every 0 --seed "$SEED" \
    --out "arm_${G}.net" --ledger "arm_${G}.jsonl" > "arm_${G}.log" 2>&1
  gens=$(grep -cE '^gen ' "arm_${G}.log")
  echo "  reached generation $gens in ${SECS}s"
done

echo
echo "=== scored against the frozen origin, identical match for every arm ==="
for G in 150 600 2400; do
  [ -f "arm_${G}.net" ] || { echo "  $G games/gen: NO CHAMPION (never accepted one)"; continue; }
  printf "  %5s games/gen  " "$G"
  taskset -c 15 nice -n 19 ionice -c 3 ./target/release/examples/control \
    --champion "arm_${G}.net" --pairs "$PAIRS" --seed 987654 2>/dev/null \
    | grep "fixed depth 2" | sed 's/^ *//'
done
echo "=== done ==="
