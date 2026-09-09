#!/usr/bin/env bash
# Wait for the blend A/B v2, then run the RATCHET TEST -- the payoff experiment for the day's
# diagnosis. All arms pin to core 15; overlapping corrupts the comparisons. Waits by PID.
set -uo pipefail
cd "$(dirname "$0")"
PID=${1:?need the pid to wait on}
while [ -d "/proc/$PID" ]; do sleep 30; done
echo "$(date +%F_%H:%M) blend A/B v2 finished; starting ratchet test" >> chain_after_blend.log
exec ./ratchet_test.sh >> chain_ratchet.log 2>&1
