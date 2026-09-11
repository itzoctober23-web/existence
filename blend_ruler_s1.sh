#!/usr/bin/env bash
# ABSOLUTE RULER FOR THE SEED-1 BLEND ARMS — measured from IMMUTABLE COPIES, by explicit path.
#
# WHY THIS REPLACES blend_sweep_ruler.sh, WHICH SILENTLY MEASURED THE WRONG NETS.
# That script waited on `blend-sweep.service` and then read `blend_075.net` / `blend_085.net` by
# their fixed names. Both assumptions broke at 09:55:
#
#   * `blend_sweep_seed2.sh` invokes `./blend_sweep.sh` DIRECTLY, not through that unit. So the
#     unit went inactive while a SECOND sweep was actively running -- the wait condition was
#     satisfied by a proxy that had stopped meaning what it used to mean.
#   * The fixed filenames were being REWRITTEN by that second sweep. The ruler logged
#     "arms completed: 0.75 -> 13 gens" and began sampling a 13-generation net as though it were
#     the 2000-generation arm.
#
# Caught after ONE sample; it was voided and nothing reached `live_ruler.out`, so the pooled ledger
# and the watch page were never contaminated. This is the same lesson applied to the datagen lane
# job an hour earlier: WAIT ON THE CONDITION, NOT A PROXY FOR IT -- and better still, do not wait
# at all when the inputs can simply be named.
#
# THREE DEFENCES, because a wrong reading here is indistinguishable from a real one:
#   1. NETS ARE NAMED EXPLICITLY on the command line. No globs, no fixed names that a later run
#      reuses.
#   2. GENERATION COUNT IS ASSERTED. Each arm's log must show 2000 generations. A 13-generation net
#      fails loudly instead of producing a plausible number.
#   3. IMMUTABLE COPIES ARE TAKEN ONCE, up front, and every sample reads those. Nothing that runs
#      later can change what is being measured half way through the series.
set -uo pipefail
cd "$(dirname "$0")"
N=${N:-4}
WANT_GENS=${WANT_GENS:-2000}
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/blend_s1
LOG=blend_ruler_s1.log
say(){ echo "$(date +%F_%H:%M) [ruler_s1] $*" | tee -a "$LOG"; }

mkdir -p "$SCR"
declare -A SNAP
for arm in 075 085; do
  net="blend_${arm}_s1.net"; log="blend_${arm}_s1.log"
  [ -s "$net" ] || { say "ABORT: $net missing -- refusing to fall back to a fixed name"; exit 1; }
  g=$(grep -cE '^[[:space:]]*gen ' "$log" 2>/dev/null); g=${g:-0}
  if [ "$g" -ne "$WANT_GENS" ]; then
    say "ABORT: $log shows $g generations, expected $WANT_GENS."
    say "  That is the exact failure this script exists to prevent -- a short net measured as if"
    say "  it were the full-length arm. Fix the input, do not relax the check."
    exit 1
  fi
  cp -f "$net" "$SCR/$arm.net"
  SNAP[$arm]=$(md5sum "$SCR/$arm.net" | cut -c1-12)
  say "arm 0.${arm}: $g generations, immutable copy ${SNAP[$arm]}"
done

for arm in 075 085; do
  say "arm 0.${arm}: $N ruler samples from the frozen copy"
  for i in $(seq 1 "$N"); do
    now=$(md5sum "$SCR/$arm.net" | cut -c1-12)
    [ "$now" = "${SNAP[$arm]}" ] || { say "  ABORT: frozen copy changed ($now != ${SNAP[$arm]})"; exit 1; }
    nice -n 19 taskset -c 6-11 python3 sf_ruler.py --net "$SCR/$arm.net" \
      --depth 4 --sf-elo 1320 --sf-nodes 10000 --games 120 > "blend_s1_${arm}_ruler_$i.log" 2>&1
    R=$(grep -oE '[+-][0-9]+ \+/- [0-9]+' "blend_s1_${arm}_ruler_$i.log" | head -1)
    say "  sample $i/$N: ${R:-NO READING}"
    # Tag carries the SEED and the generation count, so a reading can never again be mistaken for
    # a different run's arm in the pooled ledger.
    [ -n "$R" ] && echo "$(date +%H:%M) blend${arm}s1 gen $WANT_GENS Elo vs this opponent: $R" >> live_ruler.out
  done
done

say "POOLED per arm (inverse-variance; samples remain in the logs):"
python3 - <<'PY' | tee -a "$LOG"
import re, math, glob
out = {}
for arm, lab in (("075", "blend 0.75 (shipped)"), ("085", "blend 0.85 (candidate)")):
    obs = []
    for f in sorted(glob.glob(f"blend_s1_{arm}_ruler_*.log")):
        m = re.search(r'([+-]\d+)\s*\+/-\s*(\d+)', open(f).read())
        if m: obs.append((float(m.group(1)), float(m.group(2))))
    if not obs:
        print(f"  {lab}: no readings"); continue
    w = sum(1/(s*s) for _, s in obs)
    pool = sum(v/(s*s) for v, s in obs)/w
    se = math.sqrt(1/w)
    lo = min(v for v, _ in obs); hi = max(v for v, _ in obs)
    out[arm] = (1320+pool, se)
    print(f"  {lab}: POOLED {1320+pool:.0f} +/- {se:.0f}  (n={len(obs)}, raw span {1320+lo:.0f}-{1320+hi:.0f})")
if "075" in out and "085" in out:
    d = out["085"][0] - out["075"][0]
    cse = math.sqrt(out["085"][1]**2 + out["075"][1]**2)
    print(f"  difference 0.85 - 0.75: {d:+.0f} +/- {cse:.0f}  ({abs(d)/cse:.1f} sigma)")
    print("  The ruler measures ALTITUDE, not the contrast -- netmatch resolved that at 0.541 +/- 0.032.")
    best = max(v[0] for v in out.values())
    print(f"  against the 1600 stop condition: best arm is {1600-best:+.0f} away")
    print("  shipped champion pools to 1481 +/- 19 for reference.")
PY
say "BLENDRULERS1DONE"
