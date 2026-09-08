#!/usr/bin/env bash
# CAN THE GATE SEE IMPROVEMENT? 40 pairs vs 224, everything else identical.
#
# THE DIAGNOSIS BEING TESTED. Measured over 230 generations of ledger, the per-generation NET
# gate at 32-40 pairs has a median ci95 of 0.079 -- so it only resolves a candidate better than
# 0.579, which is +56 Elo. Self-play gains do not arrive in +56 Elo steps. The claim is that the
# loop has therefore been rejecting real small improvements as noise and accepting noise that
# happened to look large, and that this is why the origin control has sat flat at ~0.84 for 150+
# generations while capacity, data volume and horizon were all investigated and cleared.
#
# WHY IT IS AFFORDABLE. The gate is 2.7% of a generation (64 games against 2400 for datagen).
# 224 pairs resolves ~+21 Elo for 17% overhead, and SPRT stops early, so 224 is a CAP: decisive
# candidates still cost far fewer.
#
# SECOND EFFECT, and the reason this is not just "more games". The acceptance rule gives the
# decision to the GAMES when ci95 < 0.05 and otherwise falls through to the held-out-loss
# surrogate. Expected interval is 0.071 at 40 pairs and 0.030 at 224 -- so at the old default the
# surrogate was the de-facto decider (measured: `resolves` fired once in ten generations). The
# surrogate disagrees with the games: the width-64 ARCH candidate had the strongest surrogate
# reading of the day (paired z 4.12) and then failed to beat width 16 at EQUAL NODES.
#
# PRE-REGISTERED READING. The diagnosis predicts BOTH:
#   (a) more accepts -- small real gains stop being discarded, AND
#   (b) a HIGHER origin control -- those gains accumulate into strength.
# (a) alone REFUTES it: more accepts with a flat control means the extra accepts are noise and
# the blunt gate was right to reject them. That outcome must be reported as a refutation, not
# quietly dropped. A tie on both is NOT RESOLVED at this budget -- the loop carries 0.151
# run-to-run variance.
#
# Both arms resume from the same champion and are scored against the same frozen origin, with
# the seed pinned to 20260907 because control.rs reconstructs the origin as
# Net::random(hidden, 20260907).
set -uo pipefail
cd "$(dirname "$0")"
SECS=${SECS:-1200}
PAIRS_SCORE=${PAIRS_SCORE:-64}
SEED=20260907
INIT=${INIT:-champion_long.net}
ARMS=${ARMS:-"40 224"}

[ -f "$INIT" ] || { echo "no champion at $INIT"; exit 1; }
echo "=== gate resolution A/B: ${SECS}s per arm, both resuming from $INIT ==="
for GP in $ARMS; do
  echo "--- arm: gate-pairs $GP ---"
  timeout "$SECS" taskset -c 15 nice -n 19 ionice -c 3 ./target/release/learn \
    --init "$INIT" --gens 1000000 --games 2400 --threads 1 --depth 2 --epochs 3 \
    --gate-pairs "$GP" --arch-every 0 --control-every 0 \
    --seed "$SEED" --out "gp_${GP}.net" --ledger "gp_${GP}.jsonl" > "gp_${GP}.log" 2>&1
  gens=$(grep -cE '^gen ' "gp_${GP}.log")
  acc=$(grep -cE 'ACCEPT' "gp_${GP}.log")
  med=$(grep -oE '\+/-[0-9.]+' "gp_${GP}.log" | tr -d '+/-' | sort -n | awk '{a[NR]=$1} END{print a[int(NR/2)]}')
  echo "  generation $gens, $acc accepted, median gate ci95 ${med:-?}"
done

echo
echo "=== scored against the frozen origin, identical match for both arms ==="
for GP in $ARMS; do
  [ -f "gp_${GP}.net" ] || { echo "  gate-pairs $GP: NO CHAMPION (never accepted one)"; continue; }
  printf "  gate-pairs %-4s " "$GP"
  taskset -c 15 nice -n 19 ionice -c 3 ./target/release/examples/control \
    --champion "gp_${GP}.net" --pairs "$PAIRS_SCORE" --seed 987654 2>/dev/null \
    | grep "fixed depth 2" | sed 's/^ *//'
done
echo
echo "  Read it against the pre-registration: more accepts AND higher control = confirmed."
echo "  More accepts with a FLAT control = REFUTED, and say so."
echo "=== done ==="
