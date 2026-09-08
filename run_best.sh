#!/usr/bin/env bash
# Chain the LONG run onto the shape A/B: wait for it, read which arm actually won against the
# frozen origin, and launch that shape continuously. No idle gap, and no hand-picked constant.
#
# WHY THE WINNER IS NOT ASSUMED. Generation count is the wrong metric and this run proves it:
# 150 games/gen reached generation 140 and ended on the SAME decisive rate it started with
# (19/150 -> 19/150, 15 accepts in 140 generations), while 600 games/gen climbed 9.5% -> 28%
# decisive in 45 generations. The arm that "wins" by generations is the arm that learned
# nothing. So the shape is chosen by the origin-scored match, which is the only number here
# that measures strength rather than throughput.
set -uo pipefail
cd "$(dirname "$0")"

# Wait for the A/B to finish. Poll the log for its own end marker rather than a pid, so this
# works whether the A/B is still running or already done when this starts.
for _ in $(seq 1 400); do
  grep -q "=== done ===" shape_ab.log 2>/dev/null && break
  sleep 3
done
grep -q "=== done ===" shape_ab.log 2>/dev/null || { echo "A/B did not finish; not launching"; exit 1; }

# Parse "  <games> games/gen  fixed depth 2, uncapped   NW-ND-NL   rate 0.xxx +/- 0.yyy"
best_g=""; best_r=0
while read -r g r; do
  [ -n "$g" ] || continue
  if awk "BEGIN{exit !($r > $best_r)}"; then best_r=$r; best_g=$g; fi
done < <(grep -oE "^ +[0-9]+ games/gen.*rate [0-9.]+" shape_ab.log \
         | sed -E 's/^ +([0-9]+) games\/gen.*rate ([0-9.]+)/\1 \2/')

if [ -z "$best_g" ]; then
  echo "could not parse a winner from shape_ab.log — refusing to guess a shape"; exit 1
fi
echo "WINNER: $best_g games/gen at rate $best_r vs origin — launching the long run"

# Threads 3 on the E-cores only. He games on this box and noticed the spike when this loop
# had 3 threads competing with a rebuild; nice 19 + ionice idle + cores 12-15 keeps it off
# everything he touches.
exec taskset -c 12-15 nice -n 19 ionice -c 3 ./target/release/learn \
  --gens 1000000 --games "$best_g" --threads 3 --depth 2 --epochs 3 \
  --gate-pairs 32 --arch-pairs 160 --rung 0 --arch-every 25 --control-every 25 \
  --seed 20260907 --out champion_long.net --ledger ledger_long.jsonl
