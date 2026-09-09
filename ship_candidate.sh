#!/usr/bin/env bash
# THE SHIP CANDIDATE: blend 1.00 + epochs 2, against the shipped blend 0.75 + epochs 3.
#
# Two shipped defaults now look wrong, each on a direct match with the interval clear of 0.5:
#     blend    bn_075 vs bh_100   0.450 +/- 0.016 (896 pairs, seed 777)  -> 1.00 stronger
#     epochs   ep_2   vs ep_3     0.528 +/- 0.020 (448 pairs)            -> 2 stronger
# Both were set from the frozen-origin metric, which has since been measured to SATURATE and to
# reverse signs (instrument_saturation_RESULT.md).
#
# WHY THIS RUNS AFTER blend_seed2.sh AND NOT INSTEAD OF IT. blend_seed2 trains (1.00, epochs 3) and
# (0.75, epochs 3) at seed 424242. This adds ONE arm -- (1.00, epochs 2) -- and reuses both. Three
# arms total instead of four, and the reuse is only sound because run-to-run is BIT-EXACT here
# (verified: two independent processes agreed on every line of nine generations), so s2_100.net is
# exactly the net this arm should be compared against.
#
# WHAT EACH MATCH ANSWERS:
#   s2_100 vs s2_075   does the blend change hold at a SECOND TRAINING SEED? (blend_seed2 prints it)
#   sc_c   vs s2_100   does epochs 2 add anything ON TOP of blend 1.00?
#   sc_c   vs s2_075   is the COMBINATION better than what ships? <- the only one that decides a ship
#
# The third is the ship test and the first two are the attribution. Running all three means a
# combination that wins can be explained, and one that loses can be blamed on the right arm --
# testing only the combination would leave a loss uninterpretable.
#
# PRE-REGISTERED READING:
#   * COMBINATION WINS and both components win => change both defaults. Report as "passed the gate".
#     NEVER as "+N Elo": no Elo has been measured for any of this, and quoting one for an unshipped
#     change is the specific error the brief forbids.
#   * COMBINATION WINS but a component loses => the two interact; ship the combination only, and say
#     so, because the per-component story would be wrong.
#   * COMBINATION LOSES => the components are antagonistic at this strength. Nothing ships, and the
#     single-component results stand as measured rather than being quietly dropped.
#
# A REMAINING LIMIT, STATED NOT BURIED: head-to-head is not an oracle either. Non-transitivity is
# demonstrated in this tree, and nets trained toward different targets can carry a matchup edge that
# is not general strength. Two training seeds agreeing is the bar this campaign can afford; it is
# not proof, and the champion it produces should be re-checked against a third opponent before it is
# treated as the new baseline.
set -uo pipefail
cd "$(dirname "$0")"
GENS=${GENS:-20}
SEED=${SEED:-424242}
CORE=${CORE:-14}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt2/release/learn}
NM=${NM:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt4/release/examples/netmatch}

[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
[ -x "$NM" ]    || { echo "REFUSING: no netmatch at $NM, and the head-to-head IS the verdict"; exit 1; }
for n in s2_100.net s2_075.net; do
  [ -f "$n" ] || { echo "REFUSING: $n missing -- blend_seed2.sh must finish first. A ship test"; \
                   echo "  against a net that does not exist would silently compare nothing."; exit 1; }
done

if grep -q 'CONTROL  final champion' sc_c.log 2>/dev/null; then
  echo "--- combination arm already complete, skipping ---"
else
  echo "=== ship candidate: blend 1.00 + epochs 2, seed $SEED, $GENS generations (core $CORE) ==="
  timeout 4200 taskset -c "$CORE" nice -n 19 ionice -c 3 "$LEARN" \
    --rung 0 --gens "$GENS" --games 2400 --threads 1 --depth 2 --epochs 2 \
    --blend 1.00 --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out sc_c.net --ledger sc_c.jsonl > sc_c.log 2>&1
  echo "  gens $(grep -cE '^gen ' sc_c.log), origin control $(grep -A 1 'CONTROL  final champion' sc_c.log 2>/dev/null | head -1 | grep -oE 'rate [0-9.]+ \+/- [0-9.]+') [CONTEXT ONLY -- saturating]"
fi

echo
echo "=== VERDICTS, head-to-head at 896 pairs ==="
[ -f sc_c.net ] || { echo "  combination arm produced no net. NO VERDICT."; exit 1; }
printf "  epochs 2 on top of blend 1.00   sc_c vs s2_100: "
taskset -c "$CORE" nice -n 19 "$NM" sc_c.net s2_100.net 896 2 777 2>/dev/null | grep -E 'scores' | sed 's/^ *//'
printf "  SHIP TEST  combination vs shipped  sc_c vs s2_075: "
taskset -c "$CORE" nice -n 19 "$NM" sc_c.net s2_075.net 896 2 777 2>/dev/null | grep -E 'scores' | sed 's/^ *//'
echo
echo "  sc_c ABOVE 0.5 in the SHIP TEST => the combination beats the shipped defaults at a second"
echo "  training seed. That is 'passed the gate' and nothing more -- no Elo has been measured."
echo "  sc_c BELOW 0.5 => nothing ships; the single-component results stand as measured."
