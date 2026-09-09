#!/usr/bin/env bash
# RE-ASK: is the plateau a MEASUREMENT failure or a LEARNING failure? gate-every 5 vs 1.
#
# batch_ab.sh already asked this and returned "learning failure -- every batch ROLLED BACK". That
# answer is WITHDRAWN, for three reasons found by reading the run rather than the script.
#
# 1. THE INSTRUMENT WAS THE BROKEN ONE. bg_5.log printed
#        batch gate g5 (last 5 gens): 0.500+/-0.007 LLR +0.00 -> ROLL BACK
#    A single head-to-head rate with an LLR -- champion against the net the batch started from.
#    The CURRENT source prints "champ-vs-origin ... base ... increment" (main.rs:873), because the
#    batch gate was rewritten at 19:18 in bae8c7b to measure the INCREMENT OVER A FIXED ANCHOR. The
#    binary that produced bg_5.log was built at 17:13 and the run wrote its log at 17:22 -- TWO
#    HOURS BEFORE the fix. Verified by CONTENT, not by mtime: `strings` on that binary has no
#    'champ-vs-origin', and the binary this script uses does.
#
#    And 0.500 +/- 0.007 is not a null, it is the blind gate's SIGNATURE -- the same tight-interval
#    0.500 two random movers produce. The old batch gate compared the candidate against the thing it
#    was trained from, which is precisely the comparison measured today as compressing ~3x
#    (+0.036 +/- 0.016 on the champion gate for a pair the frozen origin puts at +0.112 +/- 0.033).
#
# 2. THE ARMS WERE UNEQUAL. Time-boxed at 1800s each, the run got ELEVEN generations at K=5 and
#    FIVE at K=1 -- because the K=1 arm stops for a 224-pair match every single generation. Two
#    variables moved and the uncontrolled one favoured the batch arm. Matched on GENERATIONS here.
#
# 3. THE NEGATIVE WAS PRE-REGISTERED AS EXPECTED. batch_ab.sh:29 reads "Given 0.5024 I expect this
#    outcome", and the expected outcome is what a later-condemned instrument then reported. An
#    expectation confirmed by a broken ruler is the cheapest kind of wrong answer to get.
#
# WHY IT MATTERS ENOUGH TO SPEND THE COMPUTE. The acceptance rule is
#     main.rs:675   resolved_up = pent_rate - ci95 > 0.5      (accept needs edge > ci95)
#     main.rs:733   gate 224 pairs -> median ci95 0.031       (1.96*0.2362/sqrt(224) = 0.0309, checks)
#     STATE.md      a real per-generation edge is ~0.0114     (needs ~1,650 pairs, checks)
# so the shipped gate demands an edge 2.7x LARGER than a generation produces. main.rs:757 states
# this premise itself -- "at 224 pairs its ci95 is ~0.031, and real steps are far smaller than
# that" -- but concludes only that the ANCHOR should veto rather than demand improvement. The same
# sentence applied to the CHAMPION gate says the loop cannot accept a real step at all, and that
# consequence appears nowhere. The anchor cannot rescue it either: main.rs:752 is explicit that it
# runs only on candidates that already passed the champion match, "so it can only ever veto".
#
# Even taking the champion gate at FACE VALUE with no compression, its 0.0309 floor exceeds datagen
# depth (+0.025) -- the only ceiling candidate still standing. That conclusion needs no compression
# estimate, which is why it is the one worth acting on.
#
# K=5 IS DELIBERATELY UNCHANGED from batch_ab.sh so the instrument is the only difference. The
# arithmetic also supports it independently: the two-sample bar at 224 pairs/arm is
# sqrt(2)*0.0309 = 0.0437, needing 3.8 generations of accumulation at ~0.0114 each, so 5 clears it.
#
# PRE-REGISTERED READING (and this time the hopeful branch is the one with a mechanism behind it):
#   * KEEPs, and the arm ends ABOVE 0.861 vs the frozen origin => MEASUREMENT failure. The gains
#     were real and unmeasurable one at a time, batching is the repair, and the shipped default of
#     --gate-every 1 is the brake.
#   * Every batch ROLLS BACK ON THE FIXED GATE => now a real learning-failure result, because the
#     rolling back would be done by an instrument that is not compressed and not head-to-head.
#   * KEEPs but ends AT OR BELOW 0.861 => the rollback rule is passed by drift; that indicts the
#     two-sample threshold, not the learning.
#
# THE VERDICT BLOCK REFUSES TO COMPARE UNEQUAL ARMS. That is the defect that made the first run
# uninterpretable, and a script that can still print a tidy comparison after a timeout kill will
# print one.
set -uo pipefail
cd "$(dirname "$0")"
GENS=${GENS:-20}
SEED=${SEED:-20260907}
INIT=${INIT:-champion_long.net}
# Core is a VARIABLE so this can run beside the core-15 chain instead of queueing behind it. The
# whole chain is serial on one core while cores 12-14 hold only the search track; that is why this
# -- the item that decides whether the loop can accept anything at all -- was six hours out.
CORE=${CORE:-15}
# xt3: the ONLY scratch build containing the anchor-increment batch gate. Verified by content.
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt3/release/learn}

