#!/usr/bin/env python3
"""Fit the uncertainty head's 17 parameters and export them for injection into a real net.

WHY THIS IS A FIT AND NOT A TRAINING RUN. `Net::spread_from` is a linear map over the ReLU'd hidden
layer followed by a scale -- 16 weights and a bias at width 16. That is a closed-form least-squares
problem, so there is no gradient loop, no GPU, and no datagen. `unc_probe.py` already fits exactly
this hypothesis class to answer whether a signal EXISTS; this file fits the same class to KEEP the
coefficients.

WHY THE HEAD HAS TO BE FITTED BEFORE ANY GRAMMAR DISCOVERY RUNS. An untrained head returns exactly 0
on every position -- `crates/interp/tests/unc_primitive.rs` asserts it. A grammar row with zero
variance cannot change any program's behaviour, so a mutation that inserts `unc(p)` produces a
program that plays identically to its parent. Fitness cannot distinguish them, selection cannot
retain the mutation, and `UNC_READ_IN_MAIN` would be measuring drift. Discovery on a constant row is
guaranteed-null BY CONSTRUCTION, not by the paradigm failing. So the head is fitted first.

SPLIT IS IDENTICAL TO unc_probe.py -- same seed, same shuffle, same 70/30 cut, same net filter and
the same refusal to pool nets -- so the holdout AUC printed here is comparable, position for
position, with the numbers in unc_signal_is_inverted_RESULT.md.

UNITS. The fit is done in CENTIPAWNS, the units of the residual column. `spread_from` computes
(acc + bu) * scale, so the injector divides both by the NET'S OWN scale rather than this file
assuming 600.0. A hardcoded scale here would silently mis-scale any net that used a different one.

USAGE:  ./fit_unc_head.py <dump.tsv> <net-name> <out-weights> <out-holdout> [target]
        target = resid (default) | cost
"""
import sys
import random

MATERIAL_CP = 10          # same threshold confident_when_wrong and unc_probe use
HOLD = 0.3


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
        rows.append({"net": parts[0], "cost": cost, "flip": flip, "resid": resid, "acts": acts})
    return rows


def fit_ridge(X, y, lam=1.0):
    """Normal equations with a ridge term, byte-for-byte the routine unc_probe.py validated."""
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


def auc(scored, label):
    pos = [sc for sc, r in scored if label(r)]
    neg = [sc for sc, r in scored if not label(r)]
    if not pos or not neg:
        return None
    wins = sum((1.0 if a > b else 0.5 if a == b else 0.0) for a in pos for b in neg)
    return wins / (len(pos) * len(neg))


def main():
    if len(sys.argv) < 5:
        print(__doc__)
        return 2
    path, only, wpath, hpath = sys.argv[1:5]
    target = sys.argv[5] if len(sys.argv) > 5 else "resid"

    rows = load(path)
    nets = sorted({r["net"] for r in rows})
    rows = [r for r in rows if r["net"] == only]
    if not rows:
        print(f"no rows for net '{only}'. dump holds: {nets}")
        return 1
    print(f"net {only}: {len(rows)} rows  (dump holds {len(nets)}: {nets})")

    # EXACTLY unc_probe.py's split so the holdout is the same positions.
    random.seed(20260911)
    random.shuffle(rows)
    cut = int(len(rows) * (1 - HOLD))
    tr, te = rows[:cut], rows[cut:]
    costly = lambda r: r["flip"] == 1 and r["cost"] >= MATERIAL_CP
    n_te_costly = sum(1 for r in te if costly(r))
    print(f"train {len(tr)}   holdout {len(te)} ({n_te_costly} costly)")
    if n_te_costly < 5:
        print("fewer than 5 costly flips held out -- refusing to fit to a holdout that cannot be read")
        return 1

    relu = lambda a: [x if x > 0 else 0.0 for x in a]
    Xtr = [relu(r["acts"]) for r in tr]
    ytr = [r["resid"] for r in tr] if target == "resid" else [float(r["cost"]) for r in tr]
    w = fit_ridge(Xtr, ytr)
    wu, bu = w[:-1], w[-1]

    # The offline number the injected head must reproduce. Unclamped float, so any divergence in
    # Rust is attributable to the clamp/cast rather than to a different fit.
    pred_te = [(sum(a * b for a, b in zip(relu(r["acts"]), wu)) + bu, r) for r in te]
    a_raw = auc(pred_te, costly)
    a_neg = auc([(-s, r) for s, r in pred_te], costly)

    trials = []
    for t in range(200):
        random.seed(1000 + t)
        trials.append(auc([(random.random(), r) for r in te], costly))
    trials.sort()

    print(f"\ntarget = {target}")
    print(f"  offline head AUC (float, unclamped): {a_raw:.3f}")
    print(f"  NEGATED                            : {a_neg:.3f}")
    print(f"  random band                        : [{trials[10]:.3f}, {trials[189]:.3f}]")

    # How much of the holdout would a floor-at-zero destroy? This is the whole reason the injected
    # head might not reproduce the offline number: the allocator consumes the signal NEGATED, so the
    # positions it cares about are the LOWEST predictions -- exactly the ones a clamp flattens to a
    # single tied value.
    neg_pred = sum(1 for s, _ in pred_te if s <= 0.0)
    print(f"  holdout predictions <= 0           : {neg_pred}/{len(te)} "
          f"({100.0*neg_pred/len(te):.0f}%) -- these TIE at 0 once clamped")

    with open(wpath, "w") as f:
        f.write(f"# fitted by fit_unc_head.py  net={only} target={target} train={len(tr)}\n")
        f.write("# CENTIPAWN units. The injector divides by the net's own scale.\n")
        f.write(f"bu\t{bu!r}\n")
        for i, v in enumerate(wu):
            f.write(f"wu\t{i}\t{v!r}\n")
    with open(hpath, "w") as f:
        f.write("# cost\tflip\tresid\tacts...   (holdout only, same split as unc_probe.py)\n")
        for r in te:
            f.write("\t".join([str(r["cost"]), str(r["flip"]), repr(r["resid"])]
                              + [repr(x) for x in r["acts"]]) + "\n")
    print(f"\nwrote {wpath} and {hpath} ({len(te)} holdout rows)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
