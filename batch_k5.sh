#!/usr/bin/env bash
# THE MISSING MIDDLE: batch gating (K=5), matched on GENERATIONS to the two arms already measured.
#
# `acceptance_floor_RESULT.md` (2026-09-08) established the P1 gate's arithmetic: the rule is
# `pent_rate - ci95 > 0.5`, the gate's ci95 at 224 pairs is 0.0309, and a real per-generation edge
# is 0.0114 -- so **the gate demands an edge 2.7x larger than a generation produces**. Its proposed
# answer was BATCH GATING: auto-accept K-1 generations, then test the accumulated batch, so the
# edge under test is ~K times larger while the gate's ci95 is unchanged.
#
# THAT EXPERIMENT WAS DESIGNED AND NEVER CONCLUDED. `batch_ab2.sh` is on disk dated 09-08 and there
# is no batch RESULT file. Its own header records why v1 failed -- "THE ARMS WERE UNEQUAL: eleven
# generations at K=5 and FIVE at K=1" because the K=1 arm stops for a 224-pair match every
# generation -- which is the same match-on-generations trap `resume_dip_RESULT.md` re-derived
# independently tonight.
#
# WHAT IS NEW SINCE THAT DESIGN, and it reframes the question: the ungated loop is now known to dip
# ~95 Elo by generation 100 and then RECOVER to +39.8 by generation 4,327. So "adopt everything" is
# not simply broken -- it is a slow biased random walk that climbs out. And it is ~130x faster per
# generation than K=1. The 09-08 design could not know either fact.
#
# THE TWO ENDPOINTS ARE NOW MEASURED, same champion, same instrument, netmatch vs own start:
#     K=inf (never gate)   gen 100   0.366 +/- 0.035   adopts everything (mean candidate 0.4931)
#     K=1   (gate always)  gen  37   0.520 +/- 0.038   adopted 2 of 35; a 0.518 candidate REJECTED
# This fills the middle at the SAME generation count as the ungated arm.
#
# PRE-REGISTERED READING:
#   * K=5 lands near 0.52+ AND completes 100 generations quickly -> batch gating is the answer
#     acceptance_floor proposed, and it is cheap. Ship it as the default.
#   * K=5 lands near 0.366 -> batching does not rescue the filter; auto-accepting 4 of every 5
#     generations is close enough to never gating that the dip follows. The gate's FLOOR, not its
#     frequency, is the problem.
#   * K=5 lands between -> the trade is continuous and K is a real dial, worth a sweep.
#   * K=5 is SLOWER per generation than expected -> report it; a middle ground that costs K=1 time
#     for K=inf behaviour is the worst of both and must not be shipped on a strength reading alone.
#
# Matched to the ungated arm on GENERATIONS (100), not wall clock. Both questions are legitimate and
# they are different; this one is "which filter is better per generation", and the rate is reported
# so the per-wall-clock question can be answered from the same run.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
LEARN=$SCR/xt_cap/release/learn
NM=$SCR/xt_cap/release/examples/netmatch
K=${K:-5}
GENS=${GENS:-100}
PAIRS=${PAIRS:-160}
CAP=${CAP:-3000}
OUT=bk${K}.net; LOG=bk${K}.log
[ -x "$LEARN" ] && [ -x "$NM" ] || { echo "missing binaries"; exit 1; }

cp -f p1_champion.net "bk${K}_start.net"
echo "$(date '+%H:%M') batch K=$K: start $(md5sum bk${K}_start.net | cut -c1-12), target $GENS gens"
t0=$(date +%s)
timeout "$CAP" taskset -c 6-11 nice -n 19 ionice -c 3 "$LEARN" \
  --init "bk${K}_start.net" --gens "$GENS" --games 8 --threads 4 --depth 3 --epochs 3 \
  --gate-every "$K" --gate-pairs 224 --arch-every 0 --control-every 0 \
  --seed 20260910 --out "$OUT" --ledger "bk${K}.jsonl" > "$LOG" 2>&1
EL=$(( $(date +%s) - t0 ))

G=$(grep -cE '^gen ' "$LOG"); G=${G:-0}
echo "  $G generations in ${EL}s = $(python3 -c "print(f'{60*$G/max($EL,1):.1f}')") gen/min"
echo "  batch gate decisions: $(grep -c 'batch gate' "$LOG")"
grep -E 'batch gate' "$LOG" | tail -4 | sed 's/^/    /'

# The trainer writes --out only when a champion is adopted. If the batch gate rejected every batch
# the file will not exist, and that ABSENCE is the result -- say so rather than skipping silently,
# which is how the K=1 run lost its first two snapshots.
if [ ! -s "$OUT" ]; then
  echo "  NO OUTPUT NET: every batch was rejected, so the champion is still byte-identical to the"
  echo "  start. That is a verdict (K=$K froze the loop), not a failed run."
  exit 0
fi
nice -n 19 taskset -c 6-11 "$NM" "$OUT" "bk${K}_start.net" "$PAIRS" > "bk${K}_vs_start.log" 2>&1
echo "  K=$K gen $G vs its own start: $(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+ +\(interval \[0\.[0-9]+, 0\.[0-9]+\]\)' "bk${K}_vs_start.log" | head -1)"
echo "    compare  K=inf gen 100: 0.366 +/- 0.035     K=1 gen 37: 0.520 +/- 0.038"
echo "BATCHK${K}DONE"
