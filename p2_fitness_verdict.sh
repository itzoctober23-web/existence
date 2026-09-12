#!/usr/bin/env bash
# READ THE P2 FITNESS ARM BY THE RULE PRE-REGISTERED BEFORE IT RAN.
#
# Written 2026-09-11 23:25 while prop_disagree40 is at generation 34 of 40 and its outcome does not
# yet exist. Same reason p1_compounding_verdict.sh was written mid-run: a verdict rule chosen after
# seeing the numbers is not a rule. Everything here transcribes p2_fitness_PREREG.md and its
# amendment.
#
# IT REFUSES AN UNFINISHED ARM. A running experiment writes to STATE; a _RESULT.md requires planned N.
#
# THE PRE-REGISTERED PRIMARY: decisive-game fraction of the population's candidates, against the
# 14.7% baseline from gate_candidates_are_game_neutral_RESULT.md, with a CI excluding it.
# NOT the accept count: 6-pair arithmetic admits an accept in 14% of outcomes by luck.
#
# POWER IS QUOTED WITH THE ESTIMATE, NOT UNDER IT. The arm reaches the gate far less often than the
# control, so the decision count goes in the headline rather than a footnote.
set -uo pipefail
cd /home/maswabe/existence || exit 1
WANT=${WANT:-40}
T=${T:-prop_disagree40.log}      # treatment: 4+8+7, 21% mate-in-1
C=${C:-prop_gens40.log}          # control:  10+4+5, 53% mate-in-1  (COMPLETE, prop_gens40_RESULT.md)

g=$(grep -oE 'gen +[0-9]+' "$T" 2>/dev/null | sort -u | wc -l)
echo "treatment generations: $g of $WANT"
[ "$g" -ge "$WANT" ] || { echo "REFUSING: $T reached $g of $WANT generations. This is STATE, not a result."; exit 1; }

python3 - "$T" "$C" <<'PY'
import re,sys,math
def rows(p):
    out=[]
    for ln in open(p):
        m=re.search(r'gen\s+(\d+)\s+(MAIN|MCTS)\s+gate (ACCEPT|REJECT) ([0-9.]+)\+/-([0-9.]+) \((\d+) games W-D-L (\d+)-(\d+)-(\d+)\)', ln)
        if m:
            g,arm,v,rate,ci,ga,w,d,l=m.groups()
            out.append(dict(gen=int(g),arm=arm,verdict=v,games=int(ga),w=int(w),d=int(d),l=int(l)))
    return out
T,C = rows(sys.argv[1]), rows(sys.argv[2])
assert T, "broken parse on the treatment -- an empty result is a broken probe, never a finding"
BASE = 0.147   # gate_candidates_are_game_neutral_RESULT.md

def decisive(rs):
    g=sum(r['games'] for r in rs); dec=sum(r['w']+r['l'] for r in rs)
    if g==0: return None
    p=dec/g; se=math.sqrt(p*(1-p)/g)
    return dict(games=g,dec=dec,p=p,lo=p-1.96*se,hi=p+1.96*se,n=len(rs))

for name,rs in (("CONTROL  (10+4+5, 53% mate-in-1)",C),("TREATMENT (4+8+7, 21% mate-in-1)",T)):
    s=decisive(rs)
    if s is None:
        print(f"{name}: NO GAMES -- the primary metric is UNDEFINED, not zero"); continue
    print(f"{name}")
    print(f"   gate decisions {s['n']:>3}   games {s['games']:>4}   decisive {s['dec']:>4}"
          f"   fraction {s['p']:.3f}  [{s['lo']:.3f}, {s['hi']:.3f}]")
    print(f"   accepts {sum(1 for r in rs if r['verdict']=='ACCEPT')}")

t=decisive(T)
print()
print(f"PRE-REGISTERED PRIMARY: treatment decisive fraction vs the {BASE:.3f} baseline")
if t is None:
    print("  UNDEFINED -- the arm produced no games. Report that, not a zero.")
else:
    excl = t['lo'] > BASE or t['hi'] < BASE
    print(f"  {t['p']:.3f} [{t['lo']:.3f}, {t['hi']:.3f}] vs {BASE:.3f}"
          f"  ->  {'CI EXCLUDES the baseline' if excl else 'CI CONTAINS the baseline -- NOT a pass'}")
    print(f"  direction: {'ABOVE' if t['p']>BASE else 'at or below'} baseline")
    print()
    print(f"  POWER, quoted with the estimate: {t['n']} decisions / {t['games']} games.")
    c=decisive(C)
    if c: print(f"  The control produced {c['n']} decisions / {c['games']} games -- "
                f"{c['games']/t['games']:.1f}x more, so this arm is the weaker measurement.")
print()
print("An accept count alone does NOT pass: gate_arithmetic enumerated that 29 of 210 six-pair")
print("outcomes (14%) can accept by luck. And a null here has a pre-registered reading: if candidates")
print("barely differ behaviourally, changing what we weigh does not change what there is to weigh,")
print("and the named successor is GRAMMAR 4.")
PY
