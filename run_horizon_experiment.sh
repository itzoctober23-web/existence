#!/usr/bin/env bash
# HORIZON CAP, properly seeded. The default was changed to 40 on ONE run per arm, and the
# step-budget experiment has since measured run-to-run variance at 0.151 on the same control
# metric -- larger than most effects being tested. So that change needs the same treatment I
# demanded of --steps-per-gen.
#
# The within-run evidence was stronger than a single endpoint (uncapped showed EIGHT consecutive
# generations below 0.5 with four flagged `regression` in the ledger, which is a pattern rather
# than one noisy number), so this is a confirmation rather than a challenge. But it was still
# n=1 and I said so nowhere.
set -u
cd "$(dirname "$0")"
failed=0
for seed in 20260907 424242 987654; do
  for cap in 40 1000; do
    tag="h${cap}_s${seed}"
    echo "=== $tag ==="
    taskset -c 12-15 nice -n 15 ./target/release/learn \
      --gens 20 --games 6000 --threads 4 --depth 2 --epochs 3 \
      --gate-pairs 32 --arch-pairs 160 --rung 0 --arch-every 5 \
      --control-every 10 --horizon-cap "$cap" \
      --seed "$seed" --out "champion_${tag}.net" --ledger "ledger_${tag}.jsonl" \
      > "run_${tag}.log" 2>&1
    rc=$?; g=$(grep -c '^gen ' "run_${tag}.log")
    if [ "$rc" -ne 0 ] || [ "$g" -eq 0 ]; then
      echo "  *** ARM FAILED: $tag (rc=$rc, $g gens) ***"; tail -4 "run_${tag}.log" | sed 's/^/      /'
      failed=$((failed+1))
    else echo "  done: $g gens"; fi
  done
done
[ "$failed" -gt 0 ] && { echo "INCOMPLETE: $failed arm(s) failed"; exit 1; }
echo "ALL ARMS COMPLETE"
