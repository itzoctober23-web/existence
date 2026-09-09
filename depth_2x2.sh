#!/usr/bin/env bash
# DEPTH x PARITY, 2x2, AT EQUAL GENERATIONS. The mechanism question, separated from the budget one.
#
# TWO DIFFERENT QUESTIONS HAVE BEEN SHARING ONE EXPERIMENT:
#   (a) "Given a fixed budget, should the loop label many positions shallowly or fewer deeply?"
#       This is the equal-wall-clock framing, and it is ANSWERED. With --horizon-cap 45 removing the
#       confound and the verdict taken head-to-head instead of against the saturating frozen origin,
#       the depth-2 arm beats the depth-3 arm 0.612 +/- 0.018 judged at depth 2 and 0.580 +/- 0.023
#       judged at depth 3 -- same winner across the parity boundary, so it is a fact about the nets.
#       That REVERSES the original +0.025-for-depth-3 claim, exactly as depth_RESULT.md feared.
#   (b) "Are deeper labels better PER LABEL?" Equal wall clock cannot answer this, because it hands
#       the shallow arm more generations by construction: 48 vs 4, a 12x disparity in training
#       amount. At this tree's ~0.0114 per generation that extrapolates to 0.502 of advantage, which
#       is off the top of the 0.5-1.0 scale -- the linear figure has left its valid range entirely,
#       and the honest statement is that the comparison is DOMINATED by training amount.
#
# This script asks (b) by holding generations AND games-per-generation fixed and varying only depth.
#
# WHY 300 GAMES AND 4 GENERATIONS. Measured cost per 300 games on this binary: d1 1s, d2 3s, d3 31s,
# so d4 is ~310s. At the 2400 games the other scripts use, one d4 generation needs ~2480s of datagen
# alone -- more than the entire 2400s time box -- which is why depth_parity.sh as configured
# completes ZERO generations and writes no net. Its own header admits the shape of this: "~10 hours
# for a 20-generation d4 arm". Shrinking the games per generation is what makes depth 4 reachable at
# all, and it costs nothing to validity as long as every arm gets the same number.
#
# THE 2x2 IS THE POINT. STATE.md:326-350 establishes via distill_gap that crossing search parity
# moves the training target 2.6x while two extra plies inside a class move it 2.5%, and concludes the
# d2-vs-d3 "depth lever" was really even-vs-odd. That is measured on the TARGET distribution. This
# tests whether it survives into TRAINED STRENGTH:
#         odd class:  d1 -> d3     even class:  d2 -> d4
# Depth varies within each row; parity varies between them. One run, both contrasts.
#
# --horizon-cap 45 on every arm: horizon = 10 + (g-1)*5 and horizon is a real effect
# (+0.064 +/- 0.034). With equal generations the horizons would match anyway, so the cap is belt and
# braces -- but it also keeps these nets comparable to the dr_* arms, which are capped.
#
# VERDICT BY netmatch AT DEPTH 4, never the frozen-origin control, which SATURATES: it reversed the
# sign on blend 0.75-vs-1.00 and on capacity w16-vs-w64, both with intervals clear of 0.5
# (depth_replicate.sh:78-80). Depth 4 because netmatch.rs:23-29 is explicit that it is this
# project's strength standard and that judging at depth 2 silently answers a different question --
# which is what depth_replicate.sh and depth_parity.sh both do.
#
# PRE-REGISTERED:
#   * d3~d1 AND d4~d2, with the classes differing => PARITY IS THE WHOLE STORY. The depth campaign
#     closes, and the real finding is that odd-depth targets are systematically optimistic.
#   * d3>d1 AND d4>d2 => depth genuinely helps per label, and the equal-wall-clock loss is purely a
#     throughput problem worth attacking with the accumulator and the bytecode.
#   * The two rows DISAGREE => neither story is right and the effect is something else; report it as
#     unexplained rather than picking whichever row is more convenient.
#   * NOTHING SEPARATES at 448 pairs => say so, and quote the pair count netmatch says it needs.
set -uo pipefail
cd "$(dirname "$0")"
CORE=${CORE:-12}
GENS=${GENS:-4}
GAMES=${GAMES:-300}
PAIRS=${PAIRS:-448}
SEEDS=${SEEDS:-"424242 987654"}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt2/release/learn}
NM=${NM:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt4/release/examples/netmatch}

