#!/usr/bin/env bash
# IS THE HARD SET A GRADIENT, OR ONE LUCKY POSITION WEARING A PLURAL?
#
# evolve.rs:651 pre-registers this and it has never been run:
#   "8 positions with capture extension scoring 1 is not yet a gradient -- it is one position, and
#    one position is as consistent with luck as with a real signal. If the score scales with the set
#    (about 5 of 40), the dimension is real and can carry a ranking. If it stays at 1, the 'hard
#    set' is a single lucky position wearing a plural."
#
# WHY IT MATTERS NOW. proposals_choice_RESULT.md measured the HARD set INERT for the MAIN lineage:
# 0 of 10 generations had any MAIN candidate score above zero at n_hard=8. HARD_FITNESS acts only
# through the RATE of guard survivors, so a set nothing scores on adds a CONSTANT and cannot
# re-rank. The open question is whether that is the SET SIZE or the set's difficulty: 8 positions
# is a small sample, and 40 gives five times the chances to differ.
#
# harder_set() takes positions where the seed's move at `depth` differs from its move at `depth+1`,
# recording the DEEPER move as correct -- so scoring requires finding the depth+1 move while
# searching at depth. That is a narrow target, which is consistent with both explanations.
#
# PRE-REGISTERED, so the result cannot reshape the question:
#   * MAIN scores > 0 at n=40  -> the dimension is real and size was the constraint. Re-open the
#     gradient lever with EXISTENCE_HARD_N raised, and re-run the hard+p32 cell.
#   * MAIN still 0 at n=40     -> the set is not a gradient at any size reachable this way. The
#     difficulty itself is wrong and it must be rebuilt around positions the population can
#     PARTIALLY solve, not ones defined by a one-ply search gap.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
SNAP=$SCR/prop_ab/evolve
LOG=hardn_probe.log
say(){ echo "$(date +%F_%H:%M) [hardn] $*" | tee -a "$LOG"; }
[ -x "$SNAP" ] || { say "ABORT: no evolve snapshot"; exit 1; }
say "waiting for choice-2x2 so the box is not oversubscribed"
while systemctl --user is-active choice-2x2.service >/dev/null 2>&1; do sleep 30; done
sleep 10
say "n_hard=40, proposals=32, 4 generations, seed 1 (same as the 2x2 cells)"
EXISTENCE_EVOLVE_SEED=1 EXISTENCE_PROPOSALS=32 EXISTENCE_HARD_FITNESS=1 EXISTENCE_HARD_N=40 \
  nice -n 19 taskset -c 6-11 timeout 3600 "$SNAP" 4 4 10 4 > prop_hardn40.log 2>&1 || true
say "RESULT:"
python3 - <<'PY' | tee -a "$LOG"
import re
try: txt=open('prop_hardn40.log').read()
except OSError: print("  no log"); raise SystemExit
hdr=[l for l in txt.split('\n') if 'HARD set:' in l]
for l in hdr: print("  "+l.strip()[:110])
vals=set()
for l in txt.split('\n'):
    if not re.match(r'^\s*gen\s+\d+\s+MAIN', l): continue
    m=re.search(r'hard (\d+)-(\d+)', l)
    if m: vals.add((int(m.group(1)),int(m.group(2))))
print(f"  MAIN hard-set ranges seen: {sorted(vals) if vals else 'none parsed'}")
hi=max((b for _,b in vals), default=0)
print()
if hi>0:
    print(f"  MAIN SCORED {hi} on a 40-position set. The dimension is REAL and SIZE was the")
    print("  constraint -- re-open the gradient lever with EXISTENCE_HARD_N raised.")
else:
    print("  MAIN still scores ZERO at n=40, five times the positions. The set is not a gradient")
    print("  at any size reachable this way: the DIFFICULTY is wrong, not the sample. It must be")
    print("  rebuilt around positions the population can PARTIALLY solve.")
PY
say "HARDNDONE"
