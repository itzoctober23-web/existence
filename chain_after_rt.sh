#!/usr/bin/env bash
set -uo pipefail
cd "$(dirname "$0")"
PID=${1:?need pid}
while [ -d "/proc/$PID" ]; do sleep 30; done
echo "$(date +%F_%H:%M) ratchet done; compounding run" >> chain_compound.log
exec ./compound.sh >> chain_compound.log 2>&1
