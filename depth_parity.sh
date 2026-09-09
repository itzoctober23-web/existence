#!/usr/bin/env bash
# IS THE DEPTH LEVER ACTUALLY DEPTH, OR IS IT SEARCH PARITY? Adds a datagen-depth-4 arm.
#
# THE CONFOUND, measured not guessed. examples/distill_gap.rs reports the gap between the search
# root score and the static eval in the trainer's own frame -- which at blend 1 IS the training
# signal -- and it is not monotone in depth:
#
#   net                    d2       d3       d4
#   origin(random)     0.0092   0.0219   0.0129
#   bn_000             0.1538   0.3541   0.2181
#   bn_075 (20 gen)    0.1599   0.5286   0.1945
#   champion_long      0.1860   0.5851   0.2276
#
# d3 >> d4 > d2 on EVERY net, reproduced on a second position set and seed. That is the classic
# alpha-beta odd-even effect: at odd depth the side to move gets the last ply and takes material
# without reply, so the root is systematically inflated relative to a quiet static eval.
#
# THE DEPTH LEVER IS d2 vs d3 -- EVEN vs ODD. So the surviving +0.025 is confounded: the depth-3 arm
# trains toward targets carrying a parity inflation that has nothing to do with label quality. Every
# reading of that lever to date, including the replication running now, shares the confound.
#
# d2 vs d4 holds parity FIXED and doubles depth. If the effect is depth, it survives. If it was
# parity, it vanishes -- and datagen depth joins width, draws and horizon as closed, which would
# leave the ceiling investigation with NO surviving candidate and force the question elsewhere.
#
# SAME PROTOCOL AS depth_replicate.sh -- time-boxed SECS, --horizon-cap 45 on every arm -- so the
# new arm is directly comparable to the d2 and d3 arms already on disk rather than needing its own
# baseline. The horizon cap is not optional: without it, equal wall clock gives the arms different
# generation counts and horizon widens with generation, which is the confound that made the FIRST
# depth result unreadable (d2 reached horizon 465, d3 only 45).
#
# THE VERDICT IS HEAD-TO-HEAD. The frozen-origin metric saturates and has reversed signs twice
# (instrument_saturation_RESULT.md), and depth's claimed effect is smaller than the +0.015 that
# reversed, so origin rates are context here and nothing more.
#
# PRE-REGISTERED READING:
#   * d4 BEATS d2 => depth is real, parity is not the whole story, and deeper datagen is a lever.
#   * d4 TIES d2 while d3 BEAT d2 => the lever was PARITY. Depth closes, and the honest consequence
#     is that no ceiling candidate survives -- which is a result, not a failure.
#   * d4 LOSES to d2 => deeper datagen costs more than it returns at equal wall clock, which is a
#     statement about COMPUTE not about label quality, and must be reported that way.
set -uo pipefail
cd "$(dirname "$0")"
SECS=${SECS:-2400}
CORE=${CORE:-12}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt2/release/learn}
NM=${NM:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt4/release/examples/netmatch}

[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
[ -x "$NM" ]    || { echo "REFUSING: no netmatch at $NM, and the head-to-head IS the verdict"; exit 1; }

for SEED in ${SEEDS:-424242 987654}; do
  if grep -q 'CONTROL  final champion' "dr_${SEED}_d4.log" 2>/dev/null; then
    echo "--- seed $SEED depth 4: already complete, skipping ---"; continue
  fi
  echo "--- seed $SEED, datagen depth 4, horizon-capped at 45 (core $CORE) ---"
  timeout "$SECS" taskset -c "$CORE" nice -n 19 ionice -c 3 "$LEARN" \
    --rung 0 --gens 1000000 --games 2400 --threads 1 --depth 4 --epochs 3 \
    --horizon-cap 45 \
    --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "dr_${SEED}_d4.net" --ledger "dr_${SEED}_d4.jsonl" \
    > "dr_${SEED}_d4.log" 2>&1
  echo "  $(grep -cE '^gen ' "dr_${SEED}_d4.log") generations"
done

echo
echo "=== VERDICT (head-to-head, 448 pairs). d2 vs d4 is PARITY-MATCHED; d2 vs d3 is not. ==="
for SEED in ${SEEDS:-424242 987654}; do
  for opp in d3 d4; do
    if [ -f "dr_${SEED}_d2.net" ] && [ -f "dr_${SEED}_${opp}.net" ]; then
      printf "  seed %-9s d2 vs %s: " "$SEED" "$opp"
      taskset -c "$CORE" nice -n 19 "$NM" "dr_${SEED}_d2.net" "dr_${SEED}_${opp}.net" 448 2 777 \
        2>/dev/null | grep -E 'scores' | sed 's/^ *//'
    else
      echo "  seed $SEED d2 vs $opp: a net is missing, no match"
    fi
  done
done
echo
echo "  d2 BELOW 0.5 means the deeper arm is stronger."
echo "  d2-vs-d4 tying while d2-vs-d3 loses => the lever was PARITY, and depth is closed."
echo "  Note the generation counts above: at equal wall clock d4 gets far fewer generations, so a"
echo "  d4 loss is a statement about COMPUTE COST, not about whether deeper labels are better."
