#!/bin/bash
# SCHEDULED CORRECTION — switch all four arms to the FITNESS §7.2 bootstrap bounds [3,5].
#
# DO NOT RUN THIS UNTIL THE HARD_FITNESS A/B HAS RESOLVED. It is written now so the switch is one
# command instead of an analysis redone under time pressure, and so the cap is sized from the
# measurement rather than guessed at the moment of use.
#
# WHY. §7.2 fixes the acceptance criterion as HUMAN ("an instrument calibrated by its subject
# measures nothing"), fixes width e1-e0 at 2 Elo as a declared resolution constant, and bootstraps
# e1 = 5 until 20 acceptances exist. There are zero acceptances, so the compliant bound is [3,5].
# The draft §7.2 criticises for being hand-picked is [0,10] -- where I started -- and [0,30] is a
# further 3x hand-picked widening.
#
# AND IT IS NOT MERELY COMPLIANCE. [0,30] tests H1: elo >= 30, so it REJECTS a genuine +10 Elo
# candidate 72.5% of the time. Simulated, 400 runs/cell, LLR transcribed from gate.rs:
#
#     true elo   [0,30] cap 400        [3,5] uncapped
#         -55    100% REJ,  11 pairs   100% REJ,   255 pairs   1.4 h
#         -25    100% REJ,  23 pairs   100% REJ,   591 pairs   3.3 h
#           0      5% acc (= alpha)    100% REJ,  3861 pairs  21.4 h
#         +10     25% acc  <-- MISSES  100% ACC,  2687 pairs  14.9 h
#         +50   98.5% acc               100% ACC,   332 pairs   1.8 h
#
# THE CAP IS SIZED ABOVE THE DISTANCE TO THE BOUND, which is the 4PC lesson stated in OPEN_LEADS:
# "a cap below the distance to the bound is not a test" -- SPSA round 3 was filed as inconclusive
# for exactly that reason and turned out to be a PASS. 4000 covers every cell above including the
# 3861-pair worst case at true 0. It binds only on a genuinely neutral candidate; the population
# actually observed (-25 to -55) resolves in 255-591 pairs.
#
# COST, stated honestly: ~1.4-3.3 h per gate on the observed population against minutes at [0,30].
# The informative window is gens 3-6, so ~4 gated generations = 6-13 h. An overnight run, not an
# afternoon one. That is the price of a gate that can see a +10 gain, and the spec is explicit that
# a candidate near a bound deserves thousands of pairs.
set -u
BIN=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt_r/release/examples/evolve
cd /home/maswabe/existence || exit 1
[ -x "$BIN" ] || { echo "missing binary: $BIN" >&2; exit 1; }

for f in gate_sprt30_s1.log gate_sprt30_eps_s1.log gate_hardfit_s1.log gate_hardw4_s1.log; do
  [ -f "$f" ] && mv -f "$f" "$f.bounds0_30"
done

common() {
  EXISTENCE_EVOLVE_SEED=1 \
  EXISTENCE_GATE_SPRT=1 \
  EXISTENCE_GATE_ELO0=3 \
  EXISTENCE_GATE_ELO1=5 \
  EXISTENCE_GATE_MAXPAIRS=4000 \
  EXISTENCE_GATE_VERIFY=96 "$@"
}

common taskset -c 15 nice -n 19 "$BIN" 25 8 12 6 3 > gate_sprt30_s1.log 2>&1 &
EXISTENCE_EPS=0.10 common taskset -c 13 nice -n 19 "$BIN" 25 8 12 6 3 > gate_sprt30_eps_s1.log 2>&1 &
EXISTENCE_HARD_FITNESS=1 EXISTENCE_HARD_WEIGHT=1 common taskset -c 12 nice -n 19 "$BIN" 25 8 12 6 3 > gate_hardfit_s1.log 2>&1 &
EXISTENCE_HARD_FITNESS=1 EXISTENCE_HARD_WEIGHT=4 common taskset -c 14 nice -n 19 "$BIN" 25 8 12 6 3 > gate_hardw4_s1.log 2>&1 &
echo "  relaunched all four on FITNESS 7.2 bootstrap bounds [3,5], cap 4000"
