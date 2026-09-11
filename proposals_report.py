#!/usr/bin/env python3
"""Read the proposals A/B arms and answer one question: does selection get a CHOICE?

SEPARATE FROM `proposals_ab.sh` ON PURPOSE. That script was already running when its parsing bug
was found, and bash reads a script by BYTE OFFSET while executing it -- editing it mid-run has
destroyed a job on this box before. So the fix lives here instead of being patched into a live file.

THE BUG THIS EXISTS TO AVOID REPEATING. The in-script summary matched generation lines with
`^gen ` / `startswith('gen ')`. The evolve loop INDENTS them:

    "  gen   1 MAIN  ..none[above 0, gated-skip 0] (4 cand, 0 ill, mate-ok 0, rates none [...distinct:0], hard 0-0)  pop 1"

so the anchored pattern matched ZERO lines and the summary would have printed "the arm did not
run" for a perfectly healthy arm -- a false negative on the measurement itself.

WHAT IS MEASURED, and deliberately not more: `search_has_no_choice_RESULT.md` established that 94%
of generations hand selection zero or one distinct fitness, and that selection cannot select from a
set of size <= 1. So the question here is the FUNNEL SHAPE -- candidates proposed, how many survive
the mate guard, and how many DISTINCT rates result -- not whether anything was accepted. An accept
is several stages downstream, and this project has been misled by proxies four times; calling a
funnel improvement "progress toward a stronger program" would be the fifth.
"""
import re, sys

GEN = re.compile(r'^\s*gen\s+(\d+)\s+(\S+)')
CAND = re.compile(r'\((\d+) cand')
ILL = re.compile(r'(\d+) ill')
MATEOK = re.compile(r'mate-ok\s+(\d+)')
DISTINCT = re.compile(r'distinct:(\d+)')
POP = re.compile(r'pop\s+(\d+)')


def summarise(path, label):
    try:
        txt = open(path).read()
    except OSError:
        print(f"  {label:<9} NO LOG at {path}")
        return None
    rows = []
    for line in txt.split('\n'):
        m = GEN.match(line)
        if not m:
            continue
        def g(rx, d=0):
            mm = rx.search(line)
            return int(mm.group(1)) if mm else d
        rows.append(dict(gen=int(m.group(1)), lineage=m.group(2),
                         cand=g(CAND), ill=g(ILL), mate_ok=g(MATEOK),
                         distinct=g(DISTINCT), pop=g(POP)))
    # MAIN ONLY. evolve runs two INDEPENDENT lineages (MAIN and MCTS) and logs one line each per
    # generation, so counting lines as generations both inflates the denominator and pools two
    # populations that are not comparable -- MAIN proposes 4 candidates per generation here and
    # MCTS proposes 2. Read uncorrected, the control cell showed "1/7 generations with a choice";
    # the single qualifying row was an MCTS row and MAIN's true figure is 0/4.
    rows = [r for r in rows if r['lineage'] == 'MAIN']
    if not rows:
        print(f"  {label:<9} 0 generation lines parsed -- check the PATTERN before concluding the")
        print(f"  {'':<9} arm failed; these lines are INDENTED and an anchored '^gen ' finds none.")
        return None
    n = len({r['gen'] for r in rows})   # DISTINCT generations, not log lines
    tot_cand = sum(r['cand'] for r in rows)
    tot_ok = sum(r['mate_ok'] for r in rows)
    multi = sum(1 for r in rows if r['distinct'] > 1)
    anyd = sum(1 for r in rows if r['distinct'] > 0)
    popend = rows[-1]['pop']
    print(f"  {label:<9} gens {n:<3} proposed {tot_cand:<5} mate-ok {tot_ok:<4} "
          f"({100.0*tot_ok/max(tot_cand,1):4.1f}%)  gens>1 distinct {multi}/{n}  "
          f"gens>=1 distinct {anyd}/{n}  final pop {popend}")
    return dict(n=n, cand=tot_cand, ok=tot_ok, multi=multi, anyd=anyd, pop=popend)


def main():
    print("  DOES SELECTION GET A CHOICE?  (baseline: 5 of 79 generations, 6%, had >1 distinct rate)")
    c = summarise(sys.argv[1] if len(sys.argv) > 1 else "prop_control.log", "control")
    p = summarise(sys.argv[2] if len(sys.argv) > 2 else "prop_prop32.log", "prop32")
    print()
    if not c or not p:
        print("  One arm did not produce parseable generations. No comparison is made -- a missing")
        print("  arm is not a result in either direction.")
        return 1
    if c['cand'] == 0 or p['cand'] == 0:
        print("  An arm proposed ZERO candidates. That is a broken run, not a finding.")
        return 1
    print(f"  proposals per generation: control {c['cand']/c['n']:.1f}  ->  prop32 {p['cand']/p['n']:.1f}")
    print(f"  mate-ok  per generation: control {c['ok']/c['n']:.2f}  ->  prop32 {p['ok']/p['n']:.2f}")
    print(f"  generations with a CHOICE (>1 distinct): control {c['multi']}/{c['n']}  ->  prop32 {p['multi']}/{p['n']}")
    print()
    if p['multi'] > c['multi']:
        print("  DIRECTION: prop32 offers selection a choice more often. That is the funnel lever")
        print("  search_has_no_choice_RESULT.md named, moving. It is NOT an accept, and not strength.")
    elif p['ok'] > c['ok']:
        print("  PARTIAL: more guard-passing candidates, but not more generations with >1 distinct")
        print("  rate. More survivors landing on the SAME rate is not a choice -- the duplication")
        print("  stage, not the proposal stage, would then be the binding one.")
    else:
        print("  NO MOVEMENT: raising proposals did not raise guard-passers or distinct rates.")
        print("  That would point at the mutation operators producing neutral rewrites rather than")
        print("  at the proposal count -- the other half of the lever that file names.")
    print()
    print("  Sample is small by design (short arms, small sets). This measures the SHAPE of the")
    print("  funnel, not its precise size, and a single seed is one trajectory.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
