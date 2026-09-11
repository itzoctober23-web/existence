#!/usr/bin/env bash
# DOES A DECAY SCHEDULE BEAT THE BEST FIXED RATE?
#
# THE EVIDENCE THAT MOTIVATES IT. Every drop in learning rate has bought a burst that then faded:
#
#   from a net plateaued at lr 0.01        lr 0.002 scored 0.692 +/- 0.028 vs 0.499 +/- 0.030
#   from a net already trained at 0.002    lr 0.002 gains at 2,000 gens (0.544 +/- 0.025) but
#                                          prod3 read 0.478 at gen 5,778 and an independent run
#                                          read 0.458 at gen 1,248 -- the gain does not hold
#   dropping again to 0.0005               passed auto_promote at 0.625 +/- 0.029 (gen 723)
#
# `trainer.rs` is plain SGD: no momentum, no decay, no schedule. A CONSTANT step keeps displacing
# the weights by the same amount however close to a basin they are, which is why every fixed rate
# eventually wanders. Measured in weight space already: the lr 0.01 arm travelled 1.73x farther than
# the 0.002 arm and gained nothing (`learning_rate_is_the_plateau_RESULT.md`).
#
# So the question is not "which constant is right" -- it is whether a constant is the wrong SHAPE.
#
# THE ARMS, matched on generations from one start, one seed, differing only in the schedule:
#   A  lr 0.002 constant              the current shipped setting, and the control
#   B  lr 0.002 decaying to ~0.0005   --lr-decay 0.9993 (0.9993^2000 = 0.247, so 0.002 -> ~0.0005)
#   C  lr 0.0005 constant             the arm that keeps winning at low generation counts
#
# B and C END at the same rate. That is deliberate: if B beats C, the advantage came from the LARGE
# EARLY STEPS, not from the small final one, and the schedule is doing real work rather than being a
# slow way to arrive at 0.0005.
#
# PRE-REGISTERED READING (verdict = netmatch of each arm against the shared start, 2,000 gens):
#   * B beats both A and C          -> the schedule is the lever. Large early + small late is the
#                                      shape, and the next move is to tune the decay, not the rate.
#   * B ~ C, both above A           -> what matters is ENDING low; the early steps are wasted and a
#                                      plain lower constant is simpler and equal. Ship 0.0005.
#   * B ~ A, both above C           -> ending low is not the point either, and 0.0005 was winning at
#                                      723 generations only because it had not yet gone anywhere.
#   * all three within noise        -> the rate lever is SPENT at this start, and the plateau's next
#                                      cause is elsewhere. That is a real finding and gets recorded.
#
# NOT a re-run of lr_sweep: that compared three CONSTANTS. This asks whether the constant-ness is
# itself the defect, which no run so far has tested.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
# IMMUTABLE SNAPSHOT, not the build tree. `xt_cap` predates --lr-decay and `t2` is what `run.sh`
# rebuilds into -- a cargo build during this experiment would swap the binary under it, which is
# the recorded "never rebuild under a running job" failure.
LEARN=$SCR/xt_lrd/learn
NM=$SCR/xt_lrd/netmatch
GENS=${GENS:-2000}
PAIRS=${PAIRS:-224}
CAP=${CAP:-5400}
SEED=${SEED:-20260914}
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }

# The flag must be honoured by the binary that will RUN. Capture to a file, never `| head`: with
# -o pipefail a SIGPIPE'd upstream turns a check that PASSED into a failure, which aborted this
# experiment's sibling earlier tonight.
"$LEARN" --gens 0 --games 2 --lr 0.002 --lr-decay 0.9993 --out "$SCR/probe.net" > /tmp/dec_probe.txt 2>&1 || true
grep -q 'lr-decay=0.9993' /tmp/dec_probe.txt \
  || { echo "ABORT: deployed learn does not honour --lr-decay (header: $(head -c 60 /tmp/dec_probe.txt))"; exit 1; }

# REFUSE TO RUN BESIDE OTHER ARMS. Existence owns cores 6-11; three arms at --threads 1 plus the
# production run already fill them, and adding more makes every arm slower without making any
# comparison fairer. rc=75 is the queue runner's "not yet runnable", not a failure.
live=$(for p in $(ls /proc 2>/dev/null | grep -E '^[0-9]+$'); do
  e=$(readlink "/proc/$p/exe" 2>/dev/null) || continue; e=${e% (deleted)}
  case "${e##*/}" in learn) tr '\0' ' ' < "/proc/$p/cmdline" 2>/dev/null | grep -oE '[-][-]out [a-z0-9_./]+';; esac
done)
n=$(printf '%s\n' "$live" | grep -c 'out ')
[ "${n:-0}" -gt 1 ] && { echo "ABORT: $n learn arms already live -- not yet runnable"; printf '%s\n' "$live" | sed 's/^/    /'; exit 75; }

[ -s p1_champion.net ] || { echo "no champion"; exit 1; }
cp -f p1_champion.net dec_start.net
echo "$(date '+%H:%M') lr decay A/B, seed $SEED, start $(md5sum dec_start.net | cut -c1-12), $GENS gens per arm"

run_arm() {  # name lr decay
  timeout "$CAP" taskset -c 6-11 nice -n 19 ionice -c 3 "$LEARN" \
    --init dec_start.net --gens "$GENS" --games 8 --threads 1 --depth 3 --epochs 3 \
    --lr "$2" --lr-decay "$3" --gate-every 1000000 --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "dec_$1.net" --ledger "dec_$1.jsonl" > "dec_$1.log" 2>&1
}
run_arm A 0.002  1.0    &
run_arm B 0.002  0.9993 &
run_arm C 0.0005 1.0    &
wait

ok=1
for A in A:lr=0.002:1 B:lr=0.002:0.9993 C:lr=0.0005:1; do
  n=${A%%:*}; rest=${A#*:}; want_lr=${rest%%:*}; want_d=${rest##*:}
  H=$(head -1 "dec_$n.log")
  G=$(grep -cE '^gen ' "dec_$n.log"); G=${G:-0}
  printf "  arm %s  %-12s lr-decay=%-7s %s generations\n" "$n" "$want_lr" "$want_d" "$G"
  case "$H" in
    "$want_lr lr-decay=$want_d "*) ;;
    *) echo "    ABORT: header does not match the requested settings, arm not comparable"; echo "    got: ${H:0:70}"; ok=0;;
  esac
done
[ "$ok" -eq 1 ] || exit 1

for n in A B C; do
  [ -s "dec_$n.net" ] || { echo "  arm $n produced no net"; continue; }
  nice -n 19 taskset -c 6-11 "$NM" "dec_$n.net" dec_start.net "$PAIRS" > "dec_${n}_vs_start.log" 2>&1
  echo "  arm $n vs shared start: $(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+ +\(interval \[0\.[0-9]+, 0\.[0-9]+\]\)' "dec_${n}_vs_start.log" | head -1)"
done
echo "  reference, same start, seed 20260912: lr 0.01 -> 0.358 +/- 0.026, lr 0.002 -> 0.544 +/- 0.025"
echo "LRDECAYDONE"
