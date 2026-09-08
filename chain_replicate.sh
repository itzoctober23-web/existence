#!/usr/bin/env bash
set -uo pipefail
cd "$(dirname "$0")"
for _ in $(seq 1 1400); do
  grep -q "=== done ===" blend_ab.log 2>/dev/null && break
  sleep 5
done
grep -q "=== done ===" blend_ab.log 2>/dev/null || { echo "blend_ab never finished; not chaining"; exit 1; }
echo "blend A/B finished — starting the epochs-2 replication"
exec ./replicate_ep2.sh
