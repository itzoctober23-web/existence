#!/usr/bin/env bash
# REPLAY WINDOW SWEEP: is throwing away old positions right, and is 8 generations the right amount?
#
# WHY. `replay_gens` was a bare `8` with no derivation and no measurement. He asked whether
# discarding data is worth it -- 22.6M positions generated, the loop trains on the newest ~250k.
# The argument for a window is real (old positions came from a weaker champion; AlphaZero and
# Leela both use one) but 8 was never tested, and today FOUR constants failed for exactly that
# reason: a gate threshold that could never fire, a pair count derived to hit it exactly and
# missing by 0.001, a training budget tuned for one width that silently blocked others, and a
# proposal stride that could not reach past one rung.
#
# DESIGN. Equal WALL-CLOCK per arm, then every arm's champion scored against the SAME frozen
# origin with an identical match. Equal generations would be wrong: a larger window costs more
# per generation, so equal-generation arms would not have done equal work. This is the design
# that settled games-per-generation, where counting generations would have picked the arm that
# learned nothing.
#
# The window is PATH-DEPENDENT -- which positions are in the buffer depends on the trajectory --
# so a fixed-dataset A/B cannot ask this question. It has to be the loop, which means the 0.151
# run-to-run variance applies and a small difference will NOT resolve. Treat a null as "not
# resolved at this budget", not as "no effect".
#
# --seed 20260907 is not arbitrary: examples/control.rs reconstructs the origin as
# Net::random(hidden, 20260907), so an arm trained under a different seed would be scored against
# a DIFFERENT opponent and the comparison would be silently meaningless.
set -uo pipefail
cd "$(dirname "$0")"
SECS=${SECS:-420}
PAIRS=${PAIRS:-64}
SEED=20260907
WINDOWS=${WINDOWS:-"2 8 32"}

echo "=== replay window sweep: ${SECS}s per arm, scored vs the same origin (seed $SEED) ==="
echo "=== 2 = aggressive discard, 8 = current default, 32 = keep 4x longer ==="
for W in $WINDOWS; do
  echo "--- arm: replay-gens $W ---"
  timeout "$SECS" taskset -c 15 nice -n 19 ionice -c 3 ./target/release/learn \
    --gens 1000000 --games 2400 --threads 1 --depth 2 --epochs 3 \
    --gate-pairs 32 --arch-every 0 --control-every 0 --replay-gens "$W" \
    --seed "$SEED" --out "rw_${W}.net" --ledger "rw_${W}.jsonl" > "rw_${W}.log" 2>&1
  gens=$(grep -cE '^gen ' "rw_${W}.log")
  acc=$(grep -cE 'ACCEPT' "rw_${W}.log")
  echo "  reached generation $gens ($acc accepted) in ${SECS}s"
done

echo
echo "=== scored against the frozen origin, identical match for every arm ==="
for W in $WINDOWS; do
  [ -f "rw_${W}.net" ] || { echo "  replay-gens $W: NO CHAMPION (never accepted one)"; continue; }
  printf "  replay-gens %-3s  " "$W"
  taskset -c 15 nice -n 19 ionice -c 3 ./target/release/examples/control \
    --champion "rw_${W}.net" --pairs "$PAIRS" --seed 987654 2>/dev/null \
    | grep "fixed depth 2" | sed 's/^ *//'
done
echo "=== done ==="
