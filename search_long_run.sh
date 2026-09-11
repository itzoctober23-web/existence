#!/usr/bin/env bash
# DOES A CHOICE EVER BECOME AN ACCEPT? — the long prop32 arm.
#
# WHY THIS IS THE EXPERIMENT THAT MATTERS, and the funnel work was only the setup.
# `proposals_choice_RESULT.md` showed the no-choice constraint is binomial: a choice needs TWO
# guard survivors in the SAME generation, and at the measured 10.5% guard rate 4 proposals give
# P(>=2) = 0.057 while 32 give 0.863. The historical rate was 5/79 = 6.3% against a predicted 5.7%,
# so the constraint is fully explained, and the first prop32 generation produced a choice exactly
# as predicted (32 cand, mate-ok 2, distinct 2, pop 1 -> 3).
#
# But a choice is a PRECONDITION for progress, not progress. P2 has 1,062 proposals and 0 ACCEPTS,
# and an accept is several stages past the funnel: guard -> distinct rate -> EPS retention ->
# VERIFY -> game gate. Everything measured so far says only that selection is now offered
# something to choose BETWEEN. Whether anything survives the rest is unmeasured, and claiming the
# funnel fix "makes the search work" without running it out would be the fifth proxy failure this
# project has recorded.
#
# SO: run it long enough for the downstream stages to actually fire, and report the funnel AND the
# accept count, with the accept count as the primary.
#
# SIZING, measured rather than guessed: the control ran 3.2 min/generation at 4 proposals; the
# prop32 arm is running ~11 min/generation at 32. 40 generations is therefore ~7 hours. The
# timeout is set to 9 hours so the GENERATION COUNT is what stops it, never the clock -- an arm cut
# off by its timeout produces an unequal comparison, which is the failure that invalidated the
# first low-lr sweep.
#
# WAITS for the 2x2 so the box is not oversubscribed, and refuses to stack trainers.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
SNAP=$SCR/prop_ab/evolve
GENS=${GENS:-40}
POP=${POP:-4}
N1=${N1:-10}
N2=${N2:-4}
PROPOSALS=${PROPOSALS:-32}
SEED=${SEED:-7}
LOG=search_long_run.log
OUT=prop_long32.log
say(){ echo "$(date +%F_%H:%M) [longrun] $*" | tee -a "$LOG"; }

say "waiting for choice-2x2 to finish before adding load"
while systemctl --user is-active choice-2x2.service >/dev/null 2>&1; do sleep 60; done
sleep 20

[ -x "$SNAP" ] || { say "ABORT: no evolve snapshot at $SNAP"; exit 1; }
say "snapshot $(md5sum "$SNAP" | cut -c1-12)"

# A FRESH SEED. The funnel arms all used seed 1; reusing it would re-walk the same trajectory and
# an accept found there would be one seed's luck rather than a property of the configuration.
say "GENS=$GENS PROPOSALS=$PROPOSALS seed=$SEED  (fresh seed: the funnel arms all used seed 1)"
say "  sized from measurement: ~11 min/generation at 32 proposals -> ~$((GENS * 11 / 60))h"
say "  timeout 9h so the GENERATION COUNT stops it, never the clock"

EXISTENCE_EVOLVE_SEED=$SEED EXISTENCE_PROPOSALS=$PROPOSALS \
  nice -n 19 taskset -c 6-11 timeout 32400 "$SNAP" "$GENS" "$POP" "$N1" "$N2" > "$OUT" 2>&1 || true

say "RESULT — accepts are the primary, the funnel is context:"
python3 - "$OUT" <<'PY' | tee -a "$LOG"
import re, sys
GEN = re.compile(r'^\s*gen\s+(\d+)\s+(\S+)')
txt = open(sys.argv[1]).read()
rows = []
for line in txt.split('\n'):
    m = GEN.match(line)
    if not m: continue
    if m.group(2) != 'MAIN': continue      # MAIN is the lineage an accept would come from
    def g(rx, d=0):
        mm = re.search(rx, line); return int(mm.group(1)) if mm else d
    rows.append(dict(gen=int(m.group(1)), cand=g(r'\((\d+) cand'),
                     ok=g(r'mate-ok\s+(\d+)'), distinct=g(r'distinct:(\d+)'),
                     pop=g(r'pop\s+(\d+)'), line=line))
if not rows:
    print("  no MAIN generations parsed -- check the pattern before concluding the arm failed")
    raise SystemExit
n = len({r['gen'] for r in rows})
multi = sum(1 for r in rows if r['distinct'] > 1)
print(f"  generations           {n}")
print(f"  proposed              {sum(r['cand'] for r in rows)}")
print(f"  guard survivors       {sum(r['ok'] for r in rows)}")
print(f"  gens with a CHOICE    {multi}/{n}  ({100.0*multi/max(n,1):.0f}%)  -- arithmetic predicts ~86%")
print(f"  final population      {rows[-1]['pop']}")
# ACCEPTS: the loop reports a promotion distinctly from a gate rejection. Count both so a zero is
# attributable rather than ambiguous.
acc = len(re.findall(r'ACCEPT|PROMOTE|promoted', txt))
rej = len(re.findall(r'REJECT', txt))
ver = len(re.findall(r'VERIFY', txt))
print(f"  VERIFY runs           {ver}")
print(f"  gate REJECTs          {rej}")
print(f"  ACCEPTS               {acc}")
print()
if acc > 0:
    print("  AN ACCEPT. That is the first from this track; it must now be verified on its own terms")
    print("  -- an accept is a gate outcome, not yet strength. Re-gate it against the champion.")
elif (ver + rej) > 0:
    # REJECT, NOT VERIFY, is the "reached the gate" signal in this configuration. Checked against
    # the control arm before arming this: VERIFY appears ZERO times there while REJECT appears 5,
    # so branching on VERIFY alone would have fallen through to the "nothing reached the gate"
    # message while the gate was demonstrably running and rejecting.
    print("  Choices reached the GATE and were rejected. That is a different failure from the one")
    print("  this fix addressed: the funnel is no longer the constraint, the CANDIDATES are.")
    print("  The suspect moves to the mutation operators -- what they produce is reaching a real")
    print("  test and losing it.")
else:
    print("  Choices were offered but nothing reached VERIFY. The binding stage is between the")
    print("  distinct-rate step and the gate -- EPS retention or the acceptance floor, both of")
    print("  which have their own RESULT files and can now be re-read with a funnel that works.")
PY
say "SEARCHLONGDONE"
