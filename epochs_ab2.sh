#!/usr/bin/env bash
# A FIFTH CEILING CANDIDATE, never tested on a metric that works: EPOCHS.
#
# The four named candidates are now resolved -- capacity REFUTED (w64 loses 0.179 +/- 0.021 at
# equal time), the draw filter REFUTED (+0.086 +/- 0.015 on two protocols), the horizon schedule
# REFUTED (widening beats narrow by +0.064 +/- 0.034), and datagen depth surviving at
# +0.025 +/- 0.013. Epochs was never on that list and it should have been.
#
# WHY IT NEEDS RE-ASKING. It HAS been measured, and on the wrong instrument. The earlier finding --
# epochs 2 posting a 0.5114 mean gate rate against a control straddling 0.5 -- came from
# CANDIDATE-VS-CHAMPION gate rate, which has since been measured as near-blind: two similar nets at
# depth 2 draw almost everything, so that gate returns 0.500 +/- 0.007 on pairs a fixed anchor
# separates easily, and non-transitivity was shown directly. Every conclusion resting on it is
# unsupported, including this one.
#
# There is also a documented contradiction it might explain. main.rs records epochs 3/10/30 giving
# held-out losses of 0.041705 / 0.031720 / 0.025284 -- falling monotonically -- while the paired
# surrogate collapsed over the same range (+0.359 / +0.141 / -0.445). Loss and the surrogate moved
# in OPPOSITE directions on identical data. proxies_RESULT.md later measured loss itself as
# uninformative about strength (r = +0.379, CI [-0.249, +0.783], n=12), so the honest question is
# what epochs does to the only metric that has resolved anything: games against a frozen anchor.
#
# PRE-REGISTERED READING:
#   * If a LOWER epoch count wins, the loop is overfitting its target and epochs is a live lever --
#     and cheap, unlike depth.
#   * If 3 (the shipped value) wins, it was right, and the earlier gate-rate finding that favoured
#     2 was an artefact of the blind metric. Recorded as "right for the wrong reason".
#   * If a HIGHER count wins, more fitting of a self-referential target helps at this strength,
#     which would be genuinely surprising given the loss/strength decoupling and worth a second
#     seed before anyone believes it.
#   * UNRESOLVED is likely: the between-run band is ~0.07, wider than one run's ci95. More seeds.
#
# ARMS MATCHED ON GENERATIONS, not wall clock, so the horizon is identical in every arm -- the
# confound that caught the depth A/B, where equal wall clock silently produced h465 vs h45.
set -uo pipefail
cd "$(dirname "$0")"
GENS=${GENS:-20}
SEED=${SEED:-20260907}
ARMS=${ARMS:-"2 3 10"}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt2/release/learn}

[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
rm -f ep2_*.log   # stale outputs read as current results; see horizon_ab2.sh for what that cost

echo "=== epochs A/B v2: $ARMS, $GENS generations each, nothing gated ==="
echo "=== verdict = built-in final control vs the FROZEN ORIGIN, never the gate rate ==="
for E in $ARMS; do
  echo "--- arm: epochs $E ---"
  timeout 4200 taskset -c 15 nice -n 19 ionice -c 3 "$LEARN" \
    --rung 0 --gens "$GENS" --games 2400 --threads 1 --depth 2 --epochs "$E" \
    --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "ep2_${E}.net" --ledger "ep2_${E}.jsonl" > "ep2_${E}.log" 2>&1
  echo "  generations $(grep -cE '^gen ' "ep2_${E}.log"), final loss $(grep -oE 'loss [0-9.]+' "ep2_${E}.log" | tail -1)"
done

echo
echo "=== VERDICT: each arm vs its FROZEN ORIGIN (448 pairs, built-in control) ==="
for E in $ARMS; do
  printf "  epochs %-3s %s\n" "$E" "$(grep -A 1 'CONTROL  final champion' "ep2_${E}.log" 2>/dev/null | head -1 | sed 's/^.*net: //')"
done
echo
echo "  Watch the LOSS column against the rate: main.rs records loss falling monotonically with"
echo "  epochs while the surrogate collapsed, and loss has since been measured uninformative about"
echo "  strength (r = +0.379, CI [-0.249, +0.783]). If loss falls while the rate falls too, that is"
echo "  a fourth independent instance of the same decoupling and worth more than the epochs answer."
