#!/usr/bin/env python3
"""Merge sharded pool_rating output into one rating table.

pool_rating's shard mode deliberately refuses to print a rating, because a rating computed over
part of a field is exactly the single-opponent number the pool exists to replace. This reads the
match lines back from every shard log and does the rating once, over the whole field.

It REFUSES to rate an incomplete pool. A missing match is not a small gap: the rating is a mean over
opponents, so a net that is short one match is rated over a different (easier or harder) field than
the others and its number is not comparable. Silently averaging over whatever showed up is how a
partial run becomes a confident wrong ranking.

usage: pool_merge.py <shard log> [shard log ...]
"""
import re, sys
from itertools import combinations

LINE = re.compile(r'^\s*(\S+)\s+vs\s+(\S+)\s+([0-9.]+)\s*$')

def main(paths):
    score, nets = {}, set()
    dupes = []
    for p in paths:
        for ln in open(p, errors='ignore'):
            m = LINE.match(ln)
            if not m:
                continue
            a, b, r = m.group(1), m.group(2), float(m.group(3))
            if (a, b) in score and abs(score[(a, b)] - r) > 1e-9:
                dupes.append((a, b, score[(a, b)], r))
            score[(a, b)] = r
            score[(b, a)] = 1.0 - r
            nets.update((a, b))

    names = sorted(nets)
    n = len(names)
    if n < 2:
        print(f"  only {n} net(s) found across {len(paths)} file(s) -- nothing to rate")
        return 1

    missing = [(a, b) for a, b in combinations(names, 2) if (a, b) not in score]
    print(f"  {n} nets, {len(score)//2} of {n*(n-1)//2} matches present")
    if dupes:
        print(f"  WARNING: {len(dupes)} pairing(s) reported twice with DIFFERENT values;")
        print("  shards overlapped, which means the partition is broken. Not rating.")
        for a, b, x, y in dupes[:3]:
            print(f"    {a} vs {b}: {x} then {y}")
        return 2
    if missing:
        print(f"  REFUSING TO RATE: {len(missing)} match(es) missing, e.g. {missing[0][0]} vs {missing[0][1]}")
        print("  A net short one match is rated over a different field than the others, so the")
        print("  ranking would not be comparable across rows. Finish the shards, then merge.")
        return 3

    rated = sorted(((sum(score[(a, b)] for b in names if b != a) / (n - 1), a) for a in names),
                   reverse=True)
    print("\n  === RATING: mean score against the field ===")
    for r, a in rated:
        print(f"  {a:>20}  {r:.4f}")

    beats = lambda a, b: score[(a, b)] > 0.5
    cyc = sum(1 for a, b, c in combinations(names, 3)
              if (beats(a, b), beats(b, c), beats(c, a)) in ((True,)*3, (False,)*3))
    tri = n * (n - 1) * (n - 2) // 6
    print(f"\n  === NON-TRANSITIVITY: {cyc} cyclic triples out of {tri} ===")
    print("  Cycles mean the pool has no consistent ordering, so the ranking above is partly an"
          if cyc else "  No cycles: the pool orders consistently and the ranking reads as one.")
    if cyc:
        print("  artefact of which nets are in the pool. Do not move a default on a gap smaller")
        print("  than the cycling.")
    return 0

if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]) or 0)
