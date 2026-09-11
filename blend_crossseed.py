#!/usr/bin/env python3
"""Cross-seed reading for the blend sweep — with a STALENESS GUARD on the seed-2 slot.

WHY THE GUARD IS THE POINT, not a nicety.

`blend_sweep.sh` writes FIXED filenames: `blend_085_vs_075.log` and friends. Seed 1's values were
archived to `*_s1.log` before seed 2 launched, but the UNSUFFIXED files still hold seed 1's numbers
until seed 2's netmatch overwrites them. Verified at 10:33 — both slots read `0.541 +/- 0.032`,
identical, because seed 2 had finished training but not yet gated.

So a cross-seed reader that simply opens both files can compare **seed 1 against itself**, find
perfect agreement, and report "BOTH seeds point the same way, the pool clears" — which is the exact
sentence that licenses moving a shipped default. A stale read here does not produce a slightly
wrong number; it produces a confident green light.

This is the same failure that nearly landed this morning, when a stale
`low_00001_vs_start.log` from a truncated run would have been read as that run's verdict.

THE GUARD: seed 2's file must be NEWER than the seed-1 archive. If it is not, this refuses to
report rather than comparing a file with itself.

    ./blend_crossseed.py
"""
import os, re, sys, math, time

PAIRS = [
    ("blend_085_vs_075.log",   "blend_085_vs_075_s1.log",   "0.85 vs 0.75 (the contrast)"),
    ("blend_085_vs_start.log", "blend_085_vs_start_s1.log", "0.85 vs shared start"),
    ("blend_075_vs_start.log", "blend_075_vs_start_s1.log", "0.75 vs shared start"),
]


def read(path):
    if not os.path.exists(path):
        return None
    m = re.search(r'scores (0\.\d+) \+/- (0\.\d+)', open(path).read())
    return (float(m.group(1)), float(m.group(2))) if m else None


def main():
    stale = []
    rows = []
    for live, arch, label in PAIRS:
        v2, v1 = read(live), read(arch)
        if v1 is None:
            print(f"  {label}: seed-1 ARCHIVE missing ({arch}) -- cannot compare")
            return 1
        if v2 is None:
            print(f"  {label}: seed-2 slot has no verdict yet")
            stale.append(label); continue
        # THE GUARD. mtime, not value equality: two seeds CAN legitimately produce the same
        # rounded score, so equality alone would reject a valid result. The file being older
        # than the archive it is supposed to supersede cannot be legitimate.
        if os.path.getmtime(live) <= os.path.getmtime(arch):
            print(f"  {label}: seed-2 slot is STALE -- {live} is not newer than {arch}")
            print(f"      live  mtime {time.strftime('%H:%M:%S', time.localtime(os.path.getmtime(live)))}")
            print(f"      arch  mtime {time.strftime('%H:%M:%S', time.localtime(os.path.getmtime(arch)))}")
            stale.append(label); continue
        rows.append((label, v1, v2))

    if stale:
        print()
        print("  REFUSING TO REPORT A CROSS-SEED VERDICT.")
        print("  A stale seed-2 slot would compare seed 1 against ITSELF, find perfect agreement,")
        print("  and produce the sentence that licenses moving a shipped default. Wait for")
        print("  blend-sweep-seed2 to finish its netmatch.")
        return 1

    print("  CROSS-SEED READING — both seeds, full length, matched arms")
    for label, v1, v2 in rows:
        print(f"  {label}")
        for tag, (r, c) in (("seed 20260917", v1), ("seed 20260918", v2)):
            print(f"      {tag}: {r:.3f} +/- {c:.3f}   lower bound {r-c:.3f}"
                  f"   {'clears 0.500' if r-c >= 0.5 else 'does NOT clear'}")
    contrast = [r for r in rows if r[0].startswith("0.85 vs 0.75")]
    if contrast:
        _, v1, v2 = contrast[0]
        m = (v1[0] + v2[0]) / 2
        se = math.sqrt(v1[1]**2 + v2[1]**2) / 2
        print()
        print(f"  POOLED contrast: {m:.3f} +/- {se:.3f}   lower bound {m-se:.3f}")
        if (v1[0] - 0.5) * (v2[0] - 0.5) < 0:
            print("  THE SEEDS DISAGREE IN SIGN. 0.75 stays the default; a five-seed short-run")
            print("  prior does not override a full-length disagreement.")
        elif v1[0] > 0.5 and v2[0] > 0.5 and m - se >= 0.5:
            print("  BOTH seeds above 0.5 and the pool clears. That is what a default change needs:")
            print("  the effect survives the between-seed spread (sd 0.047) a single arm cannot see.")
        else:
            print("  Same direction, pool does not clear. NOT a default change -- record the bound")
            print("  and stop rather than buying a third seed for a sub-noise effect.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
