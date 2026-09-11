#!/usr/bin/env bash
# IS WIDTH STILL REFUTED, NOW THAT THE LABELS CARRY INFORMATION?
#
# `width_RESULT.md` says "Width is NOT the ceiling -- REFUTED, resolved", and `width_clock_RESULT.md`
# records 11 widening attempts with the sign never turning. Both were measured under DEPTH-1 labels.
#
# Today established what those labels were worth: a one-ply root score barely depends on the
# position, and raising datagen to depth 3 was +128 Elo with two shipped gates behind it. Under a
# label a 16-wide net can already fit, extra capacity has NO residual error to remove -- so those
# measurements could not have found a width effect even if one existed. That is the shape recorded
# in `right-measurement-wrong-conclusion`: a correct measurement whose conclusion was generalised
# past the regime it was taken in.
#
# MATCHED START: both arms resume from the SAME shipped champion via --init, so the only difference
# is the accumulator width. Judged by netmatch against that shared start -- the paired instrument,
# never the ruler.
#
# PRE-REGISTERED. Unlike depth 5, I do NOT have a confident prior here:
#   * w64 WINS  -> width was refuted in the wrong regime, and capacity is a live lever again. That
#                  would also mean every width conclusion in this repo needs re-reading.
#   * w64 LOSES -> width is refuted for a second, better reason, and the question closes properly
#                  rather than resting on a measurement taken under uninformative labels.
#   * unresolved -> the honest outcome for a modest effect at this sample size; report as such.
#
# Cost note: a w64 net is ~4x the parameters, so at fixed depth it is slower per node and will
# complete fewer generations in the same wall clock. That is the trade, and it is why this is judged
# at equal WALL CLOCK rather than equal generations.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
LEARN=$SCR/xt_cap/release/learn
NM=$SCR/xt_cap/release/examples/netmatch
SECS=${SECS:-2400}
CORES=${CORES:-6-11}
[ -x "$LEARN" ] || { echo "no learn"; exit 1; }

cp -f p1_champion.net w64_start.net
timeout "$SECS" taskset -c "$CORES" nice -n 19 ionice -c 3 "$LEARN" \
  --init w64_start.net --rung 2 --gens 1000000 --games 8 --threads 4 --depth 3 --epochs 3 \
  --gate-every 1000000 --arch-every 0 --control-every 0 \
  --seed 20260910 --out w64c.net --ledger ledger_w64c.jsonl > w64c.log 2>&1

# ABORT UNLESS THE WIDTH ACHIEVED IS THE WIDTH INTENDED.
#
# `--init` ADOPTS the saved net's rung and OVERRIDES `--rung` -- deliberately, so a champion already
# widened by ARCH can be resumed without aborting on its own progress. The champion is width 16, so
# this run silently executes at width 16 and its netmatch becomes a width-16 A/A reported as a
# capacity result. Verified by running the engine with these exact arguments:
#
#   ARCH menu [16, 32, 64, 128, 256, 512]  start rung 2 (width 64)  arch-every 0
#   RESUMED at rung 0 (width 16), overriding --rung 2
#
# The ORIGINAL summary line grepped `start rung [0-9]+ (width [0-9]+)`, which matches the line
# printed immediately BEFORE the override -- so it printed "width 64" while running width 16. A
# check that reads the line above the correction manufactures confirmation; worse than none.
# Read the OVERRIDE line, and treat its absence as failure rather than as success.
ACHIEVED=$(grep -oE 'RESUMED at rung [0-9]+ \(width [0-9]+\)' w64c.log | head -1 | grep -oE 'width [0-9]+' | awk '{print $2}')
[ -n "$ACHIEVED" ] || ACHIEVED=$(grep -oE 'RESUMED champion from [^ ]+ \(width [0-9]+\)' w64c.log | head -1 | grep -oE 'width [0-9]+' | awk '{print $2}')
if [ "${ACHIEVED:-0}" != "64" ]; then
  echo "ABORT: intended width 64, ACTUALLY RAN AT WIDTH ${ACHIEVED:-unknown}."
  echo "  --init adopts the saved net's rung and overrides --rung, and the champion is width 16."
  echo "  Resuming a w16 net into a w64 slot is not what resume means, so this experiment cannot be"
  echo "  expressed this way. Either use --arch-every N > 0 and let ARCH PROPOSE the widening under"
  echo "  its own gates (the decision-relevant question: does widening THIS champion pass?), or"
  echo "  compare w16 and w64 from MATCHED RANDOM starts (a clean capacity question about neither"
  echo "  champion). See w64_misspecified_RESULT.md. No netmatch is run: an A/A reported as a"
  echo "  capacity result is exactly what this guard exists to prevent."
  exit 1
fi

echo "w64: $(grep -cE '^gen ' w64c.log) generations in ${SECS}s (VERIFIED width $ACHIEVED)"
nice -n 19 taskset -c "$CORES" "$NM" w64c.net w64_start.net 224 > w64c_vs_start.log 2>&1
echo "  w64 vs its own start: $(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+' w64c_vs_start.log | head -1)"
