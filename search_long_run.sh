#!/usr/bin/env bash
# IS THE 6-PAIR GATE REJECTING GOOD CANDIDATES, OR ARE THE CANDIDATES NOT GOOD?
#
# This REPLACES the 40-generation "does a choice ever become an accept" arm, which was stopped at
# generation 1 on 2026-09-11. The reason it was stopped is that its headline metric was already
# decided by arithmetic, so seven hours would have bought a number that could be computed for free.
#
# WHAT WAS PROVEN BEFORE STOPPING IT (exact enumeration, not a trend, not a sample):
# The gate accepts on `pent_rate - ci95 > 0.5` over 6 pairs. Enumerating ALL 210 possible 6-pair
# outcomes and applying the gate's OWN formula from `gate.rs` (pentanomial mean/2, sample variance
# over n-1, ci95 = 1.96*sqrt(var/n)/2, with the 1.5/n rule-of-three when variance is zero):
#
#     outcomes that can ever ACCEPT                      29 of 210  (14%)
#     ceiling for a candidate drawing >=4 of 6 pairs     0.4800  -- CANNOT PASS, at any decisive result
#     ceiling for a candidate drawing >=5 of 6 pairs     0.4600  -- CANNOT PASS
#
# and the 21 completed gate matches on disk drew 216 of 252 games (85.7%), every one of them inside
# that unpassable region. The formula was validated against real output first: pent [0,1,5,0,0]
# yields 0.4583 +/- 0.0817, which is the `0.458+/-0.082` the logs print.
#
# So "0 accepts" was never going to be evidence about the candidates. Worse, the old reporter's
# middle branch said a rejection means "the funnel is no longer the constraint, the CANDIDATES are"
# -- a conclusion this arithmetic contradicts. Running it would have produced a correct observation
# wearing a wrong interpretation.
#
# THE PERVERSITY WORTH KEEPING IN MIND: winning a pair OUTRIGHT inflates the sample variance, which
# widens ci95, which raises the bar against the candidate that earned it. `5 drawn + 1 pair won`
# scores 0.5833 and is REJECTED; `3 half-wins + 3 draws` scores 0.6250 and PASSES. The rule does not
# reward strength, it rewards CONSISTENCY, and 6 pairs is too few to show consistency.
#
# SO THE OPEN QUESTION IS NOT "does a choice become an accept". It is the one the code itself says
# the rule A/B cannot answer (evolve.rs ~3429): "A promotion at gate 0.500 might be a real
# improvement the strict rule wrongly rejected, or noise the veto wrongly admitted, and the gate
# score cannot distinguish those."
#
# EXISTENCE_GATE_VERIFY=96 is exactly the instrument for that, and it is already built. It re-matches
# the SAME candidate against the SAME champion at 96 pairs with a DIFFERENT seed -- an independent
# sample, not a longer version of the same one -- where ci95 is ~0.047 and a true 0.55 separates from
# a true 0.500. It is an OBSERVER: the code comment says "CHANGES NOTHING", so the trajectory here is
# byte-identical to the shipped rule's. Nothing is promoted that would not have been promoted.
#
# WHY OBSERVE BEFORE SWITCHING THE RULE. `gate_pairs` is argument 7 of the binary, so the 6-pair gate
# is a CONFIGURATION, not a law -- it can simply be raised. But raising it costs time proportionally
# on every candidate. Re-scoring the 21 matches on disk under the veto rule gives 20/21 promotions
# including candidates at 0.417 (losing 10 of 12 games), so the veto is not a usable replacement
# either: one rule resolves nothing up, the other resolves nothing down, because at 6 pairs ci95 is
# 0.08-0.25 and BOTH are thresholding noise. VERIFY tells us whether spending those pairs would
# change any decision, before spending them.
#
# PRE-REGISTERED, so the numbers cannot choose the question afterwards:
#   * rejected candidates verify at or BELOW 0.5 (interval not clear of it)  -> the strict rule is
#     RIGHT. Nothing good is being discarded, the gate is not the constraint, and the suspect moves
#     to the mutation operators. Raising gate_pairs would buy precision on a true negative.
#   * rejected candidates verify CLEAR OF 0.5 on the upside -> the 6-pair gate is discarding real
#     improvements, and the fix is PAIRS, not the acceptance rule. Re-arm with gate_pairs raised and
#     re-measure; do NOT switch to the veto rule, which admits 0.417.
#   * VERIFY and the gate disagree in BOTH directions -> 6 pairs is pure noise and no rule over it
#     can work. Same fix, stronger statement.
#   * fewer than 4 VERIFY readings -> UNDERPOWERED. Report the readings and the count, claim nothing.
#
# SIZING. The observer is ~16x the gate and is paid on every gate call, which at 32 proposals is
# roughly once per generation. 15 generations is sized to land in about the same wall clock the
# 40-generation arm would have used. GENERATION COUNT stops this, never the clock: an arm cut off by
# its timeout produces an unequal comparison, which is the failure that invalidated the first low-lr
# sweep.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
SNAP=$SCR/prop_ab/evolve
GENS=${GENS:-15}
POP=${POP:-4}
N1=${N1:-10}
N2=${N2:-4}
PROPOSALS=${PROPOSALS:-32}
VERIFY=${VERIFY:-96}
SEED=${SEED:-7}
LOG=search_long_run.log
OUT=prop_verify96.log
say(){ echo "$(date +%F_%H:%M) [verify96] $*" | tee -a "$LOG"; }

