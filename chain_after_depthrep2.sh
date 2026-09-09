#!/usr/bin/env bash
set -uo pipefail
cd "$(dirname "$0")"
PID=${1:?need pid}
while [ -d "/proc/$PID" ]; do sleep 30; done
echo "$(date +%F_%H:%M) depth_replicate done; parity-clean d4 arm" >> chain_depthparity.log
exec ./depth_parity.sh >> chain_depthparity.log 2>&1
