#!/usr/bin/env python3
"""Can a LINEAR probe on the net's own hidden layer rank the positions that matter?

This is the question `uncertainty_target_PREREG.md` registers and `flip_cost_concentration_RESULT.md`
makes worth asking. Those two established:

  * the prize is CONCENTRATED -- the top decile of flips carries 38% of all flip cost, 3.8x uniform,
    so an allocator that finds those positions has something real to win;
  * the obvious key does NOT fit -- |static - deep| is anti-correlated with costly flips (33.3%
    against a 50% base rate), so a head trained on the score residual would point the wrong way.

What neither settles is whether the head could learn the RIGHT target. That is this file.

WHY A LINEAR PROBE IS THE HONEST TEST, not a weaker one. `Net::spread_from` IS a linear map over the
hidden layer followed by a scale -- 17 parameters at width 16. So a linear probe on those same
activations is not an approximation of the head, it is the head's exact hypothesis class. If a probe
cannot rank the tail, no amount of training the real head will, because there is no other function
for it to find.

INPUT is the TSV that `UNC_DUMP=... confident_when_wrong` writes: net, cost, flip, residual, then
the hidden activations (raw, pre-ReLU).

USAGE:  ./unc_probe.py <dump.tsv> [holdout_frac]
"""
import sys
import random

MATERIAL_CP = 10          # same threshold confident_when_wrong uses for "a real error"
TOP_FRAC = 0.10           # the decile the concentration result is about


def load(path):
    rows = []
    for ln in open(path):
        parts = ln.rstrip("\n").split("\t")
        if len(parts) < 5:
            continue
        try:
            cost = int(parts[1]); flip = int(parts[2]); resid = float(parts[3])
            acts = [float(x) for x in parts[4:]]
        except ValueError:
            continue
        rows.append({"cost": cost, "flip": flip, "resid": resid, "acts": acts})
    return rows


def fit_ridge(X, y, lam=1.0):
    """Normal equations with a ridge term. 17 parameters, so this is exact and instant.

    The ridge is not decoration: with 16 correlated hidden units an unregularised fit on a few
    hundred rows will happily memorise, and a probe that memorises answers a different question
    than the one asked.
    """
    n, d = len(X), len(X[0])
    A = [[0.0] * (d + 1) for _ in range(d + 1)]
    b = [0.0] * (d + 1)
    for xi, yi in zip(X, y):
        v = list(xi) + [1.0]
        for i in range(d + 1):
            b[i] += v[i] * yi
            for j in range(d + 1):
                A[i][j] += v[i] * v[j]
    for i in range(d):                      # no penalty on the bias
        A[i][i] += lam * n
    # Gaussian elimination
    for i in range(d + 1):
        p = max(range(i, d + 1), key=lambda r: abs(A[r][i]))
        if abs(A[p][i]) < 1e-12:
            continue
        A[i], A[p] = A[p], A[i]
        b[i], b[p] = b[p], b[i]
        for r in range(d + 1):
            if r == i:
                continue
            f = A[r][i] / A[i][i]
            if f == 0.0:
                continue
            for c in range(i, d + 1):
                A[r][c] -= f * A[i][c]
            b[r] -= f * b[i]
    return [b[i] / A[i][i] if abs(A[i][i]) > 1e-12 else 0.0 for i in range(d + 1)]


def enrichment(scored, label, frac=TOP_FRAC):
    """Share of the top `frac` by score that carries `label`, against the base rate."""
    if not scored:
        return None, None, 0
    s = sorted(scored, key=lambda t: -t[0])
    k = max(1, int(round(len(s) * frac)))
    top = sum(1 for _, r in s[:k] if label(r))
    base = sum(1 for _, r in s if label(r)) / len(s)
    return top / k, base, k


def main():
    path = sys.argv[1]
    hold = float(sys.argv[2]) if len(sys.argv) > 2 else 0.3
    rows = load(path)
    if len(rows) < 40:
        print(f"only {len(rows)} rows -- too few to split. Let the labelling run finish.")
        return 1

    random.seed(20260911)
    random.shuffle(rows)
    cut = int(len(rows) * (1 - hold))
    tr, te = rows[:cut], rows[cut:]
    costly = lambda r: r["flip"] == 1 and r["cost"] >= MATERIAL_CP

    n_costly_tr = sum(1 for r in tr if costly(r))
    n_costly_te = sum(1 for r in te if costly(r))
    print(f"rows {len(rows)}   train {len(tr)} ({n_costly_tr} costly)   "
          f"holdout {len(te)} ({n_costly_te} costly)")
    if n_costly_te < 5:
        print("fewer than 5 costly flips held out -- the enrichment number would be noise. "
              "Run more positions before reading this.")
        return 1

    # ReLU, because that is what the head applies (`spread_from`: `if s > 0 { acc += s*wu[h] }`).
    relu = lambda a: [x if x > 0 else 0.0 for x in a]
    Xtr = [relu(r["acts"]) for r in tr]
    Xte = [relu(r["acts"]) for r in te]

    print("\n--- can the head's own hypothesis class rank the costly tail? ---")
    for name, y in (("T2  flip cost", [float(r["cost"]) for r in tr]),
                    ("T1  residual ", [r["resid"] for r in tr])):
        w = fit_ridge(Xtr, y)
        pred = [(sum(a * b for a, b in zip(x, w[:-1])) + w[-1], r) for x, r in zip(Xte, te)]
        top, base, k = enrichment(pred, costly)
        lift = top / base if base else float("nan")
        print(f"  probe on {name}: top decile ({k}) = {100*top:4.1f}% costly, "
              f"base {100*base:4.1f}%, lift {lift:.2f}x")

    # The baselines that make the probe numbers mean something.
    resid_rank = [(r["resid"], r) for r in te]
    top, base, k = enrichment(resid_rank, costly)
    print(f"  rank by RAW residual   : top decile ({k}) = {100*top:4.1f}% costly, "
          f"base {100*base:4.1f}%, lift {top/base if base else float('nan'):.2f}x")

    rnd = [(random.random(), r) for r in te]
    top, base, k = enrichment(rnd, costly)
    print(f"  rank RANDOM (control)  : top decile ({k}) = {100*top:4.1f}% costly, "
          f"base {100*base:4.1f}%, lift {top/base if base else float('nan'):.2f}x")

    print("\n  lift ~1.0 means the ranking carries no information about which positions matter.")
    print("  A probe that cannot beat the random control is a head that cannot be trained to,")
    print("  because spread_from IS this hypothesis class -- there is no richer function for it")
    print("  to find at width 16.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
