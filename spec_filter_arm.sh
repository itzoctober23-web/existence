#!/usr/bin/env bash
# EXISTENCE_SPEC_FILTER: run the fitness filter FITNESS 3 actually specifies, against the CLIMB the
# loop implements. Pre-registered before any result is seen.
#
# WHY THIS IS THE HIGHEST-VALUE UNTESTED LEVER. `evolve.rs:1758` admits a candidate to the game gate
# only on `popn[0].2 > best_rate` -- a STRICT surrogate improvement. FITNESS 3 asks only that a
# candidate not be much worse. `evolve.rs:1740-1750` measures what that costs against the reference
# rungs:
#     hash reuse            1.024x   spec PASS    strict PASS
#     table reduction       0.992x   spec PASS    strict REJECT
#     hash + ID             0.933x   spec PASS    strict REJECT
#     iterative deepening   0.914x   spec PASS    strict REJECT
#     capture extension     0.340x   spec reject  strict REJECT
# The spec admits FOUR rungs; the strict rule admits ONE. Three rungs die BEFORE a game is played, so
# no amount of gate power recovers them -- which is why this ranks ahead of raising `gate_pairs`.
#
# IT HAS NEVER BEEN RUN. Verified: absent from every .md, every .sh, every run log, and both running
# arms' environments. Only two commits ever touched it, and the one that added it (f27bd57) is titled
# "FITNESS 3 specifies a FILTER and the loop implements a CLIMB -- env-gated fix".
#
# SAME SEED AS THE OTHER TWO ARMS. `EXISTENCE_EVOLVE_SEED=1` matches the veto and control arms, so all
# three are comparable and the logs are byte-identical until the filter first changes a `pick`. That
# comparability was verified independently today: the veto and control arms' gen-1 lines are
# md5-identical despite being different builds.
#
# VERIFY=96 keeps the independent observer, which is the only instrument here that measures a
# candidate's true strength rather than the 6-pair gate's view of it.
#
# PRE-REGISTERED READING:
#   * MORE PROMOTIONS THAN THE CONTROL, and the VERIFY lines confirm the promoted candidates are not
#     worse => the strict surrogate filter was a real blocker, FITNESS 3's filter is the correct
#     reading, and P2's kill criterion is localised in the fitness exactly as evolve.rs argues.
#   * MORE PROMOTIONS BUT VERIFY SHOWS THEM WORSE => the filter admits junk and the strict rule was
#     doing real work. That is a genuine result and closes the lever.
#   * NO DIFFERENCE => the surrogate filter is not what blocks the ladder, and attention returns to
#     the gate's power for the ~53% of decisions that measure something.
# Report as "passed/failed the comparison". No Elo is measured here and none may be quoted.
set -uo pipefail
cd "$(dirname "$0")"
CORE=${CORE:?usage: CORE=<n> ./spec_filter_arm.sh}
SP=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
BIN=${BIN:-$SP/xt_sx/release/examples/evolve}
[ -x "$BIN" ] || { echo "ABORT: no evolve at $BIN"; exit 1; }

# The flag must BIND. `is_ok()` means ANY value counts, including empty, so the check is that the
# variable is present -- but an inert flag would make this an A/A against the control, which is the
# failure mode this project has shipped three times under other names.
grep -q 'EXISTENCE_SPEC_FILTER' crates/pipeline/examples/evolve.rs || {
  echo "ABORT: the flag is not in the source this binary was built from"; exit 1; }

echo "=== SPEC_FILTER arm, seed 1, core $CORE, 25 generations ==="
exec env EXISTENCE_EVOLVE_SEED=1 EXISTENCE_SPEC_FILTER=1 EXISTENCE_GATE_VERIFY=96 \
  taskset -c "$CORE" nice -n 19 ionice -c 3 "$BIN" 25 8 12 6 3
