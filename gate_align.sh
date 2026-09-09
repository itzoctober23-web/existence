#!/usr/bin/env bash
# DOES SELECTING AT DEPTH 4 PRODUCE A BETTER DEPTH-4 NET? The alignment experiment.
#
# THE FINDING THIS TESTS. Datagen runs at --depth 2 and the batch gate scored with
# match_nets(..., depth, ...) -- the SAME depth. So the loop learns from depth-2 labels AND selects
# on depth-2 matches, while this project judges strength at depth 4. It optimises the game it
# measures, and both of today's candidates are gains in that game rather than the judged one:
#     b2_5 vs champion_long   +0.029 at depth 2 (resolved)   +0.002 at depth 4 (unresolved)
#     bh_100 vs origin        0.864 at depth 2               0.832 at depth 4
#
# --gate-match-depth (verified to BIND, not merely parse: at 32 pairs the gate's own champ-vs-origin
# line reads 0.516+/-0.038 at depth 2 and 0.422+/-0.060 at depth 4 from identical training) lets the
# GATE select at depth 4 while datagen stays at depth 2. Cost is ~1.9x total compute, against ~25x
# for moving datagen to depth 4 -- and depth does NOT change pair variance for trained nets, so the
# gate needs the same pair count and pays only the node cost.
#
# WHY THIS REPLACED compound.sh IN THE QUEUE. compound asks whether the batch gain repeats over 40
# more generations. rt_k5 has largely answered that already: 8 gates, KEEPs at g10 and g25, and
# NOTHING keepable in the last 15 generations. The gains are front-loaded and then stall, so
# compound's expected value dropped while this tests a mechanism nothing has probed.
#
# BOTH ARMS RUN ON THE SAME BINARY. b2_5 already is the depth-2-gated arm, but it was built with an
# older build, so arm A is re-run here rather than reused. Run-to-run is bit-exact, so if arm A does
# NOT reproduce b2_5's gate lines then the two builds differ behaviourally and that is worth knowing
# before any comparison is read -- the check is free and it validates the binary, not just the arm.
#
# PRE-REGISTERED READING:
#   * ARM B BEATS ARM A AT DEPTH 4 => aligning selection to the judged depth is a real lever, and it
#     is the first thing measured to improve depth-4 strength. Report as "passed the gate".
#   * TIES => selection depth is not what limits depth-4 transfer; the datagen depth or the training
#     target is, and both are far more expensive to change. That is a useful negative: it would mean
#     the cheap version of the fix does not work and the expensive one has to be justified separately.
#   * ARM B LOSES => selecting at a depth the labels do not come from actively hurts, which would say
#     the mismatch is load-bearing in the opposite direction and is worth a second seed before belief.
#
# The verdict is head-to-head AT DEPTH 4, since depth-2 verdicts are exactly what this experiment
# exists to stop trusting.
set -uo pipefail
cd "$(dirname "$0")"
GENS=${GENS:-20}
SEED=${SEED:-20260907}
CORE=${CORE:-15}
INIT=${INIT:-champion_long.net}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt6/release/learn}
NM=${NM:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt5/release/examples/netmatch}

[ -f "$INIT" ]  || { echo "no champion at $INIT"; exit 1; }
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
[ -x "$NM" ]    || { echo "REFUSING: no netmatch at $NM, and the head-to-head IS the verdict"; exit 1; }
"$LEARN" --help 2>&1 | head -1 | grep -q 'gate-match-depth' || \
  grep -qa 'gate-match-depth' "$LEARN" || { echo "REFUSING: $LEARN has no --gate-match-depth"; exit 1; }

for D in 2 4; do
  tag="ga_d${D}"
  if grep -q 'CONTROL  final champion\|no candidate was accepted' "$tag.log" 2>/dev/null; then
    echo "--- gate-match-depth $D: already complete, skipping ---"; continue
  fi
  echo "=== arm: gate-match-depth $D, $GENS generations from $INIT (core $CORE) ==="
  timeout 10800 taskset -c "$CORE" nice -n 19 ionice -c 3 "$LEARN" \
    --init "$INIT" --gens "$GENS" --games 2400 --threads 1 --depth 2 --epochs 3 \
    --gate-match-depth "$D" --gate-every 5 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "$tag.net" --ledger "$tag.jsonl" > "$tag.log" 2>&1
  echo "  gens $(grep -cE '^gen ' "$tag.log"), gates $(grep -c 'batch gate' "$tag.log") ($(grep -c 'batch gate.*KEEP' "$tag.log") KEEP)"
done

echo
echo "=== BINARY CHECK: arm A must reproduce b2_5's gate lines (run-to-run is bit-exact) ==="
if [ -f b2_5.log ]; then
  a=$(grep -oE 'increment [+-][0-9.]+' ga_d2.log 2>/dev/null | tr '\n' ' ')
  b=$(grep -oE 'increment [+-][0-9.]+' b2_5.log 2>/dev/null | tr '\n' ' ')
  if [ "$a" = "$b" ]; then echo "  MATCH -- xt6 reproduces the older build, so the arms are comparable"
  else echo "  DIFFER: xt6 [$a] vs b2_5 [$b]"
       echo "  The builds are not behaviourally identical. Read the verdict below knowing that."; fi
fi

echo
echo "=== VERDICT: head-to-head AT DEPTH 4, 448 pairs ==="
if [ -f ga_d4.net ] && [ -f ga_d2.net ]; then
  printf "  ga_d4 vs ga_d2 (aligned vs current) d4: "
  taskset -c "$CORE" nice -n 19 "$NM" ga_d4.net ga_d2.net 448 4 777 2>/dev/null | grep -E 'scores|MARGINAL' | sed 's/^ *//'
  printf "  ga_d4 vs champion_long              d4: "
  taskset -c "$CORE" nice -n 19 "$NM" ga_d4.net champion_long.net 448 4 777 2>/dev/null | grep -E 'scores|MARGINAL' | sed 's/^ *//'
  echo "  ga_d4 ABOVE 0.5 in the first line => aligning selection to the judged depth is a lever."
  echo "  Reference: b2_5 (depth-2 gated) vs champion_long is 0.502 +/- 0.022 at depth 4 -- i.e. the"
  echo "  current loop's 20 generations bought nothing measurable at the depth that counts."
else
  echo "  an arm produced no net; NO VERDICT. Check the logs rather than reading a missing file as a tie."
fi
