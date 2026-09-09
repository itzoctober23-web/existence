#!/usr/bin/env bash
# Wait for the depth A/B, then start the horizon A/B v2 -- the LAST of the four ceiling candidates.
# All these arms pin to core 15; overlapping them corrupts the comparisons. Waits by PID, never by
# pgrep -f, which also matches this shell.
set -uo pipefail
cd "$(dirname "$0")"
PID=${1:?need the pid to wait on}
while [ -d "/proc/$PID" ]; do sleep 30; done
echo "$(date +%F_%H:%M) depth A/B finished; starting horizon A/B v2" >> chain_after_depth.log
exec ./horizon_ab2.sh >> chain_horizon2.log 2>&1
