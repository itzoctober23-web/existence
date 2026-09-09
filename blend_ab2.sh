#!/usr/bin/env bash
# WAS THE BLEND CHOSEN ON A BROKEN METRIC? blend 0.75 (shipped) vs 0.25 vs 0.00, scored vs ORIGIN.
#
# WHY RE-ASK A SETTLED QUESTION. `blend_ab.sh` ran today and its verdict was MEAN GATE RATE --
# candidate against its own champion. That metric has since been measured as near-blind here: two
# similar nets at depth 2 draw almost everything, so the gate returns 0.500 +/- 0.007 on a pair a
# fixed anchor separates easily, and non-transitivity was demonstrated directly (a net can beat its
# parent while being weaker against a third opponent). Every conclusion that rests on it is
# unsupported, and the shipped blend of 0.75 is one of them.
#
# WHAT POINTS AT THE TARGET SPECIFICALLY. Twice today, from unrelated experiments, LOWER TRAINING
# LOSS CAME WITH WORSE PLAY:
#     width A/B  w64 final loss 0.0262 vs w16 0.0397, and w64 LOSES 0.179 +/- 0.021 at equal time
#     draws A/B  include loss 0.0191 vs exclude 0.0726, and include scores 0.774 vs 0.828
# A model fitting its target better and playing worse is a statement about the TARGET. `blend` is
# the knob that defines it: target = (1 - blend) * z + blend * root, so at the shipped 0.75
# three-quarters of what the net is trained to reproduce is the CHAMPION'S OWN SEARCH SCORE.
# main.rs:272 says so outright -- "Mixing the net's OWN root score into its target is
# self-referential and teaches nothing. The blend only earns its place once the search score is
# better than [the net]."
#
# PRE-REGISTERED READING:
#   * CONFIRMED (blend is the brake) if a LOWER blend ends measurably above 0.75 against the origin.
#     Then the target is too self-referential, the earlier A/B picked 0.75 on a blind metric, and
#     the blend should track measured search quality instead of sitting fixed.
#   * REFUTED if 0.75 holds or wins on this metric too. Then the bootstrap value of the root score
#     outweighs its self-reference even while the champion is weak, the original choice was RIGHT
#     FOR THE WRONG REASON, and that is worth recording as such rather than as a vindication.
#   * A LOW BLEND SCORING BADLY IS INFORMATIVE, not a failure: blend 0.00 trains on the raw game
#     result alone, and if that is worse then the outcome label really is too noisy at this
#     strength to learn from unaided -- which is a fact about the DATA, and the ceiling's remaining
#     suspect.
#   * UNRESOLVED is likely; the run-to-run band is ~0.07, wider than one run's ci95. It means
#     "more seeds", not "no effect".
#
# Verdict is the built-in final control against the FROZEN ORIGIN -- the metric that resolved
# capacity and the draw filter -- never the gate rate that made this question need asking twice.
set -uo pipefail
cd "$(dirname "$0")"
GENS=${GENS:-20}
SEED=${SEED:-20260907}
ARMS=${ARMS:-"0.75 0.25 0.00"}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt2/release/learn}

[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }

echo "=== blend A/B v2: $ARMS, $GENS generations from scratch, nothing gated ==="
echo "=== verdict = built-in final control vs the FROZEN ORIGIN, not the gate rate ==="
for B in $ARMS; do
  tag=${B/./}
  echo "--- arm: blend $B ---"
  timeout 4200 taskset -c 15 nice -n 19 ionice -c 3 "$LEARN" \
    --rung 0 --gens "$GENS" --games 2400 --threads 1 --depth 2 --epochs 3 \
    --blend "$B" --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "bn_${tag}.net" --ledger "bn_${tag}.jsonl" > "bn_${tag}.log" 2>&1
  echo "  generations $(grep -cE '^gen ' "bn_${tag}.log"), final loss $(grep -oE 'loss [0-9.]+' "bn_${tag}.log" | tail -1)"
done

echo
echo "=== VERDICT: each arm vs its FROZEN ORIGIN (448 pairs, built-in control) ==="
for B in $ARMS; do
  tag=${B/./}
  printf "  blend %-5s %s\n" "$B" "$(grep -A 1 'CONTROL  final champion' "bn_${tag}.log" 2>/dev/null | head -1 | sed 's/^.*net: //')"
done
echo
echo "  A lower blend winning => the target is too self-referential and 0.75 was picked blind."
echo "  0.75 winning here too => the original choice was right FOR THE WRONG REASON. Say so."
echo "  blend 0.00 losing badly => the raw game result is too noisy to learn from unaided,"
echo "                             which is a fact about the DATA, the ceiling's remaining suspect."
