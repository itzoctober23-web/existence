#!/usr/bin/env bash
# Start the epochs-2 replication when the blend A/B finishes.
#
# WAITS ON chain_blend.log, NOT blend_ab.log. The first version watched blend_ab.log, which is
# never created: chain_blend.sh runs `exec ./blend_ab.sh` and that output goes to chain_blend.log.
# So the waiter would have spun for ~2 hours and exited with "blend_ab never finished", silently
# skipping the replication that settles today's only positive signal. A chain that waits on a file
# nothing writes fails exactly like a chain that is not armed, and looks identical in `ps`.
set -uo pipefail
cd "$(dirname "$0")"
for _ in $(seq 1 1600); do
  grep -q "=== done ===" chain_blend.log 2>/dev/null && break
  sleep 5
done
grep -q "=== done ===" chain_blend.log 2>/dev/null || { echo "blend A/B never finished; not chaining"; exit 1; }
echo "blend A/B finished — starting the epochs-2 replication"
exec ./replicate_ep2.sh
