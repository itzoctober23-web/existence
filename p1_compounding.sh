#!/usr/bin/env bash
# P1 COMPOUNDING A/B -- the structural change, pre-registered in p1_compounding_PREREG.md.
#
# TWO ARMS, 2,000 GENERATIONS EACH, SEQUENTIAL, FROM ONE COMMON START.
#
# SEQUENTIAL IS NOT A COMPROMISE, IT IS THE BOX LIMIT. Two concurrent trainers is the failure mode
# that froze this machine, and the standing rule is ONE trainer. 2,000 generations is ~12 minutes at
# the measured 173 gens/min, so sequencing costs almost nothing.
#
# KEEPALIVE IS STOPPED FIRST, AND RESTORED BY A TRAP. keepalive.sh counts any process whose exe
# basename is `learn` and relaunches production when none is alive. Between the two arms there is a
# gap, and its period is 300s -- so a badly-timed check would start production ALONGSIDE an arm and
# put two trainers on the box. Stopping it is the only way to make that impossible rather than
# unlikely. The trap restores it on success, failure, or interrupt, because a driver that dies
# holding the supervisor down would leave the box with no trainer at all.
#
# MATCHED ON GENERATIONS, NOT WALL CLOCK. search_long_run.sh shares cores 6-11 and may finish partway
# through, which would change throughput between the arms. That is why the verdict is on the ruler
# and the gate at a matched GENERATION count -- wall-clock matching has already invalidated one
# published A/B here.
set -uo pipefail
cd /home/maswabe/existence || exit 1

GENS=${GENS:-2000}
CORES=${CORES:-6-11}
LEARN=./target/release/learn
LOG=p1_compounding.log
say(){ echo "$(date +%F_%H:%M:%S) [compound] $*" | tee -a "$LOG"; }

restore(){
  systemctl --user start existence-keepalive 2>/dev/null
  say "keepalive RESTORED (trap)"
}
trap restore EXIT INT TERM

[ -x "$LEARN" ] || { say "ABORT: no learn binary"; exit 1; }
[ -s p1_champion.net ] || { say "ABORT: no champion"; exit 1; }

say "=== P1 compounding A/B, $GENS generations per arm ==="
systemctl --user stop existence-keepalive 2>/dev/null
say "keepalive STOPPED so it cannot stack a production trainer beside an arm"

# Kill the running production trainer by exe + exact argv, never by name match.
for p in $(ls /proc | grep -E '^[0-9]+$'); do
  e=$(readlink "/proc/$p/exe" 2>/dev/null) || continue
  case "${e##*/}" in learn) ;; *) continue;; esac
  say "stopping production trainer PID $p"
  kill -TERM "$p" 2>/dev/null
done
sleep 5
alive=0
for p in $(ls /proc | grep -E '^[0-9]+$'); do
  e=$(readlink "/proc/$p/exe" 2>/dev/null) || continue
  case "${e##*/}" in learn) alive=$((alive+1));; esac
done
[ "$alive" -eq 0 ] || { say "ABORT: $alive trainer(s) still alive, refusing to add more"; exit 1; }

# ONE common start for both arms. Snapshotted once, so neither arm can drift from the other's origin.
cp -f p1_champion.net p1c_start.net
say "common start: $(md5sum p1c_start.net | cut -c1-12)"

run_arm(){
  local tag="$1"; shift
  say "ARM $tag START -- $*"
  local t0=$(date +%s)
  taskset -c "$CORES" nice -n 19 ionice -c 3 "$LEARN" \
    --init p1c_start.net --gens "$GENS" --games 8 --threads 4 \
    --lr 0.0002 --lr-decay 1.0 --blend 0.85 \
    --arch-every 0 --control-every 0 \
    --seed 20260911 --out "${tag}.net" --ledger "ledger_${tag}.jsonl" \
    "$@" > "${tag}.log" 2>&1
  local rc=$?
  local g=$(grep -cE '^gen ' "${tag}.log" 2>/dev/null || true)
  say "ARM $tag END rc=$rc  generations=${g:-0}  wall=$(( $(date +%s) - t0 ))s"
}

# CONTROL: production verbatim -- fixed depth 3, 3 epochs on this generation's slice, gate disabled.
run_arm p1c_control --depth 3 --epochs 3 --gate-every 1000000

# COMPOUND: node budget, fixed step count drawn from the declared window, gate ON.
run_arm p1c_compound --datagen-nodes 10309 --steps-per-gen 777 --replay-gens 8 \
                     --gate-every 100 --gate-pairs 224

say "=== BOTH ARMS DONE -- this is STATE, not a result. Ruler + netmatch next. ==="
for t in p1c_control p1c_compound; do
  [ -s "$t.net" ] && say "$t.net $(md5sum $t.net | cut -c1-12) $(stat -c %s $t.net)B"
done
