#!/usr/bin/env python3
"""Read all four cells of the choice 2x2 and say which lever moves DISTINCT fitness values.

The quantity that is short is not candidates and not survivors -- it is DISTINCT fitness values
per generation. `search_has_no_choice_RESULT.md`: 94% of generations hand selection zero or one,
and selection cannot select from a set of size <= 1.

So `gens>1 distinct` is the PRIMARY column. `mate-ok` is secondary and can move without it: more
survivors landing on one identical rate is still no choice, and that distinction is the whole
reason this is a 2x2 rather than two separate A/Bs.

Generation lines are INDENTED by the evolve loop. An anchored '^gen ' matches ZERO of them and
would report a healthy arm as "did not run" -- that bug is why this reporter exists as a separate
file from the scripts that produce the logs.
"""
import re, os, sys

GEN = re.compile(r'^\s*gen\s+(\d+)\s+(\S+)')
FIELDS = {
    'cand': re.compile(r'\((\d+) cand'),
    'ill': re.compile(r'(\d+) ill'),
    'mate_ok': re.compile(r'mate-ok\s+(\d+)'),
    'distinct': re.compile(r'distinct:(\d+)'),
    'pop': re.compile(r'pop\s+(\d+)'),
}

CELLS = [
    ("prop_control.log",  "control",  "proposals=pop, guard set"),
    ("prop_prop32.log",   "prop32",   "proposals=32,  guard set"),
    ("prop_hard.log",     "hard",     "proposals=pop, HARD set *"),
    ("prop_hardp32.log",  "hard+p32", "proposals=32,  HARD set"),
]

# * THE `hard` CELL CANNOT SHOW A CHOICE, BY CONSTRUCTION -- checked in the source, not assumed.
#   evolve.rs:2758-2762 returns (f, cc, new_rate) under EXISTENCE_HARD_FITNESS: `f` is UNCHANGED and
#   only the cost and the RATE move. The mate guard tests `f >= guard_floor` (evolve.rs:1795), which
#   HARD never touches. So HARD cannot alter how many candidates SURVIVE -- it can only re-rank the
#   ones that do. At proposals=pop the arm yields ~0.4 survivors per generation, and a single
#   survivor has exactly one rate however it is computed.
#
#   Therefore `hard` reading 0/N is GUARANTEED and is NOT evidence against the gradient lever. Only
#   `hard+p32`, where 32 proposals produce several survivors to rank, can test it. This was a design
#   weakness in the 2x2: one cell was uninformative before it ran.


def parse(path):
    """Rows keyed by (generation, LINEAGE).

    EVOLVE RUNS TWO INDEPENDENT LINEAGES -- MAIN and MCTS -- and logs ONE LINE EACH per
    generation. Counting lines as generations is a unit error twice over: it inflates the
    denominator (7 lines were only 4 generations) and it POOLS two populations that are not
    comparable -- in the control arm MAIN proposes 4 candidates per generation and MCTS proposes 2,
    with different seeds and different reference programs. A "1/7" built that way understates a
    1/4 and mixes two searches into one fraction.
    """
    if not os.path.exists(path):
        return None
    rows = []
    for line in open(path):
        m = GEN.match(line)
        if not m:
            continue
        r = {'gen': int(m.group(1)), 'lineage': m.group(2)}
        for k, rx in FIELDS.items():
            mm = rx.search(line)
            r[k] = int(mm.group(1)) if mm else 0
        rows.append(r)
    return rows or None


def by_lineage(rows):
    out = {}
    for r in rows:
        out.setdefault(r['lineage'], []).append(r)
    return out


