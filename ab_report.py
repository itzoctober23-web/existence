#!/usr/bin/env python3
"""Read the HARD_FITNESS A/B without repeating today's counting error.

WHY THIS EXISTS. On 2026-09-09 I read these arms by hand and reported "4 of 4 MAIN candidates
resolved worse" plus a Spearman over n=7. Both were wrong:

  * Arms with identical configuration replay the SAME trajectory. seed-1 gen-3 appeared in three
    separate log files and seed-2 gen-1 in two, and I counted them as independent observations.
  * The pool mixed a relaxed-guard arm (tolerance 7 / MAIN floor 16) with the standard ones
    (tolerance 4 / floor 19). At tolerance 7 a candidate may SHED mates to buy cost, which is a
    different mechanism from "keep every mate, get cheaper".

So this script DEDUPLICATES by (run_seed, lineage, gen, surrogate) and STRATIFIES by both guard
tolerance and the HARD_FITNESS flag, which it reads from each log's own header rather than from the
filename. A filename is a label; the header is evidence.

WHAT IT DOES NOT DO. It prints counts and the falsifier's verdict. It does not compute a correlation:
at n=3 per lineage a Spearman is not interpretable, and quoting one is how an interval becomes a
claim.

THE FALSIFIER (pre-registered in fitness_saturation_RESULT.md): if saturation is the cause, the
treatment arms' MAIN candidates should stop being resolved WORSE by VERIFY. If they are still
resolved worse, saturation is NOT the mechanism and that document is wrong.
"""
import re, sys, glob, os

VERIFY_RE = re.compile(r'gen\s+(\d+)\s+(\w+)\s+VERIFY\s+([0-9.]+)\+/-([0-9.]+)')
GATE_RE   = re.compile(r'gen\s+(\d+)\s+(\w+)\s+gate\s+(ACCEPT|REJECT|INCONCLUSIVE)')
SURRO_RE  = re.compile(r'surrogate\s+([0-9.]+)')
SEED_RE   = re.compile(r'run seed (\d+)')
TOL_RE    = re.compile(r'guard tolerance (\d+)')
LLR_RE    = re.compile(r'llr\s+([-+0-9.]+)')

def parse(path):
    """Return (meta, rows). Reads the guard tolerance and seed from the header, not the name."""
    txt = open(path, encoding='utf-8', errors='replace').read()
    seed = int(m.group(1)) if (m := SEED_RE.search(txt)) else None
    tol  = int(m.group(1)) if (m := TOL_RE.search(txt)) else None
    # HARD_FITNESS is not printed, but it lowers the seed's own surrogate because the hard set's
    # cost enters the denominator while the seed contributes hf=0. MAIN seed: 0.002490 without,
    # 0.002084 with. That is a header-derived tell, not a filename guess.
    hard = None
    if (m := re.search(r'lineage MAIN.*?([0-9]\.[0-9]{6}) mates/Mcost', txt, re.S)):
        hard = abs(float(m.group(1)) - 0.002084) < 1e-6
    rows, pend = [], {}
    for line in txt.splitlines():
        if (m := VERIFY_RE.search(line)):
            pend[(int(m.group(1)), m.group(2))] = (float(m.group(3)), float(m.group(4)))
        elif (m := GATE_RE.search(line)):
            key = (int(m.group(1)), m.group(2))
            v, ci = pend.pop(key, (None, None))
            s  = float(sm.group(1)) if (sm := SURRO_RE.search(line)) else None
            lr = float(lm.group(1)) if (lm := LLR_RE.search(line)) else None
            rows.append(dict(seed=seed, tol=tol, hard=hard, gen=key[0], lin=key[1],
                             verify=v, ci=ci, surro=s, verdict=m.group(3), llr=lr))
    return dict(seed=seed, tol=tol, hard=hard), rows

