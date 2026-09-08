#!/usr/bin/env bash
# TRACK A's PRE-REGISTERED KILL CRITERION, as a command rather than an impression.
#
# "If after (1)-(4) the MAIN population cannot hold a probe-only or store-only intermediate for
#  >= 5 generations at eps = 0.02, raise eps once to 0.05 and record it; if it still cannot, stop
#  and write up."
#
# "Holds an intermediate" is made checkable by the tt[] counter: the number of Probe/Store/Key/
# Field nodes in each population member. Non-zero means a member carrying transposition-table
# primitives is being retained BELOW the best rate -- which is the whole mechanism. Reading it off
# a log by eye is how "the first step into the valley is being taken" got claimed from a rate
# coincidence once already.
set -uo pipefail
cd "$(dirname "$0")"
python3 - "${1:-search_track.log}" <<'PY'
import re, sys
gens=[]
for l in open(sys.argv[1], errors='ignore'):
    m=re.search(r'gen\s+(\d+) MAIN.*?spread ([0-9.]+)-([0-9.]+) tt\[([0-9, ]*)\]', l)
    if m:
        gens.append((int(m.group(1)), float(m.group(2)), float(m.group(3)),
                     [int(x) for x in m.group(4).split(',') if x.strip()]))
run=best=0
for g,lo,hi,tt in gens:
    run = run+1 if any(t>0 for t in tt) else 0
    best=max(best,run)
print(f"  MAIN generations logged: {len(gens)}")
if gens:
    g,lo,hi,tt = gens[-1]
    print(f"  latest gen {g}: tt={tt}  spread {lo:.6f}-{hi:.6f} ({lo/hi:.4f}x of best)")
print(f"  longest consecutive run holding a TT-carrying member: {best}")
if best>=5:
    print("  => CRITERION MET at eps=0.02. Do NOT raise eps; the population holds the intermediate.")
elif len(gens)<10:
    print("  => NOT YET DECIDABLE. The run is young; the criterion is about capability, not speed.")
else:
    print("  => CRITERION FAILED at eps=0.02. Raise eps ONCE to 0.05 in configs/search_track.conf,")
    print("     record the date there, and re-run. Do not raise a third time.")
PY
