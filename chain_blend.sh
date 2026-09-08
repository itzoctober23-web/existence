#!/usr/bin/env bash
# Start the blend A/B the moment the epochs 1-vs-2 arms finish. Both use core 15, so they must be
# sequential -- but a human-shaped gap between them is wasted box time.
set -uo pipefail
cd "$(dirname "$0")"
for _ in $(seq 1 1200); do
  grep -q "=== done ===" epochs_low.log 2>/dev/null && break
  sleep 5
done
grep -q "=== done ===" epochs_low.log 2>/dev/null || { echo "epochs_low never finished; not chaining"; exit 1; }
echo "epochs_low finished — starting the blend A/B"
exec ./blend_ab.sh
