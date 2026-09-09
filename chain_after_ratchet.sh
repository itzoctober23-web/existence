#!/usr/bin/env bash
# Wait for the ratchet test, then replicate the depth result on two more seeds -- the only ceiling
# candidate that pointed UP, and the one whose own pre-registration demands more seeds. Waits by
# PID, never pgrep -f.
set -uo pipefail
cd "$(dirname "$0")"
PID=${1:?need the pid to wait on}
while [ -d "/proc/$PID" ]; do sleep 30; done
echo "$(date +%F_%H:%M) ratchet finished; starting depth replication" >> chain_after_ratchet.log
exec ./depth_replicate.sh >> chain_depthrep.log 2>&1
