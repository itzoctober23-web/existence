#!/usr/bin/env python3
"""REQUESTED vs OBSERVED — assert a setting reached the code that ran.

    ./assert_setting_took.py prop_hardn40.log EXISTENCE_HARD_N=40 EXISTENCE_HARD_FITNESS=1

Exit 1 if any requested setting is not visible in the run's OWN output, so this can gate a
launcher rather than decorate a log.

WHY, and why the obvious version of this tool does not work.

On 2026-09-11 `hardn_probe.sh` set `EXISTENCE_HARD_N=40` and concluded the HARD set "is not a
gradient at any size reachable this way ... it must be rebuilt". The knob was inert:
`evolve.rs:2627` built the set with a hardcoded 8 on the path the evolve loop runs, while the read
at :657 fed a different entry point. The arm measured the default configuration.

TWO detectors were tried first and BOTH failed their control test, which is why this one is
specific rather than general:

1. *Static call-graph audit* of all 19 EXISTENCE_* variables -- all-clear. Useless here: the read
   IS reachable from main, the VALUE simply never reaches the code that runs. Reachability cannot
   see a hardcoded literal at the use site.

2. *Whole-header comparison* between a treatment arm and its control -- inverted on both controls.
   It passed the known-bad pair, because hardn_probe also set HARD_FITNESS and that difference
   showed up while `HARD set: 8 positions` was identical in both. And it FAILED the known-good pair
   (control vs 32 proposals), because the header never prints the proposal count at all, so two arms
   that genuinely differed looked identical.

The lesson both failures share: **"something differed" is not "the thing under test differed".** So
each variable is mapped to the ONE observable it controls, and the check is requested-vs-observed on
that observable. This is the project's standing rule -- read the tool's own "loaded X" line, not the
flag you passed -- made specific enough to be true.

A variable with no known observable is reported as UNCHECKABLE, never as passing. An absent
observable is a broken probe until proven otherwise, and silently counting it as OK would rebuild
the exact false all-clear this file exists to prevent.
"""
import sys, re, os

# PRECONDITIONS: some observables only print when something ELSE happens first, and their absence
# before that is silence, not evidence. EXISTENCE_GATE_VERIFY prints inside the gate block, so an
# arm that has not yet made a gate call CANNOT show it. Reporting that as "DID NOT TAKE" is a false
# alarm, and a checker that cries wolf is one that gets ignored the day it is right -- caught on the
# live search-verify96 arm at generation 1 with zero gate calls.
PRECONDITION = {
    'EXISTENCE_GATE_VERIFY': (r'gate (?:REJECT|ACCEPT)', 'a gate call (VERIFY prints only inside the gate block)'),
}

# var -> (regex over the whole log, how to reduce matches, how to compare to the request)
# Every pattern below was checked against real output before being relied on.
OBSERVABLES = {
    # "HARD set: 8 positions the seed FAILS by construction"
    'EXISTENCE_HARD_N':        (r'HARD set:\s*(\d+) positions', 'first', 'int'),
    # generation lines: "(32 cand, 0 ill, mate-ok 5, ..."
    'EXISTENCE_PROPOSALS':     (r'\((\d+) cand', 'max', 'int'),
    # "population MU=8, lambda=4, EPS=0.020, guard tolerance 4, HARD_FITNESS on weight 1.00"
    'EXISTENCE_HARD_FITNESS':  (r'HARD_FITNESS (on|off)', 'first', 'onoff'),
    'EXISTENCE_SPEC_FILTER':   (r'SPEC_FILTER (on|off)', 'first', 'onoff'),
    'EXISTENCE_EPS':           (r'EPS=([\d.]+)', 'first', 'float'),
    'EXISTENCE_GUARD_TOL':     (r'guard tolerance (\d+)', 'first', 'int'),
    # "gen  1 MAIN  VERIFY 0.500+/-0.047 (96 pairs, independent seed)"
    'EXISTENCE_GATE_VERIFY':   (r'VERIFY [\d.]+\+/-[\d.]+ \((\d+) pairs', 'first', 'int'),
    'EXISTENCE_EVOLVE_SEED':   (r'run seed (\d+)', 'first', 'int'),
    'EXISTENCE_HARD_WEIGHT':   (r'HARD_FITNESS on weight ([\d.]+)', 'first', 'float'),
}


def observe(txt, rx, how):
    m = re.findall(rx, txt)
    if not m:
        return None
    return m[0] if how == 'first' else max(m, key=lambda v: int(v) if v.isdigit() else 0)


def check(path, requests):
    if not os.path.exists(path):
        print(f"  {path}: MISSING"); return 2
    txt = open(path).read()
    bad = unchecked = 0
    print(f"  log: {path}")
    for req in requests:
        if '=' not in req:
            print(f"    {req}: not a VAR=value pair"); bad += 1; continue
        var, want = req.split('=', 1)
        if var not in OBSERVABLES:
            print(f"    {var:<26} UNCHECKABLE -- no known observable. NOT counted as passing.")
            unchecked += 1; continue
        rx, how, kind = OBSERVABLES[var]
        got = observe(txt, rx, how)
        if got is None:
            pre = PRECONDITION.get(var)
            if pre and not re.search(pre[0], txt):
                print(f"    {var:<26} requested {want:<6} UNDETERMINED -- needs {pre[1]};")
                print(f"    {'':<26} none has happened yet, so silence is not evidence. Re-check later.")
                unchecked += 1; continue
            print(f"    {var:<26} requested {want:<6} OBSERVABLE NOT FOUND -- broken probe or the")
            print(f"    {'':<26} print moved. Not a pass.")
            bad += 1; continue
        if kind == 'onoff':
            ok = (got == 'on') == (want not in ('0', '', 'off'))
        elif kind == 'float':
            ok = abs(float(got) - float(want)) < 1e-6
        else:
            ok = int(got) == int(want)
        print(f"    {var:<26} requested {want:<6} observed {got:<6} {'OK' if ok else '*** DID NOT TAKE'}")
        if not ok:
            bad += 1
    print()
    if bad:
        print(f"  {bad} SETTING(S) DID NOT REACH THE CODE THAT RAN.")
        print("  Any verdict from this arm is a verdict about a DIFFERENT configuration. Find the")
        print("  hardcoded value at the use site -- EXISTENCE_HARD_N traced to evolve.rs:2627.")
        return 1
    if unchecked:
        print(f"  all checkable settings took; {unchecked} had no observable and were NOT verified.")
        return 0
    print("  every requested setting is visible in the run's own output.")
    return 0


if __name__ == '__main__':
    if len(sys.argv) < 3:
        print(__doc__); sys.exit(2)
    sys.exit(check(sys.argv[1], sys.argv[2:]))
