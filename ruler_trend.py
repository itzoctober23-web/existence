#!/usr/bin/env python3
"""The TREND half of the stop condition — is the pooled ruler actually RISING?

His stop condition, in full: "1600 on the pooled ruler **with a rising trend** by day 7."

`ruler_pool.py` answers the first half (the level, pooled per rung, per his pooling directive).
It cannot answer the second, and the way the ruler is sampled makes the second half the one that
is easy to get wrong: `live_ruler.sh` takes ONE 120-game sample per rung as production advances,
so a production run is a sequence of n=1 readings at +/-50-67 each. `prodk0759` reads

    1503 1447 1444 1496 1496 1565 1496 1396 1485 1519

across generations 26 -> 8584. That is a 169-Elo span, and every bit of it is consistent with
noise around a CONSTANT. Reading "it is climbing" off the last two points, or "it is collapsing"
off the 1396, is exactly the error the pooling directive exists to prevent -- and this project
has already been burned by it once: CHAMPIONS.md records the ruler "produced a four-reading
DECLINE while the net was genuinely stronger".

WHAT THIS DOES. Weighted least squares of absolute Elo on generation, weights 1/sigma^2, giving
a slope in Elo per 1000 generations WITH ITS STANDARD ERROR. A trend is only claimed at |z| > 2.
It also reports chi2/dof: if that is near or below 1, the scatter is fully explained by the
quoted error bars and there is no signal hiding under them.

THE SATURATION CONTROL, which must run BEFORE any "flat" is believed. A ruler that has run out
of range reports flat no matter how strong the engine gets, and this project has met exactly that
failure twice -- `instrument_saturation_RESULT.md` ("the frozen-origin metric saturates, and it
has now reversed two signs") and the 4PC external anchor, which went 31-0-0 and bought a lower
BOUND rather than a reading. So: convert the pooled level into the score it implies against the
rung, and refuse to call anything flat if that score is in the saturated band.

    ./ruler_trend.py              # every run with >= 3 rungs
    ./ruler_trend.py prodk0759    # one run
"""
import re, sys, math

LOG = "/home/maswabe/existence/live_ruler.out"
RUNG_ELO = 1320
SATURATED = 0.95     # above this a score buys a lower bound, not a measurement
MIN_RUNGS = 3

LINE = re.compile(r'^(\d\d:\d\d)\s+(\S+)\s+gen\s+(\d+)\s+Elo vs this opponent:\s*([+-]?\d+)\s*\+/-\s*(\d+)')


def read(path=LOG):
    runs = {}
    with open(path) as f:
        for line in f:
            m = LINE.match(line.strip())
            if m:
                _, run, gen, elo, sig = m.groups()
                runs.setdefault(run, []).append((int(gen), RUNG_ELO + float(elo), float(sig)))
    for r in runs:
        runs[r] = sorted(runs[r])
    return runs


def pooled(pts):
    w = sum(1.0 / (s * s) for _, _, s in pts)
    m = sum(e / (s * s) for _, e, s in pts) / w
    return m, math.sqrt(1.0 / w)


def se_guard(Sxx, span):
    """True when the design is too degenerate for the slope to mean anything.

    Sxx is the weighted spread in x. If the resulting slope SE exceeds what a whole run's worth
    of Elo could plausibly be (1000 Elo per 1000 generations), the fit is not measuring a trend.
    """
    return math.sqrt(1.0 / Sxx) * 1000.0 > 1000.0


