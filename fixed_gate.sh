#!/usr/bin/env bash
# DOES THE LOOP IMPROVE A CHAMPION ONCE THE GATE ACTUALLY GATES? Re-run with the batch_base fix.
#
# THE BUG THIS RE-TESTS. `batch_base` was set lazily inside the gate block, so the baseline became
# the champion AFTER the first K generations rather than the net the run started from. Two effects,
# both measured:
#   * the first gate compared a net against ITSELF -- increments -0.009, -0.009, -0.009, -0.002
#     across four runs;
#   * generations 1..K were NEVER GATED. ga_d4 ran 20 generations with ZERO KEEPs and still finished
#     at 0.813 against champion_long's 0.861: five ungated generations cost 0.048 and there was no
#     baseline to roll back to.
# Fixed by seeding batch_base from the starting champion. Verified by behaviour: the first gate now
# reads champ 0.863 base 0.883 increment -0.020, a real comparison that catches exactly that early
# degradation.
#
# WHY RE-RUN RATHER THAN REASON. Every batch result today carries 5 ungated generations at its
# start, including b2_5, which is the net that beat champion_long by 0.029 at depth 2. That gain is
# still real -- it was measured by a direct match against champion_long, not by the gate -- but it
# was achieved DESPITE an ungated and measurably harmful opening block. With generations 1..K under
# rollback protection the same 20 generations should do at least as well, and the question is
# whether they now do better.
#
# ARMS. One run, because the comparison is against nets that already exist:
#   fg_20 vs champion_long   did the loop improve its starting point?
#   fg_20 vs b2_5            did the fix beat the same 20 generations run with the broken gate?
# b2_5 was produced from the same INIT, seed, gens, gate-every and gate-pairs, and ga_d2.net is
# md5-identical to b2_5.net, so the only difference between fg_20 and b2_5 is the fix itself.
#
# PRE-REGISTERED READING:
#   * fg_20 BEATS b2_5 => the ungated opening block was costing real strength, and the fix is a
#     genuine improvement to the loop rather than only a correctness repair.
#   * TIES b2_5 => the opening block was harmless in this run and the fix is correctness-only. Still
#     worth having: ga_d4 shows it is NOT harmless in general.
#   * LOSES to b2_5 => rolling back the first block discards something the loop needed, which would
#     be surprising and needs a second seed before belief.
#
# Verdicts at BOTH depths. Depth 4 decides -- it is the strength standard and depth-2 gains have
# repeatedly failed to transfer -- but depth 2 is reported alongside because b2_5's known number is
# a depth-2 number and dropping it would make the comparison unreadable.
set -uo pipefail
cd "$(dirname "$0")"
GENS=${GENS:-20}
SEED=${SEED:-20260907}
CORE=${CORE:-13}
INIT=${INIT:-champion_long.net}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt8/release/learn}
NM=${NM:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt5/release/examples/netmatch}

[ -f "$INIT" ]  || { echo "no champion at $INIT"; exit 1; }
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
[ -x "$NM" ]    || { echo "REFUSING: no netmatch at $NM, and the head-to-head IS the verdict"; exit 1; }

if grep -q 'CONTROL  final champion\|no candidate was accepted' fg_20.log 2>/dev/null; then
  echo "--- fixed-gate arm already complete, skipping ---"
else
  echo "=== fixed gate: $GENS generations from $INIT, gate-every 5 (core $CORE) ==="
  timeout 7200 taskset -c "$CORE" nice -n 19 ionice -c 3 "$LEARN" \
    --init "$INIT" --gens "$GENS" --games 2400 --threads 1 --depth 2 --epochs 3 \
    --gate-every 5 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out fg_20.net --ledger fg_20.jsonl > fg_20.log 2>&1
  echo "  gens $(grep -cE '^gen ' fg_20.log), gates $(grep -c 'batch gate' fg_20.log) ($(grep -c 'batch gate.*KEEP' fg_20.log) KEEP)"
  echo "  FIRST gate (was a net-vs-itself no-op before the fix):"
  grep -m1 'batch gate' fg_20.log | sed 's/^ */    /'
fi

echo
echo "=== VERDICTS ==="
if [ -f fg_20.net ]; then
  for D in 4 2; do
    printf "  fg_20 vs champion_long  d%s: " "$D"
    taskset -c "$CORE" nice -n 19 "$NM" fg_20.net champion_long.net 448 "$D" 555 2>/dev/null | grep -E 'scores' | sed 's/^ *//'
    printf "  fg_20 vs b2_5 (broken gate, same config) d%s: " "$D"
    taskset -c "$CORE" nice -n 19 "$NM" fg_20.net b2_5.net 448 "$D" 555 2>/dev/null | grep -E 'scores' | sed 's/^ *//'
  done
  echo
  echo "  Reference, broken gate: b2_5 vs champion_long = 0.529 +/- 0.015 at depth 2,"
  echo "  0.502 +/- 0.022 at depth 4. Depth 4 decides."
else
  echo "  no fg_20.net -- check the log rather than reading a missing file as a tie."
fi
