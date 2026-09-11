#!/usr/bin/env bash
set -uo pipefail
cd "$(dirname "$0")"
while true; do
  G=$(grep -cE '^gen ' deep1.log 2>/dev/null)
  cp -f deep1.net /tmp/live_g${G}.net 2>/dev/null || { sleep 120; continue; }
  nice -n 19 taskset -c 6-11 python3 sf_ruler.py --net /tmp/live_g${G}.net \
    --depth 4 --sf-elo 1320 --sf-nodes 10000 --games 120 > live_g${G}_ruler.log 2>&1
  echo "$(date '+%H:%M') gen $G $(grep -oE 'Elo vs this opponent: .*' live_g${G}_ruler.log)"
  rm -f /tmp/live_g${G}.net
  sleep 600
done
