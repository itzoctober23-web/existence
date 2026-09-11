#!/usr/bin/env bash
# THE RULER HALF OF THE LOW-LR SWEEP VERDICT.
#
# His instruction for the sweep (2026-09-11): "Verdict on the shared start AND on the ruler."
# `lr_sweep_low.sh` produces only the first -- `grep -c ruler` on it returns 0. This is the second
# half, run as a separate stage because the sweep itself is already running and must not be touched.
#
# WHY BOTH, AND WHY THEY ANSWER DIFFERENT QUESTIONS:
#   * netmatch vs the shared start is PAIRED and precise (+/-0.028 at 224 pairs). It says which arm
#     moved furthest from where they all began. It cannot say where that is in absolute terms.
#   * the ruler is ABSOLUTE -- Elo against a fixed external rung (SF-1320) -- and is what the day-7
#     stop condition (1600, rising) is written against. It is also far noisier: +/-60 per 120-game
#     sample.
#
# SO IT IS POOLED, which is the whole point of his instruction. Eight readings of ONE unchanged net
# at prod4 gen 2052 spanned 1424-1543 -- a 119-point swing from a net that never changed. N samples
# per arm here, inverse-variance pooled by `ruler_pool.py`'s own arithmetic, so the headline is the
# pool and the samples stay in the log as the ledger.
#
# N=4 PER ARM, NOT 9. Nine would give +/-20 but costs 27 x 120 games across three arms. Four gives
# +/-30, which is enough to place an arm against 1600 and enough to see a gap of the size the
# truncated run suggested. Stated rather than silently chosen: this is a precision/cost trade, and
# +/-30 CANNOT resolve the 0.0002-vs-0.0001 question, which netmatch is for.
#
# WAITS FOR THE SWEEP, never interrupts it: polls `low-sweep.service` until it is no longer active.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
N=${N:-4}
LOG=low_sweep_ruler.log
say(){ echo "$(date +%F_%H:%M) [sweepruler] $*" | tee -a "$LOG"; }

say "waiting for low-sweep.service to finish (will not interrupt it)"
while systemctl --user is-active low-sweep.service >/dev/null 2>&1; do sleep 60; done
sleep 20

for L in 00005 00002 00001; do
  NET=low_$L.net
  [ -s "$NET" ] || { say "arm $L produced no net -- skipping"; continue; }
  G=$(grep -cE '^gen ' "low_$L.log" 2>/dev/null); G=${G:-0}
  say "arm lr 0.$L ($G gens): $N ruler samples"
  for i in $(seq 1 "$N"); do
    cp -f "$NET" "/tmp/r_sweep_$L.net"
    nice -n 19 taskset -c 6-11 python3 sf_ruler.py --net "/tmp/r_sweep_$L.net" \
      --depth 4 --sf-elo 1320 --sf-nodes 10000 --games 120 > "sweep_${L}_ruler_$i.log" 2>&1
    R=$(grep -oE '[+-][0-9]+ \+/- [0-9]+' "sweep_${L}_ruler_$i.log" | head -1)
    say "  sample $i/$N: ${R:-NO READING}"
    # Append in live_ruler.out's format so ruler_pool.py and the watch page both pick it up.
    [ -n "$R" ] && echo "$(date +%H:%M) sweep$L gen $G Elo vs this opponent: $R" >> live_ruler.out
  done
done

say "POOLED per arm (inverse-variance; individual samples remain in the logs):"
python3 - <<'PY' | tee -a "$LOG"
import re, math, glob
for L in ("00005","00002","00001"):
    obs=[]
    for f in sorted(glob.glob(f"sweep_{L}_ruler_*.log")):
        m=re.search(r'([+-]\d+)\s*\+/-\s*(\d+)', open(f).read())
        if m: obs.append((float(m.group(1)), float(m.group(2))))
    if not obs:
        print(f"  lr 0.{L}: no readings"); continue
    w=sum(1/(s*s) for _,s in obs)
    pool=sum(v/(s*s) for v,s in obs)/w
    se=math.sqrt(1/w)
    lo=min(v for v,_ in obs); hi=max(v for v,_ in obs)
    print(f"  lr 0.{L}: POOLED {1320+pool:.0f} +/- {se:.0f}  (n={len(obs)}, raw span {1320+lo:.0f}-{1320+hi:.0f})")
print("  Day-7 stop condition is 1600 on the POOLED ruler with a rising trend.")
print("  A gap smaller than the combined SE is not a difference -- netmatch is the precise instrument.")
PY
say "SWEEPRULERDONE"
