#!/usr/bin/env bash
# Launch cell C once arms A and B are COMPLETE. See candidate_a_channel_FINDING.md for why C exists.
#
# Cell C = the control's POSITIONS with the budget's LABELS. B - C is the position contribution,
# C - A the label contribution. Without it, an arm-B win would be credited to label quality -- the
# channel label_source_RESULT measured as null at a far larger label upgrade than this one.
#
# rc=75 = not yet runnable (arms still going). rc=0 = launched or already present. rc=1 = refused.
set -uo pipefail
D=/home/maswabe/existence
cd "$D" || exit 1
LOG=$D/launch_cell_c.log
say(){ echo "$(date +%F_%H:%M) [cellC] $*" | tee -a "$LOG"; }

# Already done or running?
if systemctl --user is-active cand-c-labels.service >/dev/null 2>&1; then say "already running"; exit 0; fi
[ -s "$D/candC_labels.net" ] && { say "candC_labels.net already exists -- not relaunching"; exit 0; }

# Arms A and B must be finished. The trainer plays a ~900-game post-loop control after its last
# generation, so the UNIT going inactive is the real completion signal, not the generation count.
for u in cand-a-fixed cand-b-budget; do
  [ "$(systemctl --user is-active $u.service 2>/dev/null)" = "active" ] && { say "DEFER: $u still active"; exit 75; }
done
for f in candA_fixed candB_budget; do
  g=$(grep -c '^gen ' "$D/$f.log" 2>/dev/null || echo 0)
  [ "$g" -lt 2000 ] && { say "REFUSE: $f reached only $g/2000 -- not launching C against unmatched arms"; exit 1; }
done

# The binary MUST be the one that implements the flag. learn_cand ACCEPTS it and ignores it, which
# would make cell C a silent duplicate of arm B.
B=$D/target/release/learn_cand2
[ -x "$B" ] || { say "REFUSE: $B missing"; exit 1; }
# CAPABILITY CHECK, and NOT `--help`: the trainer has no --help and PANICS on it, dumping a ~40MB
# core every time. An earlier version of this script called it inside a no-op `if ... then : fi`,
# which did nothing except abort a process and leave a core behind (observed 02:40:06, SIGABRT from
# existence-cell-c.service). What actually matters is that this binary implements the flag --
# learn_cand ACCEPTS --datagen-budget-labels-only and silently IGNORES it, which would make cell C a
# byte-identical duplicate of arm B.
if ! strings "$B" 2>/dev/null | grep -q 'labels-only'; then
  say "REFUSE: $B does not implement --datagen-budget-labels-only; cell C would duplicate arm B"
  exit 1
fi
say "launching cell C on learn_cand2 (equivalence to learn_cand verified byte-identical on both the"
say "  arm-A path 334c3545d248ab7c and the arm-B path 1c1f5341fe00039c)"

systemd-run --user --unit=cand-c-labels -p WorkingDirectory=$D -p Nice=19 \
  bash -c "taskset -c 6-11 $B --init cand_start.net --gens 2000 --games 8 --threads 1 --depth 3 \
    --epochs 3 --lr 0.0002 --lr-decay 1.0 --blend 0.85 --gate-every 1000000 --arch-every 0 \
    --control-every 0 --seed 20260912 --datagen-budget 5269 --datagen-budget-labels-only 1 \
    --out candC_labels.net --ledger ledger_candC.jsonl > candC_labels.log 2>&1" >>"$LOG" 2>&1
sleep 6
say "unit: $(systemctl --user is-active cand-c-labels.service)"
head -1 "$D/candC_labels.log" 2>/dev/null | tee -a "$LOG"
exit 0
