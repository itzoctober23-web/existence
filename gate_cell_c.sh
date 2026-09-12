#!/usr/bin/env bash
# Gate cell C against arms A and B.
#
# WHAT CELL C ACTUALLY IS, restated because its original framing was wrong.
# It was built to hold the control's POSITIONS fixed and vary only the label. It cannot:
# candidate_a_channel_FINDING.md records that in a self-play loop the generator IS the net being
# trained, so C tracks A for generation 1 and diverges thereafter (positions identical on 4 of 1180
# generations). What it DOES hold fixed is the MOVE-SELECTION POLICY -- every move in cell C is
# chosen by the fixed-depth-3 search, exactly as in arm A, while the recorded label comes from the
# budget search. So:
#
#   C vs A  : changing ONLY the labelling policy, move selection held at fixed depth
#   C vs B  : changing ONLY the move-selection policy, labelling held at the budget
#
# That is a real decomposition, just not the one registered. Neither comparison isolates positions.
#
# Load-immune by construction: netmatch plays at FIXED DEPTH 4, so node counts are deterministic.
set -uo pipefail
D=/home/maswabe/existence
cd "$D" || exit 1
NM=$D/target/release/examples/netmatch
PAIRS=${PAIRS:-224}
DEPTH=4
LOG=$D/gate_cell_c.log
say(){ echo "$(date +%F_%H:%M) [cellC-gate] $*" | tee -a "$LOG"; }

exec 9>"$D/.gate_cell_c.lock"
flock -n 9 || { say "DEFER: another instance holds the lock"; exit 75; }
[ -x "$NM" ] || { say "ABORT: no netmatch at $NM"; exit 1; }

systemctl --user is-active cand-c-labels.service >/dev/null 2>&1 && { say "DEFER: cell C still training"; exit 75; }
g=$(grep -c '^gen ' "$D/candC_labels.log" 2>/dev/null || echo 0)
[ "$g" -lt 2000 ] && { say "ABORT: cell C reached only $g/2000 -- unmatched against A and B, which both ran 2000"; exit 1; }

for f in candC_labels.net candA_fixed.net candB_budget.net cand_start.net; do
  [ -s "$D/$f" ] || { say "ABORT: $f missing or empty"; exit 1; }
done
s_md5=$(md5sum cand_start.net | cut -d' ' -f1)
[ "$(md5sum candC_labels.net | cut -d' ' -f1)" = "$s_md5" ] && { say "ABORT: cell C is identical to the start net -- it trained nothing"; exit 1; }
say "cell C complete at 2000 generations and moved off the start net"

run(){ # run <netA> <netB> <seed> <label>
  local out="$D/gcc_$4_$3.log" line rate ci
  nice -n 19 taskset -c 6-11 "$NM" "$1" "$2" "$PAIRS" "$DEPTH" "$3" > "$out" 2>&1
  line=$(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+' "$out" | head -1)
  rate=$(echo "$line" | grep -oE '0\.[0-9]+' | head -1); ci=$(echo "$line" | grep -oE '0\.[0-9]+' | tail -1)
  [ -z "$rate" ] && { say "  $4 seed $3: NO RATE PARSED (see $out)"; return 1; }
  say "  $4 seed $3: $rate +/- $ci" >&2
}

# Two seeds each, not three: this is supplementary to the A-vs-B verdict and the box has the
# replication waiting behind it. Say so rather than let the smaller N pass unremarked.
say "C vs A (labelling policy varied, move selection fixed at depth 3)"
for sd in 20260907 911911; do run candC_labels.net candA_fixed.net "$sd" "CvA" || true; done
say "C vs B (move selection varied, labelling fixed at the budget)"
for sd in 20260907 911911; do run candC_labels.net candB_budget.net "$sd" "CvB" || true; done

say "READING: A-vs-B is 0.4383 over 672 pairs (candidate_a_budget_loses_RESULT.md). If C sits near A,"
say "  the labelling policy is not what costs the budget arm its 0.0617; if C sits near B, it is."
say "  Two seeds and 448 pairs per comparison -- weaker than the A/B verdict, and NOT a ship either way."
say "done"
exit 0
