#!/usr/bin/env bash
# Keep the blend head-to-head matches on core 13 instead of sharing 15 with judge_depth.
#
# WHY A WATCHER RATHER THAN AN EDIT. `blend_h2h.sh` pins every netmatch it spawns to CORE=15, and it
# is RUNNING. Editing a running bash script is what destroyed 2811 gate pairs in this project: bash
# reads the file by byte offset as it executes, so an edit shifts the offsets under it. The script
# also cannot be restarted without discarding the match already in flight. A watcher changes the
# affinity of the CHILD after it spawns, which touches nothing the running shell reads.
#
# WHY IT MATTERS. Both netmatch jobs were on core 15, each running at roughly half speed. The blend
# ladder is the largest unresolved lever on the board (+0.112 +/- 0.033, bigger than draws, horizon
# or depth) and judge_depth's question is already answered at three of four depths, so the blend
# matches are the ones worth the dedicated core.
#
# SAFE FOR THE RESULT. netmatch plays a fixed pair count at a fixed depth from seeded openings, so
# its output is deterministic: moving it between cores changes how long it takes and nothing else.
# That is exactly why this is safe to do to a job mid-flight, and why it would NOT be safe to do to
# a time-boxed arm.
#
# Identifies its targets by PROCESS ANCESTRY, not by name: the grandparent pid is passed in, so no
# pattern is matched against any command line and this cannot select the wrong process or itself.
# Stops on its own when the blend script exits.
set -uo pipefail
PARENT=${1:?usage: repin_blend.sh <blend_h2h.sh pid> [core]}
CORE=${2:-13}
while [ -d "/proc/$PARENT" ]; do
    for d in /proc/[0-9]*; do
        e=$(readlink "$d/exe" 2>/dev/null) || continue
        case "$e" in *netmatch) ;; *) continue ;; esac
        p=${d#/proc/}
        pp=$(awk '/^PPid:/{print $2}' "/proc/$p/status" 2>/dev/null) || continue
        gp=$(awk '/^PPid:/{print $2}' "/proc/$pp/status" 2>/dev/null) || continue
        [ "$gp" = "$PARENT" ] || continue
        cur=$(taskset -cp "$p" 2>/dev/null | grep -oE '[0-9,-]+$')
        [ "$cur" = "$CORE" ] && continue
        taskset -cp "$CORE" "$p" >/dev/null 2>&1 \
            && echo "$(date +%H:%M:%S) moved netmatch $p from core $cur to $CORE"
    done
    sleep 20
done
echo "$(date +%H:%M:%S) blend script $PARENT exited; watcher done"
