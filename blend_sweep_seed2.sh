#!/usr/bin/env bash
# SECOND FULL-LENGTH SEED FOR blend_sweep.sh — the confirmation that licenses a default change.
#
# NAMED FOR ITS PARENT. `blend_seed2.sh` already exists (2026-09-08) and is a DIFFERENT experiment
# -- the blend HIGH side, 1.00 vs 0.75. Writing to that name would have destroyed the evidence
# behind a recorded result. This is the second seed of `blend_sweep.sh`, and says so.
#
# WHY A SECOND SEED, WHEN SEED 1 ALREADY CLEARED.
# Seed 20260917 gave `0.85 vs 0.75 = 0.541 +/- 0.032`, lower bound 0.509, which clears the
# promotion rule (rate - ci95 >= 0.500). But the margin above 0.5 is 0.041 while the measured
# BETWEEN-SEED sd on this project is 0.047 -- larger than the margin. A single seed clearing by
# less than the seed-to-seed spread is the lottery this loop has been burned by before, and it is
# the same standard used hours earlier to REFUSE shipping lr 0.0001 on a 0.011 gap. A rule applied
# only when its answer is convenient is decoration.
#
# The prior is strong: blend_RESULT.md replicated 0.85 > 0.75 on FIVE independent training seeds,
# and the response is NON-MONOTONIC (0.85 gains, 1.00 gives it back), so this is not a
# "more is better" artefact. What none of those five had is FULL LENGTH -- every arm was a
# TWENTY-generation run, and the lr sweep is the standing proof that 20-generation and
# 2000-generation answers differ in magnitude.
#
# REUSES blend_sweep.sh UNEDITED, including its --blend binding probe and its UNEQUAL ARMS check.
# That script may still be running its netmatch verdicts, and bash reads a script by BYTE OFFSET
# while executing it -- a mid-run edit destroyed 2,811 pairs on this box once. So: wait, ARCHIVE
# seed 1 (the script writes FIXED filenames and a second run would silently overwrite them), then
# re-invoke with a different SEED.
set -uo pipefail
cd "$(dirname "$0")"
LOG=blend_sweep_seed2.log
SEED2=${SEED2:-20260918}
say(){ echo "$(date +%F_%H:%M) [blend2] $*" | tee -a "$LOG"; }

say "waiting for the seed-1 sweep to finish all three netmatch verdicts"
while systemctl --user is-active blend-sweep.service >/dev/null 2>&1; do sleep 60; done
sleep 20

say "archiving seed-1 artifacts as *_s1 (fixed filenames would otherwise be clobbered)"
for f in blend_075.net blend_085.net blend_start.net blend_075.log blend_085.log \
         blend_085_vs_075.log blend_085_vs_start.log blend_075_vs_start.log; do
  [ -e "$f" ] && cp -f "$f" "${f%.*}_s1.${f##*.}"
done
say "  archived $(ls blend_*_s1.* 2>/dev/null | wc -l) files"
[ -s blend_085_vs_075_s1.log ] || { say "ABORT: seed-1 contrast did not archive -- refusing to overwrite it"; exit 1; }

# Do not stack trainers: production is one `learn`, two arms make three, which fits 6 cores.
n=0
for p in /proc/[0-9]*; do
  e=$(readlink "$p/exe" 2>/dev/null) || continue; e=${e% (deleted)}
  case "${e##*/}" in learn) n=$((n+1));; esac
done
say "learn processes live: $n"
[ "$n" -gt 1 ] && { say "ABORT: $n learn processes already live -- not yet runnable"; exit 75; }

say "launching seed $SEED2, 2000 gens per arm, everything else identical"
GENS=2000 CAP=18000 SEED="$SEED2" PAIRS=224 ./blend_sweep.sh 2>&1 | tail -20 | tee -a "$LOG"

say "CROSS-SEED READING — both seeds must point the same way to license a default change:"
python3 - <<'PY' | tee -a "$LOG"
import re, os, math
def read(f):
    if not os.path.exists(f): return None
    m = re.search(r'scores (0\.\d+) \+/- (0\.\d+)', open(f).read())
    return (float(m.group(1)), float(m.group(2))) if m else None
s1 = read("blend_085_vs_075_s1.log")
s2 = read("blend_085_vs_075.log")
for lab, v in (("seed 20260917", s1), ("seed 20260918", s2)):
    if not v:
        print(f"  {lab}: NO VERDICT -- not a result in either direction"); continue
    r, c = v
    print(f"  {lab}: 0.85 vs 0.75 = {r:.3f} +/- {c:.3f}   lower bound {r-c:.3f}  "
          f"{'clears' if r - c >= 0.5 else 'does NOT clear'}")
if s1 and s2:
    m = (s1[0] + s2[0]) / 2
    se = math.sqrt(s1[1]**2 + s2[1]**2) / 2
    print(f"  POOLED: {m:.3f} +/- {se:.3f}   lower bound {m - se:.3f}")
    print()
    if (s1[0] - 0.5) * (s2[0] - 0.5) < 0:
        print("  THE SEEDS DISAGREE IN SIGN. That is exactly what a second seed exists to catch:")
        print("  0.75 STAYS the default, and the five-seed short-run prior does not override a")
        print("  full-length disagreement.")
    elif s1[0] > 0.5 and s2[0] > 0.5 and m - se >= 0.5:
        print("  BOTH seeds above 0.5 and the pool clears. That is what a default change needs --")
        print("  the effect survives the between-seed spread a single arm cannot see.")
        print("  Ship via the normal rule, measured against the CHAMPION, not against this arm.")
    else:
        print("  Same direction, pool does not clear. NOT a default change: record the bound and")
        print("  stop rather than buying a third seed for a sub-noise effect.")
PY
say "BLENDSWEEPSEED2DONE"
