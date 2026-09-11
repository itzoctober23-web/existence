#!/usr/bin/env bash
# THE ABSOLUTE HALF OF THE BLEND VERDICT — does blend 0.85 move the LEVEL toward 1600?
#
# WHY BOTH INSTRUMENTS, AND WHY THEY ANSWER DIFFERENT QUESTIONS.
#   * `blend_sweep.sh`'s netmatch is PAIRED and precise (+/-0.028 at 224 pairs). It answers "is
#     0.85 stronger than 0.75", which is what the promotion rule needs. It cannot say WHERE either
#     arm sits in absolute terms.
#   * The ruler is ABSOLUTE (Elo against a fixed external SF-1320 rung) and is the instrument his
#     stop condition is written against: "1600 on the pooled ruler with a rising trend by day 7".
#     It is far noisier -- +/-55-60 per 120-game sample.
#
# THIS IS THE QUESTION THAT ACTUALLY MATTERS THIS WEEK. `ruler_trend_RESULT.md` showed every
# production run is FLAT (|z| < 2 on all seven, chi2/dof 0.32-0.75, ruler NOT saturated) while the
# LEVEL moved +166 +/- 22 ACROSS runs. So a configuration change is the only thing that has ever
# moved the number, and the only useful question about blend 0.85 is how far it moves it. The
# shipped champion pools to 1481 +/- 19 and the target is 1600 -- a gap of 119.
#
# POOLED, per his directive: four samples per arm, inverse-variance combined, individual readings
# left in live_ruler.out as the ledger. Four gives ~+/-30, which places an arm against 1600 but
# CANNOT resolve a 0.75-vs-0.85 difference of the size netmatch reports. That is not a flaw --
# netmatch is the instrument for the contrast, this one is for the altitude. Stated rather than
# silently assumed.
#
# WAITS, NEVER INTERRUPTS: polls blend-sweep.service until it is gone.
set -uo pipefail
cd "$(dirname "$0")"
N=${N:-4}
LOG=blend_sweep_ruler.log
say(){ echo "$(date +%F_%H:%M) [blendruler] $*" | tee -a "$LOG"; }

say "waiting for blend-sweep.service to finish (will not interrupt it)"
while systemctl --user is-active blend-sweep.service >/dev/null 2>&1; do sleep 60; done
sleep 20

# A truncated arm makes its ruler reading meaningless as a comparison. Report, do not hide.
g075=$(grep -cE '^gen ' blend_075.log 2>/dev/null); g085=$(grep -cE '^gen ' blend_085.log 2>/dev/null)
say "arms completed: 0.75 -> ${g075:-0} gens, 0.85 -> ${g085:-0} gens"
[ "${g075:-0}" -ne "${g085:-0}" ] && say "  WARNING: UNEQUAL ARMS -- the level comparison is confounded with training amount"

for B in 075 085; do
  NET=blend_$B.net
  [ -s "$NET" ] || { say "arm $B produced no net -- skipping"; continue; }
  G=$(grep -cE '^gen ' "blend_$B.log" 2>/dev/null); G=${G:-0}
  say "arm blend 0.$B ($G gens): $N ruler samples"
  for i in $(seq 1 "$N"); do
    # Copy first: sampling the live file while a later stage could rewrite it is how a reading
    # ends up describing a net that no longer exists.
    cp -f "$NET" "/tmp/r_blend_$B.net"
    nice -n 19 taskset -c 6-11 python3 sf_ruler.py --net "/tmp/r_blend_$B.net" \
      --depth 4 --sf-elo 1320 --sf-nodes 10000 --games 120 > "blend_${B}_ruler_$i.log" 2>&1
    R=$(grep -oE '[+-][0-9]+ \+/- [0-9]+' "blend_${B}_ruler_$i.log" | head -1)
    say "  sample $i/$N: ${R:-NO READING}"
    # live_ruler.out's exact format, so ruler_pool.py and the watch page both pick it up.
    [ -n "$R" ] && echo "$(date +%H:%M) blend$B gen $G Elo vs this opponent: $R" >> live_ruler.out
  done
done

say "POOLED per arm (inverse-variance; individual samples stay in the logs):"
python3 - <<'PY' | tee -a "$LOG"
import re, math, glob
TARGET = 1600
out = {}
for B, lab in (("075", "blend 0.75 (shipped)"), ("085", "blend 0.85 (candidate)")):
    obs = []
    for f in sorted(glob.glob(f"blend_{B}_ruler_*.log")):
        m = re.search(r'([+-]\d+)\s*\+/-\s*(\d+)', open(f).read())
        if m:
            obs.append((float(m.group(1)), float(m.group(2))))
    if not obs:
        print(f"  {lab}: no readings"); continue
    w = sum(1/(s*s) for _, s in obs)
    pool = sum(v/(s*s) for v, s in obs)/w
    se = math.sqrt(1/w)
    lo = min(v for v, _ in obs); hi = max(v for v, _ in obs)
    out[B] = (1320+pool, se)
    print(f"  {lab}: POOLED {1320+pool:.0f} +/- {se:.0f}  (n={len(obs)}, raw span {1320+lo:.0f}-{1320+hi:.0f})")
if "075" in out and "085" in out:
    d = out["085"][0] - out["075"][0]
    cse = math.sqrt(out["085"][1]**2 + out["075"][1]**2)
    print(f"  difference 0.85 - 0.75: {d:+.0f} +/- {cse:.0f}  ({abs(d)/cse:.1f} sigma)")
    print("  A gap smaller than the combined SE is NOT a difference -- netmatch is the precise")
    print("  instrument for the contrast. This measures ALTITUDE, not the contrast.")
    best = max(out.values())[0]
    print(f"  against the {TARGET} stop condition: best arm is {TARGET-best:+.0f} away "
          f"({abs(TARGET-best)/max(out['085'][1],1e-9):.1f} SE)")
    print(f"  shipped champion pools to 1481 +/- 19 for reference.")
PY
say "BLENDRULERDONE"
