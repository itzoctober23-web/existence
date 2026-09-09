#!/usr/bin/env bash
# DOES DEEPER DATAGEN BREAK THE MEASURED PLATEAU? One arm, because the control already exists.
#
# THE PLATEAU IS MEASURED, not assumed. Twenty generations from champion_long at datagen depth 2
# produced nothing, under BOTH gating schemes:
#     gate-every 1   0 accepts in 20 generations; the program itself printed "no candidate was
#                    accepted; nothing to control against" and wrote no net
#     gate-every 5   1 KEEP, 3 ROLL BACK; final net vs champion_long HEAD-TO-HEAD 0.513 +/- 0.022,
#                    indistinguishable
# Meanwhile 20 generations from --rung 0 reach 0.847. The loop learns from random and not from a
# trained champion.
#
# THAT d2-FROM-CHAMPION CONTROL IS WHY THIS SCRIPT RUNS ONE ARM AND NOT TWO. Re-running it would
# spend 40 minutes reproducing a number already measured twice, and run-to-run here is bit-exact.
#
# WHAT THIS TESTS. If the ceiling is the DATAGEN SEARCH being too shallow to teach a good net
# anything, then deepening it should move a champion that depth 2 cannot move. Depth 4 is chosen,
# not depth 3, because distill_gap measured the search-minus-eval gap as d3 >> d4 > d2 on every net
# across two position sets -- the alpha-beta odd-even effect -- so a depth-3 arm would confound
# "deeper" with "odd parity inflation". Depth 4 keeps the parity of the depth-2 baseline.
#
# PRE-REGISTERED READING:
#   * d4 BEATS champion_long head-to-head => datagen depth IS the ceiling mechanism, and the loop
#     stalls because its teacher is too weak rather than because its filter is too strict. That
#     would be the first thing measured to move a champion that the shipped loop cannot move.
#   * d4 TIES champion_long => depth is not the mechanism either. Combined with capacity refuted,
#     the gate refuted and the training-signal-collapse hypothesis refuted, the ceiling would not be
#     in any component tested so far, and the honest next step is the FEATURE SET -- 782
#     piece-square-ish inputs cannot express king safety or pawn structure at any width, which is
#     consistent with w64 failing to beat w16 head-to-head.
#   * d4 LOSES => deeper datagen at equal generations actively hurts here, which would be genuinely
#     surprising and would need a second seed before anyone believes it.
#
# EQUAL GENERATIONS, not equal wall clock. Depth 4 datagen is several times slower per generation,
# so this arm takes much longer than its control did -- that is the intended trade. It answers "are
# deeper labels better", NOT "are deeper labels worth the compute", and those must not be conflated:
# the original depth result was an equal-wall-clock claim and a different question entirely.
#
# The verdict is head-to-head. The frozen-origin control still prints, as context only, because that
# metric saturates and has reversed signs twice (instrument_saturation_RESULT.md).
set -uo pipefail
cd "$(dirname "$0")"
GENS=${GENS:-20}
SEED=${SEED:-20260907}
CORE=${CORE:-14}
INIT=${INIT:-champion_long.net}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt3/release/learn}
NM=${NM:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt4/release/examples/netmatch}

[ -f "$INIT" ]  || { echo "no champion at $INIT"; exit 1; }
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
[ -x "$NM" ]    || { echo "REFUSING: no netmatch at $NM, and the head-to-head IS the verdict"; exit 1; }

if grep -q 'CONTROL  final champion\|no candidate was accepted' pd_d4.log 2>/dev/null; then
  echo "--- depth-4 arm already complete, skipping ---"
else
  echo "=== plateau depth: datagen depth 4 from $INIT, $GENS generations (core $CORE) ==="
  timeout 14400 taskset -c "$CORE" nice -n 19 ionice -c 3 "$LEARN" \
    --init "$INIT" --gens "$GENS" --games 2400 --threads 1 --depth 4 --epochs 3 \
    --gate-every 5 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out pd_d4.net --ledger pd_d4.jsonl > pd_d4.log 2>&1
  echo "  gens $(grep -cE '^gen ' pd_d4.log), batch gates $(grep -c 'batch gate' pd_d4.log) ($(grep -c 'batch gate.*KEEP' pd_d4.log) KEEP)"
fi

echo
echo "=== VERDICT: head-to-head against the champion it started from, 896 pairs ==="
if [ -f pd_d4.net ]; then
  taskset -c "$CORE" nice -n 19 "$NM" pd_d4.net "$INIT" 896 2 777 2>&1 | grep -E 'scores|=>' | sed 's/^/  /'
  echo
  echo "  CONTROL, already measured, depth 2 from the same champion for the same 20 generations:"
  echo "    gate-every 5  ->  0.513 +/- 0.022 vs champion_long   (indistinguishable)"
  echo "    gate-every 1  ->  0 accepts; champion unchanged by construction"
  echo
  echo "  pd_d4 ABOVE 0.5 with the interval clear => deeper datagen moves a champion that depth 2"
  echo "  cannot. Report as 'passed the gate'; no Elo has been measured for it."
else
  echo "  no pd_d4.net -- if the arm accepted nothing, that IS the result: depth 4 could not move"
  echo "  the champion either, which is the TIE branch, not a failed run. Check pd_d4.log."
fi
