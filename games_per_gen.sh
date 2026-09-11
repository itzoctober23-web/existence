#!/usr/bin/env bash
# IS A GENERATION NET-POSITIVE AT A DIFFERENT DATA BUDGET?
#
# `generator_is_net_negative_RESULT.md` measured what five generations are worth, directly and on an
# unsaturated scale: **mean 0.4684, 95% CI [0.4525, 0.4843], 16 of 20 batches below parity**. Five
# generations of this loop make the net WORSE than the net they started from. That relocates the
# problem from the filter to the GENERATOR -- a filter cannot find signal that is not in its input.
#
# WHICH GENERATOR KNOB. Checked the results directory first rather than guessing:
#   * label depth   -- SPENT. 1 -> 3 was +128 +/- 72 Elo (`datagen_depth_RESULT`), 3 -> 5 loses
#                      (`depth5_vs_depth3_RESULT`). The knee is at or below 3.
#   * blend         -- SPENT. 0.75/0.85/0.95/1.00 is "a flat plateau ... 0.5258/0.5234/0.5293/0.5281,
#                      all overlapping" (`blend_RESULT`); 0.25 is worse; 1.00 is dead at depth.
#   * epochs        -- measured (`epochs_ab_RESULT`: under-fitting REFUTED).
#   * horizon       -- measured (`horizon_RESULT`: the cap is obsolete past bootstrap).
#   * GAMES/GEN     -- **no dedicated result anywhere.** `--games 8` has been the default throughout.
#
# WHY IT IS A PLAUSIBLE CAUSE. At `--games 8` a generation contributes ~800 positions to a replay
# pool with a steady state near 2,000, and trains on ~270 of them for 3 epochs. That is a small,
# high-variance fresh sample being fitted repeatedly -- exactly the shape that produces steps with
# negative expectation even when the learner is sound.
#
# THE CONTROL IS ALREADY BANKED, so this runs ONE arm, not two. `bd.log` holds 20 direct batch
# decisions at `--games 8` from this same champion, seed, K and depth. This arm changes ONLY
# `--games`, so the two distributions are comparable batch-for-batch.
#
# PRE-REGISTERED READING (the statistic is the MEAN BATCH SCORE, n=20 per arm):
#   * mean clears 0.5 -> the generator is net-positive at this budget and `--games 8` was the cause.
#     That is the plateau's root and it is a one-flag fix.
#   * mean rises but stays under 0.5 -> data volume helps and is not sufficient; report the slope and
#     test a larger value before concluding.
#   * mean is unchanged -> volume is not the lever; the negative expectation is intrinsic to the
#     learner or the target, and the next suspects are the optimiser and the replay window.
#   * mean FALLS -> more data per generation is worse, which would be a genuine surprise and worth
#     more than the other three outcomes.
#
# COST IS REPORTED, NOT HIDDEN: more games means slower generations. "Is a generation positive?" is a
# question about QUALITY; whether it is worth the wall clock is a separate question and needs the
# rate, which is printed.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
LEARN=./target/release/learn          # repo build: carries EXISTENCE_DIRECT_BATCH
G=${G:-32}                            # games per generation; control is 8
GENS=${GENS:-100}
CAP=${CAP:-3600}
TAG=g${G}
LOG=${TAG}.log
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }

cp -f p1_champion.net "${TAG}_start.net"
echo "$(date '+%H:%M') games/gen=$G: start $(md5sum ${TAG}_start.net | cut -c1-12), target $GENS gens"
t0=$(date +%s)
EXISTENCE_DIRECT_BATCH=1 timeout "$CAP" taskset -c 6-11 nice -n 19 ionice -c 3 "$LEARN" \
  --init "${TAG}_start.net" --gens "$GENS" --games "$G" --threads 4 --depth 3 --epochs 3 \
  --gate-every 5 --gate-pairs 224 --arch-every 0 --control-every 0 \
  --seed 20260910 --out "${TAG}.net" --ledger "${TAG}.jsonl" > "$LOG" 2>&1
EL=$(( $(date +%s) - t0 ))

D=$(grep -c 'DIRECT: champ-vs-base' "$LOG")
if [ "$D" -eq 0 ]; then
  echo "  ABORT: no DIRECT decisions -- the flag did not take, so this arm is not comparable."
  exit 1
fi
N=$(grep -cE '^gen ' "$LOG")
echo "  $N generations in ${EL}s = $(python3 -c "print(f'{60*$N/max($EL,1):.1f}')") gen/min, $D decisions"
grep -oE 'champ-vs-base 0\.[0-9]+' "$LOG" | awk '{print $2}' | python3 -c "
import sys, statistics as st
v=[float(x) for x in sys.stdin]
if len(v) < 2: print('  too few decisions to summarise'); raise SystemExit
se=st.stdev(v)/len(v)**0.5
print(f'  games/gen=$G  n={len(v)}  mean {st.mean(v):.4f}  95% CI [{st.mean(v)-1.96*se:.4f}, {st.mean(v)+1.96*se:.4f}]')
print(f'    below 0.5: {sum(1 for x in v if x<0.5)}/{len(v)}')
print(f'    CONTROL games/gen=8: mean 0.4684  CI [0.4525, 0.4843]  16/20 below 0.5')
print('    VERDICT: ' + ('generator is NET-POSITIVE at this budget' if st.mean(v)-1.96*se > 0.5
      else 'still net-negative' if st.mean(v)+1.96*se < 0.5 else 'unresolved -- interval contains 0.5'))"
echo "GAMESPERGEN${G}DONE"
