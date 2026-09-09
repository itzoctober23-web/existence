#!/usr/bin/env bash
# Wait for the horizon A/B v2, then start the blend A/B v2. Every one of these arms pins to core 15
# and overlapping them corrupts the comparisons. Waits by PID, never pgrep -f (which matches this
# shell and has already caused one self-kill today).
set -uo pipefail
cd "$(dirname "$0")"
PID=${1:?need the pid to wait on}
while [ -d "/proc/$PID" ]; do sleep 30; done
echo "$(date +%F_%H:%M) horizon A/B v2 finished; starting blend A/B v2" >> chain_after_horizon.log
exec ./blend_ab2.sh >> chain_blend2.log 2>&1
