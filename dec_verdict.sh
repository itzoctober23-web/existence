#!/usr/bin/env bash
# Verdict matches for the decay A/B, run OUTSIDE lr_decay_ab.sh.
#
# WHY THIS EXISTS: I launched `lr_decay_ab.sh` through `| head -5` while testing its refusal path.
# `head` exits after the 5th line, and the script's NEXT write then takes SIGPIPE -- which would
# have killed it after arm A's verdict, so arm C would never have been matched at all. The same
# SIGPIPE trap is already documented at length inside `lr_sweep.sh`, and I reintroduced it by piping
# a long-running launcher into `head`.
#
# The arms are finished and their nets are final, so nothing is lost by matching them here instead.
# A launcher should never be piped into anything.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
NM=$SCR/xt_lrd/netmatch
PAIRS=${PAIRS:-224}

for n in A B C; do
  [ -s "dec_$n.net" ] || { echo "  arm $n: no net"; continue; }
  nice -n 19 taskset -c 6-11 "$NM" "dec_$n.net" dec_start.net "$PAIRS" > "dec_${n}_vs_start.log" 2>&1
  echo "  arm $n vs shared start: $(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+ +\(interval \[0\.[0-9]+, 0\.[0-9]+\]\)' "dec_${n}_vs_start.log" | head -1)"
done
echo "  A = lr 0.002 constant (control) | B = 0.002 decaying to 0.000493 | C = lr 0.0005 constant"
echo "  same start, seed 20260912 reference: lr 0.01 -> 0.358, lr 0.002 -> 0.544, lr 0.0005 -> 0.589"
echo "DECVERDICTDONE"
