#!/usr/bin/env bash
set -uo pipefail
cd "$(dirname "$0")"
PID=${1:?need pid}
while [ -d "/proc/$PID" ]; do sleep 30; done
echo "$(date +%F_%H:%M) blend_seed2 done; ship candidate" >> chain_ship.log
exec ./ship_candidate.sh >> chain_ship.log 2>&1
