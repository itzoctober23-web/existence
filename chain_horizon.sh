#!/usr/bin/env bash
# Runs after the anchor A/B. Waits on chain_anchor.log, which is where anchor_ab.sh's output goes
# (chain_anchor.sh execs it) -- not anchor_ab.log, which nothing writes. Third link in this chain
# and the one place a waiter has already silently failed today.
set -uo pipefail
cd "$(dirname "$0")"
for _ in $(seq 1 2600); do
  grep -q "=== done ===" chain_anchor.log 2>/dev/null && break
  sleep 5
done
grep -q "=== done ===" chain_anchor.log 2>/dev/null || { echo "anchor A/B never finished; not chaining"; exit 1; }
echo "anchor A/B finished — starting the horizon A/B"
exec ./horizon_ab.sh
