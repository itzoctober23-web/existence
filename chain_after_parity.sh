#!/usr/bin/env bash
set -uo pipefail
cd "$(dirname "$0")"
PID=${1:?need pid}
while [ -d "/proc/$PID" ]; do sleep 30; done
echo "$(date +%F_%H:%M) parity test done; plateau depth-4 arm" >> chain_plateau.log
exec ./plateau_depth.sh >> chain_plateau.log 2>&1
