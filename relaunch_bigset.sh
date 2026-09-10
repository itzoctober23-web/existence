#!/bin/bash
# BIGGER MATE SET — test the root cause the evidence now points at, with NO rebuild.
#
# WHY. Two corrections tonight converged on the SET, not the dial:
#   * tolerance 4  -> candidates pass by SELLING mates (mates 19 of 23), VERIFY 0.422
#   * tolerance 0  -> nothing passes at all (mate-ok 0 of 8 at gen 3), the search freezes
# Both horns of that dilemma come from the set being 23 positions, where ANY behaviour change costs
# at least one mate and "keep every mate" therefore means "change nothing".
# `search_track_WHY_NOTHING.md:344` records the same thing from the other side: "0 of 33
# behaviour-changing edits pass an all-or-nothing guard".
#
# The fix a bigger set buys is not more data — it is that the guard becomes PROPORTIONAL. The
# tolerance is an ABSOLUTE count, so its licence is tolerance/|set|:
#
#     |set| = 23  (n1 12 + n2 6 + n3 5)   tolerance 4 = 17.4% of the set   <- today
#     |set| = 77  (n1 48 + n2 24 + n3 5)  tolerance 4 =  5.2% of the set   <- this arm
#
# Same absolute tolerance, a third of the licence. FITNESS §3 specifies 500 at each of MATE-1..4
# (2,000 total, stratified); 77 is a step toward that which costs ~3.3x fitness rather than ~87x.
#
# NO CODE CHANGE. `n1` and `n2` are positional args 3 and 4 (`evolve.rs:1283`, and mate_set(n) returns
# exactly n), so this is a different command line, not a different binary. Same xt_r as every other
# arm.
#
# WHAT IT REPLACES. Core 12 held guard tolerance 0, which is now measured and FROZEN: mate-ok 1, 1, 0
# across gens 1-3, and the one survivor at gens 1-2 scores rates 0.999, below best_rate, so it cannot
# gate either. That arm has already delivered its finding; leaving it running re-measures a zero.
#
# COST, stated: ~3.3x the fitness evaluation per candidate, 8 candidates per generation. Ungated
# generations should go from ~3.5 min to ~11 min. That is the price of the measurement.
#
# PRE-REGISTERED READING:
#   * mate-ok RISES and the winner's `mates` is at or near the seed's count -> the set size was the
#     binding constraint. Candidates can now change behaviour without paying a whole mate.
#   * mate-ok stays ~1 and the winner still sells mates -> set size is NOT sufficient; the guard is
#     all-or-nothing for a structural reason and §3's stratification (per-N thresholds) is required,
#     not merely a bigger pool.
#   * `mates` must be read against THIS arm's own seed count, which will NOT be 23 — a larger set
#     means a different denominator. Compare fractions, never raw counts, across arms.
set -u
BIN=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt_r/release/examples/evolve
cd /home/maswabe/existence || exit 1
[ -x "$BIN" ] || { echo "missing binary: $BIN" >&2; exit 1; }
[ -f gate_bigset_s1.log ] && mv -f gate_bigset_s1.log gate_bigset_s1.log.prev

EXISTENCE_EVOLVE_SEED=1 \
EXISTENCE_GATE_SPRT=1 \
EXISTENCE_GATE_ELO0=0 \
EXISTENCE_GATE_ELO1=30 \
EXISTENCE_GATE_MAXPAIRS=400 \
EXISTENCE_GATE_VERIFY=96 \
  taskset -c 12 nice -n 19 "$BIN" 25 8 48 24 3 > gate_bigset_s1.log 2>&1 &
echo "  core 12 -> gate_bigset_s1.log  (n1=48 n2=24, set ~77, tolerance 4 = 5.2% licence)"
