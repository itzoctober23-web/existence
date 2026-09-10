#!/usr/bin/env bash
# WHAT IS A SPEEDUP ACTUALLY WORTH? Elo as a function of CLOCK, measured rather than modelled.
#
# `speed_cannot_pay_RESULT.md` sized int16 quantization at ~5 Elo. That number is a MODEL, not a
# measurement: it assumes Elo is linear in log(nodes), takes 89 Elo/ply from `elo_per_ply_RESULT.md`
# and a branching factor of 9.17, and divides. Every step is defensible and the whole thing is still
# an extrapolation, which this repo has been bitten by before -- a microbenchmark is an UPPER BOUND
# and a measured 12% has previously become a wall-clock LOSS.
#
# This measures the quantity directly. Doubling the CLOCK is exactly equivalent to doubling the nps,
# because the engine converts a movetime into a node budget: a 2x faster engine at T ms searches the
# same tree as today's engine at 2T ms. So the Elo gap between adjacent points IS "Elo per doubling
# of nps", with no model in between.
#
# WHY THIS COULD NOT BE RUN BEFORE TODAY. The engine ignored `go` parameters and played a fixed
# depth, so every point on this curve would have returned the same number.
#
# PRE-REGISTERED READING:
#   * If the measured gap per doubling is ~28 Elo, the model in speed_cannot_pay_RESULT is sound and
#     the ~5 Elo sizing for quantization stands.
#   * If it is MUCH LARGER, then Elo is steeper in time than log-linear at this strength and eval
#     work is worth more than that file claims -- the sizing must be revised UP and quantization
#     becomes more attractive.
#   * If it is MUCH SMALLER or flat, extra search is not converting at this strength and neither
#     speed nor depth is the lever, which would contradict elo_per_ply_RESULT and mean one of the two
#     instruments is wrong.
#
# The opponent is held FIXED (SF-1320 @10k nodes) so only our clock varies. Points are powers of two
# so each adjacent pair is exactly one doubling.
set -uo pipefail
cd "$(dirname "$0")"

NET=${NET:-p1_champion.net}
GAMES=${GAMES:-60}
CORES=${CORES:-12-14}
TAG=${TAG:-c1}

[ -f "$NET" ] || { echo "no net at $NET"; exit 1; }
echo "=== Elo vs CLOCK on $NET, $GAMES games per point, opponent fixed at SF-1320 @10k ==="
for MT in 25 50 100 200; do
  nice -n 19 taskset -c "$CORES" python3 sf_ruler.py --net "$NET" \
    --movetime "$MT" --sf-elo 1320 --sf-nodes 10000 --games "$GAMES" \
    > "evt_${TAG}_${MT}ms.log" 2>&1
  printf "  %4sms  %s\n" "$MT" "$(grep -E 'Elo vs|BOUND' "evt_${TAG}_${MT}ms.log" | head -1)"
done
echo
echo "  Adjacent points are ONE DOUBLING apart. The gap between them is Elo per doubling of nps,"
echo "  which is the number speed_cannot_pay_RESULT.md currently MODELS at 27.8."
echo "ELOVSTIMEDONE"
