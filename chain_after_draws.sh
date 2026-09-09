#!/usr/bin/env bash
# Wait for the draws A/B, then start the depth A/B. Both pin to core 15, so overlapping them would
# corrupt both -- the depth A/B's arms are compared at EQUAL WALL CLOCK, and a contended clock is
# not a clock. Waits on the driver by PID, never by pgrep -f, which also matches this shell.
set -uo pipefail
cd "$(dirname "$0")"
DPID=${1:?need the draws_ab.sh pid}
while [ -d "/proc/$DPID" ]; do sleep 30; done
echo "$(date +%F_%H:%M) draws A/B finished; starting depth A/B" >> chain_after_draws.log
exec ./depth_ab.sh >> chain_depth.log 2>&1