[ -f "$INIT" ]  || { echo "no champion at $INIT"; exit 1; }
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
# grep -qa on the FILE, never `strings | grep -q`: under `set -o pipefail` the grep exits on its
# first match, closes the pipe, strings dies of SIGPIPE (141), and pipefail reports the pipeline as
# FAILED -- so the guard refused a binary that passes. It cost this experiment one launch. The
# behaviour test I ran only proved the guard could say NO; the yes-case was never exercised.
grep -qa 'champ-vs-origin' "$LEARN" 2>/dev/null || {
  echo "REFUSING TO RUN: $LEARN predates the anchor-increment batch gate (bae8c7b)."
  echo "Running the old head-to-head gate again would reproduce the withdrawn result."
  exit 1; }

rm -f b2_*.log b2_*.jsonl b2_*.net   # scoped; never touches bg_*, bn_*, bh_*, ep2_*

echo "=== batch-gate A/B v2: gate-every 5 vs 1, $GENS generations each, init $INIT ==="
echo "=== instrument: anchor-increment batch gate (verified present in the binary) ==="
for K in 5 1; do
  echo "--- arm: gate-every $K ---"
  timeout 9000 taskset -c "$CORE" nice -n 19 ionice -c 3 "$LEARN" \
    --init "$INIT" --gens "$GENS" --games 2400 --threads 1 --depth 2 --epochs 3 \
    --gate-every "$K" --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "b2_${K}.net" --ledger "b2_${K}.jsonl" > "b2_${K}.log" 2>&1
  echo "  gens $(grep -cE '^gen ' "b2_${K}.log"), accepts $(grep -cE 'ACCEPT' "b2_${K}.log"), \
batch gates $(grep -c 'batch gate' "b2_${K}.log") ($(grep -c 'batch gate.*KEEP' "b2_${K}.log") KEEP)"
done

echo
g5=$(grep -cE '^gen ' b2_5.log 2>/dev/null || echo 0)
g1=$(grep -cE '^gen ' b2_1.log 2>/dev/null || echo 0)
if [ "$g5" != "$g1" ]; then
  echo "=== NO VERDICT: arms ran unequal generations ($g5 vs $g1). ==="
  echo "    That is the exact defect that made batch_ab.sh's 11-vs-5 result uninterpretable."
  echo "    Reporting the raw arms; a comparison here would be the same mistake twice."
else
  echo "=== VERDICT: each arm vs the FROZEN ORIGIN, $g5 generations each (448 pairs) ==="
fi
for K in 5 1; do
  printf "  gate-every %-3s %s\n" "$K" "$(grep -A 1 'CONTROL  final champion' "b2_${K}.log" 2>/dev/null | head -1 | sed 's/^.*net: //')"
done
echo "  baseline: champion_long scores 0.861 vs the frozen origin (RE-MEASURED today;"
echo "            batch_ab.sh's 0.864 was the stale figure)."
echo
echo "=== the batch arm's own KEEP / ROLL BACK lines, in full ==="
grep 'batch gate' b2_5.log 2>/dev/null | sed 's/^ */  /' \
  || echo "  NONE -- zero batch gates means the mode never fired: the arm is INERT, not a null."
