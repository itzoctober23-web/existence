#!/usr/bin/env bash
# 3 seeds x {epochs 3, steps-per-gen 20000}. EVERY other flag identical, including --threads,
# because --threads repartitions the RNG stream across datagen workers and is therefore a
# variable, not a resource knob. That confound is what made the previous n=1 comparison
# uninterpretable (EXPERIMENTS.md 2026-09-08).
#
# Games reduced 10000 -> 6000: the 4PC NNUE trainer holds ~12GB and swap is at 7.9GB, so this
# keeps peak resident data near 90MB. Volume still 75x the pre-fix setting, which is what made
# the gate resolve.
set -u
cd "$(dirname "$0")"
for seed in 20260907 424242 987654; do
  for arm in ep3 steps; do
    if [ "$arm" = ep3 ]; then FLAGS="--epochs 3 --steps-per-gen 0"; else FLAGS="--epochs 3 --steps-per-gen 20000"; fi
    tag="${arm}_s${seed}"
    echo "=== $tag ==="
    taskset -c 12-15 nice -n 15 ./target/release/learn \
      --gens 20 --games 6000 --threads 4 --depth 2 $FLAGS \
      --gate-pairs 32 --arch-pairs 160 --rung 0 --arch-every 5 \
      --cost-nodes 4000 --control-every 10 --horizon-cap 40 \
      --seed "$seed" --out "champion_${tag}.net" --ledger "ledger_${tag}.jsonl" \
      > "run_${tag}.log" 2>&1
    echo "  done: $(grep -c '^gen ' "run_${tag}.log") gens"
  done
done
echo "ALL ARMS COMPLETE"
