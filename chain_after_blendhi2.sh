#!/usr/bin/env bash
# Wait for blend_hi (0.85 arm), then run the SECOND TRAINING SEED of the high side -- the only
# thing between the head-to-head reversal and moving a shipped default. Waits by PID, never pgrep.
set -uo pipefail
cd "$(dirname "$0")"
PID=${1:?need pid}
while [ -d "/proc/$PID" ]; do sleep 30; done
echo "$(date +%F_%H:%M) blend_hi done; starting blend second-seed" >> chain_after_blendhi2.log
exec ./blend_seed2.sh >> chain_blendseed2.log 2>&1
