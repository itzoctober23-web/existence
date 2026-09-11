#!/usr/bin/env python3
"""Read the two discrimination arms, ENFORCING the pre-registration in refmatch_discrimination_PREREG.md.

    ./refmatch_report.py <hash.log> <capture.log>

The pre-registration says arm 1 (hash reuse, documented to play IDENTICALLY to the seed) is the
CONTROL and is checked FIRST: "If it fails, arm 2 is not interpreted." This enforces that in code
rather than leaving it to whoever reads the logs, because the failure mode being guarded against is
reading the interesting arm and rationalising the control afterwards.

WHAT THE CONTROL PREDICTS, and why. `gate.rs:191-203` plays the SAME opening from both sides
(`for a_is_white in [true, false]`) and scores it as a PAIR. Two programs that play identically
therefore produce mirror games that cancel: either both drawn, or split one-all. Both score the pair
at 1.0 of 2, so every pair lands in the middle bucket and the observed variance is ZERO.

Zero variance triggers `gate.rs`'s rule-of-three branch, `ci95 = 1.5/n` -- 0.250 at the gate's 6
pairs, 0.0625 at 24. So the control should reproduce on demand the degenerate reading measured in
28.6% of real gate decisions.
"""
import sys, re, os, math


def read(path):
    if not os.path.exists(path):
        return None, f"missing: {path}"
    txt = open(path).read()
    m = re.search(r'(\d+)W-(\d+)D-(\d+)L\s+rate\s+([\d.]+)\s+\+/-\s+([\d.]+)', txt)
    if not m:
        # An absent verdict is a broken or unfinished run, never a null result.
        return None, "no 'W-D-L ... rate' line -- run unfinished, timed out, or the print moved"
    w, d, l = int(m.group(1)), int(m.group(2)), int(m.group(3))
    return dict(w=w, d=d, l=l, rate=float(m.group(4)), ci=float(m.group(5)), games=w + d + l), None


def main(hash_log, cap_log):
    h, herr = read(hash_log)
    print("  ARM 1 — hash reuse vs seed (CONTROL, documented to play identically)")
    if herr:
        print(f"    {herr}")
        print("    CANNOT PROCEED. The pre-registration makes arm 1 the gate on reading arm 2.")
        return 2
    pairs = h['games'] // 2
    predicted_ci = 1.5 / pairs
    print(f"    {h['w']}W-{h['d']}D-{h['l']}L over {h['games']} games ({pairs} pairs)")
    print(f"    rate {h['rate']:.4f} +/- {h['ci']:.4f}")
    print(f"    PREDICTED: rate 0.5000, zero variance -> ci95 = 1.5/{pairs} = {predicted_ci:.4f}")

    rate_ok = abs(h['rate'] - 0.5) < 1e-6
    # evolve.rs prints with {:.3}, so 1.5/24 = 0.0625 renders as 0.063. Comparing at 1e-4
    # would fail on the ROUNDING and report a control failure that is a formatting artifact.
    ci_ok = abs(h['ci'] - predicted_ci) < 0.001
    if rate_ok and ci_ok:
        print("    CONTROL PASSES exactly. The pairing model is right, and a behaviourally identical")
        print("    program produces the degenerate zero-variance reading by construction.")
    elif rate_ok and not ci_ok:
        print(f"    Rate is 0.5000 as predicted but ci95 is {h['ci']:.4f}, not {predicted_ci:.4f}.")
        print("    So the pairs did NOT all land in one bucket -- the two programs' games differ")
        print("    somewhere despite the documented identity. Arm 2 IS still interpretable (the")
        print("    pairing model holds), but the 'identical play' claim needs re-checking.")
    else:
        print("    *** CONTROL FAILS. Rate is not 0.5000.")
        print("    My pairing model is wrong, and every inference in gate_arithmetic_RESULT.md that")
        print("    rests on it needs re-checking. NOT interpreting arm 2, per the pre-registration.")
        return 1

    c, cerr = read(cap_log)
    print()
    print("  ARM 2 — capture extension vs seed (a genuinely different program, 1.679x cost)")
    if cerr:
        print(f"    {cerr}")
        print("    Arm 1 stands on its own; re-run arm 2 before reading the comparison.")
        return 0
    cp = c['games'] // 2
    decisive = c['w'] + c['l']
    print(f"    {c['w']}W-{c['d']}D-{c['l']}L over {c['games']} games ({cp} pairs)")
    print(f"    rate {c['rate']:.4f} +/- {c['ci']:.4f}   draws {100.0*c['d']/c['games']:.1f}%   decisive {decisive}")
    print()
    if c['ci'] > 1.5 / cp + 0.001 and decisive > 0:
        print("    THE GAMES CAN DISCRIMINATE. A real behavioural difference produced decisive")
        print("    results and genuine variance, unlike the control. So the 85.7% draw rate in real")
        print("    gate decisions is about the CANDIDATES the operators produce, not about the game")
        print("    conditions — and the suspect moves to the mutation operators.")
    else:
        print("    THE GAMES CANNOT DISCRIMINATE even a program built specifically to search")
        print("    differently: it lands in the same degenerate bucket as the identical-play control.")
        print("    A 6-pair gate is then unfixable by pair count alone, and the fitness's GAME half")
        print("    needs rethinking rather than resizing. This is the stronger and more expensive")
        print("    of the two pre-registered outcomes.")
    print()
    print(f"    For scale, the real gate's draw rate is 85.7% (216/252 games, 21 matches).")
    return 0


if __name__ == '__main__':
    if len(sys.argv) != 3:
        print(__doc__); sys.exit(2)
    sys.exit(main(sys.argv[1], sys.argv[2]))