def main():
    print("  WHICH LEVER GIVES SELECTION A CHOICE?")
    print("  baseline on file: 5 of 79 generations (6%) had more than one distinct fitness")
    print()
    print("  (MAIN lineage only -- MCTS is a separate search with its own population and is not")
    print("   pooled with it; generations are DISTINCT gen numbers, not log lines)")
    print(f"  {'cell':<10} {'configuration':<26} {'gens':>4} {'prop':>5} {'ok':>4} "
          f"{'>1 distinct':>12} {'>=1':>5} {'pop':>4}")
    res = {}
    for path, lab, cfg in CELLS:
        rows = parse(path)
        if not rows:
            print(f"  {lab:<10} {cfg:<26} {'--':>4}  (no parseable generations)")
            continue
        lins = by_lineage(rows)
        # MAIN is the lineage the search track's accepts would come from; report it as the cell's
        # headline and show MCTS beside it rather than averaging two different searches together.
        main = lins.get('MAIN', [])
        if not main:
            print(f"  {lab:<10} {cfg:<26} (no MAIN lineage rows)")
            continue
        n = len({r['gen'] for r in main})
        prop = sum(r['cand'] for r in main)
        ok = sum(r['mate_ok'] for r in main)
        multi = sum(1 for r in main if r['distinct'] > 1)
        anyd = sum(1 for r in main if r['distinct'] > 0)
        res[lab] = dict(n=n, prop=prop, ok=ok, multi=multi, anyd=anyd, pop=main[-1]['pop'],
                        rows=main,
                        others={k: len({r['gen'] for r in v}) for k, v in lins.items() if k != 'MAIN'})
        print(f"  {lab:<10} {cfg:<26} {n:>4} {prop:>5} {ok:>4} "
              f"{str(multi)+'/'+str(n):>12} {anyd:>5} {main[-1]['pop']:>4}")
    print()
    if len(res) < 2:
        print("  Fewer than two cells have run. No comparison is made -- a missing arm is not a")
        print("  result in either direction.")
        return 1

    # UNEQUAL ARMS. Each cell is wrapped in a `timeout`, and the prop32 cells propose 8x the
    # candidates per generation, so they can be CUT OFF after fewer generations than the control.
    # Comparing a 6-generation arm against a 2-generation one confounds the lever with the amount
    # of training -- the exact failure that invalidated the first low-lr sweep, whose arms were
    # unmatched at ~650 generations. Report it loudly rather than quietly dividing by a different n.
    ns = {lab: d['n'] for lab, d in res.items()}
    lo, hi = min(ns.values()), max(ns.values())
    if hi > 0 and lo < hi:
        print(f"  ** UNEQUAL ARMS: generations per cell {ns}")
        if lo * 2 <= hi:
            print("     The shortest arm has less than HALF the generations of the longest. The")
            print("     comparison below is CONFOUNDED with training amount and is not a verdict --")
            print("     re-run with a longer timeout before reading any direction from it.")
        else:
            print("     Mild imbalance; the per-generation rates below are still comparable, but the")
            print("     absolute totals are not.")
        print()

    def rate(lab, key):
        d = res.get(lab)
        return (d[key] / d['n']) if d else None

    # COMMON-RANGE COMPARISON. Unequal arms do not have to be thrown away: the cells are PAIRED
    # (same seed, same pop, same sets), so truncating every cell to the shortest arm's generation
    # count compares like with like. This matters because the prop32 cells propose 8x the
    # candidates and can hit their timeout after 1-2 generations while the control reaches 6 --
    # and a funnel rate is not constant across a run, since later generations act on a population
    # that earlier ones shaped. Comparing gens 1-2 against gens 1-6 confounds the lever with
    # training amount; comparing gens 1-2 against gens 1-2 does not.
    if len(res) >= 2:
        k = min(d['n'] for d in res.values())
        if k >= 1 and any(d['n'] != k for d in res.values()):
            print(f"  COMMON RANGE -- every cell truncated to its first {k} generation(s), which is")
            print("  the paired comparison the unequal arms still support:")
            for lab, d in res.items():
                rr = [r for r in d['rows'] if r['gen'] <= k]
                gg = len({r['gen'] for r in rr})
                pm = sum(1 for r in rr if r['distinct'] > 1)
                po = sum(r['mate_ok'] for r in rr)
                pc = sum(r['cand'] for r in rr)
                print(f"    {lab:<10} gens {gg}  proposed {pc:<5} mate-ok {po:<3} >1 distinct {pm}/{gg}")
            print()

    c, p, h = res.get('control'), res.get('prop32'), res.get('hard')
    if c and p:
        print(f"  PROPOSALS lever: {c['multi']}/{c['n']} -> {p['multi']}/{p['n']} generations with a choice"
              f"   (candidates/gen {c['prop']/c['n']:.1f} -> {p['prop']/p['n']:.1f})")
    if c and h:
        print(f"  GRADIENT  lever: {c['multi']}/{c['n']} -> {h['multi']}/{h['n']} generations with a choice"
              f"   (mate-ok/gen {c['ok']/c['n']:.2f} -> {h['ok']/h['n']:.2f})")
        print("     ** the `hard` cell CANNOT show a choice: HARD_FITNESS changes the RATE, not `f`,")
        print("        and the guard tests `f`. With ~0.4 survivors/generation there is nothing to")
        print("        re-rank. Read `hard+p32` for the gradient lever, never this cell.")
    print()
    best = max(res.items(), key=lambda kv: (kv[1]['multi'] / kv[1]['n'], kv[1]['ok'])) if res else None
    if best and best[1]['multi'] == 0:
        print("  NO CELL produced a single generation with more than one distinct fitness.")
        print("  Neither lever is sufficient at this scale. That is informative and it is NOT a")
        print("  failed experiment: it moves the suspect to the MUTATION OPERATORS -- if candidates")
        print("  survive and still land on identical rates, the operators are producing neutral")
        print("  rewrites, which is the other half of the lever that file names.")
    elif best:
        print(f"  BEST CELL: {best[0]} with {best[1]['multi']}/{best[1]['n']} generations offering a choice.")
        print("  NOT claimed: that this is progress toward a stronger program. An accept is several")
        print("  stages downstream, and this project has been misled by proxies four times already.")
    print()
    print("  Short arms, small sets, one seed. This measures the SHAPE of the funnel, not its size.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