# WAIT FOR EVERY COMPETING EVOLVE ARM, not just the 2x2. hardn-probe launched the moment choice-2x2
# finished and would have been invisible to a 2x2-only wait, putting two 32-proposal arms plus the
# production trainers on the same 12 cores.
for u in choice-2x2 hardn-probe; do
  if systemctl --user is-active "$u.service" >/dev/null 2>&1; then
    say "waiting for $u to finish before adding load"
    while systemctl --user is-active "$u.service" >/dev/null 2>&1; do sleep 60; done
  fi
done
sleep 20

[ -x "$SNAP" ] || { say "ABORT: no evolve snapshot at $SNAP"; exit 1; }
say "snapshot $(md5sum "$SNAP" | cut -c1-12)"
say "GENS=$GENS PROPOSALS=$PROPOSALS VERIFY=$VERIFY pairs seed=$SEED"
say "  acceptance rule UNCHANGED (shipped). VERIFY is an observer -- it decides nothing."

EXISTENCE_EVOLVE_SEED=$SEED EXISTENCE_PROPOSALS=$PROPOSALS EXISTENCE_GATE_VERIFY=$VERIFY \
  nice -n 19 taskset -c 6-11 timeout 32400 "$SNAP" "$GENS" "$POP" "$N1" "$N2" > "$OUT" 2>&1 || true

say "RESULT — the 96-pair reading is the primary; the 6-pair gate decision is what it judges:"
python3 - "$OUT" <<'PY' | tee -a "$LOG"
import re, sys
txt = open(sys.argv[1]).read()
VER  = re.compile(r'gen\s+(\d+)\s+(\S+)\s+VERIFY\s+([\d.]+)\+/-([\d.]+)\s+\((\d+) pairs')
GATE = re.compile(r'gen\s+(\d+)\s+(\S+)\s+gate\s+(REJECT|ACCEPT)\s+([\d.]+)\+/-([\d.]+)')
ver  = {(int(m[0]), m[1]): (float(m[2]), float(m[3]), int(m[4])) for m in VER.findall(txt)}
gate = {(int(m[0]), m[1]): (m[2], float(m[3]), float(m[4])) for m in GATE.findall(txt)}

gens = len({int(m.group(1)) for m in re.finditer(r'^\s*gen\s+(\d+)\s', txt, re.M)})
print(f"  generations completed   {gens}")
print(f"  gate calls              {len(gate)}")
print(f"  VERIFY readings         {len(ver)}")
if not ver:
    # An empty result here is almost always a broken pattern, not an absent phenomenon -- say which.
    print("  NO VERIFY LINES PARSED. Either no gate call happened (VERIFY is paid only on a gate")
    print("  call), or the print format moved. Check the raw log for the word VERIFY before")
    print("  concluding the observer found nothing.")
    raise SystemExit

paired = sorted(k for k in ver if k in gate)
print()
print("  gen  lin    6-pair gate (decides)      96-pair verify (truth)     agree?")
disc_up = disc_dn = 0
for k in paired:
    d, gr, gc = gate[k]
    vr, vc, vp = ver[k]
    vup, vdn = vr - vc > 0.5, vr + vc < 0.5
    truth = "BETTER" if vup else ("WORSE" if vdn else "tie")
    if d == "REJECT" and vup: disc_up += 1; flag = "<< GATE THREW AWAY A WINNER"
    elif d == "ACCEPT" and vdn: disc_dn += 1; flag = "<< GATE ADMITTED A LOSER"
    else: flag = ""
    print(f"  {k[0]:>3}  {k[1]:<5}  {d:<6} {gr:.3f}+/-{gc:.3f}      {vr:.3f}+/-{vc:.3f} ({vp}p) {truth:<6} {flag}")

n = len(paired)
print()
if n < 4:
    print(f"  UNDERPOWERED: {n} paired readings. Reporting them and claiming nothing -- the")
    print("  pre-registered floor is 4.")
elif disc_up == 0 and disc_dn == 0:
    print("  THE GATE AND THE 96-PAIR MEASUREMENT AGREE ON EVERY CANDIDATE.")
    print("  Nothing good is being discarded. The 6-pair gate is NOT the binding constraint, and")
    print("  raising gate_pairs would buy precision on a true negative. The suspect moves to the")
    print("  MUTATION OPERATORS: what they produce reaches a fair test and is genuinely not better.")
elif disc_up > 0:
    print(f"  THE GATE DISCARDED {disc_up} CANDIDATE(S) THAT 96 PAIRS RESOLVES AS BETTER.")
    print("  The binding constraint is the gate's SAMPLE SIZE. The fix is PAIRS (gate_pairs is")
    print("  argument 7), not the acceptance rule -- re-scoring the matches on disk shows the veto")
    print("  rule admits 20/21 including candidates at 0.417, so it trades this error for a worse one.")
else:
    print(f"  {disc_dn} candidate(s) the gate ACCEPTED verify as worse. 6 pairs is admitting noise.")
PY
say "VERIFY96DONE"
