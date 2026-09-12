#!/usr/bin/env bash
# Launch the Candidate A replication (candidate_a_replication_PREREG.md) once the box frees.
# rc=75 not-yet-runnable, rc=0 launched or already present, rc=1 refused.
set -uo pipefail
D=/home/maswabe/existence
cd "$D" || exit 1
LOG=$D/launch_cand_replication.log
say(){ echo "$(date +%F_%H:%M) [repl] $*" | tee -a "$LOG"; }

exec 9>"$D/.launch_cand_replication.lock"
flock -n 9 || { say "DEFER: another instance holds the lock"; exit 75; }

for u in cand-a2-fixed cand-b2-budget; do
  systemctl --user is-active "$u.service" >/dev/null 2>&1 && { say "already running: $u"; exit 0; }
done
[ -s "$D/candA2_fixed.net" ] && { say "candA2_fixed.net exists -- not relaunching"; exit 0; }

# Wait for the in-flight measurements. Cell C and the gate own two cores on 6-11; adding two more
# arms while they run would oversubscribe the range the production trainer also lives in.
systemctl --user is-active cand-c-labels.service >/dev/null 2>&1 && { say "DEFER: cell C still running"; exit 75; }
for p in $(ls /proc | grep -E '^[0-9]+$'); do
  e=$(readlink "/proc/$p/exe" 2>/dev/null) || continue
  case "${e##*/}" in netmatch) say "DEFER: a netmatch is in flight (pid $p)"; exit 75;; esac
done

# The start net must still BE the champion the original arms used. Verified, not assumed.
EXPECT=9545a35289e9
got=$(md5sum "$D/cand_start.net" 2>/dev/null | cut -c1-12)
[ "$got" = "$EXPECT" ] || { say "REFUSE: cand_start.net is $got, expected $EXPECT -- not the original start"; exit 1; }

B=$D/target/release/learn_cand
[ -x "$B" ] || { say "REFUSE: $B missing"; exit 1; }

COMMON="--init cand_start.net --gens 2000 --games 8 --threads 1 --depth 3 --epochs 3 --lr 0.0002 \
--lr-decay 1.0 --blend 0.85 --gate-every 1000000 --arch-every 0 --control-every 0 --seed 777777"
say "launching A2/B2 on training seed 777777 (original used 20260912), start md5 $got"
systemd-run --user --unit=cand-a2-fixed -p WorkingDirectory=$D -p Nice=19 \
  bash -c "taskset -c 6-11 $B $COMMON --out candA2_fixed.net --ledger ledger_candA2.jsonl > candA2_fixed.log 2>&1" >>"$LOG" 2>&1
systemd-run --user --unit=cand-b2-budget -p WorkingDirectory=$D -p Nice=19 \
  bash -c "taskset -c 6-11 $B $COMMON --datagen-budget 5269 --out candB2_budget.net --ledger ledger_candB2.jsonl > candB2_budget.log 2>&1" >>"$LOG" 2>&1
sleep 6
say "A2=$(systemctl --user is-active cand-a2-fixed.service) B2=$(systemctl --user is-active cand-b2-budget.service)"
head -1 candB2_budget.log 2>/dev/null | tee -a "$LOG"
exit 0
