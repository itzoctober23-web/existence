#!/usr/bin/env bash
# DOES EXCLUDING DRAWS CAP THE CEILING? --include-draws off vs on, scored against the FROZEN ORIGIN.
#
# THE MEASUREMENT THAT MOTIVATES IT, from 750 generations across 49 runs already on disk:
#     decisive-game rate   mean 0.360   min 0.013   max 0.561
#     train samples/gen    mean 35,754
# So ~64% of every self-play batch is DRAWN and thrown away by `.filter(|s| s.z != 0.0 && ...)`.
#
# Volume is not the complaint -- 35,754 samples per generation against a 12,528-weight net is
# ample. The DISTRIBUTION is: the value head only ever sees positions from games that ended
# decisively, so it is never taught what a drawn position looks like, while most positions are
# drawn. Excluding the majority class outright is unusual; AlphaZero-style loops train draws at
# target 0, and this target already supports it since z = 0 is well defined.
#
# WHY THIS IS SECOND IN RANK, NOT FIRST. ceiling_ANALYSIS.md nominates WIDTH: every run sits in a
# 0.79-0.86 band at ARCH rung 0, and flat-across-many-anchors is the capacity signature. That A/B
# is running. This is the next candidate down and it is cheap, so it is queued rather than skipped.
#
# THE COUNTER-ARGUMENT, stated up front because it may well win: the eval feeds alpha-beta, which
# needs a RANKING, not calibrated draw probabilities. Separating win-ish from loss-ish may be all
# the search requires, and draws may be exactly the uninformative middle the filter removes on
# purpose. The horizon half of that same filter is backed by a real measurement -- training on ALL
# decided positions moved sign accuracy 0.452 -> 0.441, while <=10 plies moved it to 0.543 -- and
# the draw half has simply never been separated from it.
#
# PRE-REGISTERED READING:
#   * CONFIRMED if the include-draws arm ends measurably ABOVE the exclude arm against the origin.
#     Then the filter is discarding the majority class and costing strength, and the horizon
#     schedule should be re-examined the same way (its evidence pre-dates the draw split).
#   * REFUTED if it ends at or below. Then the filter is doing its job, draws really are the
#     uninformative middle, and this lever is CLOSED -- which is worth the box time, because it is
#     currently the second-ranked explanation for a ceiling nothing else has moved.
#   * UNRESOLVED is likely and is NOT a null: the run-to-run band is ~0.07 wide, wider than a
#     single run's ci95. Treat it as "needs more seeds".
#
# Both arms: same seed, same generations, NOTHING gated (--gate-every 100 never fires), so the
# comparison isolates the training pool. The verdict is the control against the frozen origin --
# never the gate rate, because beating the net you were trained from is not strength and
# non-transitivity was demonstrated here directly.
set -uo pipefail
cd "$(dirname "$0")"
GENS=${GENS:-20}
SEED=${SEED:-20260907}
PAIRS=${PAIRS:-1000}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt2/release/learn}
CTRL=${CTRL:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt2/release/examples/control}

[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
[ -x "$CTRL" ]  || { echo "no control at $CTRL"; exit 1; }

echo "=== draws A/B: exclude (shipped) vs include, $GENS generations from scratch, nothing gated ==="
for ARM in exclude include; do
  EXTRA=""
  [ "$ARM" = "include" ] && EXTRA="--include-draws"
  echo "--- arm: $ARM ---"
  timeout 4200 taskset -c 15 nice -n 19 ionice -c 3 "$LEARN" \
    --rung 0 --gens "$GENS" --games 2400 --threads 1 --depth 2 --epochs 3 \
    --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 0 \
    $EXTRA --seed "$SEED" --out "dw_${ARM}.net" --ledger "dw_${ARM}.jsonl" > "dw_${ARM}.log" 2>&1
  echo "  generations $(grep -cE '^gen ' "dw_${ARM}.log"), pool at end: $(grep -oE 'pool +[0-9]+' "dw_${ARM}.log" | tail -1)"
done

echo
echo "=== VERDICT: each arm vs the FROZEN ORIGIN (the only number that counts) ==="
for ARM in exclude include; do
  if [ -f "dw_${ARM}.net" ]; then
    timeout 1800 taskset -c 15 nice -n 19 ionice -c 3 "$CTRL" \
      --champion "dw_${ARM}.net" --pairs "$PAIRS" > "dw_${ARM}_ctrl.log" 2>&1
    printf "  %-8s %s\n" "$ARM" "$(grep -E 'fixed depth 2' "dw_${ARM}_ctrl.log" | head -1)"
  else
    echo "  $ARM produced no net -- read dw_${ARM}.log"
  fi
done
echo
echo "  include ABOVE exclude => the filter discards the majority class and costs strength."
echo "  include AT OR BELOW  => the filter is right, draws are the uninformative middle, lever CLOSED."
echo "  unresolved => more seeds; the run-to-run band is ~0.07, wider than one run's ci95."
