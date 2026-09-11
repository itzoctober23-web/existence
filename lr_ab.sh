#!/usr/bin/env bash
# IS THE CONSTANT LEARNING RATE WHAT HOLDS THE PLATEAU?
#
# `nontransitive_walk_RESULT.md` dates the plateau precisely, on a paired instrument:
#
#     gen 2,162 -> 4,818   0.541 +/- 0.040   +28.6 Elo
#     gen 4,818 -> 6,803   0.499 +/- 0.030    -0.7 Elo   <- nothing, over 1,985 generations
#     5-generation steps   0.4869 [0.4364, 0.5374]       <- indistinguishable from a coin flip
#
# A net that changes every generation and goes nowhere is what a CONSTANT step size produces: it
# keeps kicking the weights around a basin instead of settling into it. `trainer.rs` is plain SGD --
# `Trainer { lr, blend }`, no momentum, no decay, no schedule -- and `lr` was the literal 0.01,
# hardcoded, unreachable from the command line. It is the ONE generator knob never varied:
#
#   label depth       spent at 3          (datagen_depth, depth5_vs_depth3)
#   blend             flat 0.75-1.00      (blend_RESULT)
#   epochs            under-fitting refuted (epochs_ab_RESULT)
#   horizon           cap obsolete        (horizon_RESULT)
#   games/generation  4x changes nothing  (games_per_gen_RESULT)
#   LEARNING RATE     never tested
#
# BOTH ARMS RUN HERE, from a shared start, matched on GENERATIONS. The control is re-run rather than
# quoted from prod2's 0.499: that reading came from a different starting net, and matching on the
# axis that carries the effect is the lesson `resume_dip_RESULT.md` cost a whole A/B to learn.
#
# PRE-REGISTERED READING (verdict = netmatch of each arm against the shared start):
#   * lr 0.002 clears 0.5 while the control sits on it -> the constant step size IS the plateau, and
#     a decay schedule is the fix. That would be the first movement on this plateau in the project.
#   * both sit on 0.5 -> the learning rate is not the lever either, and the generator's suspects are
#     exhausted at this architecture. That is a real result: it says the ceiling is the MODEL or the
#     TARGET, not the optimisation.
#   * lr 0.002 falls BELOW the control -> the net is under-trained at the lower rate; the step size
#     is doing useful work and the plateau is elsewhere.
#   * control itself clears 0.5 -> prod2's 0.499 did not replicate and the plateau claim needs
#     re-examining before anything is concluded about lr.
#
# The last row is the one that must be checked FIRST. A treatment reading is meaningless if its own
# control disagrees with the measurement that motivated the experiment.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
LEARN=./target/release/learn          # repo build: carries --lr
NM=$SCR/xt_cap/release/examples/netmatch
GENS=${GENS:-2000}
PAIRS=${PAIRS:-224}
CAP=${CAP:-2400}
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
grep -q '"--lr"' crates/pipeline/src/main.rs || { echo "source lacks --lr"; exit 1; }

cp -f prod2.net lr_start.net
echo "$(date '+%H:%M') lr A/B from prod2 (gen $(grep -cE '^gen ' prod2.log)), start $(md5sum lr_start.net | cut -c1-12), $GENS gens each"

for arm in A B; do
  L=$([ "$arm" = A ] && echo 0.01 || echo 0.002)
  ( timeout "$CAP" taskset -c 6-11 nice -n 19 ionice -c 3 "$LEARN" \
      --init lr_start.net --gens "$GENS" --games 8 --threads 2 --depth 3 --epochs 3 \
      --lr "$L" --gate-every 1000000 --arch-every 0 --control-every 0 \
      --seed 20260911 --out "lr${arm}.net" --ledger "lr${arm}.jsonl" > "lr${arm}.log" 2>&1 ) &
done
wait

for arm in A B; do
  L=$([ "$arm" = A ] && echo 0.01 || echo 0.002)
  # PRECONDITION: the flag must have taken. The header prints it, so this is checkable rather than
  # assumed -- an env/flag set on a binary that ignores it is silent, which retracted a finding today.
  H=$(head -1 "lr${arm}.log" | grep -oE '^lr=[0-9.]+')
  [ "$H" = "lr=$L" ] || { echo "  ABORT arm $arm: header says '$H', expected 'lr=$L'"; exit 1; }
  echo "  arm $arm (lr $L): $(grep -cE '^gen ' lr${arm}.log) generations, header $H"
done

for arm in A B; do
  [ -s "lr${arm}.net" ] || { echo "  arm $arm produced no net"; continue; }
  nice -n 19 taskset -c 6-11 "$NM" "lr${arm}.net" lr_start.net "$PAIRS" > "lr${arm}_vs_start.log" 2>&1
  echo "  arm $arm vs shared start: $(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+ +\(interval \[0\.[0-9]+, 0\.[0-9]+\]\)' "lr${arm}_vs_start.log" | head -1)"
done
echo "  reference: prod2 at lr 0.01 read 0.499 +/- 0.030 over 1,985 generations"
echo "LRABDONE"
