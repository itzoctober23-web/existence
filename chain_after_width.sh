#!/usr/bin/env bash
# Wait for the width A/B to finish, then start the draws A/B. Keeps core 15 busy without ever
# running two heavy jobs on it at once -- the width arms and the draws arms both pin to core 15,
# so overlapping them would corrupt both by contention.
#
# Waits on the DRIVER by pid, not by pgrep -f: matching on a command-line pattern also matches this
# script's own shell, which has already cost an accidental self-kill today.
set -uo pipefail
cd "$(dirname "$0")"
WPID=${1:?need the width_ab.sh pid}
while [ -d "/proc/$WPID" ]; do sleep 30; done
echo "$(date +%F_%H:%M) width A/B finished; starting draws A/B" >> chain_after_width.log
exec ./draws_ab.sh >> chain_draws.log 2>&1
