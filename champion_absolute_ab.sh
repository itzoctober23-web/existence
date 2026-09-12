#!/usr/bin/env bash
# DID THE 04:43 PROMOTION MAKE THE CHAMPION WEAKER? Measure both champion FILES against the same
# external anchor, back to back, at 5x the ruler's usual game count.
#
# WHY THIS EXISTS. Two individually-sound instruments disagree about the same net over the same span:
#
#   896-pair netmatch (auto_promote, seed 911911, confirmed)  gen 24941 beats its start by
#                                                             +25.1 Elo  95% CI [+14.6, +35.6]
#   live ruler, 38 readings vs SF-1320 @10k nodes, OLS on gen  -61.2 Elo  95% CI [-99.2, -23.2]
#
# The intervals do not overlap. Both measurements are sound on their own terms -- the netmatch is
# paired, fixed-depth and was confirmed on an independent seed; the ruler is fixed-depth/fixed-nodes
# (load-immune), snapshots the net before playing, and uses one unchanging anchor. They disagree
# because they measure DIFFERENT THINGS: one is strength against the net's own predecessor, the
# other is strength against a fixed external opponent. That is the signature of self-play
# non-transitivity, and if it is real the promotion ladder can climb forever without absolute gain.
#
# It matters right now because auto_promote OVERWROTE p1_champion.net at 04:43. The comparator it
# beat is preserved as p1_champion_prev_g24941.net (md5 9545a35289e9, the shipped champion), so the
# two files can be compared directly. That is what this does.
#
# WHY THE TREND ALONE IS NOT ENOUGH. The OLS slope is measured over prodk0127's whole run and the
# newest readings are at gen 29386, not at the promoted gen 24941. This compares the exact two FILES
# that are at stake instead.
#
# SIZE. The ruler's usual 120 games gives +/-66 Elo, so a difference of two such readings carries
# +/-93 -- too wide to resolve a 61 Elo gap. 600 games per side gives roughly +/-30 each and +/-42 on
# the difference. Sequential, not concurrent: the two runs would otherwise contend, and while both
# are load-IMMUNE in result (fixed depth, fixed nodes) there is no reason to spend the wall clock.
#
#   rc=0   both sides measured, verdict printed
#   rc=75  deferred (another instance, or no capacity)
#   rc=1   a precondition failed -- abort rather than report half an answer
set -uo pipefail
D=/home/maswabe/existence
cd "$D" || exit 1
LOG=$D/champion_absolute_ab.log
GAMES=${GAMES:-600}
say(){ echo "$(date +%F_%H:%M) [chabs] $*" | tee -a "$LOG" >&2; }

exec 9>"$D/.champion_absolute_ab.lock"
flock -n 9 || { say "DEFER: another instance holds the lock"; exit 75; }
[ -s "$D/champion_absolute_ab_done" ] && { say "already complete -- not re-running"; exit 0; }

NEW=$D/p1_champion.net                      # promoted 04:43
OLD=$D/p1_champion_prev_g24941.net          # the comparator it beat, = the shipped champion
for f in "$NEW" "$OLD"; do
  [ -r "$f" ] || { say "ABORT: $f unreadable"; exit 1; }
done
# The files must actually DIFFER, or this measures nothing twice.
mn=$(md5sum "$NEW" | cut -c1-12); mo=$(md5sum "$OLD" | cut -c1-12)
[ "$mn" != "$mo" ] || { say "ABORT: both champions hash $mn -- nothing to compare"; exit 1; }
[ "$mo" = "9545a35289e9" ] || say "NOTE: old champion is $mo, expected the shipped 9545a35289e9"
say "NEW=$mn  OLD=$mo  ($GAMES games each vs SF-1320 @10k nodes)"

# Capacity, measured rather than assumed -- same shape as launch_cand_replication.sh. A 2-second
# delta from /proc/PID/stat, not `ps %cpu`, which is a lifetime average.
jif(){ awk '{print $14+$15}' "/proc/$1/stat" 2>/dev/null || echo 0; }
pids=""; for p in $(ls /proc | grep -E '^[0-9]+$'); do
  e=$(readlink "/proc/$p/exe" 2>/dev/null) || continue
  case "${e##*/}" in learn|learn_cand|learn_cand2|netmatch) pids="$pids $p";; esac
done
t0=0; for p in $pids; do t0=$((t0+$(jif $p))); done
sleep 2
t1=0; for p in $pids; do t1=$((t1+$(jif $p))); done
used=$(( (t1-t0)*100 / ($(getconf CLK_TCK)*2) ))
say "cores 6-11 in use: ${used}% of 600%"
[ "$used" -gt 520 ] && { say "DEFER: ${used}% leaves no room for a 100% ruler run"; exit 75; }

run_side(){ # $1=label $2=net
  local out=$D/chabs_$1.log
  cp -f "$2" "/tmp/chabs_$1.net" || return 1
  say "measuring $1 ..."
  nice -n 19 taskset -c "${RULER_CORES:-6-11}" python3 sf_ruler.py --net "/tmp/chabs_$1.net" \
    --depth 4 --sf-elo 1320 --sf-nodes 10000 --games "$GAMES" > "$out" 2>&1
  rm -f "/tmp/chabs_$1.net"
  grep -oE 'Elo vs this opponent: .*' "$out" | tail -1
}
RN=$(run_side new "$NEW"); RO=$(run_side old "$OLD")
say "NEW $mn : ${RN:-NO RESULT}"
say "OLD $mo : ${RO:-NO RESULT}"
[ -n "$RN" ] && [ -n "$RO" ] || { say "ABORT: a side produced no result -- reporting nothing"; exit 1; }

python3 - "$RN" "$RO" <<'PY' | tee -a "$LOG"
import sys,re,math
def p(s):
    m=re.search(r'([-+]?\d+)\s*\+/-\s*(\d+)', s)
    return (float(m.group(1)), float(m.group(2))) if m else (None,None)
(n,cn),(o,co)=p(sys.argv[1]),p(sys.argv[2])
if n is None or o is None: print("  could not parse"); sys.exit(0)
d=n-o; sd=math.sqrt((cn/1.96)**2+(co/1.96)**2); lo,hi=d-1.96*sd,d+1.96*sd
print(f"\n  NEW (promoted 04:43) {n:+.0f} +/- {cn:.0f}")
print(f"  OLD (shipped champ)  {o:+.0f} +/- {co:.0f}")
print(f"  DIFFERENCE           {d:+.0f}  95% CI [{lo:+.0f}, {hi:+.0f}]\n")
if hi < 0:
    print("  VERDICT: the promotion made the champion WEAKER in absolute terms.")
    print("  The 896-pair netmatch is then measuring strength against the predecessor only,")
    print("  and the promotion criterion does not track absolute strength. REVERT the champion")
    print("  to the previous file and treat the ladder as unsound until the criterion changes.")
elif lo > 0:
    print("  VERDICT: the promotion made the champion STRONGER. The ruler's negative within-run")
    print("  trend is then NOT about the promoted net, and the plateau doc's reading needs redoing.")
else:
    print("  VERDICT: UNRESOLVED -- the interval contains 0. This does not confirm the promotion")
    print("  and does not refute it. Do NOT report either direction; say the size needed is larger.")
PY
date +%F_%H:%M > "$D/champion_absolute_ab_done"
say "done"
exit 0
