#!/usr/bin/env bash
# DOES ONLY THE PRODUCT lr x epochs MATTER, OR THE STEP SIZE ITSELF?
#
# `epochs_ab_RESULT.md` (2026-09-08) tested 3 / 10 / 30 epochs and found more training made things
# WORSE -- train_loss fell 0.0417 -> 0.0253 while mcnemar_z went +0.359 -> +0.141 -> -0.445. It
# concluded "under-fitting REFUTED, the labels are the problem", and that conclusion has stood since.
#
# IT WAS MEASURED AT lr 0.01, WHICH IS NO LONGER THE SETTING. `learning_rate_is_the_plateau_RESULT.md`
# shipped lr 0.002 tonight: matched arms at 2,000 generations scored 0.499 +/- 0.030 (lr 0.01,
# reproducing the plateau) against 0.692 +/- 0.028 (lr 0.002). Each epoch at 0.002 displaces the
# weights five times less, so "3 epochs" now means something different from what that file tested.
# A closed question can be reopened by a change to a parameter it was conditioned on, and nothing
# in the results index tests the two together.
#
# THE TWO MODELS PREDICT OPPOSITE SIGNS, which is what makes this worth the cores:
#
#   PRODUCT MODEL -- only total displacement per generation matters (lr x epochs):
#       lr 0.01  x 3  = 0.030   plateau   (measured 0.499)
#       lr 0.002 x 3  = 0.006   gain      (measured 0.692)
#       lr 0.002 x 10 = 0.020   ~ lr 0.0067 x 3  -> should be MUCH WORSE than 0.692
#
#   STEP-SIZE MODEL -- a smaller step converges better regardless of how many are taken:
#       lr 0.002 x 10 -> as good as or better than lr 0.002 x 3, because the extra epochs are
#       refinement rather than overshoot. Under this model epochs_ab's finding was really a finding
#       about lr 0.01 being too large, and more epochs should now HELP.
#
# PRE-REGISTERED READING (verdict = netmatch of each arm against the shared start, 2,000 gens each):
#   * epochs 10 scores materially BELOW epochs 3  -> product model. epochs_ab generalises, and the
#     lever is total displacement; a decay schedule is then the natural next move.
#   * epochs 10 scores at or ABOVE epochs 3       -> step-size model. epochs_ab's conclusion was
#     conditioned on lr 0.01 and must be narrowed, and epochs becomes a live knob again at the new
#     rate -- which would make it the second lever found tonight.
#   * indistinguishable                           -> epochs is inert at this rate; epochs_ab's
#     direction does not transfer but nothing is gained either. Report and stop.
#
# NOT a re-run of epochs_ab: that compared 3/10/30 at the OLD rate on mcnemar_z, a held-out surrogate
# with no games. This compares 3 vs 10 at the NEW rate on netmatch, the paired strength instrument.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
LEARN=$SCR/xt_cap/release/learn
NM=$SCR/xt_cap/release/examples/netmatch
GENS=${GENS:-2000}
PAIRS=${PAIRS:-224}
CAP=${CAP:-4200}
SEED=${SEED:-20260913}
LR=${LR:-0.002}
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }

# Capture to a file, never `| head`: with `-o pipefail` a SIGPIPE'd upstream turns a check that
# PASSED into a failure, which aborted this experiment's sibling earlier tonight.
"$LEARN" --gens 0 --games 2 --lr "$LR" > /tmp/ep_flag_probe.txt 2>&1 || true
grep -q "^lr=$LR" /tmp/ep_flag_probe.txt \
  || { echo "ABORT: deployed learn does not honour --lr (header: $(head -c 40 /tmp/ep_flag_probe.txt))"; exit 1; }

# REFUSE TO RUN BESIDE THE SWEEP. Three arms at --threads 1 already hold Existence's cores; adding
# two more would make every arm slower without making any comparison fairer, and the sweep's arms
# are matched to each other, not to these.
for p in $(ls /proc 2>/dev/null | grep -E '^[0-9]+$'); do
  e=$(readlink "/proc/$p/exe" 2>/dev/null) || continue; e=${e% (deleted)}
  case "${e##*/}" in learn) ;; *) continue;; esac
  o=$(tr '\0' ' ' < "/proc/$p/cmdline" 2>/dev/null | grep -oE '[-][-]out lrs_[^ ]+')
  [ -n "$o" ] && { echo "ABORT: the lr sweep is still running ($o) -- not yet runnable"; exit 75; }
done

cp -f p1_champion.net ep_start.net
echo "$(date '+%H:%M') epochs at lr $LR, seed $SEED, start $(md5sum ep_start.net | cut -c1-12), $GENS gens per arm"

for E in 3 10; do
  ( timeout "$CAP" taskset -c 6-11 nice -n 19 ionice -c 3 "$LEARN" \
      --init ep_start.net --gens "$GENS" --games 8 --threads 2 --depth 3 --epochs "$E" \
      --lr "$LR" --gate-every 1000000 --arch-every 0 --control-every 0 \
      --seed "$SEED" --out "ep_$E.net" --ledger "ep_$E.jsonl" > "ep_$E.log" 2>&1 ) &
done
wait

ok=1
for E in 3 10; do
  H=$(head -1 "ep_$E.log" | grep -oE "^lr=[0-9.]+ .*epochs=$E" | grep -oE "epochs=$E")
  G=$(grep -cE '^gen ' "ep_$E.log")
  printf "  epochs %-3s header %-10s %s generations\n" "$E" "${H:-MISSING}" "$G"
  [ "$H" = "epochs=$E" ] || { echo "    ABORT: header mismatch, arm not comparable"; ok=0; }
done
[ "$ok" -eq 1 ] || exit 1

for E in 3 10; do
  [ -s "ep_$E.net" ] || { echo "  epochs $E produced no net"; continue; }
  nice -n 19 taskset -c 6-11 "$NM" "ep_$E.net" ep_start.net "$PAIRS" > "ep_${E}_vs_start.log" 2>&1
  echo "  epochs $E vs shared start: $(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+ +\(interval \[0\.[0-9]+, 0\.[0-9]+\]\)' "ep_${E}_vs_start.log" | head -1)"
done
echo "  reference at lr 0.002, epochs 3, different seed/start: 0.692 +/- 0.028"
echo "EPOCHSLOWLRDONE"