def main():
    paths = sorted(sum((glob.glob(p) for p in
                        ('gate_hardfit_s*.log', 'gate_sprt30*.log', 'gate_control*.log',
                         'gate_veto*.log', 'gate_guardtol*.log')), []))
    if not paths:
        print("no arm logs found in", os.getcwd()); return 1
    allrows, seen = [], set()
    print("  arm logs read:")
    for p in paths:
        meta, rows = parse(p)
        hf = {True: 'HARD_FITNESS', False: 'surrogate-only', None: '?'}[meta['hard']]
        print(f"    {p:<30} seed {meta['seed']}  tolerance {meta['tol']}  {hf}  ({len(rows)} gated gens)")
        for r in rows:
            # DEDUPLICATE: identical config replays the identical trajectory.
            k = (r['seed'], r['tol'], r['hard'], r['lin'], r['gen'], r['surro'])
            if k in seen:
                continue
            seen.add(k); allrows.append(r)

    strata = {}
    for r in allrows:
        strata.setdefault((r['tol'], r['hard']), []).append(r)

    for (tol, hard), rows in sorted(strata.items(), key=lambda kv: (kv[0][0] or 0, kv[0][1] or False)):
        hf = {True: 'HARD_FITNESS=1', False: 'surrogate only', None: 'unknown'}[hard]
        print(f"\n  === guard tolerance {tol} | {hf} | {len(rows)} unique gated gens ===")
        print("    seed gen lin    surrogate   VERIFY            gate")
        for r in sorted(rows, key=lambda r: (r['lin'], r['seed'] or 0, r['gen'])):
            v = f"{r['verify']:.3f}+/-{r['ci']:.3f}" if r['verify'] is not None else "   --        "
            s = f"{r['surro']:.6f}" if r['surro'] is not None else "   --   "
            llr = f" llr {r['llr']:+.2f}" if r['llr'] is not None else ""
            print(f"    {r['seed']:>4} {r['gen']:>3} {r['lin']:<5}  {s}  {v}  {r['verdict']}{llr}")
        for lin in ('MAIN', 'MCTS'):
            sub = [r for r in rows if r['lin'] == lin and r['verify'] is not None]
            if not sub:
                continue
            worse = sum(1 for r in sub if r['verify'] + r['ci'] < 0.5)
            print(f"    {lin}: {worse}/{len(sub)} resolved WORSE by VERIFY")

    treat = [r for r in allrows if r['hard'] and r['lin'] == 'MAIN' and r['verify'] is not None]
    print("\n  === FALSIFIER ===")
    if not treat:
        print("    No MAIN VERIFY reading from a HARD_FITNESS arm yet. UNDECIDED — do not conclude.")
        return 0
    worse = sum(1 for r in treat if r['verify'] + r['ci'] < 0.5)
    print(f"    HARD_FITNESS MAIN: {worse}/{len(treat)} still resolved WORSE")
    if len(treat) < 3:
        print(f"    n={len(treat)} is too few to decide. Keep running.")
    elif worse == len(treat):
        print("    SATURATION IS REFUTED as the mechanism: the fix did not change the outcome.")
        print("    fitness_saturation_RESULT.md must be corrected.")
    elif worse == 0:
        print("    CONSISTENT with saturation being the cause.")
        print("    NOT PROOF, and the confirming check is NOT AVAILABLE from these logs: a gated")
        print("    generation prints only VERIFY and gate lines, and the `hard h-h` field appears")
        print("    only on non-gated `..none` lines. So for exactly the candidates that reach a")
        print("    gate, the log never says whether the surrogate rose via the hard set or via")
        print("    cost. It cannot be recovered arithmetically either: the guard pins f at 23, so")
        print("    rate=(23+hf)/(cost+hard_cost) has two unknowns and one equation.")
        print("    The gap is STRUCTURAL, not a missing println: hf is built at evolve.rs:1656 and")
        print("    dropped at the population boundary, because popn is Vec<(Program,u32,f64)> --")
        print("    a 3-tuple (evolve.rs:1387). Confirming the MECHANISM means widening it and")
        print("    every destructuring site, then rebuilding. Until then this reads as 'the fix")
        print("    worked', not 'saturation was why'.")
    else:
        print("    MIXED. Report the count, claim nothing.")
    return 0

if __name__ == '__main__':
    sys.exit(main())
