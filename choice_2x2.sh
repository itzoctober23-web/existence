#!/usr/bin/env bash
# WHICH LEVER GIVES SELECTION A CHOICE: MORE CANDIDATES, OR A FITNESS WITH A GRADIENT?
#
# `search_has_no_choice_RESULT.md` measures the binding constraint: 94% of generations hand
# selection zero or one DISTINCT fitness, and selection cannot select from a set of size <= 1.
# That is why P2 has 1,062 proposals and 0 accepts. Two different things can cause it, and they
# need separating rather than guessing between:
#
#   PROPOSALS   too few candidates reach scoring at all. Candidates were proposed as `(0..pop)`,
#               so the proposal count WAS the population size, and pop collapses to 2 -- 2
#               proposals at the guard's 10.5% survival is 0.21 expected survivors, which cannot
#               climb back out. EXISTENCE_PROPOSALS separates the two quantities.
#
#   GRADIENT    candidates reach scoring but land on the SAME value. On the shipped guard set the
#               seed already scores 25/25, so it is a pass/fail filter, and surviving candidates
#               differ only in COST. The HARD set is 8 positions the seed fails BY CONSTRUCTION --
#               it scores 0/8 -- so `f` itself can vary. EXISTENCE_HARD_FITNESS turns it on.
#
# THESE ARE NOT THE SAME FIX, and the 2x2 is the point: more candidates landing on one identical
# rate is still no choice. Only the `distinct` column can tell them apart, which is why it is the
# primary measure and `mate-ok` is secondary.
#
# WHY HARD_FITNESS IS PLAUSIBLY THE STRONGER LEVER. The quantity that is short is DISTINCT values.
# Raising proposals multiplies attempts at a fitness that may still be flat; the HARD set changes
# the fitness itself from saturated to graded. Stated as the hypothesis this tests, not as a
# result.
#
# IT ALSO CARRIES A REPAIRED BUG, which is why it is worth re-testing rather than trusting its
# reputation: evolve.rs:2748 records that the incumbent was once scored `f/cst` while candidates
# were scored `(f+hf)/(cst+hcst)`. Since the seed scores hf = 0, the new formula only enlarged its
# denominator, so EVERY candidate scored below an incumbent measured a different way -- "rates
# 0.894-0.927x ... pop 1". The flag looked strictly harmful; it had only made the comparison
# invalid. That is fixed. A flag that failed for a harness reason deserves one clean measurement.
#
# WAITS for proposals-ab so the box is not oversubscribed, and re-uses its control/prop32 arms
# rather than re-running them.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
SNAP=$SCR/prop_ab/evolve
GENS=${GENS:-6}
POP=${POP:-4}
N1=${N1:-10}
N2=${N2:-4}
SEED=${SEED:-1}
LOG=choice_2x2.log
say(){ echo "$(date +%F_%H:%M) [2x2] $*" | tee -a "$LOG"; }

say "waiting for proposals-ab to finish (its control and prop32 arms are two cells of this 2x2)"
while systemctl --user is-active proposals-ab.service >/dev/null 2>&1; do sleep 30; done
sleep 10

[ -x "$SNAP" ] || { say "ABORT: no evolve snapshot at $SNAP -- proposals_ab.sh builds it"; exit 1; }
say "using snapshot $(md5sum "$SNAP" | cut -c1-12) -- the SAME binary the first two arms ran"

run_arm(){ # $1=label  $2=proposals(empty=default)  $3=hard(1=on)
  local lab=$1 prop=$2 hard=$3
  local out="prop_${lab}.log"
  say "arm $lab: PROPOSALS=${prop:-<unset>} HARD_FITNESS=${hard:-off}"
  local -a env=(EXISTENCE_EVOLVE_SEED="$SEED")
  [ -n "$prop" ] && env+=(EXISTENCE_PROPOSALS="$prop")
  [ "$hard" = "1" ] && env+=(EXISTENCE_HARD_FITNESS=1)
  env "${env[@]}" nice -n 19 taskset -c 6-11 timeout 2400 \
    "$SNAP" "$GENS" "$POP" "$N1" "$N2" > "$out" 2>&1 || true
  say "  $(grep -cE '^[[:space:]]*gen ' "$out" 2>/dev/null) generation lines"
}

run_arm hard   ""   1
run_arm hardp32 32  1

say "2x2 RESULT — primary column is 'gens>1 distinct', not mate-ok:"
./choice_report.py 2>&1 | tee -a "$LOG"
say "CHOICE2X2DONE"
