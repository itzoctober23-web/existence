#!/usr/bin/env bash
# RE-SCORE the replay sweep at honest power, and against the point all three arms STARTED from.
#
# TWO DEFECTS IN THE SWEEP'S OWN SCORING, both found by reading replay_ab2.sh rather than waiting
# for its output.
#
# 1. UNDERPOWERED. It scores each arm at PAIRS=64. Measured ci95 at 64 pairs is 0.038-0.060, so
#    two arms separate only if the gap exceeds ~1.41 x ci95 = 0.054-0.085. The pre-registration in
#    replay_ab_CAVEAT.md already says a gap under 0.054 is NOT RESOLVED -- which means the sweep as
#    launched could not detect its own hypothesis and would have returned a confident-looking null.
#    That is the SAME defect I found and fixed in gate_ab.sh (64 -> 600) before launching it; the
#    sweep was already running by then, and a running script must not be edited. So: re-score.
#    600 pairs puts ci95 near 0.020, resolving ~0.028.
#
# 2. NO BASELINE. Every arm resumes from champion_long.net, and the sweep scores only the arms --
#    so it can say which window is best but NOT whether any window improved on where they all
#    began. That is the more important question and it was not being asked.
#
#    The baseline cannot be taken from the header comment. That comment says "the resumed champion
#    measured ~0.83-0.85", but those readings predate today's control fix: examples/control had a
#    hardcoded depth-6/4000-node budget that searched 0.4% of the intended tree, and fixing it moved
#    a reading from 0.504 +/- 0.008 to 0.781 +/- 0.060. An inherited number measured with a since-
#    repaired instrument is not a baseline, it is a hypothesis. So champion_long.net is re-measured
#    HERE, with the same binary, seed and pair count as the arms.
#
# CORE 14, not 15. chain_gate_ab.sh execs gate_ab.sh on core 15 the moment the sweep ends; this
# runs beside it rather than behind it.
set -uo pipefail
cd "$(dirname "$0")"
PAIRS=${PAIRS:-600}
SEED=${SEED:-987654}
OUT=rescore_replay.log

# Wait for the sweep so the .net files are final. The arms' files exist mid-run (written on every
# accept), and scoring a half-finished arm would silently compare different amounts of training.
for _ in $(seq 1 900); do
  grep -q "=== done ===" replay_ab2.log 2>/dev/null && break
  sleep 4
done
grep -q "=== done ===" replay_ab2.log 2>/dev/null || { echo "sweep never finished; not re-scoring"; exit 1; }

{
  echo "=== replay sweep re-scored at $PAIRS pairs (sweep used 64) ==="
  echo "=== all four scored by the SAME fixed control binary, seed $SEED ==="
  echo
  # champion_long.net FIRST and labelled as the baseline, so the arms are read against it rather
  # than against the stale 0.83 in the sweep header.
  for N in champion_long rp_1 rp_8 rp_999; do
    [ -f "$N.net" ] || { printf "  %-14s MISSING\n" "$N"; continue; }
    if [ "$N" = champion_long ]; then printf "  %-14s " "BASELINE(start)"; else printf "  %-14s " "$N"; fi
    taskset -c 14 nice -n 19 ionice -c 3 ./target/release/examples/control \
      --champion "$N.net" --pairs "$PAIRS" --seed "$SEED" 2>/dev/null \
      | grep "fixed depth 2" | sed 's/^ *//'
  done
  echo
  echo "  READING, pre-registered before the numbers exist:"
  echo "   * vs BASELINE is the question that matters: an arm above it LEARNED, at or below it did not."
  echo "   * arm-vs-arm resolves only a gap > ~0.028 at $PAIRS pairs. Below that: NOT RESOLVED,"
  echo "     never 'no effect'. The loop also carries 0.151 run-to-run variance across full restarts,"
  echo "     though these arms share a start so that is an upper bound here, not the applicable one."
  echo "   * 1-vs-999 is the cleanest contrast; 8-vs-999 is the weakest, because those two arms are"
  echo "     the same run for their first 8 generations."
  echo "=== done ==="
} > "$OUT" 2>&1