def wls(pts):
    """Weighted least squares elo ~ gen. Slope returned per 1000 generations.

    X IS CENTERED FIRST, and that is not a style choice. The textbook form
    `den = S*Sxx - Sx^2` is non-negative by Cauchy-Schwarz, but generation numbers reach ~18,000,
    so Sxx carries terms of order 3e8 scaled by the weights and the subtraction cancels away the
    significant digits. Computed that way this function returned den = -1.27e12 on `prod1` and
    `math.sqrt` raised ValueError -- a crash, which was lucky. The same cancellation one digit
    smaller would have returned a plausible WRONG slope silently, which is the failure mode that
    matters. Centering makes the sums O(spread) instead of O(position).
    """
    n = len(pts)
    w = [1.0 / (s * s) for _, _, s in pts]
    S = sum(w)
    xbar = sum(wi * g for (g, _, _), wi in zip(pts, w)) / S
    ybar = sum(wi * e for (_, e, _), wi in zip(pts, w)) / S
    Sxx = sum(wi * (g - xbar) ** 2 for (g, _, _), wi in zip(pts, w))
    Sxy = sum(wi * (g - xbar) * (e - ybar) for (g, e, _), wi in zip(pts, w))
    # DEGENERATE FIT: refuse, do not emit a number.
    # The `sweep*` arms have all 4 rungs at the SAME generation (2000) because they are repeat
    # samples of one frozen net, not a time series. Sxx is then a floating-point crumb rather
    # than an exact 0, so a `<= 0` test misses it and the fit reported slope +354.7 with an SE of
    # 1.2e17 -- a formatted, plausible-looking row carrying no information. Require real spread.
    span = pts[-1][0] - pts[0][0]
    if Sxx <= 0 or span <= 0 or se_guard(Sxx, span):
        return None
    b = Sxy / Sxx
    a = ybar - b * xbar
    se_b = math.sqrt(1.0 / Sxx)
    chi2 = sum(wi * (e - (a + b * g)) ** 2 for (g, e, _), wi in zip(pts, w))
    dof = max(n - 2, 1)
    return b * 1000.0, se_b * 1000.0, chi2 / dof


def expected_score(elo):
    return 1.0 / (1.0 + 10 ** (-(elo - RUNG_ELO) / 400.0))


def main():
    runs = read()
    want = sys.argv[1:] or None
    names = [r for r in runs if len(runs[r]) >= MIN_RUNGS and (not want or r in want)]
    if not names:
        print("no run has >= %d rungs -- check the log format before trusting an empty result" % MIN_RUNGS)
        return 1
    names.sort(key=lambda r: runs[r][-1][0])

    print(f"{'run':<12} {'n':>3} {'gens':>16} {'POOLED':>9} {'+/-':>4} "
          f"{'slope/1k':>10} {'+/-':>6} {'z':>6} {'chi2/dof':>9}  verdict")
    for r in names:
        pts = runs[r]
        lvl, lse = pooled(pts)
        fit = wls(pts)
        if not fit:
            # Never drop a run silently -- a missing row reads as "not measured".
            span = pts[-1][0] - pts[0][0]
            why = "all rungs at one generation" if span <= 0 else "generation span too short"
            print(f"{r:<12} {len(pts):>3} {pts[0][0]}-{pts[-1][0]:<10} {lvl:>9.0f} {lse:>4.0f} "
                  f"{'--':>10} {'--':>6} {'--':>6} {'--':>9}  NO TREND DEFINED ({why})")
            continue
        slope, sse, red = fit
        z = slope / sse if sse else 0.0
        sat = expected_score(lvl)
        if sat > SATURATED:
            verdict = f"SATURATED (score {sat:.3f}) -- flat is UNREADABLE here"
        elif z > 2:
            verdict = "RISING"
        elif z < -2:
            verdict = "FALLING"
        else:
            verdict = "FLAT (indistinguishable from zero)"
        span = f"{pts[0][0]}-{pts[-1][0]}"
        print(f"{r:<12} {len(pts):>3} {span:>16} {lvl:>9.0f} {lse:>4.0f} "
              f"{slope:>+10.1f} {sse:>6.1f} {z:>+6.2f} {red:>9.2f}  {verdict}")

    print()
    print(f"A slope is only a trend at |z| > 2. chi2/dof <= 1 means the quoted +/-{50}-ish error")
    print("bars already explain the scatter -- there is no signal hiding underneath them.")
    print(f"Saturation control: a pooled level implying a score > {SATURATED} against the "
          f"SF-{RUNG_ELO} rung is a LOWER BOUND, and 'flat' there means the ruler ran out of")
    print("range, not that the engine stopped improving.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
