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
failed=0
# NO --cost-nodes: it is DERIVED per net from the measured tree at the cap depth. Passing 4000
# explicitly is what aborted 4 of 6 arms twice -- tree size is net-dependent, so a constant that
# covers 57% of one seed's depth-4 tree covers 33% of another's, and the coverage guard
# (correctly) refuses to run a gate that would return a meaningless 0.500. The derive fix landed
# in the binary while this caller kept overriding it.
cd "$(dirname "$0")"
for seed in 20260907 424242 987654; do
  for arm in ep3 steps; do
    if [ "$arm" = ep3 ]; then FLAGS="--epochs 3 --steps-per-gen 0"; else FLAGS="--epochs 3 --steps-per-gen 20000"; fi
    tag="${arm}_s${seed}"
    echo "=== $tag ==="
    taskset -c 12-15 nice -n 15 ./target/release/learn \
      --gens 20 --games 6000 --threads 4 --depth 2 $FLAGS \
      --gate-pairs 32 --arch-pairs 160 --rung 0 --arch-every 5 \
      --control-every 10 --horizon-cap 40 \
      --seed "$seed" --out "champion_${tag}.net" --ledger "ledger_${tag}.jsonl" \
      > "run_${tag}.log" 2>&1
    rc=$?
    g=$(grep -c '^gen ' "run_${tag}.log")
    # A FAILED ARM IS NOT A COMPLETED ARM. The first version printed "ALL ARMS COMPLETE"
    # while 4 of 6 arms had aborted at startup on the gate-coverage guard, because it never
    # looked at the exit code -- the same failure the 4PC queue runner already records
    # ("a failed EXPERIMENT is a result; a failed SCRIPT is a bug").
    if [ "$rc" -ne 0 ] || [ "$g" -eq 0 ]; then
      echo "  *** ARM FAILED: $tag (rc=$rc, $g gens) ***"
      tail -4 "run_${tag}.log" | sed 's/^/      /'
      failed=$((failed+1))
    else
      echo "  done: $g gens"
    fi
  done
done
if [ "$failed" -gt 0 ]; then echo "INCOMPLETE: $failed arm(s) failed"; exit 1; fi
echo "ALL ARMS COMPLETE"
