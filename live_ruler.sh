#!/usr/bin/env bash
# Measure EVERY running arm on the absolute ruler, round-robin. Three arms are now in flight
# (depth3-w16, depth5-w16, depth3-w64) and only one was being measured, so two of the three
# experiments had no way to be decided.
set -uo pipefail
cd "$(dirname "$0")"
while true; do
  # Only arms whose trainer is STILL RUNNING. Measuring a stopped arm spends 120 games on a net
  # that cannot change, and with three arms in the list that was a third of the ruler's cycles.
  live=""
  for p in $(pgrep -f "release/learn" 2>/dev/null); do
    e=$(readlink /proc/$p/exe 2>/dev/null); e=${e% (deleted)}
    case "$e" in */release/learn)
      t=$(tr '\0' ' ' < /proc/$p/cmdline 2>/dev/null | grep -oE 'run-tag [a-z0-9]+' | awk '{print $2}')
      [ -n "$t" ] && live="$live $t";;
    esac
  done
  for arm in $live; do
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
