#!/usr/bin/env bash
# HOW FAR DOWN DOES THE LEARNING RATE KEEP PAYING?
#
# Every step down has won, and nothing has yet bracketed the optimum from BELOW:
#
#   0.01   -> 0.002    0.358 vs 0.544 against a shared start (lr_sweep_RESULT)
#   0.002  -> 0.0005   0.484 vs 0.628 against a shared start, same seed (lr_decay_RESULT)
#   0.0005 replicated three times from this lineage: 0.589, 0.628, 0.625
#
# A monotone sequence with no turning point is not an optimum, it is the part of the curve we have
# looked at. This walks two more steps down from the NEW champion (which was itself trained at
# 0.0005), so the control arm is the shipped setting and the question is whether it is already past
# the knee.
#
# THE COMPETING STORY, and why the control matters more than the treatments: each drop so far was
# measured FROM A NET TRAINED AT A HIGHER RATE, so "lower is better" and "a change of rate is
# better" predict the same thing every time. This run starts from a net already trained at 0.0005.
# If 0.0002 and 0.0001 both beat it, that is the rate. If neither does, the gains were transitions.
#
# PRE-REGISTERED READING (verdict = netmatch of each arm against the shared start, 2,000 gens each):
#   * 0.0002 and/or 0.0001 clear 0.5  -> the rate lever is STILL not spent; sweep again lower.
#   * all three sit at 0.5            -> the knee is at 0.0005 and the lever IS spent. That closes
#                                        the rate question and sends the next effort to the target
#                                        or the architecture.
#   * 0.0005 (control) clears 0.5 but the lower two do not
#                                     -> 0.0005 is the optimum and it still pays from its own
#                                        lineage, which is the cleanest possible result for the
#                                        shipped setting.
#   * everything DROPS below 0.5      -> at these rates 2,000 generations is too few to move a net
#                                        at all, and the comparison needs a longer budget, not a
#                                        lower rate. Check gens-per-arm before concluding anything.
#
# NOT a re-run of lr_sweep: that compared 0.01/0.002/0.0005 from a net trained at 0.002. This starts
# from a net trained at 0.0005 and goes below it, which no run has done.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
LEARN=$SCR/xt_lrd/learn        # immutable snapshot, not the build tree
NM=$SCR/xt_lrd/netmatch
GENS=${GENS:-2000}
PAIRS=${PAIRS:-224}
CAP=${CAP:-5400}
SEED=${SEED:-20260915}
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }

# Capture to a file, never `| head`: with -o pipefail a SIGPIPE'd upstream turns a check that PASSED
# into a failure. And never pipe THIS SCRIPT into anything either -- head -5 on the decay launcher
# would have killed it after the first verdict.
"$LEARN" --gens 0 --games 2 --lr 0.0002 --out "$SCR/probe.net" > /tmp/low_probe.txt 2>&1 || true
grep -q '^lr=0.0002' /tmp/low_probe.txt \
  || { echo "ABORT: deployed learn does not honour --lr (header: $(head -c 50 /tmp/low_probe.txt))"; exit 1; }

# Refuse if another EXPERIMENT is live. Production (one arm) is expected and fine; the sweep is
# matched on generations, so sharing cores with it costs wall clock and not validity.
n=0
for p in $(ls /proc 2>/dev/null | grep -E '^[0-9]+$'); do
  e=$(readlink "/proc/$p/exe" 2>/dev/null) || continue; e=${e% (deleted)}
  case "${e##*/}" in learn) n=$((n+1));; esac
done
[ "$n" -gt 1 ] && { echo "ABORT: $n learn arms already live -- not yet runnable"; exit 75; }

[ -s p1_champion.net ] || { echo "no champion"; exit 1; }
cp -f p1_champion.net low_start.net
echo "$(date '+%H:%M') low-lr sweep, seed $SEED, start $(md5sum low_start.net | cut -c1-12), $GENS gens per arm"

for L in 0.0005 0.0002 0.0001; do
  T=$(echo "$L" | tr -d '.')
  ( timeout "$CAP" taskset -c 6-11 nice -n 19 ionice -c 3 "$LEARN" \
      --init low_start.net --gens "$GENS" --games 8 --threads 1 --depth 3 --epochs 3 \
      --lr "$L" --gate-every 1000000 --arch-every 0 --control-every 0 \
      --seed "$SEED" --out "low_$T.net" --ledger "low_$T.jsonl" > "low_$T.log" 2>&1 ) &
done
wait

ok=1
for L in 0.0005 0.0002 0.0001; do
  T=$(echo "$L" | tr -d '.')
  H=$(head -1 "low_$T.log" | grep -oE '^lr=[0-9.]+')
  G=$(grep -cE '^gen ' "low_$T.log"); G=${G:-0}
  printf "  lr %-8s header %-14s %s generations\n" "$L" "${H:-MISSING}" "$G"
  [ "$H" = "lr=$L" ] || { echo "    ABORT: header mismatch, arm not comparable"; ok=0; }
done
[ "$ok" -eq 1 ] || exit 1

for L in 0.0005 0.0002 0.0001; do
  T=$(echo "$L" | tr -d '.')
  [ -s "low_$T.net" ] || { echo "  lr $L produced no net"; continue; }
  nice -n 19 taskset -c 6-11 "$NM" "low_$T.net" low_start.net "$PAIRS" > "low_${T}_vs_start.log" 2>&1
  echo "  lr $L vs shared start: $(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+ +\(interval \[0\.[0-9]+, 0\.[0-9]+\]\)' "low_${T}_vs_start.log" | head -1)"
done
echo "  reference from the PREVIOUS champion, seed 20260914: 0.002 -> 0.484, 0.0005 -> 0.628"
echo "LOWSWEEPDONE"
