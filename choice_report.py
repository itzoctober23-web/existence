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
    ("prop_hard.log",     "hard",     "proposals=pop, HARD set"),
    ("prop_hardp32.log",  "hard+p32", "proposals=32,  HARD set"),
]


def parse(path):
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


def main():
    print("  WHICH LEVER GIVES SELECTION A CHOICE?")
    print("  baseline on file: 5 of 79 generations (6%) had more than one distinct fitness")
    print()
    print(f"  {'cell':<10} {'configuration':<26} {'gens':>4} {'prop':>5} {'ok':>4} "
          f"{'>1 distinct':>12} {'>=1':>5} {'pop':>4}")
    res = {}
    for path, lab, cfg in CELLS:
        rows = parse(path)
        if not rows:
            print(f"  {lab:<10} {cfg:<26} {'--':>4}  (no parseable generations)")
            continue
        n = len(rows)
        prop = sum(r['cand'] for r in rows)
        ok = sum(r['mate_ok'] for r in rows)
        multi = sum(1 for r in rows if r['distinct'] > 1)
        anyd = sum(1 for r in rows if r['distinct'] > 0)
        res[lab] = dict(n=n, prop=prop, ok=ok, multi=multi, anyd=anyd, pop=rows[-1]['pop'])
        print(f"  {lab:<10} {cfg:<26} {n:>4} {prop:>5} {ok:>4} "
              f"{str(multi)+'/'+str(n):>12} {anyd:>5} {rows[-1]['pop']:>4}")
    print()
    if len(res) < 2:
        print("  Fewer than two cells have run. No comparison is made -- a missing arm is not a")
        print("  result in either direction.")
        return 1

    def rate(lab, key):
        d = res.get(lab)
        return (d[key] / d['n']) if d else None

    c, p, h = res.get('control'), res.get('prop32'), res.get('hard')
    if c and p:
        print(f"  PROPOSALS lever: {c['multi']}/{c['n']} -> {p['multi']}/{p['n']} generations with a choice"
              f"   (candidates/gen {c['prop']/c['n']:.1f} -> {p['prop']/p['n']:.1f})")
    if c and h:
        print(f"  GRADIENT  lever: {c['multi']}/{c['n']} -> {h['multi']}/{h['n']} generations with a choice"
              f"   (mate-ok/gen {c['ok']/c['n']:.2f} -> {h['ok']/h['n']:.2f})")
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
