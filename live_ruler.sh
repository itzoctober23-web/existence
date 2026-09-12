#!/usr/bin/env bash
# Measure EVERY running arm on the absolute ruler, round-robin. Three arms are now in flight
# (depth3-w16, depth5-w16, depth3-w64) and only one was being measured, so two of the three
# experiments had no way to be decided.
set -uo pipefail
cd "$(dirname "$0")"
while true; do
  # Only arms whose trainer is STILL RUNNING. Measuring a stopped arm spends 120 games on a net
  # that cannot change, and with three arms in the list that was a third of the ruler's cycles.
  # Arms are identified by the running trainer's --out, not --run-tag: the A/B arms were launched
  # without a run-tag, so a tag-based list silently found nothing and the ruler measured NOTHING
  # while reporting healthy. The --out net is what a trainer always has.
  live=""
  for p in $(ls /proc 2>/dev/null | grep -E '^[0-9]+$'); do
    e=$(readlink /proc/$p/exe 2>/dev/null); e=${e% (deleted)}
    case "$e" in */release/learn)
      o=$(tr '\0' ' ' < /proc/$p/cmdline 2>/dev/null | grep -oE '[-][-]out [^ ]+' | awk '{print $2}')
      [ -n "$o" ] && live="$live ${o%.net}";;
    esac
  done
  for arm in $live; do
    net="${arm}.net"; log="${arm}.log"
    [ -s "$net" ] || continue
    G=$(grep -cE '^gen ' "$log" 2>/dev/null)
    [ "${G:-0}" -lt 20 ] && continue          # too few generations to be worth 120 games
    cp -f "$net" "/tmp/r_${arm}.net" 2>/dev/null || continue
    # PINNED, not just niced. Without taskset this whole tree -- sf_ruler.py plus its `engine`
    # and `stockfish` children -- inherits an all-cores mask and lands on 12-15, which are HIS.
    # Measured 2026-09-12 02:46: cpu12 67.2%, cpu13 73.7%, cpu14 70.4%, cpu15 100.0%, and the
    # only unpinned CPU of mine was this ruler. Live-tasksetting the running tree dropped them to
    # 44/35/42/49%. Priority does not fix core ownership; affinity does.
    nice -n 19 taskset -c "${RULER_CORES:-6-11}" python3 sf_ruler.py --net "/tmp/r_${arm}.net" \
      --depth 4 --sf-elo 1320 --sf-nodes 10000 --games 120 > "${arm}_ruler.log" 2>&1
    echo "$(date '+%H:%M') $arm gen $G $(grep -oE 'Elo vs this opponent: .*' "${arm}_ruler.log")"
    rm -f "/tmp/r_${arm}.net"
  done
  sleep 120
done
