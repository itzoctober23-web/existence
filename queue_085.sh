#!/usr/bin/env bash
# Wait for depth_2x2 to release core 12, then run the 0.85 second-seed replication.
# Waits on a PID passed in, never a name pattern: a waiter that greps for its own target matches
# its own command line and blocks forever. And never on $! from a setsid launch, which is the
# wrapper -- it exits immediately and the wait fires at once, stacking jobs onto a live one.
set -uo pipefail
cd "$(dirname "$0")"
W=3764540
if [ "$W" != 0 ]; then
  echo "waiting for pid $W (depth_2x2) to release core 12"
  while [ -d "/proc/$W" ]; do sleep 30; done
  echo "core 12 free at $(date +%H:%M)"
fi
exec ./blend_085_seed2.sh
