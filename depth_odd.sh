#!/usr/bin/env bash
# DOES DEPTH MATTER WITHIN A PARITY CLASS? The ODD-class twin of depth_parity.sh.
#
# STATE.md:326-350 establishes that the d2-vs-d3 "depth lever" was an EVEN-vs-ODD comparison:
# crossing parity moves the distill gap 2.6x while two extra plies inside a class move it 2.5%.
# depth_parity.sh answers the follow-up in the EVEN class by pitting d2 against d4. This answers the
# same question in the ODD class by pitting d1 against d3, and the two together are what separate
# "deeper search gives better labels" from "odd targets are optimistic".
#
# WHY THE ODD CLASS IS THE MORE INFORMATIVE HALF, and worth running even though d2-vs-d4 is queued:
#   * d1 is the SHIPPED setting. MASTER_PLAN.md:617 -- "until then depth 1 is the correct datagen
#     setting" -- so this is the only arm in the whole depth campaign that tells us whether any of it
#     beats what the loop actually uses. Every depth experiment so far (d2 vs d3, d2 vs d4) compares
#     two settings we do not run.
#   * It is CHEAP. Measured on this binary, 300 games costs 1s at d1, 3s at d2, 31s at d3 and ~10x
#     that at d4. The odd-class test is available for a fraction of the even-class test's compute.
#
# THE COST RATIO IS ALSO THE CONFOUND, and it is not fixable by capping the horizon. Equal wall clock
# gives d1 roughly 30x the generations of d3, and this tree measures ~0.0114 of advantage per
# generation, so the arms differ in TRAINING AMOUNT as well as in depth. netmatch prints the
# generation counts and the advantage that disparity is worth on its own; any effect smaller than
# that figure is training amount, not depth. That is why the verdict below is read WITH the arm sizes
# and not without them.
#
# --horizon-cap 45 ON BOTH ARMS, non-negotiable: horizon = 10 + (g-1)*5, so unequal generation counts
# produce unequal horizons, and horizon is a REFUTED-but-real effect (+0.064 +/- 0.034, STATE.md:319)
# pointing the same way as the generation disparity. Uncapped, this experiment would confound depth
# with horizon exactly as the original d2-vs-d3 run did.
#
# THE VERDICT IS netmatch AT DEPTH 4, NOT THE FROZEN-ORIGIN CONTROL. The origin metric SATURATES:
# depth_replicate.sh:78-80 records that it reversed the sign on blend 0.75-vs-1.00 and on capacity
# w16-vs-w64, both with intervals clear of 0.5. And depth 4 rather than the depth 2 that
# depth_replicate.sh and depth_parity.sh both pass, because netmatch.rs:23-29 is explicit that depth
# 4 is this project's strength standard and that judging at 2 silently answers a different question.
#
# PRE-REGISTERED:
#   * d3 BEATS d1 by more than the generation disparity is worth => depth is a real lever inside the
#     odd class, and combined with a d4-beats-d2 result it means deeper labels genuinely help.
#   * d1 >= d3 => the shipped setting stands, and the entire depth campaign has been comparing
#     settings that are all worse than the one already in use. Close the line.
#   * A gap SMALLER than the generation disparity is worth => not a depth result at all; report it as
#     training amount and say so.
set -uo pipefail
cd "$(dirname "$0")"
SECS=${SECS:-2400}
CORE=${CORE:-12}
WAIT_PID=${WAIT_PID:-0}
SEEDS=${SEEDS:-"424242 987654"}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt2/release/learn}
NM=${NM:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt4/release/examples/netmatch}

[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
[ -x "$NM" ]    || { echo "no netmatch at $NM"; exit 1; }

# Wait on a PID passed in, never on a name pattern (a waiter that greps for its target matches its
# own command line and blocks forever), and never on the pid `setsid` reports -- that is the wrapper,
# which exits immediately, so the wait returns at once and stacks this arm onto a live one.
if [ "$WAIT_PID" != 0 ]; then
  echo "waiting for pid $WAIT_PID to release core $CORE"
  while [ -d "/proc/$WAIT_PID" ]; do sleep 60; done
  echo "core $CORE free at $(date +%H:%M)"
fi

for SEED in $SEEDS; do
  if grep -qE '^gen ' "dr_${SEED}_d1.log" 2>/dev/null; then
    echo "--- seed $SEED depth 1: already has generations, skipping ---"; continue
  fi
  echo "--- seed $SEED, datagen depth 1 (SHIPPED setting), horizon-capped 45, core $CORE ---"
  timeout "$SECS" taskset -c "$CORE" nice -n 19 ionice -c 3 "$LEARN" \
    --rung 0 --gens 1000000 --games 2400 --threads 1 --depth 1 --epochs 3 \
    --horizon-cap 45 \
    --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "dr_${SEED}_d1.net" --ledger "dr_${SEED}_d1.jsonl" \
    > "dr_${SEED}_d1.log" 2>&1
  echo "  $(grep -cE '^gen ' "dr_${SEED}_d1.log") generations"
done

echo
echo "=== VERDICT: d1 vs d3, head to head at depth 4 (the strength standard) ==="
for SEED in $SEEDS; do
  if [ -f "dr_${SEED}_d1.net" ] && [ -f "dr_${SEED}_d3.net" ]; then
    echo "--- seed $SEED ---"
    timeout 5400 taskset -c "$CORE" nice -n 19 "$NM" "dr_${SEED}_d1.net" "dr_${SEED}_d3.net" 448 4 777 \
      2>&1 | grep -E 'scores|=>|arms:' | sed 's/^/  /'
  else
    echo "  seed $SEED: a net is missing, no match"
  fi
done
echo
echo "  d1 scoring BELOW 0.5 means depth 3 is stronger INSIDE the odd parity class."
echo "  Read it next to the arms line: d1 completes far more generations, and this tree measures"
echo "  ~0.0114 of advantage per generation. An effect smaller than that disparity is not depth."
