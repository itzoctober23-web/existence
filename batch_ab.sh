#!/usr/bin/env bash
# IS THE PLATEAU A MEASUREMENT FAILURE OR A LEARNING FAILURE? --gate-every 5 vs 1.
#
# THE DIAGNOSIS THIS TESTS. The pair standard deviation is 0.2362 (MEASURED, 15,008 real
# pentanomial pairs), so an N-pair gate resolves no better than 1.96*0.2362/sqrt(N). That model is
# validated against this harness's OWN logged bars: it predicts +/-0.0732 at 40 pairs and an_0.log
# printed +/-0.073, 0.076, 0.069, 0.074.
#
# The signal the gate is asked to filter is far smaller than its floor:
#     pooled over 239 generations, 7 runs   rate 0.5024   needs 37,209 pairs to resolve
#     epochs-2 arm, replicated on 2 seeds   rate 0.5114   needs  1,649 pairs to resolve
# So a 40-pair gate is 6.4x too coarse to see the best per-generation edge ever measured here, and
# a 224-pair gate is still 2.7x too coarse. It rejects EVERYTHING -- an_0.log is 13 generations, 0
# accepts, every one "reject". When the same gate was made permissive instead (the surrogate
# fallback branch) it accepted noise and drove the champion 0.864 -> 0.826. Both failure modes,
# one cause: a filter whose resolution is coarser than its signal.
#
# --gate-every K trains K generations unconditionally, then gates the ACCUMULATED change against
# the net the batch started from, rolling back if it did not resolve upward. K=5 multiplies the
# signal by 5 while the floor stays put.
#
# PRE-REGISTERED READING, written before the run:
#   * MEASUREMENT FAILURE (the hopeful one) if the batch arm posts KEEPs and ends ABOVE 0.864
#     against the frozen origin. Then per-generation gains were real and merely unmeasurable one
#     at a time, and batching is the repair.
#   * LEARNING FAILURE (the likely one, and I say so in advance) if every batch ROLLS BACK. The
#     pooled 0.5024 says the typical generation has no edge at all; 5 x nothing is nothing. Then
#     the problem is the TRAINING SIGNAL, not the gate, and this closes the gate as a lever
#     instead of leaving it as a suspect. Given 0.5024 I expect this outcome.
#   * A batch that KEEPs but ends at or below 0.864 means the batch gate is being passed by drift
#     -- it would say the rollback rule is too weak, not that the loop is learning.
#
# THE ONLY NUMBER THAT COUNTS IS vs THE FROZEN ORIGIN. Beating the net you were trained from is
# not strength: non-transitivity was DEMONSTRATED here at depth 2 (ep_1 beat champion_long
# 0.545+/-0.018 while scoring 0.834 vs origin where champion_long scored 0.864). So --control-every
# is ON in both arms and the verdict is read off the control, never off the gate rate.
#
# Batch arm runs FIRST. It is the arm that could move the number; the control is a constant I
# already have from an_0.log, and queueing a negative control ahead of the Elo-bearing arm is a
# mistake I have already been called out for.
set -uo pipefail
cd "$(dirname "$0")"
SECS=${SECS:-1800}
SEED=20260907
INIT=${INIT:-champion_long.net}
# The scratch build, NOT ./target/release: the search track holds
# ./target/release/examples/evolve and a rebuild under a running job has already cost this project
# a gate at 323 pairs.
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xtarget/release/learn}

[ -f "$INIT" ] || { echo "no champion at $INIT"; exit 1; }
[ -x "$LEARN" ] || { echo "no learn binary at $LEARN"; exit 1; }
echo "=== batch-gate A/B: ${SECS}s per arm, gate-pairs 224, control vs frozen origin ON ==="
echo "=== started $(date +%F_%H:%M) ==="
for K in 5 1; do
  echo "--- arm: gate-every $K ---"
  timeout "$SECS" taskset -c 15 nice -n 19 ionice -c 3 "$LEARN" \
    --init "$INIT" --gens 1000000 --games 2400 --threads 1 --depth 2 --epochs 3 \
    --gate-every "$K" --gate-pairs 224 --arch-every 0 --control-every 6 \
    --seed "$SEED" --out "bg_${K}.net" --ledger "bg_${K}.jsonl" > "bg_${K}.log" 2>&1
  echo "  generation $(grep -cE '^gen ' "bg_${K}.log"), $(grep -cE 'ACCEPT' "bg_${K}.log") accepted, \
$(grep -cE 'batch gate' "bg_${K}.log") batch gates ($(grep -cE 'batch gate.*KEEP' "bg_${K}.log") KEEP)"
done

echo
echo "=== VERDICT: champion vs the FROZEN ORIGIN (the only number that counts) ==="
for K in 5 1; do
  echo "--- gate-every $K ---"
  grep -E "control|vs origin" "bg_${K}.log" 2>/dev/null | tail -6
  grep -E "batch gate" "bg_${K}.log" 2>/dev/null | tail -8
done
echo
echo "  Baseline to beat: champion_long scores 0.864 vs the frozen origin."
echo "  ABOVE 0.864 with KEEPs => measurement failure, batching is the repair."
echo "  Every batch ROLLING BACK => learning failure, the gate is not the lever. Expected."
echo "=== done ==="
