#!/usr/bin/env bash
set -uo pipefail
cd "$(dirname "$0")"
PID=${1:?need pid}
while [ -d "/proc/$PID" ]; do sleep 20; done
echo "$(date +%F_%H:%M) rt_k5 done; gate alignment experiment" >> chain_gatealign.log
exec ./gate_align.sh >> chain_gatealign.log 2>&1
