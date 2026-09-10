#!/bin/bash
# SCHEDULED — run FITNESS §3's actual rule: the surrogate as a FILTER, not a ranking function.
#
# DO NOT RUN UNTIL THE HARD_FITNESS DOSE-RESPONSE HAS RESOLVED. Written now so the switch is one
# command, and because the reasoning is easier to get right while the evidence is in front of me.
#
# WHY THIS IS THE SPEC AND NOT AN IDEA. FITNESS §3 defines mates-per-cost with
#   "Role: (a) a filter — a PROGRAM candidate must score >= 0.9x the champion on MATE-{1,2} and
#    >= 0.8x on MATE-{3,4} to reach the ladder"
# and closes "Not a substitute for Elo." The shipped default RANKS by it and sends only popn[0] to
# the gate under a strict `rate > best_rate`, which is using it as a substitute for Elo.
#
# WHY THE ROLE IS THE CONSEQUENTIAL PART. A filter cannot be gamed by cheapness — once a candidate
# is over the bar, being cheaper buys nothing. A RANKING function rewards cheapness without limit,
# and that is the measured failure: mates/Mcost is a SPEED metric, and the only candidate ever
# accepted in this project improved it 31.4% while holding the seed's own mate count (STATE.md:2668,
# "ACCEPT 15 mates"), with VERIFY reading 0.490.
#
# WHY IT IS SAFE TO RUN NOW, WHICH IT WAS NOT THIS MORNING. evolve.rs:1399 warns that under a
# TOLERANCE filter a bar-raise ratchets the reference DOWNWARD — every rejection lowers what the next
# 0.9x is measured against. That hazard is gone: both reject paths now record into `gated` and
# NEITHER touches best_rate (verified — the only two writes left are the ACCEPT paths at 1893 and
# 2190). So the filter's reference stays pinned to the champion.
#
# ONE VARIABLE. Same binary, same run seed, same [0,30] bounds, same 96-pair VERIFY as the control it
# will be compared against. The only change is EXISTENCE_SPEC_FILTER=1.
#
# WHAT IT DOES NOT FIX. §3 also specifies 500 positions at each of MATE-1..4 (2,000 total, stratified,
# reported per N); the run builds 23. This arm tests the ROLE inversion alone, which costs nothing to
# change. The set-size deviation needs code and is a separate question — and §3's set must be "mined
# by retrograde walk from actual game endings in own self-play", of which there is little at
# iteration zero, so the small set may be a deliberate bootstrap.
#
# PRE-REGISTERED READING. Under the strict rule the champion has NEVER moved in any standard arm, and
# PATH 1 has fired 8 times in this project's history, all 8 under SPEC_FILTER. So a champion that
# moves here is expected and is NOT by itself evidence of strength — the ladder decides that, which
# is exactly §3's point. What matters is whether VERIFY on those promotions stops reading below 0.5.
set -u
BIN=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt_sf/release/examples/evolve
cd /home/maswabe/existence || exit 1
[ -x "$BIN" ] || { echo "missing binary: $BIN" >&2; exit 1; }
[ -f gate_specfilter_s1.log ] && mv -f gate_specfilter_s1.log gate_specfilter_s1.log.prev

EXISTENCE_SPEC_FILTER=1 \
EXISTENCE_EVOLVE_SEED=1 \
EXISTENCE_GATE_SPRT=1 \
EXISTENCE_GATE_ELO0=0 \
EXISTENCE_GATE_ELO1=30 \
EXISTENCE_GATE_MAXPAIRS=400 \
EXISTENCE_GATE_VERIFY=96 \
  taskset -c "${1:-14}" nice -n 19 "$BIN" 25 8 12 6 3 > gate_specfilter_s1.log 2>&1 &
echo "  spec-filter arm launched on core ${1:-14} -> gate_specfilter_s1.log"
