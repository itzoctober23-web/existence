#!/usr/bin/env bash
# IS THE CEILING CAPACITY? width 16 (rung 0) vs width 64 (rung 2), from scratch, head to head.
#
# THE CASE FOR ASKING. ceiling_ANALYSIS.md pools 44 control-vs-origin readings from 17 runs. The
# two long runs, each holding its settings fixed, are FLAT for 125 generations:
#     long_run2  gens 25->150: 0.831 0.808 0.825 0.816 0.791 0.808
#     long_run4  gens 30->150: 0.838 0.853 0.831 0.823 0.856
# and runs starting from a random net climb INTO that 0.79-0.86 band and stop. champion_long,
# re-measured at 0.861 +/- 0.010, sits at the top of it. Every one of those runs is at ARCH rung 0
# -- width 16, a 782->16->1 net, 12,528 weights -- with --arch-every 0 disabling the width step.
#
# Flat-across-many-anchors is the capacity signature. The sibling GPU-RL project recorded exactly
# this shape: a policy flat across EIGHT anchors at 302k parameters that broke through within
# minutes at 3.12M. That is the strongest prior available and it is for this failure mode.
#
# WHY HEAD TO HEAD AND NOT vs EACH ARM'S OWN ORIGIN. `origin = Net::random(WIDTH_MENU[rung], seed)`,
# so a width-16 champion scores against a width-16 origin and a width-64 champion against a
# width-64 origin. Those are DIFFERENT OPPONENTS and the two rates are not commensurable -- 0.861
# and 0.861 at different widths need not mean equal strength. The only valid comparison is the two
# champions against each other.
#
# AND IT MUST BE EQUAL COST, NOT EQUAL DEPTH. At fixed depth the wider net gets more computation per
# node and is charged nothing for it, which flatters capacity by construction. control.rs now skips
# the fixed-depth measurement across widths and reports only the equal-TIME capped result
# (arch::equal_time_caps). If width 64 wins there, it is winning per unit of CLOCK, which is the
# only sense in which more capacity is worth having.
#
# CONTROLLED: same seed, same games/gen, same epochs, same generation count, nothing gated in
# either arm (--gate-every 100 never fires with --gens 20). The arms differ in --rung and nothing
# else, so the comparison isolates width.
#
# PRE-REGISTERED READING:
#   * CAPACITY CONFIRMED if width 64 resolves above 0.5 head-to-head at equal time. Then the ceiling
#     is representational, the gate/blend/epochs/measurement-floor work was all tuning the wrong
#     component, and the next question is how far up the menu the gain continues.
#   * CAPACITY REFUTED if it resolves at or below 0.5. Then 12,528 weights are not the binding
#     constraint at this strength, the ceiling is in the DATA (depth-2 labels, or the horizon
#     schedule reaching 755 plies by generation 150 when datagen.rs:17-20 warns the outcome is
#     nearly independent of a position 40 plies back), and width is closed as a lever -- which is
#     worth knowing, because it is the expensive one.
#   * UNRESOLVED is the likely outcome at these pair counts and is NOT evidence of no effect. The
#     band itself is ~0.07 wide, wider than most single-run ci95, so run-to-run variation exceeds
#     within-run error. Treat an unresolved result as "needs more seeds", not as a null.
set -uo pipefail
cd "$(dirname "$0")"
GENS=${GENS:-20}
SEED=${SEED:-20260907}
PAIRS=${PAIRS:-600}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xtarget/release/learn}
CTRL=${CTRL:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt2/release/examples/control}

[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
[ -x "$CTRL" ]  || { echo "no control at $CTRL"; exit 1; }

echo "=== width A/B: rung 0 (w16) vs rung 2 (w64), $GENS generations from scratch, nothing gated ==="
echo "=== controls OFF during training: the verdict is the head-to-head at the end ==="
for R in 0 2; do
  echo "--- rung $R ---"
  timeout 4200 taskset -c 15 nice -n 19 ionice -c 3 "$LEARN" \
    --rung "$R" --gens "$GENS" --games 2400 --threads 1 --depth 2 --epochs 3 \
    --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "wd_r${R}.net" --ledger "wd_r${R}.jsonl" > "wd_r${R}.log" 2>&1
  echo "  generations $(grep -cE '^gen ' "wd_r${R}.log"), net $( [ -f wd_r${R}.net ] && stat -c %s "wd_r${R}.net" || echo MISSING) bytes"
done

echo
echo "=== VERDICT: width 64 vs width 16, HEAD TO HEAD at equal TIME ==="
if [ -f wd_r2.net ] && [ -f wd_r0.net ]; then
  timeout 1800 taskset -c 15 nice -n 19 ionice -c 3 "$CTRL" \
    --champion wd_r2.net --opponent wd_r0.net --pairs "$PAIRS" 2>&1 | tail -12
else
  echo "  one or both arms produced no net -- read wd_r0.log / wd_r2.log"
fi
echo
echo "  >0.5 resolved => capacity is the ceiling and width is the lever."
echo "  <=0.5 resolved => 12,528 weights are not the binding constraint; look at the DATA."
echo "  unresolved => needs more seeds. The run-to-run band is ~0.07, wider than one run's ci95."
