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

THE FALSIFIER (pre-registered in fitness_saturation_RESULT.md), in its CORRECTED three-way form. The
original two-way version was wrong and is kept here as the reason the third branch exists:

  * treatment MAIN stops being resolved WORSE            -> consistent with saturation
  * still worse AND hf > 0 reached the gate              -> saturation REFUTED as the mechanism
  * still worse AND hf stayed 0                          -> UNINTERPRETABLE, the fix never engaged

The third branch is not a technicality. With hf = 0 the treatment surrogate is
(23+0)/(cost+hard_cost), improvable only by cutting cost -- identical in incentive to the control's
23/cost. Every treatment generation so far reads `hard 0-0`, and no arm has EVER scored on the hard
set before gen 3, so the third branch is the expected early state, not a remote one.

DOSE-RESPONSE. Two treatment arms now run at HARD_WEIGHT 1 and 4 on the same run seed. The
arithmetic predicts weight 1 is too weak to outbid cost-cutting: the hard set's ceiling is 2 of 8, so
its best boost is (23+2)/23 = +8.7%, while a 10% cost cut is already +9.1% and the observed cost
gains were +13.1%..+22.3%. If weight 1 is inert and weight 4 is not, the diagnosis was right and only
the dose was too small -- which is a DIFFERENT repair from "saturation was the wrong mechanism".
"""
import re, sys, glob, os

VERIFY_RE = re.compile(r'gen\s+(\d+)\s+(\w+)\s+VERIFY\s+([0-9.]+)\+/-([0-9.]+)')
GATE_RE   = re.compile(r'gen\s+(\d+)\s+(\w+)\s+gate\s+(ACCEPT|REJECT|INCONCLUSIVE)')
SURRO_RE  = re.compile(r'surrogate\s+([0-9.]+)')
SEED_RE   = re.compile(r'run seed (\d+)')
TOL_RE    = re.compile(r'guard tolerance (\d+)')
# EPS is part of the ARM IDENTITY and must be in the dedup key. The control and the EPS 0.10
# arm are byte-identical until a candidate lands in the 90-98%% band, so they currently share
# every other key field -- and the moment EPS diverges its generations would collide with the
# control's and overwrite them. Third instance today of a dedup key that was too coarse.
EPS_RE    = re.compile(r'EPS=([0-9.]+)')
LLR_RE    = re.compile(r'llr\s+([-+0-9.]+)')

def parse(path):
    """Return (meta, rows). Reads the guard tolerance and seed from the header, not the name."""
    txt = open(path, encoding='utf-8', errors='replace').read()
    seed = int(m.group(1)) if (m := SEED_RE.search(txt)) else None
    tol  = int(m.group(1)) if (m := TOL_RE.search(txt)) else None
    eps  = float(m.group(1)) if (m := EPS_RE.search(txt)) else None
    # PREFER THE EXPLICIT HEADER. Arms now print "HARD_FITNESS on weight N" / "HARD_FITNESS off".
    #
    # The old heuristic below inferred the FLAG from the seed's own surrogate (0.002490 without the
    # hard set in the denominator, 0.002084 with it). That is correct for the flag and BLIND TO THE
    # WEIGHT, because the seed scores hf=0 and w*0 = 0 at every weight -- so a weight-1 and a
    # weight-4 arm looked identical and would be pooled as one condition. Same class of error as
    # counting duplicate trajectories as independent observations. Kept as a fallback so logs written
    # before the header existed still parse.
    # `hdr` records HOW the flag was determined, which is load-bearing for the gating rate below.
    # The explicit header was added at the same time as the no-ratchet fix, and every log that
    # predates it was renamed out of the glob -- so among the files read here, an explicit header is
    # exactly equivalent to "produced by the no-ratchet binary". Without this, the old ratcheting
    # control arms share the dedup key (seed, tol, hard, hw, lin, gen) with the new control and get
    # POOLED, which would silently mix the two sides of the very comparison being made.
    hard, hw, hdr = None, None, False
    if (m := re.search(r'HARD_FITNESS on weight ([0-9.]+)', txt)):
        hard, hw, hdr = True, float(m.group(1)), True
    elif re.search(r'HARD_FITNESS off', txt):
        hard, hw, hdr = False, None, True
    elif (m := re.search(r'lineage MAIN.*?([0-9]\.[0-9]{6}) mates/Mcost', txt, re.S)):
        hard = abs(float(m.group(1)) - 0.002084) < 1e-6
        hw = 1.0 if hard else None   # legacy logs predate the weight knob, which defaulted to 1
    rows, pend, gens = [], {}, []
    # EVERY lineage-generation, gated or not. Needed for the gating RATE, which is a
    # PRE-REGISTERED prediction of removing the best_rate ratchet (see
    # search_track_WHY_NOTHING.md): it should rise materially from the measured 99/234 =
    # 42.3%. Checked by the tool rather than by hand, so it cannot be quietly skipped.
    GEN_ANY = re.compile(r'gen\s+(\d+)\s+([A-Z]+)\s+(\.\.none|gate )')
    for line in txt.splitlines():
        g = GEN_ANY.search(line)
        if g:
            gens.append((int(g.group(1)), g.group(2), g.group(3).startswith('gate')))
    for line in txt.splitlines():
        if (m := VERIFY_RE.search(line)):
            pend[(int(m.group(1)), m.group(2))] = (float(m.group(3)), float(m.group(4)))
        elif (m := GATE_RE.search(line)):
            key = (int(m.group(1)), m.group(2))
            v, ci = pend.pop(key, (None, None))
            s  = float(sm.group(1)) if (sm := SURRO_RE.search(line)) else None
            lr = float(lm.group(1)) if (lm := LLR_RE.search(line)) else None
            rows.append(dict(seed=seed, tol=tol, eps=eps, hard=hard, hw=hw, gen=key[0], lin=key[1],
                             verify=v, ci=ci, surro=s, verdict=m.group(3), llr=lr))
    return dict(seed=seed, tol=tol, eps=eps, hard=hard, hw=hw, hdr=hdr, gens=gens), rows

def main():
    # NOTE: `gate_hardw*` must be here. It was missed when the weight-4 arm was added, which would
    # have silently dropped the entire dose-response arm from the report -- a grep that finds nothing
    # because the pattern is wrong, not because the data is absent.
    paths = sorted(sum((glob.glob(p) for p in
                        ('gate_hardfit_s*.log', 'gate_hardw*.log', 'gate_sprt30*.log',
                         'gate_control*.log', 'gate_veto*.log', 'gate_guardtol*.log')), []))
    if not paths:
        print("no arm logs found in", os.getcwd()); return 1
    allrows, seen = [], set()
    gseen = {}
    print("  arm logs read:")
    for p in paths:
        meta, rows = parse(p)
        hf = ('HARD_FITNESS w=%g' % meta['hw']) if meta['hard'] else ('surrogate-only' if meta['hard'] is False else '?')
        print(f"    {p:<30} seed {meta['seed']}  tol {meta['tol']}  EPS {meta['eps']}  {hf}  ({len(rows)} gated gens)")
        for r in rows:
            # DEDUPLICATE: identical config replays the identical trajectory.
            k = (r['seed'], r['tol'], r['eps'], r['hard'], r['hw'], r['lin'], r['gen'], r['surro'])
            if k in seen:
                continue
            seen.add(k); allrows.append(r)
        if meta['hdr']:      # post-ratchet-fix arms ONLY -- see the note in parse()
            for (gn, lin, gated) in meta['gens']:
                gseen[(meta['seed'], meta['tol'], meta['eps'], meta['hard'], meta['hw'], lin, gn)] = gated

    strata = {}
    for r in allrows:
        strata.setdefault((r['tol'], r['eps'], r['hard'], r['hw']), []).append(r)

    for (tol, eps, hard, hw), rows in sorted(strata.items(), key=lambda kv: (kv[0][0] or 0, kv[0][1] or 0, kv[0][2] or False, kv[0][3] or 0)):
        hf = ('HARD_FITNESS weight %g' % hw) if hard else ('surrogate only' if hard is False else 'unknown')
        print(f"\n  === guard tolerance {tol} | EPS {eps} | {hf} | {len(rows)} unique gated gens ===")
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

    if gseen:
        tot = len(gseen); gated = sum(1 for v in gseen.values() if v)
        print(f"\n  === GATING RATE (pre-registered: should RISE from 42.3% now the ratchet is gone) ===")
        print(f"    unique lineage-generations : {tot}   (post-ratchet-fix arms only)")
        print(f"    of those, reached a gate   : {gated}  ({100*gated/tot:.1f}%)")
        print(f"    baseline WITH the ratchet  : 99/234 = 42.3%")
        if tot < 30:
            print(f"    n={tot} is too few to compare. Keep running.")

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
        print("    Treatment MAIN is STILL resolved worse. Two readings, and they are NOT the same:")
        print("      * if hf>0 reached the gate -> saturation is REFUTED as the mechanism and")
        print("        fitness_saturation_RESULT.md must be corrected;")
        print("      * if hf stayed 0 -> UNINTERPRETABLE. With hf=0 the surrogate is")
        print("        (23+0)/(cost+hard_cost), improvable only by cutting cost -- identical in")
        print("        incentive to the control's 23/cost. The fix never engaged.")
        print("    Read `hard hlo-hhi` ON THE GATE LINE to tell them apart:")
        print("      hhi == 0  -> the fix did NOT engage. Unambiguous.")
        print("      hlo >  0  -> every retained member scored, so the winner did. Unambiguous.")
        print("      hlo == 0 < hhi -> AMBIGUOUS: some candidate scored, but the winner may still")
        print("                        be a cost-cutter with hf=0, which is what the weight-1")
        print("                        under-power prediction expects.")
        print("    Compare the weight-1 and weight-4 arms before concluding: if weight 1 is inert")
        print("    and weight 4 is not, the diagnosis was right and only the dose was too small.")
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
