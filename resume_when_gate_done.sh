#!/usr/bin/env bash
# RESUME THE PAUSED EXISTENCE ARMS ONCE THE 4PC GATE FINISHES.
#
# WHY THEY ARE PAUSED. 4PC gates on this box are MOVETIME matches (`SPRT_NODES` unset ->
# `sprt.py` takes the `game.engine_time` branch), so any other load on the machine changes their
# result. Measured 2026-09-11: my Existence work drove the 240-game anchor's loss rate from 7.5% to
# 42.9% across the run (trend p=0.0008), and a pre-registered SIGSTOP at game 223 pulled the last 17
# games back to 11.8% — a 273-Elo swing inside one run. `anchor_handicap.sh` is a TIME-handicap
# search, so it cannot be made load-immune with `SPRT_NODES` and genuinely needs a quiet box.
#
# WHY THIS SCRIPT EXISTS. A pause that depends on someone remembering to undo it is a bug. The arms
# are matched on GENERATIONS, so being stopped costs wall clock and not validity — but being stopped
# FOREVER costs the experiment. This makes the resume automatic.
#
# It does not block anything: it is detached, sleeps 60s between checks, and exits once it has
# resumed. Identify the gate by exe + EXACT argv[1] basename, never a cmdline substring, because the
# pattern would otherwise match this script's own command line.
set -uo pipefail

gate_pid() {
  local p exe a1
  for p in $(ls /proc 2>/dev/null | grep -E '^[0-9]+$'); do
    exe=$(readlink "/proc/$p/exe" 2>/dev/null) || continue; exe=${exe% (deleted)}
    case "${exe##*/}" in python3*) ;; *) continue;; esac
    a1=$(tr '\0' '\n' < "/proc/$p/cmdline" 2>/dev/null | sed -n '2p')
    [ "${a1##*/}" = "sprt.py" ] && { echo "$p"; return; }
  done
}

echo "$(date +%F_%H:%M) waiting for the 4PC gate to finish before resuming Existence"
while :; do
  P=$(gate_pid)
  [ -z "$P" ] && break
  sleep 60
done
sleep 10

# Resume every STOPPED learn process. State 'T' in /proc/PID/stat is stopped; resuming a process
# that is already running is harmless, so this is safe to run twice.
n=0
for p in $(ls /proc 2>/dev/null | grep -E '^[0-9]+$'); do
  exe=$(readlink "/proc/$p/exe" 2>/dev/null) || continue; exe=${exe% (deleted)}
  case "${exe##*/}" in learn) ;; *) continue;; esac
  st=$(awk '{print $3}' "/proc/$p/stat" 2>/dev/null)
  [ "$st" = "T" ] && { kill -CONT "$p" 2>/dev/null && n=$((n+1)); echo "  resumed pid $p"; }
done
echo "$(date +%F_%H:%M) gate finished; resumed $n stopped learn process(es)"
