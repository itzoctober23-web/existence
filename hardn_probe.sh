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

# ---- DID THE SETTING ACTUALLY TAKE? THIS GATES THE REPORT --------------------------------------
# The 2026-09-11 run of this script set EXISTENCE_HARD_N=40 against a binary that built the set with
# a hardcoded 8 (evolve.rs:2627), measured the DEFAULT configuration, and concluded the hard set
# "must be rebuilt". The arm's own header said "HARD set: 8 positions" -- the evidence was printed
# and nothing read it.
#
# So the check runs BEFORE the reporter and EXITS on failure. A check that prints a warning and then
# reports anyway is a decoration; the whole point is that the verdict never gets written when the
# configuration under test was not the one that ran.
# Captured to a file and the status taken DIRECTLY, not through a pipe. `set -o pipefail` is on at
# line 26 and would carry the status through `| tee`, but a gate that silently becomes inert if
# someone edits the set line is the wrong shape for the one check standing between a void arm and a
# published verdict.
_chk=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/hardn_assert.txt
./assert_setting_took.py prop_hardn40.log EXISTENCE_HARD_N="${HARD_N:-40}" \
    EXISTENCE_PROPOSALS=32 EXISTENCE_HARD_FITNESS=1 > "$_chk" 2>&1
_rc=$?
cat "$_chk" | tee -a "$LOG"
if [ "$_rc" -ne 0 ]; then
  say "ABORT: a requested setting did not reach the code that ran. NOT reporting a verdict --"
  say "  the arm measured a different configuration. Rebuild the binary so EXISTENCE_HARD_N is"
  say "  honoured at evolve.rs:2627, then re-run."
  exit 1
fi
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

# ---- POWER GUARD, added 2026-09-11 after this reporter published an unsupported conclusion -----
# The original `else` branch announced "the set is not a gradient at any size reachable this way,
# the DIFFICULTY is wrong" purely from hi==0, WITHOUT COUNTING GENERATIONS. It ran 4 and said it.
#
# MAIN scores on the hard set in 2 of 28 generations at the default n_hard=8 -- a base rate of
# 0.071. At that rate P(zero in 4 generations) = 0.74: zero is the SINGLE MOST LIKELY OUTCOME and
# occurs just as readily if n_hard=40 changed nothing at all. The probe therefore could not
# distinguish "the set is inert" from "the set behaves exactly as it did at n=8" -- and the
# conclusion it printed is the one that authorises REBUILDING the hard set, which is expensive.
#
# This is the same failure that was retracted earlier the same day: "0 of 10 MAIN generations
# scored, so the HARD set is inert for MAIN" was published while the arm sat mid-run, and
# generation 11 scored. A zero is only evidence when the run was long enough for a non-zero.
BASE = 2/28          # measured across the funnel-era logs; update if the base rate is re-measured
n_gen = len({int(m.group(1)) for m in re.finditer(r'^\s*gen\s+(\d+)\s+MAIN', txt, re.M)})
p_zero = (1-BASE)**n_gen if n_gen else 1.0
print(f"  MAIN generations observed: {n_gen}   P(zero | base rate {BASE:.3f}) = {p_zero:.2f}")
print()
if hi>0:
    print(f"  MAIN SCORED {hi} on a 40-position set. The dimension is REAL and SIZE was the")
    print("  constraint -- re-open the gradient lever with EXISTENCE_HARD_N raised.")
    print("  (A positive needs no power argument: one score is an existence proof.)")
elif p_zero > 0.20:
    import math
    need = math.ceil(math.log(0.2)/math.log(1-BASE))
    print(f"  UNDERPOWERED -- REFUSING A VERDICT. {n_gen} generations cannot show a zero is")
    print(f"  meaningful when zero is expected {p_zero:.0%} of the time anyway. Need >= {need}")
    print("  generations for 80% power against the known base rate. Re-run with GENS raised;")
    print("  do NOT rebuild the hard set on this reading.")
else:
    print(f"  MAIN scores ZERO at n=40 across {n_gen} generations, where the base rate predicts a")
    print(f"  score with probability {1-p_zero:.0%}. THAT is evidence the set is not a gradient at")
    print("  this size: the DIFFICULTY is wrong, not the sample. Rebuild it around positions the")
    print("  population can PARTIALLY solve.")
PY
say "HARDNDONE"
