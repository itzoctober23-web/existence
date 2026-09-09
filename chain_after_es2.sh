#!/usr/bin/env bash
set -uo pipefail
cd "$(dirname "$0")"
PID=${1:?need pid}
while [ -d "/proc/$PID" ]; do sleep 20; done
echo "$(date +%F_%H:%M) es2_2 done; fixed-gate run" >> chain_fixedgate.log
exec ./fixed_gate.sh >> chain_fixedgate.log 2>&1
