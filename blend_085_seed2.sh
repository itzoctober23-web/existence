#!/usr/bin/env bash
# SECOND SEED for blend 0.85 — the replication the one-seed +0.040 needs.
#
# WHY. Direct at depth 4, seed 20260907: `bn_075` vs `bh_085` = 0.460 +/- 0.022, interval clear of
# 0.5, so 0.85 beats the shipped 0.75 at the strength standard — where 1.00 does NOT (0.511 +/-
# 0.022). The settled block closes blend 1.00 and says nothing about 0.85.
#
# WHY ONE SEED IS NOT ENOUGH, quantified in this tree rather than asserted: the gap is +0.040 and
# STATE.md:247 records that effects of 0.02-0.05 need ~19 seeds to separate from a seed spread of
# ~0.045. A within-run interval clear of 0.5 says nothing about between-seed variance. This project
# has already withdrawn two candidates that looked exactly this good on one seed -- the "+0.025
# depth lever" and blend 1.00 itself.
#
# `blend_seed2.sh` built 0.75 and 1.00 at seed 424242 (`s2_075`, `s2_100`) and stopped there, so the
# 0.85 arm at that seed does not exist. This trains it and matches it against the 0.75 arm that
# already does — identical settings, identical seed, one variable.
#
# Cost: 20 generations at the measured ~24 s/generation, so about 8 minutes, plus one 448-pair match.
#
# PRE-REGISTERED:
#   * 0.85 wins on BOTH seeds => two independent trainings agree at the strength standard. Still not
#     ~19 seeds, so it earns "worth a proper multi-seed run", NOT a default change.
#   * 0.85 loses or ties here => the first reading was seed-specific, exactly as the band predicts,
#     and the blend axis closes at 0.75 for good.
set -uo pipefail
cd "$(dirname "$0")"
CORE=${CORE:-12}
SEED=${SEED:-424242}
GENS=${GENS:-20}
LEARN=${LEARN:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt2/release/learn}
NM=${NM:-/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt4/release/examples/netmatch}
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
[ -x "$NM" ] || { echo "no netmatch at $NM"; exit 1; }
[ -f s2_075.net ] || { echo "ABORT: s2_075.net missing -- nothing to compare against"; exit 1; }

if [ ! -f s2_085.net ]; then
  echo "=== training blend 0.85 at seed $SEED, $GENS generations ==="
  timeout 4200 taskset -c "$CORE" nice -n 19 ionice -c 3 "$LEARN" \
    --rung 0 --gens "$GENS" --games 2400 --threads 1 --depth 2 --epochs 3 --blend 0.85 \
    --gate-every 100 --gate-pairs 224 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out s2_085.net --ledger s2_085.jsonl > s2_085.log 2>&1
  echo "  $(grep -cE '^gen +[0-9]+ ' s2_085.log) generations"
fi

# ARM SIZES MUST MATCH. netmatch prints them and quantifies the bias when they do not; an arm that
# trained fewer generations is worth ~0.0114 each, which is a quarter of the effect under test.
a=$(grep -cE '^gen +[0-9]+ ' s2_075.log 2>/dev/null); b=$(grep -cE '^gen +[0-9]+ ' s2_085.log 2>/dev/null)
echo "  arms: s2_075 $a generations, s2_085 $b generations"
[ "$a" = "$b" ] || echo "  *** UNEQUAL ARMS -- the match below is confounded with training amount ***"

echo "=== seed $SEED: blend 0.75 vs 0.85, DIRECT, depth 4 ==="
timeout 5400 taskset -c "$CORE" nice -n 19 ionice -c 3 "$NM" s2_075.net s2_085.net 448 4 777 \
  2>&1 | grep -E 'scores|=>|arms:' | sed 's/^/  /'
echo "  s2_075's score is shown; below 0.5 means blend 0.85 is stronger, replicating seed 20260907."
