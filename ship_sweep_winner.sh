#!/usr/bin/env bash
# SHIP THE LOW-LR SWEEP WINNER THROUGH THE NORMAL RULE — or refuse, with the reason.
#
# His instruction (2026-09-11): "If 0.0002 wins, ship it and promote through the normal rule; if
# not, 0.0005 stays. No other training changes until that verdict exists."
#
# THE NORMAL RULE IS `rate - ci95 >= 0.5`, AND IT IS MEASURED AGAINST THE CHAMPION.
# The sweep measures each arm against the SHARED START. Those are the same net right now
# (both 34a5ace752d8), which is the only reason the sweep verdict is directly shippable.
#
# BUT AUTO_PROMOTE IS RUNNING AND CAN MOVE THE CHAMPION MID-SWEEP. It banks any production arm that
# clears the same bar, on its own 30-minute cycle, and prodk0633 is past 11,000 generations. If it
# promotes before this runs, "vs shared start" stops being "vs champion" and shipping on the sweep
# number would be promoting against a net that is no longer the incumbent.
#
# The ARMS remain matched to each other whatever auto_promote does -- their relative comparison is
# untouched, so the SCIENCE is safe either way. Only the ship step is affected. So: check, and if the
# champion moved, re-measure against the real incumbent before promoting. Never promote on a stale
# baseline.
#
# REFUSES rather than guessing in every ambiguous case, because an unnecessary promotion is harder to
# undo than a missed one -- the previous champion is preserved on disk, but a wrong champion
# poisons every measurement taken against it until someone notices.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
NM=$SCR/xt_lrd/netmatch
PAIRS=${PAIRS:-224}
LOG=ship_sweep.log
say(){ echo "$(date +%F_%H:%M) [ship] $*" | tee -a "$LOG"; }

# Parse "lr 0.0002 vs shared start: scores 0.551 +/- 0.028" out of the sweep's own output.
best=""; best_lb=""
for L in 0.0005 0.0002 0.0001; do
  T=$(echo "$L" | tr -d '.')
  line=$(grep -F "lr $L vs shared start" low_sweep2.out 2>/dev/null | tail -1)
  r=$(echo "$line" | grep -oE 'scores 0\.[0-9]+' | grep -oE '0\.[0-9]+')
  c=$(echo "$line" | grep -oE '\+/- 0\.[0-9]+' | grep -oE '0\.[0-9]+')
  [ -n "$r" ] && [ -n "$c" ] || { say "arm $L: no verdict parsed -- refusing to act on a partial run"; exit 1; }
  lb=$(python3 -c "print(f'{$r-$c:.3f}')")
  say "arm lr $L: $r +/- $c   lower bound $lb"
  if [ -z "$best_lb" ] || python3 -c "import sys; sys.exit(0 if $lb > $best_lb else 1)"; then best=$L; best_lb=$lb; fi
done

say "best arm: lr $best (lower bound $best_lb; rule needs >= 0.500)"
python3 -c "import sys; sys.exit(0 if $best_lb >= 0.5 else 1)" || {
  say "NO ARM CLEARS THE RULE. 0.0005 stays shipped, exactly as he specified. Nothing promoted."; exit 0; }

if [ "$best" = "0.0005" ]; then
  say "the winner IS the already-shipped rate. Nothing to promote; 0.0005 stays."; exit 0
fi

# The baseline check that makes this safe.
if cmp -s low_start.net p1_champion.net; then
  say "champion is still the sweep's shared start -- the verdict is measured against the incumbent."
else
  say "CHAMPION MOVED since the sweep began (start $(md5sum low_start.net|cut -c1-12), now $(md5sum p1_champion.net|cut -c1-12))."
  say "  re-measuring lr $best against the CURRENT champion before promoting -- the sweep number is vs a stale baseline."
  T=$(echo "$best" | tr -d '.')
  nice -n 19 taskset -c 6-11 "$NM" "low_$T.net" p1_champion.net "$PAIRS" > "ship_${T}_vs_champion.log" 2>&1
  line=$(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+' "ship_${T}_vs_champion.log" | head -1)
  r=$(echo "$line" | grep -oE '0\.[0-9]+' | head -1); c=$(echo "$line" | grep -oE '0\.[0-9]+' | tail -1)
  [ -n "$r" ] && [ -n "$c" ] || { say "  re-measure produced no verdict -- refusing to promote"; exit 1; }
  lb=$(python3 -c "print(f'{$r-$c:.3f}')")
  say "  vs CURRENT champion: $r +/- $c  lower bound $lb"
  python3 -c "import sys; sys.exit(0 if $lb >= 0.5 else 1)" || {
    say "  does NOT clear the rule against the current champion. Not promoting; 0.0005 stays."; exit 0; }
fi

T=$(echo "$best" | tr -d '.')
cp -f p1_champion.net "p1_champion_prev_pre_lr$T.net"
cp -f "low_$T.net" p1_champion.net
say "PROMOTED lr $best -> p1_champion (previous kept as p1_champion_prev_pre_lr$T.net)"
say "  champion is now $(md5sum p1_champion.net|cut -c1-12)"
say "  passed the gate. No Elo figure is quoted for it."
echo "SHIPDONE"
