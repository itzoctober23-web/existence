#!/usr/bin/env python3
"""DOES THE GRAMMAR SEARCH'S CHEAP FITNESS PREDICT THE GAME RESULT AT ALL?

MASTER_PLAN's P2 kill condition is "no program improves on the seed by eval ~1800 -> grammar or
fitness is wrong; fix those". Measured 2026-09-10: two arms ran 13h and 7h and accepted NOTHING --
0 accepts across 19 gate decisions, all 19 verifications at or below 0.500 -- while the surrogate
rated those same candidates 26% BETTER than the seed. So the kill condition has effectively fired
and the plan says to fix the fitness.

Before redesigning it, measure it. Two redesigns I reached for on the same day both turned out to
ban capture extension, the one reference program that scores on the hard set
(surrogate_rewards_giving_up_RESULT.md), so the formula is not the place to start guessing.

THE DATA ALREADY EXISTS. Every gate line carries both the cheap predictor and the truth:

    gen 1 MAIN  gate REJECT 0.458+/-0.082 (12 games W-D-L 1-9-2)  mates 21  surrogate 0.002297 ...
                            ^^^^^ the GAME result                             ^^^^^^^^ the predictor

~460 such lines sit in the repo's evolve logs. No new run is needed to answer this.

WHY NOT A POOLED CORRELATION. Surrogate values are not comparable across logs or lineages -- they
span 0.001 to 3.3 depending on the position set, the budget and the seed program. Pooling them
would measure "which experiment was this" and report it as predictive power. So the correlation is
computed WITHIN each (log, lineage) group and only then combined, with each group z-scored so the
groups contribute on a common scale.

WHAT WOULD FALSIFY WHAT, declared before running:
  * r clearly positive  -> the surrogate carries real signal; the 0-accept runs are a threshold or
                           search problem, NOT a fitness problem, and the fitness should be left
                           alone.
  * r ~ 0 at a tight interval -> the surrogate is uninformative about the game. Steering the search
                           with it spends the game budget at random, and P2's "fitness is wrong"
                           is confirmed with a number.
  * r clearly negative  -> actively misleading: the search is being pointed AWAY from strength,
                           which would explain 0 accepts in 19 gates better than chance does.

A CEILING TO STATE UP FRONT, so a weak r is not over-read: most gate lines are 12-game matches,
whose own ci95 is about +/-0.10-0.25. That noise is in the outcome variable, so it ATTENUATES any
true correlation -- the measured r is a floor on the real one, not an estimate of it.
"""
import re, sys, os, glob, math

PAT = re.compile(
    r'gen\s+(\d+)\s+(\w+)\s+gate\s+(REJECT|PASS|ACCEPT)\D*?'
    r'([01]\.\d+)\+/-[\d.]+\s+\((\d+) games'      # game rate, game count
    r'.*?surrogate\s+([\d.]+)',                    # the predictor
    re.S)
MATES = re.compile(r'mates\s+(\d+)')

def parse(path):
    out = []
    for line in open(path, errors="replace"):
        if 'gate ' not in line:
            continue
        m = PAT.search(line)
        if not m:
            continue
        gen, lin, verdict, rate, games, sur = m.groups()
        mm = MATES.search(line)
        out.append(dict(log=os.path.basename(path), lineage=lin, gen=int(gen),
                        verdict=verdict, rate=float(rate), games=int(games),
                        sur=float(sur), mates=int(mm.group(1)) if mm else None))
    return out

def pearson(xs, ys):
    n = len(xs)
    mx, my = sum(xs)/n, sum(ys)/n
    sxy = sum((a-mx)*(b-my) for a, b in zip(xs, ys))
    sxx = sum((a-mx)**2 for a in xs)
    syy = sum((b-my)**2 for b in ys)
    if sxx == 0 or syy == 0:
        return None
    return sxy/math.sqrt(sxx*syy)

