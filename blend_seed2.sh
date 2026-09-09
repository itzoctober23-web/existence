#!/usr/bin/env bash
# SECOND TRAINING SEED for the blend high side. This is the only thing between here and shipping.
#
# WHAT IS ALREADY SETTLED. Head-to-head, blend 1.00 beats blend 0.75, on two independent opening
# sets and at two power levels:
#     224 pairs, seed 20260907   bn_075 scores 0.459 +/- 0.030
#     896 pairs, seed 777        bn_075 scores 0.450 +/- 0.016   [0.435, 0.466]
# Consistent point estimates, both intervals clear of 0.5.
#
# WHAT IS NOT SETTLED, and why this script exists. BOTH nets were TRAINED at seed 20260907. Every
# match above re-rolls the OPENINGS, which controls for opening luck and nothing else. Two nets from
# one training seed can differ for reasons specific to that seed, and a head-to-head between them
# measures THOSE TWO NETS, not the setting. The shipped default cannot move on that.
#
# This retrains both arms at seed 424242 and matches them directly. If 1.00 wins again, the setting
# is doing the work rather than the seed.
#
# WHY IT MATTERS ENOUGH TO SPEND 80 MINUTES. Blend is the highest-leverage knob measured here --
# 0.25 -> 0.75 is +0.112 against the origin, larger than draws (+0.086), horizon (+0.064) or depth
# (+0.025). Being on the wrong side of it costs more than any other single setting, and the frozen
# origin put 0.75 AHEAD of 1.00 by +0.015, which is how it came to be the default.
#
# THE VERDICT IS HEAD-TO-HEAD, NOT THE ORIGIN CONTROL. The origin metric SATURATES: it reversed the
# sign on this very comparison and on capacity w16-vs-w64, both with intervals clear of 0.5 (see
# instrument_saturation_RESULT.md). The built-in origin control still runs, but as context.
#
# PRE-REGISTERED READING:
#   * 1.00 WINS AGAIN => two training seeds agree, and the default should move to 1.00. Ship it as
#     "passed the gate", never as "+N Elo" -- no Elo has been measured for it.
#   * 0.75 WINS => the first result was seed-specific. The default stays, and the saturation finding
#     still stands on capacity, which reversed independently.
#   * UNRESOLVED => the effect is smaller than one 896-pair match can see and needs a third seed
#     before anyone touches a shipped default.
#
# Run-to-run is BIT-EXACT here (verified: two independent processes agreed on every line of 9
# generations), so a fixed seed makes these arms exactly reproducible and a re-run is pointless.
set -uo pipefail
cd "$(dirname "$0")"
GENS=${GENS:-20}
SEED=${SEED:-424242}
CORE=${CORE:-14}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt2/release/learn}
NM=${NM:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt4/release/examples/netmatch}

[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
[ -x "$NM" ] || { echo "REFUSING: no netmatch at $NM, and the head-to-head IS the verdict"; exit 1; }

echo "=== blend high side, SECOND TRAINING SEED $SEED, $GENS generations per arm ==="
for B in 1.00 0.75; do
  tag=$(echo "$B" | tr -d '.')
  if grep -q 'CONTROL  final champion' "s2_${tag}.log" 2>/dev/null; then
    echo "--- blend $B: already complete, skipping ---"; continue
  fi
  echo "--- arm: blend $B (core $CORE) ---"
  timeout 4200 taskset -c "$CORE" nice -n 19 ionice -c 3 "$LEARN" \
    --rung 0 --gens "$GENS" --games 2400 --threads 1 --depth 2 --epochs 3 \
    --blend "$B" --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "s2_${tag}.net" --ledger "s2_${tag}.jsonl" > "s2_${tag}.log" 2>&1
  echo "  gens $(grep -cE '^gen ' "s2_${tag}.log"), origin control $(grep -A 1 'CONTROL  final champion' "s2_${tag}.log" 2>/dev/null | head -1 | grep -oE 'rate [0-9.]+ \+/- [0-9.]+') [CONTEXT, saturating]"
done

echo
echo "=== VERDICT: head-to-head at 896 pairs, seed 2 nets ==="
if [ -f s2_075.net ] && [ -f s2_100.net ]; then
  taskset -c "$CORE" nice -n 19 "$NM" s2_075.net s2_100.net 896 2 777 2>&1 | grep -E 'scores|=>' | sed 's/^/  /'
  echo
  echo "  seed 1 (20260907) gave: bn_075 scores 0.450 +/- 0.016 -> 1.00 stronger"
  echo "  s2_075 BELOW 0.5 as well => two training seeds agree, move the default to 1.00."
  echo "  s2_075 ABOVE 0.5         => the first result was seed-specific. Default stays."
else
  echo "  nets missing -- one arm did not finish. NO VERDICT; a partial pair is not a result."
fi
