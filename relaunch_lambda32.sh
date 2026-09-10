#!/bin/bash
# LAMBDA 32 — attack the GENERATION side, which is where tonight's measurement actually points.
#
# WHY. Measured over all of today's arms: 22 generations, 160 candidates,
#     ill-typed .................  16   10.0%
#     kept every mate ...........  13    8.1%
#     well-typed but LOSE mates . 131   81.9%
# At lambda 8 that is 8 x 8.1% = 0.65 VIABLE CANDIDATES PER GENERATION. The search spends most
# generations with nothing to select from, which is a supply problem, not a selection problem.
#
# Every repair proposed tonight was on the MEASUREMENT side -- gate bounds, hard-set weighting, guard
# tolerance, set size. Two of them were refuted outright. This is the first arm addressing supply.
#
# WHY THIS IS NOT ANOTHER GUESS. The failure mode is understood and documented rather than inferred:
# `mutate.rs:437` records that three random edits to a program computing the exact minimax value will
# almost always break it, with 90 of 106 rejected on the first real run. That predicts a low viable
# RATE, and a low rate is fixed by more draws, not by a better filter. Type safety is already fine
# (90% well-typed, matching mutate.rs's verified claim), so the losses are semantic, not structural.
#
# NO REBUILD. `pop` is positional arg 2 (`evolve.rs:1275`) and sets LAMBDA; MU comes from the config.
# Same xt_r binary as every other arm, different command line.
#
# WHAT IT REPLACES. Core 14 held HARD_FITNESS weight 4. That arm addresses the SATURATION mechanism,
# which was refuted at 20:50 by `mates 19` -- the numerator is sold, not stuck -- and it has not
# engaged in any generation (`hard 0-0` throughout, exactly as the gen-1..2 engagement data predicted).
# It is testing a workaround for a mechanism that does not exist as described.
#
# COST: 4x the fitness evaluations per generation, so ungated generations should go from ~3.5 min to
# ~14 min. Same total work per candidate; four times as many candidates.
#
# PRE-REGISTERED READING:
#   * viable-candidate COUNT per generation should rise ~4x (0.65 -> ~2.6). If it does not, the 8.1%
#     rate is not independent across candidates -- they share a parent, so correlated failure is a
#     real possibility and would be a finding in itself.
#   * More gates reached is EXPECTED and is NOT progress. Judge by VERIFY, never by gate count.
#   * If viable count rises but VERIFY still reads below 0.5, supply was not the constraint either,
#     and the remaining suspect is the seed itself: a program computing an exact minimax value may
#     have no nearby improvement to find at this depth.
set -u
BIN=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt_r/release/examples/evolve
cd /home/maswabe/existence || exit 1
[ -x "$BIN" ] || { echo "missing binary: $BIN" >&2; exit 1; }
[ -f gate_lam32_s1.log ] && mv -f gate_lam32_s1.log gate_lam32_s1.log.prev

EXISTENCE_EVOLVE_SEED=1 \
EXISTENCE_GATE_SPRT=1 \
EXISTENCE_GATE_ELO0=0 \
EXISTENCE_GATE_ELO1=30 \
EXISTENCE_GATE_MAXPAIRS=400 \
EXISTENCE_GATE_VERIFY=96 \
  taskset -c 14 nice -n 19 "$BIN" 25 32 12 6 3 > gate_lam32_s1.log 2>&1 &
echo "  core 14 -> gate_lam32_s1.log  (lambda 32, 4x the candidates per generation)"
