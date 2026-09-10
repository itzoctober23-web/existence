#!/usr/bin/env bash
# Measure every new rung of the deep run on the ABSOLUTE ruler, so the watch page's trend line
# fills in on its own. One rung at a time, nice 19, one core -- the production run is the priority
# and this must never contend with it.
#
# It is the ruler that decides this experiment, not the gate: the deep run is ungated by design
# (--gate-every 100), exactly as the arm that reached 1212 in six generations was.
set -uo pipefail
cd "$(dirname "$0")"
GAMES=${GAMES:-120}
CORE=${CORE:-14}
while true; do
  for net in $(ls -1 deep1.net.deep1.gen*.net 2>/dev/null | sort -t n -k4 -n); do
    stem="${net%.net}"
    log="${stem}_ruler.log"
    # Skip anything already measured. The ruler is ~3 minutes; re-measuring a rung would spend
    # that on a number already known and delay the rung that is not.
    [ -s "$log" ] && continue
    echo "$(date '+%H:%M:%S') measuring $net"
    nice -n 19 taskset -c "$CORE" python3 sf_ruler.py --net "$net" \
      --depth 4 --sf-elo 1320 --sf-nodes 10000 --games "$GAMES" > "$log" 2>&1
    grep -E 'Elo vs|BOUND' "$log" | sed "s|^|  $net |"
  done
  sleep 120
done
