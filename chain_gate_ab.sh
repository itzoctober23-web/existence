#!/usr/bin/env bash
# Run gate_ab.sh the moment the replay sweep finishes. Both use core 15, so they must be
# sequential -- but a human-shaped gap between them is wasted box time, and an idle box is the
# failure mode that looks healthy.
#
# Waits on the sweep's own end marker rather than a pid, so this works whether the sweep is still
# running or already finished when this starts.
set -uo pipefail
cd "$(dirname "$0")"
for _ in $(seq 1 900); do
  grep -q "=== done ===" replay_ab2.log 2>/dev/null && break
  sleep 4
done
grep -q "=== done ===" replay_ab2.log 2>/dev/null || { echo "sweep never finished; not chaining"; exit 1; }
echo "sweep finished — starting the pre-registered gate-resolution A/B"
exec ./gate_ab.sh
