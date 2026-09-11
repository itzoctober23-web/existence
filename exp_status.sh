#!/usr/bin/env bash
# ONE VALIDATED STATUS PROBE FOR THE RUNNING EXISTENCE EXPERIMENTS.
#
# WHY THIS EXISTS. The per-tick checks were ad-hoc greps rewritten each time, and two of them
# produced FALSE ALARMS on 2026-09-11:
#
#   * `ls -t prodk*.log | head -1` also matches `prodk0759_ruler.log`, which holds ruler readings
#     and ZERO `^gen ` lines. On a tied mtime the ruler log sorts first, so a healthy 17,616-
#     generation run reported "prod gens: 0" -- and under the standing rule a dead trainer must be
#     relaunched, so a false zero is what authorises starting a SECOND trainer on a one-trainer box.
#   * `grep -F sample ruler.log | tail -2` also matches the header line "4 ruler samples", so a
#     completed sample was invisible and a perfectly healthy ruler looked stuck for 25 minutes.
#
# Both are the standing trap: a pattern that matches the wrong thing reads as a fact. Every probe
# below is anchored to a form that was checked against real output, and anything that cannot be
# read prints WHY rather than a zero.
#
# Generation lines are INDENTED by the evolve loop, and `^gen ` finds none of them.
set -uo pipefail
cd "$(dirname "$0")"
say(){ printf '%s\n' "$*"; }
gens(){ local n; n=$(grep -cE '^[[:space:]]*gen ' "$1" 2>/dev/null); echo "${n:-0}"; }

say "=== EXISTENCE EXPERIMENTS  $(date '+%H:%M:%S') ==="

# ---- TRAINERS: derived from each process's OWN --out, never a filename glob -------------------
say "-- trainers"
n=0
for p in /proc/[0-9]*; do
  e=$(readlink "$p/exe" 2>/dev/null) || continue; e=${e% (deleted)}
  [ "${e##*/}" = learn ] || continue
  o=$(tr '\0' '\n' < "$p/cmdline" 2>/dev/null | grep -A1 -x -- '--out' | tail -1)
  [ -n "$o" ] || continue
  n=$((n+1)); b=$(basename "${o%.net}")
  say "   $(printf '%-12s' "$b") $(gens "$b.log") generations"
done
[ "$n" -eq 0 ] && say "   *** NO learn PROCESS -- production is down"
say "   alive: $n"

# ---- RULER: read the SAMPLE FILES, which are the artifacts, not the narration -----------------
# The log line can lag or be matched by a header; a sample file either holds a reading or does not.
say "-- ruler (seed-1 blend arms, frozen copies)"
for arm in 075 085; do
  done_=0; vals=""
  for f in blend_s1_${arm}_ruler_*.log; do
    [ -e "$f" ] || continue
    r=$(grep -oE 'Elo vs this opponent: [+-][0-9]+' "$f" 2>/dev/null | grep -oE '[+-][0-9]+$')
    if [ -n "$r" ]; then done_=$((done_+1)); vals="$vals $r"; fi
  done
  say "   blend 0.$arm: $done_/4 complete ${vals:+[$vals ]}"
done
python3 - <<'PY' 2>/dev/null
import re, glob, math
out={}
for arm in ("075","085"):
    obs=[]
    for f in sorted(glob.glob(f"blend_s1_{arm}_ruler_*.log")):
        m=re.search(r'Elo vs this opponent:\s*([+-]\d+)\s*\+/-\s*(\d+)', open(f).read())
        if m: obs.append((float(m.group(1)), float(m.group(2))))
    if len(obs) < 2: continue
    w=sum(1/(s*s) for _,s in obs)
    out[arm]=(1320+sum(v/(s*s) for v,s in obs)/w, math.sqrt(1/w), len(obs))
for arm,(m,se,k) in out.items():
    print(f"   blend 0.{arm} pooled {m:.0f} +/- {se:.0f} (n={k})")
if len(out)==2:
    d=out['085'][0]-out['075'][0]; cse=math.sqrt(out['085'][1]**2+out['075'][1]**2)
    print(f"   difference {d:+.0f} +/- {cse:.0f} ({abs(d)/cse:.1f} sigma)"
          + ("  [INCOMPLETE -- fewer than 4 samples on an arm]" if min(v[2] for v in out.values())<4 else ""))
PY

# ---- 2x2 CELLS: MAIN lineage only, distinct GENERATIONS not log lines -------------------------
say "-- choice 2x2 (MAIN lineage; MCTS is a separate search and is not pooled)"
./choice_report.py 2>/dev/null | sed -n '6,10p' | sed 's/^/  /'

# ---- UNITS -----------------------------------------------------------------------------------
say "-- units"
for u in blend-ruler-s1 blend-sweep-seed2 proposals-ab choice-2x2 search-long-run \
         existence-keepalive existence-live-ruler existence-auto-promote; do
  printf '   %-24s %s\n' "$u" "$(systemctl --user is-active "$u.service" 2>/dev/null)"
done

say "-- repo"
say "   existence: $(git status --porcelain 2>/dev/null | wc -l) dirty"
