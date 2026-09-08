#!/usr/bin/env bash
# REPLAY WINDOW, PLATEAU REGIME — the follow-up replay_ab.sh could not do.
#
# replay_ab.sh starts every arm from SCRATCH, so it covers generations 1-19 where the champion
# improves fast and old positions really ARE from a weaker player. That is the conventional
# staleness argument at its strongest, and the hardest case for "keep everything".
#
# His argument is about the PLATEAU. Measured against the same frozen origin:
#     gen 30  0.838 +/- 0.037
#     gen 60  0.853 +/- 0.037
#     gen 90  0.831 +/- 0.040
# flat, three overlapping intervals. If the champion is not improving, a position from 90
# generations ago came from an EQUALLY STRONG player, the staleness premise fails, and
# discarding is pure loss: 22.6M positions generated, ~250k trained on.
#
# So every arm here RESUMES from the plateaued champion. Same starting point, same wall-clock,
# only the window differs.
#
# TWO SIZING FIXES over the first sweep:
#   1. 1200s per arm, not 420. At ~22s/generation that is ~54 generations, so a 32-generation
#      window actually BINDS for ~22 of them. In the first sweep 32 never engaged at all (19
#      generations < 32), so that arm was silently "keep everything" rather than a window -- a
#      mislabel I caught mid-run and recorded rather than reporting.
#   2. Windows 8 / 32 / 999. 999 is the explicit unlimited arm, named for what it is instead of
#      being an accident of the budget.
#
# Seed pinned to 20260907: control.rs reconstructs the origin as Net::random(hidden, 20260907),
# so a different seed scores each arm against a DIFFERENT opponent -- silently meaningless
# rather than loudly wrong.
#
# PRE-REGISTERED: his argument predicts 999 >= 32 > 8. The conventional one predicts 8 > 32 >
# 999. They disagree on direction, so either result is informative. The loop carries 0.151
# run-to-run variance, so a null is NOT RESOLVED AT THIS BUDGET, never no-effect.
set -uo pipefail
cd "$(dirname "$0")"
SECS=${SECS:-1200}
PAIRS=${PAIRS:-64}
SEED=20260907
INIT=${INIT:-champion_long.net}
WINDOWS=${WINDOWS:-"1 8 999"}
# STEPS must be > 0 or the replay buffer is never read and every arm is identical. Sized to the
# measured epochs-3 update count (~51,798 on ~17k samples) so the arms differ ONLY in how much
# HISTORY the pool spans, not in how much training each one does.
STEPS=${STEPS:-50000}

[ -f "$INIT" ] || { echo "no champion at $INIT — cannot test the plateau regime without one"; exit 1; }
echo "=== replay window, PLATEAU regime: ${SECS}s per arm, all resuming from $INIT ==="
for W in $WINDOWS; do
  echo "--- arm: replay-gens $W ---"
  timeout "$SECS" taskset -c 15 nice -n 19 ionice -c 3 ./target/release/learn \
    --init "$INIT" --gens 1000000 --games 2400 --threads 1 --depth 2 --epochs 3 \
    --gate-pairs 32 --arch-every 0 --control-every 0 \
    --steps-per-gen "$STEPS" --replay-gens "$W" \
    --seed "$SEED" --out "rp_${W}.net" --ledger "rp_${W}.jsonl" > "rp_${W}.log" 2>&1
  gens=$(grep -cE '^gen ' "rp_${W}.log")
  acc=$(grep -cE 'ACCEPT' "rp_${W}.log")
  echo "  reached generation $gens ($acc accepted) — window binds from generation $((W+1))"
done

echo
echo "=== scored against the frozen origin, identical match for every arm ==="
echo "=== (the resumed champion itself measured ~0.83-0.85, so that is the baseline to beat) ==="
for W in $WINDOWS; do
  [ -f "rp_${W}.net" ] || { echo "  replay-gens $W: NO CHAMPION (never accepted one)"; continue; }
  printf "  replay-gens %-4s " "$W"
  taskset -c 15 nice -n 19 ionice -c 3 ./target/release/examples/control \
    --champion "rp_${W}.net" --pairs "$PAIRS" --seed 987654 2>/dev/null \
    | grep "fixed depth 2" | sed 's/^ *//'
done
echo "=== done ==="
