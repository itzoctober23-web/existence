#!/usr/bin/env bash
# Wait for the depth replication, then run the epochs A/B -- a fifth ceiling candidate that was
# only ever measured on the near-blind gate-rate metric. Waits by PID, never pgrep -f.
set -uo pipefail
cd "$(dirname "$0")"
PID=${1:?need the pid to wait on}
while [ -d "/proc/$PID" ]; do sleep 30; done
echo "$(date +%F_%H:%M) depth replication finished; starting epochs A/B v2" >> chain_after_depthrep.log
exec ./epochs_ab2.sh >> chain_epochs2.log 2>&1
