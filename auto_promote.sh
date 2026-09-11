#!/usr/bin/env bash
# BANK IMPROVEMENT AUTOMATICALLY. Twice tonight the running net had already passed the champion and
# only got promoted because I happened to run a head-to-head by hand. Between those checks the gain
# was real but unbanked, and a crash would have lost it.
#
# THE RULER IS NOT USED HERE, deliberately. It carries +/-50 Elo at 120 games and produced a
# four-reading "decline" while the net was genuinely stronger (champion_deep_RESULT.md). Direction
# needs the PAIRED instrument, so promotion is decided by netmatch and nothing else.
#
# Acceptance is the project's own rule: rate - ci95 >= 0.5. Same bar the manual promotions cleared.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
NM=$SCR/xt_cap/release/examples/netmatch
PAIRS=${PAIRS:-224}
EVERY=${EVERY:-1800}
CORES=${CORES:-6-15}
[ -x "$NM" ] || { echo "no netmatch at $NM"; exit 1; }

while true; do
  sleep "$EVERY"
  [ -s deep1.net ] || continue
  G=$(grep -cE '^gen ' deep1.log 2>/dev/null)
  cp -f deep1.net /tmp/ap_cand.net || continue
  nice -n 19 taskset -c "$CORES" "$NM" /tmp/ap_cand.net p1_champion.net "$PAIRS" > ap_last.log 2>&1
  line=$(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+' ap_last.log | head -1)
  rate=$(echo "$line" | grep -oE '0\.[0-9]+' | head -1)
  ci=$(echo "$line"   | grep -oE '0\.[0-9]+' | tail -1)
  [ -z "$rate" ] && { echo "$(date '+%H:%M') gen $G: netmatch produced no rate -- NOT promoting"; continue; }
  pass=$(python3 -c "print(1 if ($rate - $ci) >= 0.5 else 0)" 2>/dev/null)
  if [ "$pass" = "1" ]; then
    cp -f p1_champion.net "p1_champion_prev_g${G}.net"
    cp -f /tmp/ap_cand.net p1_champion.net
    echo "$(date '+%H:%M') gen $G: PROMOTED  $rate +/- $ci  (bar $rate-$ci >= 0.5)"
  else
    echo "$(date '+%H:%M') gen $G: hold      $rate +/- $ci"
  fi
  rm -f /tmp/ap_cand.net
done
