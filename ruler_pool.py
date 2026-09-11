#!/usr/bin/env python3
"""Pool the absolute-ruler readings PER RUNG, and report the pool as the headline.

His instruction, 2026-09-11:

    "Pool the ruler. Nine readings at gen 2052 span 1424-1543 because each is +/-60; the page and
     the daily line should show the pooled estimate (~1470 +/- 20) per rung, not each 120-game
     sample as its own point. Individual samples stay in the ledger; the headline number is the
     pool."

WHY THIS MATTERS AND IS NOT COSMETIC. Eight readings of the SAME net at gen 2052 read 1424, 1431,
1440, 1460, 1481, 1481, 1496, 1543 -- a 119-point spread from one unchanged network. Plotted as
eight points that is a wildly volatile engine; it is nothing of the sort, it is one number measured
eight times at +/-60 each. Reading any single 120-game sample as "where the engine is" invents
movement that does not exist, and this project has already been burned by exactly that: the ruler
"produced a four-reading DECLINE while the net was genuinely stronger" (CHAMPIONS.md).

Pooling is inverse-variance weighted, which for near-equal sigmas is the mean with SE = sigma/sqrt(n)
-- so eight +/-60 samples become +/-21. The individual samples are NOT discarded: they stay in
live_ruler.out, which is the ledger. Only the headline changes.

THE POOL IS STILL NOT A TREND. Two pooled rungs differing by less than their combined SE is not
movement either. The stop condition is 1600 on the POOLED ruler with a RISING trend by day 7, so the
trend must be read across pooled rungs, never across raw samples.

    ./ruler_pool.py                 # every rung, pooled, newest last
    ./ruler_pool.py --latest        # one line: the newest rung's pool (for the daily status line)
"""
import re, sys, math
from collections import OrderedDict

LOG = "/home/maswabe/existence/live_ruler.out"
# The ruler reports Elo RELATIVE to the opponent rung it played; absolute = rung + relative.
# SF-1320 is the rung these readings were taken against (sf_ruler.py --sf-elo 1320).
RUNG_ELO = 1320

# "03:20 prod4 gen 2052 Elo vs this opponent: +223 +/- 63"
LINE = re.compile(r'^(\d\d:\d\d)\s+(\S+)\s+gen\s+(\d+)\s+Elo vs this opponent:\s*([+-]?\d+)\s*\+/-\s*(\d+)')


def pool(vals, sigmas):
    """Inverse-variance weighted mean and its standard error."""
    w = [1.0 / (s * s) for s in sigmas]
    m = sum(v * x for v, x in zip(vals, w)) / sum(w)
    return m, math.sqrt(1.0 / sum(w))


def read(path=LOG):
    rungs = OrderedDict(); order = {}
    try:
        with open(path) as f:
            for line in f:
                m = LINE.match(line.strip())
                if not m:
                    continue
                _, run, gen, elo, sig = m.groups()
                rungs.setdefault((run, int(gen)), []).append((float(elo), float(sig)))
                order.setdefault((run, int(gen)), len(order))
    except FileNotFoundError:
        return rungs, order
    return rungs, order


def main():
    rungs, order = read()
    if not rungs:
        print("no ruler readings parsed -- check the log format before trusting a zero")
        return 1
    rows = []
    for (run, gen), obs in rungs.items():
        v = [o[0] for o in obs]
        s = [o[1] for o in obs]
        m, se = pool(v, s)
        rows.append((run, gen, len(obs), m, se, min(v), max(v), s[0]))
    # CHRONOLOGICAL, not alphabetical. Sorting by run name made --latest report run "rd" (an old
    # run) as the newest rung simply because r sorts last. "Latest" must mean most recently MEASURED.
    rows.sort(key=lambda r: order[(r[0], r[1])])

    if "--latest" in sys.argv:
        run, gen, n, m, se, lo, hi, sig = rows[-1]
        print(f"RULER {run} gen {gen}: pooled {RUNG_ELO + m:.0f} +/- {se:.0f} "
              f"(n={n} sample{'s' if n>1 else ''}, each +/-{sig:.0f}" + (f", raw span {RUNG_ELO+lo:.0f}-{RUNG_ELO+hi:.0f}" if n>1 else "") + ")")
        return 0

    print(f"{'run':<10} {'gen':>7} {'n':>3} {'POOLED abs':>12} {'+/-':>5}   raw span        (samples)")
    for run, gen, n, m, se, lo, hi, sig in rows:
        span = f"{RUNG_ELO+lo:.0f}-{RUNG_ELO+hi:.0f}" if n > 1 else "-"
        print(f"{run:<10} {gen:>7} {n:>3} {RUNG_ELO + m:>12.0f} {se:>5.0f}   {span:<15}")
    print()
    print("Pooled per rung, inverse-variance. Individual samples remain in live_ruler.out (the ledger).")
    print("A difference between two pooled rungs smaller than their combined SE is NOT a trend.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
