#!/usr/bin/env bash
# REPLICATE THE LEARNING-RATE RESULT ON A FRESH SEED, AND SWEEP ONE STEP FURTHER DOWN.
#
# `learning_rate_is_the_plateau_RESULT.md` measured, matched on generations from one start:
#     lr 0.01 (shipped)  0.499 +/- 0.030   [0.469, 0.529]   <- reproduced the plateau exactly
#     lr 0.002           0.692 +/- 0.028   [0.664, 0.720]
# and that net then passed the champion gate at 0.555 +/- 0.028, so 0.002 is now SHIPPED.
#
# It was ONE SEED. That file says so in its own "not claimed" section, and shipping on one seed is
# exactly what this project criticised itself for earlier tonight when re-testing a marginal
# promotion. So this replicates on seed 20260912 -- and adds 0.0005, because two points cannot
# locate a minimum and the shipped value might be on the wrong side of one.
#
# THE START IS THE NEW CHAMPION, deliberately. The original arms began from a net plateaued at
# lr 0.01; this begins from a net already trained at 0.002. That makes it a genuine replication of
# the COMPARISON rather than a re-run of the same conditions, and it asks the question that now
# matters operationally: does the low rate still win from a net that already had it?
#
# PRE-REGISTERED READING (verdict = netmatch of each arm against the shared start, matched at
# 2,000 generations):
#   * 0.002 beats 0.01 again           -> replicated on a second seed and a different start. The
#                                         shipped change is sound.
#   * 0.002 no longer beats 0.01       -> the effect was specific to a net plateaued AT 0.01, i.e.
#                                         a recovery from over-large steps rather than a better rate.
#                                         That would NOT unship it (the promotion gate passed on its
#                                         own merits) but it would narrow the claim sharply.
#   * 0.0005 beats 0.002               -> the optimum is lower still; sweep again before settling.
#   * 0.0005 loses to 0.002            -> 0.002 is at or near a minimum and the sweep can stop.
#   * all three sit together near 0.5  -> the new champion is itself plateaued and none of these
#                                         rates moves it, which would point back at the target or
#                                         the architecture.
#
# THREADS 1 PER ARM so three arms fit beside the production run without taking his cores; the
# comparison is matched on GENERATIONS, so a slower wall clock costs time and not validity.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
LEARN=$SCR/xt_cap/release/learn       # deployed copy, verified to carry --lr
NM=$SCR/xt_cap/release/examples/netmatch
GENS=${GENS:-2000}
PAIRS=${PAIRS:-224}
CAP=${CAP:-3600}
SEED=${SEED:-20260912}
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
# The flag must be honoured by the binary that will RUN, not by the one in the repo.
#
# CAPTURE TO A FILE, NEVER THROUGH `| head -1`. The first version of this check was
#   "$LEARN" ... | head -1 | grep -q '^lr=0.002'
# and it FAILED on a binary that honours the flag perfectly. Mechanism: grep -q exits the moment it
# matches, head then takes SIGPIPE, `learn` keeps printing into a closed pipe and panics on EPIPE --
# and because this script sets `-o pipefail`, that upstream abort becomes the PIPELINE's status. A
# check that succeeded reported failure and refused to launch the experiment.
#
# This is the SIGPIPE trap inverted: the recorded version is `cargo test | head` masking a failure
# behind head's exit 0. With pipefail it does the opposite and invents one.
"$LEARN" --gens 0 --games 2 --lr 0.002 > /tmp/lr_flag_probe.txt 2>&1 || true
grep -q '^lr=0.002' /tmp/lr_flag_probe.txt \
  || { echo "ABORT: deployed learn does not honour --lr (header: $(head -c 40 /tmp/lr_flag_probe.txt))"; exit 1; }

cp -f p1_champion.net lrs_start.net
echo "$(date '+%H:%M') lr sweep, seed $SEED, start $(md5sum lrs_start.net | cut -c1-12), $GENS gens per arm"

for L in 0.01 0.002 0.0005; do
  T=$(echo "$L" | tr -d '.')
  ( timeout "$CAP" taskset -c 6-11 nice -n 19 ionice -c 3 "$LEARN" \
      --init lrs_start.net --gens "$GENS" --games 8 --threads 1 --depth 3 --epochs 3 \
      --lr "$L" --gate-every 1000000 --arch-every 0 --control-every 0 \
      --seed "$SEED" --out "lrs_$T.net" --ledger "lrs_$T.jsonl" > "lrs_$T.log" 2>&1 ) &
done
wait

ok=1
for L in 0.01 0.002 0.0005; do
  T=$(echo "$L" | tr -d '.')
  H=$(head -1 "lrs_$T.log" | grep -oE '^lr=[0-9.]+')
  G=$(grep -cE '^gen ' "lrs_$T.log")
  printf "  lr %-7s header %-12s %s generations\n" "$L" "${H:-MISSING}" "$G"
  [ "$H" = "lr=$L" ] || { echo "    ABORT: header mismatch, arm not comparable"; ok=0; }
done
[ "$ok" -eq 1 ] || exit 1

for L in 0.01 0.002 0.0005; do
  T=$(echo "$L" | tr -d '.')
  [ -s "lrs_$T.net" ] || { echo "  lr $L produced no net"; continue; }
  nice -n 19 taskset -c 6-11 "$NM" "lrs_$T.net" lrs_start.net "$PAIRS" > "lrs_${T}_vs_start.log" 2>&1
  echo "  lr $L vs shared start: $(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+ +\(interval \[0\.[0-9]+, 0\.[0-9]+\]\)' "lrs_${T}_vs_start.log" | head -1)"
done
echo "  first run, seed 20260911, plateaued start: lr 0.01 -> 0.499 +/- 0.030, lr 0.002 -> 0.692 +/- 0.028"
echo "LRSWEEPDONE"
