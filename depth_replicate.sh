#!/usr/bin/env bash
# REPLICATE THE DEPTH RESULT ON TWO MORE SEEDS. This is the price its own pre-registration set.
#
# depth_RESULT.md: at equal wall clock, 8 generations of depth-3 datagen beat 92 of depth-2,
# 0.875 +/- 0.009 against 0.850 +/- 0.010, difference +0.025 +/- 0.013 -- resolved WITHIN-RUN.
#
# AND THAT IS NOT ENOUGH, for a reason written down before the run: the between-run band measured
# across 17 runs is roughly 0.07 wide, nearly THREE TIMES this effect. A single pair of arms can
# produce a 0.025 gap by chance, and a within-run interval says nothing about between-run
# variation. The pre-registration named this outcome explicitly -- "unresolved => more seeds" --
# and it landed on the confirming side, which changes nothing about what it costs to believe.
#
# So: two more seeds, same protocol, arms paired within each seed. Three independent pairs is
# enough to see whether the sign is stable; it is not enough to pin the magnitude, and this script
# does not pretend otherwise.
#
# PRE-REGISTERED READING:
#   * CONFIRMED if depth 3 wins in all three pairs. Then label QUALITY beats label QUANTITY at this
#     strength, the datagen depth is a real lever, and the loop is tuned the wrong way round.
#   * REFUTED if the sign flips in either new pair. Then the first result was the between-run band
#     showing through, exactly as its own caveat warned, and depth joins width and draws as closed.
#   * SPLIT (2-1) is NOT a confirmation. With an effect this far inside the noise band, two of three
#     is what a coin does often enough to matter, and it would mean the honest answer needs more
#     seeds than this campaign can afford -- which is itself worth knowing and reporting.
#
# The verdict is each arm's built-in control against its own frozen origin, at fixed depth 2
# uncapped -- a protocol independent of either arm's TRAINING depth, which is the part that makes
# the comparison clean.
set -uo pipefail
cd "$(dirname "$0")"
SECS=${SECS:-2400}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt2/release/learn}

[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }

echo "=== depth replication: seeds 424242 and 987654, ${SECS}s per arm ==="
echo "=== seed 20260907 already measured: d2 0.850 +/- 0.010, d3 0.875 +/- 0.009 ==="
for SEED in 424242 987654; do
  for D in 2 3; do
    echo "--- seed $SEED, datagen depth $D ---"
    timeout "$SECS" taskset -c 15 nice -n 19 ionice -c 3 "$LEARN" \
      --rung 0 --gens 1000000 --games 2400 --threads 1 --depth "$D" --epochs 3 \
      --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 0 \
      --seed "$SEED" --out "dr_${SEED}_d${D}.net" --ledger "dr_${SEED}_d${D}.jsonl" \
      > "dr_${SEED}_d${D}.log" 2>&1
    echo "  $(grep -cE '^gen ' "dr_${SEED}_d${D}.log") generations"
  done
done

echo
echo "=== VERDICT: three independent pairs ==="
echo "  seed 20260907   d2 0.850 +/- 0.010   d3 0.875 +/- 0.009   d3 WINS"
for SEED in 424242 987654; do
  a=$(grep -A 1 'CONTROL  final champion' "dr_${SEED}_d2.log" 2>/dev/null | head -1 | grep -oE 'rate [0-9.]+ \+/- [0-9.]+')
  b=$(grep -A 1 'CONTROL  final champion' "dr_${SEED}_d3.log" 2>/dev/null | head -1 | grep -oE 'rate [0-9.]+ \+/- [0-9.]+')
  printf "  seed %-9s d2 %-22s d3 %-22s\n" "$SEED" "${a:-?}" "${b:-?}"
done
echo
echo "  3/3 for depth 3 => label quality beats quantity; the loop is tuned the wrong way round."
echo "  Any sign flip  => the first result was the between-run band showing through. Closed."
echo "  2-1 split      => NOT a confirmation at an effect this far inside the noise band."
