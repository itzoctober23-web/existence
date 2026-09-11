#!/usr/bin/env bash
# THE CONTROL ARM HAD NO VERDICT. `depth5_from_champion.sh` stays alive to run `netmatch d5.net
# d5_start.net` when its trainer's timeout expires, so the d5 arm reports itself. The d3c arm was
# launched as a bare `timeout 2400 taskset ... learn` with no surrounding script, so when its
# timeout expires the trainer simply stops and NOTHING measures it.
#
# That would leave the A/B with one arm reported and its control silent -- and
# `depth5_vs_depth3_PREREG.md` is explicit that the control is what makes the result readable:
#
#   "| loses | also loses | THE CHAMPION IS AT A CEILING for this configuration, and neither depth
#    helps. That is the more interesting outcome and it reframes the next lever as capacity or
#    search, not labels |
#    The second row is the one I would have mis-read without the control: I would have blamed
#    depth 5."
#
# Without d3c's number, "d5 lost" is unreadable: it could mean depth 5 is the wrong trade, or it
# could mean 2400 seconds from this champion was a losing proposition at ANY label depth.
#
# Same instrument and same pair count as the d5 arm, so the two verdicts are comparable:
# netmatch, 224 pairs, each arm against ITS OWN start rather than against each other.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
NM=$SCR/xt_cap/release/examples/netmatch
PID=${PID:-1301544}
PAIRS=${PAIRS:-224}

[ -x "$NM" ] || { echo "no netmatch at $NM"; exit 1; }
[ -s d3_start.net ] || { echo "no d3_start.net -- nothing to compare against"; exit 1; }

# Wait for the TRAINER to exit, so the net being judged is final. Reading it while the trainer is
# still writing would compare a net that changed partway through its own match.
while [ -d "/proc/$PID" ]; do sleep 20; done
sleep 5

G=$(grep -cE '^gen ' d3c.log 2>/dev/null)
echo "$(date '+%H:%M') d3c trainer exited at $G generations; judging against its own start"
nice -n 19 taskset -c 6-11 "$NM" d3c.net d3_start.net "$PAIRS" > d3c_vs_start.log 2>&1
line=$(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+' d3c_vs_start.log | head -1)
if [ -z "$line" ]; then
  echo "$(date '+%H:%M') d3c gen $G: netmatch produced NO RATE -- read d3c_vs_start.log. Not a verdict."
  exit 1
fi
rate=$(echo "$line" | grep -oE '0\.[0-9]+' | head -1)
ci=$(echo "$line"   | grep -oE '0\.[0-9]+' | tail -1)
# The project's own acceptance rule, stated both ways so the control can report a LOSS as clearly
# as a win -- the PREREG's second row needs "also loses" to be a nameable outcome.
v=$(python3 -c "
r,c=$rate,$ci
print('WINS vs its own start' if r-c>=0.5 else ('LOSES vs its own start' if r+c<0.5 else 'UNRESOLVED'))")
echo "$(date '+%H:%M') d3c gen $G: $v   $rate +/- $ci" | tee -a ab_verdicts.out
