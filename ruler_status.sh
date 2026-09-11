#!/usr/bin/env bash
# ONE LINE PER DAY IN STATE.md: where the pooled ruler is, and whether it is RISING.
#
# His pooling directive (2026-09-11) asked for the pooled estimate per rung on "the page and the
# daily line". The page (xtask watch) does it. The daily line did not exist. This is it.
#
# It reports BOTH halves of the stop condition, because reporting only the level invites reading a
# trend off consecutive single samples -- which is exactly what the directive forbids, and what
# prodk0759's 1396-to-1565 swing would support in either direction.
set -uo pipefail
cd "$(dirname "$0")"
RUN=${1:-}
if [ -z "$RUN" ]; then
  # newest run BY MEASUREMENT ORDER, never alphabetical: sorting by name once reported run "rd"
  # as the newest simply because r sorts last.
  RUN=$(grep -oE '^[0-9]{2}:[0-9]{2} +\S+' live_ruler.out 2>/dev/null | awk '{print $2}' | tail -1)
fi
[ -n "$RUN" ] || { echo "RULER $(date +%F) — no readings parsed (check live_ruler.out format)"; exit 1; }

T=$(./ruler_trend.py "$RUN" 2>/dev/null | awk -v r="$RUN" '$1==r')
if [ -z "$T" ]; then
  L=$(./ruler_pool.py --latest 2>/dev/null)
  echo "RULER $(date +%F) — $RUN: ${L:-no pooled reading} | trend: not enough rungs yet (needs 3)"
  exit 0
fi
LVL=$(echo "$T" | awk '{print $5}'); LSE=$(echo "$T" | awk '{print $6}')
SLOPE=$(echo "$T" | awk '{print $7}'); SSE=$(echo "$T" | awk '{print $8}')
N=$(echo "$T" | awk '{print $2}')
VERD=$(echo "$T" | sed 's/.*  //')
GAP=$(python3 -c "print(f'{1600-$LVL:.0f}')" 2>/dev/null)
SE=$(python3 -c "print(f'{(1600-$LVL)/$LSE:.1f}')" 2>/dev/null)
echo "RULER $(date +%F) — $RUN pooled ${LVL} +/- ${LSE} (n=${N} rungs) | trend ${SLOPE} +/- ${SSE} Elo/1000 gens = ${VERD} | stop condition 1600: ${GAP} short (${SE} SE)"
