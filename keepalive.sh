#!/usr/bin/env bash
# KEEP EXISTENCE ALIVE. Restart production if it dies, and say so.
#
# WHY THIS EXISTS. 2026-09-11 04:20-05:51: every Existence process died and nothing noticed for
# ~90 minutes. Three failures stacked:
#   1. I SIGSTOPped the trainers to protect an asymmetric 4PC benchmark. `timeout` measures WALL
#      time and keeps running while a process is frozen, so the sweep arms were killed at their cap
#      having done no work since the freeze ([[sigstop-under-timeout-is-death]]).
#   2. The auto-resume watcher woke, found all nine recorded PIDs already dead, correctly reported
#      "resumed 0, 9 had already exited", and exited. It worked exactly as written. There was
#      nothing left to resume.
#   3. Nothing else was watching, so the box sat half-idle until the next tick looked.
#
# The lesson is not "write a better pause". It is that a long-running trainer needs a supervisor
# that is INDEPENDENT of whatever stopped it, because every specific recovery path assumes the
# thing it is recovering still exists.
#
# WHAT IT DOES NOT DO: it does not restart auto_promote or live_ruler (they are cheap and their
# absence costs banking, not progress), it does not touch 4PC, and it never starts a SECOND trainer
# — the check is "is a production trainer alive", and it exits the cycle if one is.
set -uo pipefail
cd "$(dirname "$0")"
LOG=keepalive.log
PERIOD=${PERIOD:-300}

# Count production trainers by EXE + the --out it was given. Never a cmdline substring: the pattern
# would match this script's own command line.
prod_alive() {
  local p exe c n=0
  for p in $(ls /proc 2>/dev/null | grep -E '^[0-9]+$'); do
    exe=$(readlink "/proc/$p/exe" 2>/dev/null) || continue; exe=${exe% (deleted)}
    case "${exe##*/}" in learn) ;; *) continue;; esac
    c=$(tr '\0' ' ' < "/proc/$p/cmdline" 2>/dev/null)
    case "$c" in *"--out prod"*) n=$((n+1));; esac
  done
  echo "$n"
}

echo "$(date +%F_%H:%M) keepalive started (period ${PERIOD}s)" >> "$LOG"
while :; do
  n=$(prod_alive)
  if [ "${n:-0}" -eq 0 ]; then
    # Name the run by the hour so successive restarts never collide on a log or a net.
    TAG="prodk$(date +%H%M)"
    echo "$(date +%F_%H:%M) NO PRODUCTION TRAINER -- relaunching as $TAG at lr 0.0005" >> "$LOG"
    setsid nohup env LR=0.0005 TAG="$TAG" ./p1_production.sh >> p1_production.out 2>&1 < /dev/null &
    sleep 30
    m=$(prod_alive)
    echo "$(date +%F_%H:%M) after relaunch: $m trainer(s) alive" >> "$LOG"
  fi
  sleep "$PERIOD"
done