def main():
    logs = sorted(glob.glob(os.path.join(sys.argv[1] if len(sys.argv) > 1 else '.', '*.log')))
    raw = [r for p in logs for r in parse(p)]
    # DEDUP BY ROW CONTENT, NOT BY FILE. A/B arms share a lineage: gate_diversity_s1,
    # gate_div_x_filter, gate_filter_only and gate_diversity_PAIRED_off are four DIFFERENT files
    # (different sizes, different md5) whose MCTS halves are byte-identical -- the treatment only
    # moved MAIN, so the MCTS arm replayed the same 24 gates in all four. Pooling by file counted
    # that evidence four times and narrowed the interval accordingly.
    #
    # Caught because four "independent" groups reported r=+0.338 to three decimals, and two more
    # reported -0.760. Identical statistics from independent data do not happen.
    #
    # Measured: 457 gate lines on disk, 287 unique -- 37% of the corpus was duplicate.
    seen, rows = set(), []
    for r in raw:
        key = (r['lineage'], r['gen'], r['verdict'], r['rate'], r['games'], r['sur'], r['mates'])
        if key in seen:
            continue
        seen.add(key); rows.append(r)
    print(f"  deduped {len(raw)} -> {len(rows)} rows ({len(raw)-len(rows)} duplicate gate lines dropped)")
    if not rows:
        print("no gate lines parsed -- check the pattern before concluding the logs are empty")
        return
    print(f"  parsed {len(rows)} gated candidates from {len(logs)} logs")
    print(f"  median games per gate: {sorted(r['games'] for r in rows)[len(rows)//2]}")

    groups = {}
    for r in rows:
        groups.setdefault((r['log'], r['lineage']), []).append(r)  # log still separates position sets/budgets
    usable = {k: v for k, v in groups.items() if len(v) >= 4}
    print(f"  {len(groups)} (log,lineage) groups, {len(usable)} with >=4 candidates\n")

    zx, zy, per_group = [], [], []
    for k, v in sorted(usable.items()):
        xs = [r['sur'] for r in v]; ys = [r['rate'] for r in v]
        r_ = pearson(xs, ys)
        if r_ is None:
            continue
        per_group.append((k, len(v), r_))
        mx, my = sum(xs)/len(xs), sum(ys)/len(ys)
        sx = (sum((a-mx)**2 for a in xs)/len(xs))**.5
        sy = (sum((b-my)**2 for b in ys)/len(ys))**.5
        if sx == 0 or sy == 0:
            continue
        for a, b in zip(xs, ys):
            zx.append((a-mx)/sx); zy.append((b-my)/sy)

    for (log, lin), n, r_ in sorted(per_group, key=lambda t: -t[1])[:12]:
        print(f"    {log:<38} {lin:<5} n={n:<3} r={r_:+.3f}")

    if len(zx) < 8:
        print("\n  too few within-group points to pool.")
        return
    n = len(zx)
    r_ = pearson(zx, zy)
    t = r_*math.sqrt((n-2)/(1-r_*r_))
    se = 1/math.sqrt(n-3)
    z = 0.5*math.log((1+r_)/(1-r_))
    lo, hi = (math.tanh(z-1.96*se), math.tanh(z+1.96*se))
    print(f"\n  === POOLED within-group correlation, surrogate vs GAME rate ===")
    width = hi - lo
    print(f"  n={n} candidate-gate pairs   r={r_:+.4f}   t({n-2})={t:+.2f}   95% CI [{lo:+.3f}, {hi:+.3f}]")
    print(f"  interval width {width:.3f} against the 0.350 'tight' threshold declared above")
    print()
    if lo > 0.10:
        print("  VERDICT: the surrogate CARRIES SIGNAL. 0 accepts is then a threshold or search")
        print("  problem, not a fitness problem -- do not redesign the fitness on this evidence.")
    elif hi < -0.10:
        print("  VERDICT: the surrogate is ANTI-PREDICTIVE -- the search is pointed AWAY from")
        print("  strength. That explains 0 accepts in 19 gates better than chance does.")
    elif hi - lo < 0.35:
        print("  VERDICT: NO detectable relationship, at an interval tight enough to mean it.")
        print("  Steering the search by this number spends the game budget at random, which is")
        print("  P2's 'fitness is wrong' confirmed with a measurement rather than an anecdote.")
        print("  NOTE the attenuation ceiling: 12-game gates carry ci95 ~0.10-0.25, so this r is a")
        print("  FLOOR on the true one. A floor this close to zero is still the finding.")
        if width > 0.25:
            print()
            print(f"  ** BORDERLINE: width {width:.3f} only just clears the 0.350 threshold, and that")
            print("  ** threshold was my own choice. Read this as 'no evidence of prediction, and any")
            print(f"  ** true correlation is probably under {hi:+.2f}' -- NOT as a demonstrated null.")
            print("  ** The binding constraint is the 12-game gate, not the number of candidates:")
            print("  ** more gated candidates at 12 games each will not tighten this much. Wider")
            print("  ** gates would.")
    else:
        print("  UNRESOLVED: the interval is too wide to distinguish signal from none.")
        print("  This is ignorance, not a null. More gated candidates are needed before deciding.")

main()
