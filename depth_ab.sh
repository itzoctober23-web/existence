#!/usr/bin/env bash
# ARE DEPTH-2 LABELS THE CEILING? datagen depth 2 vs depth 3, at EQUAL COMPUTE.
#
# WHY THIS IS NOW THE TOP CANDIDATE. width_RESULT.md refuted capacity: width 64 loses to width 16
# at equal time, 0.179 +/- 0.021, resolved. That was the top-ranked explanation for the 0.79-0.86
# band that 44 readings across 17 runs all sit in, and it is closed. What remains is the DATA, and
# the label's source is the first thing to test: the training target cannot be better than the
# search that produced it, and "depth 2" is a 3-ply tree here (choose applies the root move, then
# recurses with the full D).
#
# EQUAL COMPUTE, NOT EQUAL GAMES -- and this is the whole design.
#
# A depth-3 search costs multiples of a depth-2 one, so running both arms at 2400 games would hand
# the depth-3 arm several times the compute and call the result "deeper labels are better". That is
# the same error as comparing net widths at fixed DEPTH instead of fixed TIME, which flattered the
# wider net by construction and which this project already corrected once today.
#
# So the honest question is the one a practitioner actually faces: GIVEN A FIXED BUDGET, is it
# better to label many positions shallowly or fewer positions deeply? The arms therefore get the
# same wall-clock budget and the depth-3 arm simply completes fewer generations.
#
# THE RATIO IS MEASURED, NOT ASSUMED. Step 1 times one generation at each depth on this machine
# and prints it. Assuming a ratio is how three of today's corrections happened.
#
# PRE-REGISTERED READING:
#   * CONFIRMED if the depth-3 arm ends measurably ABOVE the depth-2 arm against the frozen origin,
#     despite completing fewer generations. Then label quality beats label quantity and the
#     datagen depth is the lever the ceiling has been hiding behind.
#   * REFUTED if it ends at or below. Then shallow-and-many wins at this strength, depth-2 labels
#     are not the constraint, and the remaining candidate is the HORIZON SCHEDULE (10 + (g-1)*5,
#     reaching 755 plies by generation 150 when datagen.rs:17-20 warns the outcome is nearly
#     independent of a position 40 plies earlier).
#   * UNRESOLVED is likely and is NOT a null: the run-to-run band is ~0.07, wider than a single
#     run's ci95. It means "more seeds", not "no effect".
#
# The verdict is the control against the FROZEN ORIGIN, measured at a fixed protocol that does not
# depend on either arm's training depth -- never the gate rate, since beating the net you were
# trained from is not strength and non-transitivity was demonstrated here directly.
set -uo pipefail
cd "$(dirname "$0")"
SECS=${SECS:-2400}
SEED=${SEED:-20260907}
PAIRS=${PAIRS:-1000}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt2/release/learn}
CTRL=${CTRL:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt2/release/examples/control}

[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
[ -x "$CTRL" ]  || { echo "no control at $CTRL"; exit 1; }

echo "=== STEP 1: MEASURE the depth-2 vs depth-3 datagen cost ratio on this machine ==="
for D in 2 3; do
  timeout 900 taskset -c 15 nice -n 19 ionice -c 3 "$LEARN" \
    --rung 0 --gens 1 --games 300 --threads 1 --depth "$D" --epochs 3 \
    --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "/dev/null" --ledger "/dev/null" > "dp_probe_d${D}.log" 2>&1
  echo "  depth $D, 300 games: $(grep -oE '\[[0-9]+s\]' "dp_probe_d${D}.log" | tail -1)"
done

echo
echo "=== STEP 2: both arms, EQUAL WALL-CLOCK (${SECS}s each), nothing gated ==="
for D in 2 3; do
  echo "--- arm: datagen depth $D ---"
  timeout "$SECS" taskset -c 15 nice -n 19 ionice -c 3 "$LEARN" \
    --rung 0 --gens 1000000 --games 2400 --threads 1 --depth "$D" --epochs 3 \
    --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "dp_d${D}.net" --ledger "dp_d${D}.jsonl" > "dp_d${D}.log" 2>&1
  echo "  completed $(grep -cE '^gen ' "dp_d${D}.log") generations in ${SECS}s"
done

echo
echo "=== VERDICT: each arm vs the FROZEN ORIGIN ==="
for D in 2 3; do
  if [ -f "dp_d${D}.net" ]; then
    timeout 1800 taskset -c 15 nice -n 19 ionice -c 3 "$CTRL" \
      --champion "dp_d${D}.net" --pairs "$PAIRS" > "dp_d${D}_ctrl.log" 2>&1
    printf "  depth %s  %s\n" "$D" "$(grep -E 'fixed depth 2' "dp_d${D}_ctrl.log" | head -1)"
  else
    echo "  depth $D produced no net -- read dp_d${D}.log"
  fi
done
echo
echo "  depth3 ABOVE depth2 => label QUALITY beats quantity; datagen depth is the lever."
echo "  depth3 AT OR BELOW  => shallow-and-many wins here; the horizon schedule is what is left."
echo "  unresolved => more seeds; the run-to-run band is ~0.07, wider than one run's ci95."
