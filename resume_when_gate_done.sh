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

# Returns the pid of an ASYMMETRIC timed gate, or nothing.
#
# ASYMMETRY IS THE TEST, NOT MOVETIME. Measured 2026-09-11: my Existence load destroyed the 240-game
# anchor (loss rate 7.5% -> 42.9%, trend p=0.0008) because that match is our engine against an
# EXTERNAL opponent at concurrency 1 -- no cancelling arm. But 18 of the 42 gate scripts are also
# movetime and are PAIRED A/Bs: `gate_iir.sh` runs `sprt.py "$ENG" "$ENG" 0.2 12`, the same binary on
# both sides at concurrency 12, where contention hits both arms and cancels. Waiting on those would
# idle Existence for hours for nothing, and an idle box is the failure this loop exists to avoid.
#
# sprt.py's argv is: [1] script  [2] NEW engine  [3] OLD engine. Identical paths => paired => safe.
gate_pid() {
  local p exe a1 a2 a3
  for p in $(ls /proc 2>/dev/null | grep -E '^[0-9]+$'); do
    exe=$(readlink "/proc/$p/exe" 2>/dev/null) || continue; exe=${exe% (deleted)}
    case "${exe##*/}" in python3*) ;; *) continue;; esac
    a1=$(tr '\0' '\n' < "/proc/$p/cmdline" 2>/dev/null | sed -n '2p')
    [ "${a1##*/}" = "sprt.py" ] || continue
    a2=$(tr '\0' '\n' < "/proc/$p/cmdline" 2>/dev/null | sed -n '3p')
    a3=$(tr '\0' '\n' < "/proc/$p/cmdline" 2>/dev/null | sed -n '4p')
    # Same binary on both sides is a paired A/B -- contention cancels, so it does not block us.
    [ -n "$a2" ] && [ "$a2" = "$a3" ] && continue
    echo "$p"; return
  done
}

echo "$(date +%F_%H:%M) waiting for the 4PC gate to finish before resuming Existence"
while :; do
  P=$(gate_pid)
  [ -z "$P" ] && break
  sleep 60
done
sleep 10

# Resume EXACTLY the PIDs recorded in paused_by_agent.pids.
#
# The first version resumed "every stopped `learn` process", which would have stranded five of the
# nine things actually paused: `auto_promote.sh` and `live_ruler.sh` (both loops that periodically
# spawn measurement jobs) plus a live `sf_ruler.py` with its engine and stockfish children. The
# `live_ruler` loop was the surprise -- I had paused the trainers, believed the box quiet, and
# `ops/box_quiet.sh` immediately caught stockfish at 97% of a core contaminating the very gate the
# pause existed to protect.
#
# Resuming by RECORDED PID rather than by name also means this cannot wake something it did not
# stop. A PID that has since exited is skipped; SIGCONT on an already-running process is harmless.
LIST=$(dirname "$0")/paused_by_agent.pids
[ -f "$LIST" ] || { echo "no $LIST -- nothing recorded to resume"; exit 0; }
n=0; gone=0
while read -r p; do
  case "$p" in ''|\#*) continue;; esac
  [ -d "/proc/$p" ] || { gone=$((gone+1)); continue; }
  kill -CONT "$p" 2>/dev/null && { n=$((n+1)); echo "  resumed pid $p ($(tr '\0' ' ' < /proc/$p/cmdline 2>/dev/null | cut -c1-50))"; }
done < "$LIST"
echo "$(date +%F_%H:%M) gate finished; resumed $n process(es), $gone had already exited"
