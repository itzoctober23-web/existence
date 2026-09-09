#!/usr/bin/env bash
# Wait for the high-side blend sweep, then re-run the batch-gate A/B on the FIXED instrument.
#
# Waits on the pid of chain_after_epochs.sh, which ends in `exec ./blend_hi.sh` -- exec keeps the
# pid, so that one pid covers the epochs wait AND the whole blend_hi run.
#
# This is the highest-value item in the queue and it is LAST, which is an artefact of the chain
# being pid-linked and unmodifiable while it runs, not a judgement about priority. batch_ab2 tests
# whether the loop can accept a real step AT ALL; every arm ahead of it tests what to feed a loop
# that may not be able to swallow anything. If a slot frees earlier, run batch_ab2.sh first.
set -uo pipefail
cd "$(dirname "$0")"
PID=${1:?need the pid to wait on}
while [ -d "/proc/$PID" ]; do sleep 30; done
echo "$(date +%F_%H:%M) blend_hi finished; starting batch-gate A/B v2" >> chain_after_blendhi.log
exec ./batch_ab2.sh >> chain_batch2.log 2>&1
