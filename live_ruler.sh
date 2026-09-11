#!/usr/bin/env bash
# Measure EVERY running arm on the absolute ruler, round-robin. Three arms are now in flight
# (depth3-w16, depth5-w16, depth3-w64) and only one was being measured, so two of the three
# experiments had no way to be decided.
set -uo pipefail
cd "$(dirname "$0")"
while true; do
  for arm in deep1 deep5 w64; do
    net="${arm}.net"; log="${arm}.log"
    [ -s "$net" ] || continue
    G=$(grep -cE '^gen ' "$log" 2>/dev/null)
    [ "${G:-0}" -lt 20 ] && continue          # too few generations to be worth 120 games
    cp -f "$net" "/tmp/r_${arm}.net" 2>/dev/null || continue
    nice -n 19 python3 sf_ruler.py --net "/tmp/r_${arm}.net" \
      --depth 4 --sf-elo 1320 --sf-nodes 10000 --games 120 > "${arm}_ruler.log" 2>&1
    echo "$(date '+%H:%M') $arm gen $G $(grep -oE 'Elo vs this opponent: .*' "${arm}_ruler.log")"
    rm -f "/tmp/r_${arm}.net"
  done
  sleep 120
done
