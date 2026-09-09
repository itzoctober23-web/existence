#!/usr/bin/env bash
# Wait for the epochs A/B, then run the HIGH-SIDE blend sweep. Waits by PID, never pgrep -f.
#
# The PID this waits on is chain_after_depthrep.sh, which ends in `exec ./epochs_ab2.sh`. exec
# REPLACES the shell image while keeping the PID, so that one pid covers the depth-replication wait
# AND the entire epochs run, and exits only when epochs is done. Waiting on it is waiting on the
# whole remaining queue -- which is why this needs no knowledge of the epochs pid, a pid that does
# not exist yet and could not be waited on directly.
set -uo pipefail
cd "$(dirname "$0")"
PID=${1:?need the pid to wait on}
while [ -d "/proc/$PID" ]; do sleep 30; done
echo "$(date +%F_%H:%M) epochs A/B finished; starting high-side blend sweep" >> chain_after_epochs.log
exec ./blend_hi.sh >> chain_blendhi.log 2>&1