[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
[ -x "$NM" ]    || { echo "no netmatch at $NM"; exit 1; }

for SEED in $SEEDS; do
  for D in 1 2 3 4; do
    OUT="eg_${SEED}_d${D}"
    if [ -f "${OUT}.net" ]; then echo "--- $OUT exists, skipping ---"; continue; fi
    cls=$([ $((D % 2)) -eq 0 ] && echo EVEN || echo odd)
    echo "--- seed $SEED depth $D ($cls): $GENS gens x $GAMES games, horizon-capped 45 ---"
    # Timeout scales with the measured cost ladder so a deep arm is not cut off mid-generation
    # while a shallow one finishes in seconds. A truncated arm writes a net with FEWER generations
    # than its peers, which would silently reintroduce the exact training-amount confound this
    # script exists to remove -- so the generation count is verified after the loop, not assumed.
    TO=900; [ "$D" = 3 ] && TO=1800; [ "$D" = 4 ] && TO=7200
    timeout "$TO" taskset -c "$CORE" nice -n 19 ionice -c 3 "$LEARN" \
      --rung 0 --gens "$GENS" --games "$GAMES" --threads 1 --depth "$D" --epochs 3 \
      --horizon-cap 45 \
      --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 0 \
      --seed "$SEED" --out "${OUT}.net" --ledger "${OUT}.jsonl" > "${OUT}.log" 2>&1
    printf "  %s gens, dec/labels: %s\n" "$(grep -cE '^gen ' "${OUT}.log")" \
      "$(grep -oE 'train +[0-9]+|dec +[0-9]+/[0-9]+' "${OUT}.log" | tail -2 | tr '\n' ' ')"
  done

  # EQUAL TRAINING AMOUNT IS AN ASSERTION, NOT AN ASSUMPTION. If any arm was truncated the 2x2 is
  # confounded and the matches below would be read as a depth effect. Say so loudly instead.
  echo "  generations per arm (must be identical):"
  bad=0
  for D in 1 2 3 4; do
    g=$(grep -cE '^gen ' "eg_${SEED}_d${D}.log" 2>/dev/null)
    printf "    d%s: %s\n" "$D" "$g"
    [ "$g" != "$GENS" ] && bad=1
  done
  [ "$bad" = 1 ] && echo "    *** ARMS UNEQUAL -- the 2x2 below is CONFOUNDED with training amount ***"

  echo
  echo "=== seed $SEED: 2x2, judged at depth 4 (strength standard), $PAIRS pairs ==="
  for pair in "d1 d3 depth-within-ODD" "d2 d4 depth-within-EVEN" "d1 d2 parity-at-shallow" "d3 d4 parity-at-deep"; do
    set -- $pair
    A="eg_${SEED}_$1.net"; B="eg_${SEED}_$2.net"
    if [ -f "$A" ] && [ -f "$B" ]; then
      printf -- "--- %s vs %s (%s) ---\n" "$1" "$2" "$3"
      timeout 5400 taskset -c "$CORE" nice -n 19 "$NM" "$A" "$B" "$PAIRS" 4 777 2>&1 \
        | grep -E 'scores|=>|arms:' | sed 's/^/  /'
    else
      echo "--- $1 vs $2: a net is missing, no match ---"
    fi
  done
  echo
done
echo "  Read the two depth rows FIRST. If they agree, depth has an effect at equal generations."
echo "  If both are flat while the parity rows are not, the depth campaign was measuring parity."
