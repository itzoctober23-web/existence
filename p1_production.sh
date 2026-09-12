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
#
#   --lr 0.0002    SHIPPED 2026-09-11 (`low_sweep2_RESULT.md`), and it is now this script's DEFAULT.
#                  It was NOT the default until then: the fallback still read `${LR:-0.002}`, ten
#                  times the shipped rate, so a bare `./p1_production.sh` restarted production at a
#                  rate two sweeps had superseded. keepalive.sh:46 already passed LR=0.0002, so the
#                  automatic crash-restart was correct and only a MANUAL relaunch was exposed --
#                  which is exactly what the standing brief instructs on a crash, and that brief
#                  still names LR=0.0005. That arm measured 0.412 +/- 0.026 against its own starting
#                  net, interval entirely below 0.5: it does not merely underperform, it LOSES to
#                  the net it started from. 0.0001 was indistinguishable from 0.0002 (0.546 vs
#                  0.535, gap 0.011 against a between-seed sd of 0.047) and was correctly not taken.
#   --lr 0.0005    SHIPPED 2026-09-11 (`lr_decay_RESULT.md`). Three arms from one start, one seed:
#                  lr 0.002 constant 0.484 +/- 0.032, lr 0.002 decaying 0.586 +/- 0.027, lr 0.0005
#                  constant 0.628 +/- 0.027. Replicated three times from this lineage (0.589, 0.628,
#                  0.625) while lr 0.002 read 0.544 / 0.484 / 0.478 / 0.471, the last an outright
#                  REGRESSION after 12,283 generations. The winning net cleared the bar at
#                  0.628 - 0.027 = 0.601 and is the current champion.
#
#   --lr-decay     DEFAULT 1.0 = OFF, and that is now a MEASURED negative, not a placeholder. Arm B
#                  above decayed 0.002 -> 0.000493 and arm C sat at a constant 0.0005; they END at
#                  the same rate by design, so if the large early steps did real work B would beat
#                  C. It did not. There is no schedule to tune -- do not set DECAY without a new
#                  measurement that contradicts this one.
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
# BLEND IS PASSED EXPLICITLY, not left to the binary's compiled default.
#
# SHIPPED 2026-09-11: the default moved 0.75 -> 0.85 on two full-length seeds (pooled
# 0.574 +/- 0.021 head to head, lower bound 0.553 against a between-seed sd of 0.047). But this
# script runs a SNAPSHOT binary from $SCR built BEFORE that change -- the running job's own
# header reads `lr=0.0002 blend=0.75`. A ship that reaches the source does NOT reach a job
# running an older binary.
#
# That is the 4PC defect already on file: "the 09:36 ship reached the RECIPE but not the ENGINE",
# which passed verification because the check only confirmed the recipe had moved. Passing the
# value on the command line puts the run's configuration in its own argv and log header, where it
# can be CHECKED rather than inferred from which binary happened to be snapshotted.
#
# THE SNAPSHOT MUST LIVE ON A PERSISTENT FILESYSTEM (fixed 2026-09-12). Until today this read
# `LEARN=$SCR/xt_cap/release/learn`, and $SCR is **tmpfs** -- the binary is RAM. It vanishes on
# reboot or any /tmp clear, and the path embeds one Claude SESSION id, so it cannot outlive that
# session either. The `-x` guard below would then fail on every relaunch, keepalive would restart
# this script into `exit 1` forever, and Existence production would be dead with no recovery path.
# The snapshot that exists to protect the run FROM relinks was itself the most fragile part of it.
#
# bin/learn_prod is a byte-identical copy on btrfs -- md5 bb7ad37d93a9, verified equal to the
# tmpfs original at the moment of the swap, so this changes durability and NOT behaviour. The
# scratchpad stays a fallback and the copy self-heals: if the persistent binary is missing while
# the snapshot still exists, it is restored rather than aborted on.
LEARN=$PWD/bin/learn_prod
if [ ! -x "$LEARN" ] && [ -x "$SCR/xt_cap/release/learn" ]; then
  mkdir -p bin && cp -f "$SCR/xt_cap/release/learn" "$LEARN" && chmod +x "$LEARN" \
    && echo "$(date '+%H:%M') restored bin/learn_prod from the tmpfs snapshot"
fi
SECS=${SECS:-21600}          # 6h; auto_promote banks progress along the way
CORES=${CORES:-6-11}
TAG=${TAG:-prod1}
[ -x "$LEARN" ] || { echo "no learn at $LEARN (and no tmpfs snapshot to restore from)"; exit 1; }
[ -s p1_champion.net ] || { echo "no champion"; exit 1; }

cp -f p1_champion.net "${TAG}_start.net"
echo "$(date '+%H:%M') $TAG: start $(md5sum ${TAG}_start.net | cut -c1-12), ${SECS}s, depth 3, arch off"

timeout "$SECS" taskset -c "$CORES" nice -n 19 ionice -c 3 "$LEARN" \
  --init "${TAG}_start.net" --gens 1000000 --games 8 --threads 4 --depth 3 --epochs 3 \
    --lr "${LR:-0.0002}" --lr-decay "${DECAY:-1.0}" --blend "${BLEND:-0.85}" \
    --gate-every 1000000 --arch-every 0 --control-every 0 \
  --seed 20260910 --out "${TAG}.net" --ledger "ledger_${TAG}.jsonl" > "${TAG}.log" 2>&1

G=$(grep -cE '^gen ' "${TAG}.log")
echo "$(date '+%H:%M') $TAG: finished at $G generations" | tee -a ab_verdicts.out
