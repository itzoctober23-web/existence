#!/usr/bin/env bash
# DOES THE BATCH GAIN COMPOUND? Continue from b2_5.net for 40 more generations at --gate-every 5.
#
# WHAT IS ESTABLISHED. Twenty generations of batch-gated training from champion_long produced a net
# measurably stronger than its own starting point:
#     b2_5 vs champion_long   0.529 +/- 0.015 at 960 pairs   [0.514, 0.543]
# while the per-generation arm over the same span accepted NOTHING. That is a real gain and it is
# measured INDEPENDENTLY of the sample that selected it -- the batch gate chose on its own 224-pair
# match, and this is a separate 960-pair match on a different seed, so it is not the selection
# effect being read back.
#
# WHY COMPOUNDING IS THE QUESTION AND NOT "IS IT REAL". A single +0.029 step is worth little on its
# own; the engine only gets stronger if the loop can take that step REPEATEDLY. If 40 more
# generations add nothing, then the +0.029 was a one-time move off a particular starting point and
# the plateau stands one rung higher, which is a very different claim from "the loop works now".
#
# THE STATISTICAL WORRY THIS ALSO ADDRESSES, stated because it cuts against the result I want. The
# batch gate KEEPs when diff - 1.96*se > 0, a 2.5% false-positive rate per gate under a true null.
# b2_5 saw 1 KEEP in 4 gates, and P(at least one false KEEP in 4) is about 9.6% -- so the KEEP COUNT
# alone is consistent with pure noise. The independent 960-pair match is what rescues it from that,
# and a compounding run gives many more gates: if KEEPs are noise, the net will not climb.
#
# PRE-REGISTERED READING:
#   * BEATS b2_5 with the interval clear => the gain compounds, --gate-every 5 is a real repair, and
#     this net becomes the champion candidate. Report as "passed the gate", never as an Elo number.
#   * TIES b2_5 => the +0.029 was a one-off. The plateau is real, sitting one rung higher, and the
#     honest reading is that batching buys a single step rather than a working loop.
#   * LOSES to b2_5 => the batch gate ratchets on noise; KEEPs are selection, not improvement, and
#     the whole batch route closes. This is the branch the 9.6% figure above predicts.
#
# 40 generations gives 8 batch gates, twice b2_5's four, so a noise-ratchet has more chances to show
# itself and a real gain has more chances to accumulate. Both readings get more power, not just the
# hopeful one.
#
# Verdicts are head-to-head at 960 pairs. The frozen-origin control is CONTEXT: it saturates, and
# these nets sit at the top of its range where it has already been shown to compress a 0.129 gap
# into nothing.
set -uo pipefail
cd "$(dirname "$0")"
GENS=${GENS:-40}
SEED=${SEED:-20260907}
CORE=${CORE:-15}
INIT=${INIT:-b2_5.net}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt3/release/learn}
NM=${NM:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt4/release/examples/netmatch}

[ -f "$INIT" ]  || { echo "no init net at $INIT"; exit 1; }
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
[ -x "$NM" ]    || { echo "REFUSING: no netmatch at $NM, and the head-to-head IS the verdict"; exit 1; }
grep -qa 'champ-vs-origin' "$LEARN" || { echo "REFUSING: $LEARN predates the anchor-increment gate"; exit 1; }

if grep -q 'CONTROL  final champion\|no candidate was accepted' cp_40.log 2>/dev/null; then
  echo "--- compounding arm already complete, skipping ---"
else
  # SEED SHIFTED off the run that produced b2_5. Reusing seed 20260907 unchanged would replay the
  # same datagen stream this net was already trained on, and run-to-run here is bit-exact, so the
  # continuation would be measuring its own echo rather than fresh generations.
  echo "=== compounding: $GENS generations from $INIT, --gate-every 5 (core $CORE) ==="
  timeout 14400 taskset -c "$CORE" nice -n 19 ionice -c 3 "$LEARN" \
    --init "$INIT" --gens "$GENS" --games 2400 --threads 1 --depth 2 --epochs 3 \
    --gate-every 5 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed $((SEED + 1)) --out cp_40.net --ledger cp_40.jsonl > cp_40.log 2>&1
  echo "  gens $(grep -cE '^gen ' cp_40.log), batch gates $(grep -c 'batch gate' cp_40.log) ($(grep -c 'batch gate.*KEEP' cp_40.log) KEEP)"
fi

echo
echo "=== VERDICTS, head-to-head at 960 pairs ==="
if [ -f cp_40.net ]; then
  printf "  vs its own start   cp_40 vs b2_5:          "
  taskset -c "$CORE" nice -n 19 "$NM" cp_40.net "$INIT" 960 2 555 2>/dev/null | grep -E 'scores' | sed 's/^ *//'
  printf "  vs the champion    cp_40 vs champion_long: "
  taskset -c "$CORE" nice -n 19 "$NM" cp_40.net champion_long.net 960 2 555 2>/dev/null | grep -E 'scores' | sed 's/^ *//'
  echo
  echo "  Reference: b2_5 vs champion_long is 0.529 +/- 0.015. If cp_40-vs-champion_long is NOT"
  echo "  meaningfully above that, the second 40 generations added nothing and the gain did not"
  echo "  compound -- regardless of what cp_40-vs-b2_5 says on its own."
  echo "  Watch the KEEP count too: many KEEPs with no strength gain is the noise-ratchet branch."
else
  echo "  no cp_40.net. If the run accepted nothing, that IS the TIE branch, not a failed run."
fi

# DEPTH 4 IS THE VERDICT, DEPTH 2 IS CONTEXT. This project judges strength at depth 4: the gate
# derives its budget as "7061 nodes = 100% coverage of a full depth-4 search" and gate_depth_cap
# defaults to 4. Every head-to-head measured earlier today used depth 2, which is what the datagen
# uses -- so those are claims about depth-2 play and do not automatically transfer. A ship decision
# taken at depth 2 would be a decision about the wrong game.
# Depth 4 costs roughly 30x the nodes, so the pair count is lower; that is the intended trade.
echo
echo "=== VERDICT AT DEPTH 4 (448 pairs) -- this is the one that decides ==="
if [ -f cp_40.net ]; then
  printf "  cp_40 vs b2_5          d4: "
  taskset -c "$CORE" nice -n 19 "$NM" cp_40.net "$INIT" 448 4 555 2>/dev/null | grep -E 'scores|MARGINAL' | sed 's/^ *//'
  printf "  cp_40 vs champion_long d4: "
  taskset -c "$CORE" nice -n 19 "$NM" cp_40.net champion_long.net 448 4 555 2>/dev/null | grep -E 'scores|MARGINAL' | sed 's/^ *//'
  echo "  If depth 2 and depth 4 disagree, the depth-4 reading wins and the depth-2 one is recorded"
  echo "  as 'better at shallow search', which is a real but different claim."
fi
