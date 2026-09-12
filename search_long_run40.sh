#!/usr/bin/env bash
# P2 -- THE PLANNED 40-GENERATION ARM AT 32 PROPOSALS. Report accepts.
#
# WHY THIS IS THE PLANNED ARM AND NOT A NEW ONE. search_long_run.sh's own sizing note says it:
# "The observer is ~16x the gate and is paid on every gate call ... 15 generations is sized to land
# in about the same wall clock the 40-GENERATION ARM would have used." So 40 generations WAS the
# design; VERIFY=96 was the substitution that forced it down to 15. Running 40 restores the arm
# rather than inventing one.
#
# THE OBSERVER IS OFF, AND THAT CHANGES NO DECISION. evolve.rs:3622 is explicit: EXISTENCE_GATE_VERIFY
# "is an observer, not a second gate ... The decision above is untouched ... Default 0 = off, so every
# existing run is byte-identical." It measures a DIFFERENT quantity -- the proportion of rejected
# candidates truly >= 0.5 -- which is not the accept count being asked for. With it on, 40 generations
# is ~42h at the measured 63 min/generation and cannot be delivered; with it off the acceptance logic
# is unchanged and the arm fits.
#
# IT WRITES TO ITS OWN OUT FILE. The 15-generation run's prop_verify96.log holds 7 generations of
# VERIFY readings that are the only data on that question. Reusing the name would destroy them.
#
# STOPPED BY GENERATION COUNT, NEVER BY THE CLOCK. An arm cut off by its timeout produces an unequal
# comparison -- the failure that invalidated the first low-lr sweep. The timeout here is a safety net
# sized well above the projection, not the stopping rule.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
SNAP=$SCR/prop_ab/evolve
GENS=${GENS:-40}
POP=${POP:-4}
N1=${N1:-10}
N2=${N2:-4}
PROPOSALS=${PROPOSALS:-32}
VERIFY=${VERIFY:-0}          # 0 = observer OFF, decisions byte-identical
SEED=${SEED:-7}
CAP=${CAP:-43200}            # 12h safety net; projection is ~3-5h with the observer off
LOG=search_long_run40.log
OUT=prop_gens40.log
say(){ echo "$(date +%F_%H:%M) [gens40] $*" | tee -a "$LOG"; }

[ -x "$SNAP" ] || { say "ABORT: no snapshot binary at $SNAP"; exit 1; }
say "START GENS=$GENS PROPOSALS=$PROPOSALS VERIFY=$VERIFY (observer off) POP=$POP seed=$SEED"
say "snapshot $(md5sum $SNAP | cut -c1-12)"

EXISTENCE_EVOLVE_SEED=$SEED EXISTENCE_PROPOSALS=$PROPOSALS EXISTENCE_GATE_VERIFY=$VERIFY \
  nice -n 19 taskset -c 6-11 timeout "$CAP" "$SNAP" "$GENS" "$POP" "$N1" "$N2" > "$OUT" 2>&1 || true

g=$(grep -cE '^ *gen +[0-9]+ +MAIN' "$OUT" 2>/dev/null || true)
a=$(grep -cF 'gate ACCEPT' "$OUT" 2>/dev/null || true)
r=$(grep -cF 'gate REJECT' "$OUT" 2>/dev/null || true)
say "END: MAIN gate calls=${g:-0}  ACCEPT=${a:-0}  REJECT=${r:-0}  (target $GENS generations)"
say "STATE, not a result. A _RESULT.md requires the planned N complete."
