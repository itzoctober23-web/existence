#!/usr/bin/env bash
# PRODUCTION TRAINING from the current champion. The configuration that actually produced a
# promotion, run long, with banking left to `auto_promote.sh`.
#
# WHY THESE SETTINGS, each one measured rather than chosen:
#
#   --depth 3      `datagen_depth_RESULT.md`: depth 1 -> 3 is +128 +/- 72 Elo on the absolute ruler.
#                  `depth5_vs_depth3_RESULT.md`: 3 -> 5 LOSES (0.397 +/- 0.026 against its own start)
#                  while depth 3 WINS (0.557 +/- 0.032) from the identical champion. The knee is at
#                  or below 3, so the label-depth lever is SPENT and 3 is where it settles.
#
#   --lr 0.002     MEASURED 2026-09-11 (`learning_rate_is_the_plateau_RESULT.md`). Matched A/B from
#                  one start, 2,000 generations each: lr 0.01 scored 0.499 +/- 0.030 against that
#                  start -- reproducing the plateau to three decimals -- while lr 0.002 scored
#                  0.692 +/- 0.028. Disjoint intervals, ~4x the between-seed sd. The shipped 0.01
#                  was a hardcoded literal that had never been varied.
#
#   --arch-every 0 WIDTH IS A CLOSED QUESTION. `width_clock_RESULT.md`: 11 ARCH attempts across
#                  widths 32/64/128, fixed-cost 8 of 9 ABOVE 0.5 (wider is better per NODE), clock
#                  9 of 9 BELOW 0.5 (wider is worse per SECOND), zero accepted. That file moved
#                  --arch-every from 5 to 100 precisely because "the widening search was consuming
#                  95% of wall clock to re-derive the same answer".
#
#                  MEASURED AGAIN TODAY, because I re-launched it at 5 without reading that file
#                  first: 5 gens/min with ARCH on against 108 gens/min with it off, same flags
#                  otherwise -- ARCH consumed 95.4% of the run, and all three verdicts it produced
#                  fell INSIDE the already-recorded ranges (fixed-cost 0.509 in [0.490,0.583],
#                  clock 0.445 and 0.310 in [0.190,0.467]). Not one was new information.
#
#                  0 rather than the documented 100: at 108 gens/min a proposal every 100
#                  generations still costs ~50% (a proposal takes ~55 s, 100 generations take ~55 s).
#                  100 is right for a run that wants a slow trickle of vigilance on a closed
#                  question; this run's purpose is champion progress, so it pays nothing for it.
#
# BANKING IS NOT DONE HERE. `auto_promote.sh` measures the live arm against the champion every 30
# minutes by netmatch -- the paired instrument -- and promotes only on `rate - ci95 >= 0.5`, only
# when exactly one arm is training. Putting a promotion rule in two places is how two rules drift.
#
# The ruler is deliberately NOT consulted for any decision here: its sequence has now been refuted
# by netmatch three times (+50/-86 on this very champion's parent while netmatch said 0.557).
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
LEARN=$SCR/xt_cap/release/learn
SECS=${SECS:-21600}          # 6h; auto_promote banks progress along the way
CORES=${CORES:-6-11}
TAG=${TAG:-prod1}
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
[ -s p1_champion.net ] || { echo "no champion"; exit 1; }

cp -f p1_champion.net "${TAG}_start.net"
echo "$(date '+%H:%M') $TAG: start $(md5sum ${TAG}_start.net | cut -c1-12), ${SECS}s, depth 3, arch off"

timeout "$SECS" taskset -c "$CORES" nice -n 19 ionice -c 3 "$LEARN" \
  --init "${TAG}_start.net" --gens 1000000 --games 8 --threads 4 --depth 3 --epochs 3 \
  --lr "${LR:-0.002}" --gate-every 1000000 --arch-every 0 --control-every 0 \
  --seed 20260910 --out "${TAG}.net" --ledger "ledger_${TAG}.jsonl" > "${TAG}.log" 2>&1

G=$(grep -cE '^gen ' "${TAG}.log")
echo "$(date '+%H:%M') $TAG: finished at $G generations" | tee -a ab_verdicts.out
