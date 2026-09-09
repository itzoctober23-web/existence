#!/usr/bin/env bash
# BLEND HIGH SIDE, head-to-head at depth 4. The largest lever on the board, on the right instrument.
#
# WHY THIS MATTERS MOST. `target = (1 - blend) * z + blend * root`. STATE.md measures blend as the
# biggest ceiling effect by a distance: **+0.112 ± 0.033** for 0.75 over 0.25, against draws +0.086,
# horizon +0.064 and depth +0.025. The shipped value is 0.75 and no improvement is available BELOW
# it; the whole remaining question is whether going ABOVE it helps, and that has never been settled.
#
# WHY THE PREVIOUS ATTEMPT DID NOT SETTLE IT. `blend_hi.sh` ran 1.00, 0.85 and a re-run 0.75 control.
# The two treatment arms completed 20 generations; **the control stopped at generation 9**, so there
# was nothing to compare them against and no verdict was ever recorded. Its numbers also came from
# the frozen-origin control, which this tree has since documented as SATURATING -- it reversed the
# sign on blend 0.75-vs-1.00 and on capacity w16-vs-w64, both with intervals clear of 0.5.
#
# WHAT MAKES IT ANSWERABLE NOW, AT ZERO TRAINING COST. `bn_075.net` is a completed 20-generation
# 0.75 arm from the original blend experiment, and it is comparable: identical settings line, same
# seed 20260907, and -- verified before writing this -- its first 9 generation lines are BYTE
# IDENTICAL to the truncated control's. That is simultaneously the determinism check blend_hi.sh
# asked for ("if the re-run reproduces exactly, the ~0.07 between-run band is purely SEED variance")
# and the proof that bn_075 can stand in for the arm that died.
#
# So all three arms are 20 generations, same seed, same settings, differing only in blend. Judged
# head-to-head at depth 4, the strength standard per netmatch.rs:23-29, not the saturating origin
# metric that produced the unresolved 0.809 / 0.832 pair.
#
# For reference, what the origin metric said (all ±0.024, i.e. mutually unresolved):
#     blend 0.75  0.847      blend 0.85  0.809      blend 1.00  0.832
#
# PRE-REGISTERED:
#   * 1.00 >= 0.75 with an interval clear of 0.5 => the shipped default is unsupported. blend_RESULT
#     records that 0.75 was chosen OVER an equal-measuring 1.00 on a stated RISK argument -- "a pure
#     bootstrap off the current net has no anchor to reality and can drift" -- and a direct win for
#     1.00 makes that argument the only thing holding the default in place.
#   * 0.75 > 1.00 => the default is vindicated on the instrument that matters, and the risk argument
#     is no longer load-bearing because the measurement agrees with it.
#   * ALL THREE NULL => the plateau is real: above 0.75 the target's composition stops mattering, and
#     the blend lever is exhausted at its shipped value. That closes the largest remaining candidate
#     and is a result, not a disappointment.
set -uo pipefail
cd "$(dirname "$0")"
CORE=${CORE:-15}
PAIRS=${PAIRS:-448}
NM=${NM:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt4/release/examples/netmatch}
[ -x "$NM" ] || { echo "no netmatch at $NM"; exit 1; }

for pair in "bn_075 bh_085" "bn_075 bh_100" "bh_085 bh_100"; do
  set -- $pair
  if [ ! -f "$1.net" ] || [ ! -f "$2.net" ]; then echo "--- $1 vs $2: net missing ---"; continue; fi
  echo "=== $1 vs $2 (both 20 generations, seed 20260907) ==="
  timeout 5400 taskset -c "$CORE" nice -n 19 ionice -c 3 "$NM" "$1.net" "$2.net" "$PAIRS" 4 777 \
    2>&1 | grep -E 'scores|=>|arms:' | sed 's/^/  /'
  echo
done
echo "  The first-named net's score is shown. Below 0.5 means the SECOND (higher blend) is stronger."
echo "  netmatch prints 'arms:' when both logs are present -- all three arms ran 20 generations, so"
echo "  it should report them matched. If it does not, stop and read that line before the verdict."
