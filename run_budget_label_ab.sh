#!/usr/bin/env bash
# Run budget_label_ab at size, then gate its two diagnostic nets.
#
# WHY IT IS LAST IN THE PIPELINE. It answers the channel question left open by
# candidate_a_budget_loses_RESULT.md: the budget arm lost, but it changed labels, positions AND the
# decisive-game rate together. This isolates the LABEL channel alone, on a FIXED corpus with no
# self-play -- the only construction that can, per candidate_a_channel_FINDING.md.
#
# THE PRIOR IS DISCOURAGING AND IS STATED UP FRONT. label_source_RESULT.md swapped the loop's
# depth-1 label for STOCKFISH @10k -- a vastly larger label change than this one -- and got +49 Elo
# against intervals of +/-58 and +/-69, i.e. unresolved, concluding "better labels were absorbed;
# strength did not follow". So a null here is the EXPECTED outcome and must be reported as
# "consistent with label_source", not as a new discovery.
#
# rc=75 not yet runnable, rc=0 done or already done, rc=1 refused.
set -uo pipefail
D=/home/maswabe/existence
cd "$D" || exit 1
LOG=$D/run_budget_label_ab.log
NM=$D/target/release/examples/netmatch
BIN=$D/target/release/examples/budget_label_ab
GAMES=${GAMES:-60}
EPOCHS=${EPOCHS:-12}
say(){ echo "$(date +%F_%H:%M) [bla] $*" | tee -a "$LOG"; }

exec 9>"$D/.run_budget_label_ab.lock"
flock -n 9 || { say "DEFER: another instance holds the lock"; exit 75; }
[ -s "$D/bla_gate_done" ] && { say "already complete"; exit 0; }
[ -x "$BIN" ] || { say "REFUSE: $BIN missing -- build it first"; exit 1; }
[ -x "$NM" ] || { say "REFUSE: $NM missing"; exit 1; }

# LAST IN LINE, on purpose: the replication decides whether the headline result is real, and the
# cell C gate is supplementary. Neither should wait behind a diagnostic.
for u in cand-c-labels cand-a2-fixed cand-b2-budget; do
  [ "$(systemctl --user is-active $u.service 2>/dev/null)" = "active" ] && { say "DEFER: $u still running"; exit 75; }
done
# A2/B2 must have RUN, not merely be absent -- otherwise this fires before the replication starts.
[ -s "$D/candA2_fixed.net" ] || { say "DEFER: the replication has not produced candA2_fixed.net yet"; exit 75; }
for p in $(ls /proc | grep -E '^[0-9]+$'); do
  e=$(readlink "/proc/$p/exe" 2>/dev/null) || continue
  case "${e##*/}" in netmatch) say "DEFER: a netmatch is in flight (pid $p)"; exit 75;; esac
done

say "corpus $GAMES games, $EPOCHS epochs, budget 5269, fixed corpus (no self-play)"
nice -n 19 taskset -c 6-11 "$BIN" cand_start.net "$GAMES" 5269 20260912 "$EPOCHS" 16 0.0002 >>"$LOG" 2>&1 || {
  say "ABORT: budget_label_ab failed -- see $LOG"; exit 1; }
for f in bla_depth.net bla_budget.net; do
  [ -s "$D/$f" ] || { say "ABORT: $f not written"; exit 1; }
done
if [ "$(md5sum bla_depth.net|cut -d' ' -f1)" = "$(md5sum bla_budget.net|cut -d' ' -f1)" ]; then
  say "ABORT: the two arms are byte-identical -- the label column never reached the trainer"; exit 1
fi
say "both diagnostic nets written and distinct"

for sd in 20260907 911911; do
  out="$D/bla_match_$sd.log"
  nice -n 19 taskset -c 6-11 "$NM" bla_budget.net bla_depth.net 224 4 "$sd" > "$out" 2>&1
  line=$(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+' "$out" | head -1)
  [ -n "$line" ] && say "  budget-label vs depth-label seed $sd: ${line#scores }" || say "  seed $sd: NO RATE PARSED"
done
say "READING: 448 pairs over 2 seeds. A null here is CONSISTENT WITH label_source_RESULT, not a"
say "  new finding -- that file already measured a far larger label change as unresolved. A WIN"
say "  would be the surprise, and would still not be a ship."
touch "$D/bla_gate_done"
say "done"
exit 0
