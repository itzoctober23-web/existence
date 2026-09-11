#!/usr/bin/env bash
# A/B: does the batch gate work once it stops deciding on a saturated scale?
#
# Same K, same champion, same generation budget, same seed. The ONLY difference is the decision
# instrument:
#   * control  -- champ-vs-origin MINUS base-vs-origin  (the shipped batch gate)
#   * treatment -- a DIRECT champion-vs-base match       (EXISTENCE_DIRECT_BATCH=1)
#
# The control arm is already running as `bk5` and is NOT re-run here; re-running it would spend an
# hour reproducing numbers already on disk. Its decisions are read from bk5.log.
#
# WHY. Measured over 13 control decisions tonight, champ-vs-origin sat at 0.948-0.981, mean 0.963.
# `instrument_saturation_RESULT.md` records that metric REVERSING SIGN at 0.861 and 0.967 -- every
# decision was inside that band. `save_origin.rs`'s header documents the same contradiction from the
# other end: the control called the champion RESOLVED WORSE against the origin (-0.054 +/- 0.033)
# while netmatch said it WON head-to-head (0.539 +/- 0.027), two resolved results pointing opposite
# ways.
#
# A direct match is centred at 0.5 (unsaturated), is ONE measurement rather than a difference of
# two, and is paired by construction. It also costs one match instead of the paired mode's two.
#
# PRE-REGISTERED READING:
#   * DIRECT keeps materially more batches than the control's 1-in-13, and the net beats its start
#     -> the batch gate was never the problem; the SCALE was. Make direct the default.
#   * DIRECT keeps about as few -> the FLOOR is the binding constraint, not the scale, and
#     acceptance_floor_RESULT's remedy does not work even when measured correctly. That retires
#     batch gating and points at the floor itself.
#   * DIRECT keeps many MORE and the net LOSES to its start -> it is admitting noise; the standard
#     `rate - ci95 >= 0.5` rule is not protective at 224 pairs on a 5-generation edge.
#
# The last row is the one to watch: "keeps more" is not success. The verdict is the netmatch against
# its own start, not the keep count.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
LEARN=./target/release/learn          # repo build: carries EXISTENCE_DIRECT_BATCH
NM=$SCR/xt_cap/release/examples/netmatch
K=${K:-5}; GENS=${GENS:-100}; PAIRS=${PAIRS:-160}; CAP=${CAP:-3000}
OUT=bd.net; LOG=bd.log
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
grep -q 'EXISTENCE_DIRECT_BATCH' crates/pipeline/src/main.rs || { echo "source lacks the flag"; exit 1; }

cp -f p1_champion.net bd_start.net
echo "$(date '+%H:%M') batch DIRECT K=$K: start $(md5sum bd_start.net | cut -c1-12), target $GENS gens"
t0=$(date +%s)
EXISTENCE_DIRECT_BATCH=1 timeout "$CAP" taskset -c 6-11 nice -n 19 ionice -c 3 "$LEARN" \
  --init bd_start.net --gens "$GENS" --games 8 --threads 4 --depth 3 --epochs 3 \
  --gate-every "$K" --gate-pairs 224 --arch-every 0 --control-every 0 \
  --seed 20260910 --out "$OUT" --ledger bd.jsonl > "$LOG" 2>&1
EL=$(( $(date +%s) - t0 ))

# PRECONDITION, checked rather than assumed: the flag must actually have taken. If the DIRECT line
# is absent the run measured the control instrument and the A/B is void -- say so, do not report a
# number. An env var set on a binary that ignores it is silent, which is how a published finding got
# retracted here today.
D=$(grep -c 'DIRECT: champ-vs-base' "$LOG")
if [ "$D" -eq 0 ]; then
  echo "  ABORT: no DIRECT decisions in the log -- the flag did not take, so this arm ran the"
  echo "  CONTROL instrument and is not comparable. Not reporting a verdict."
  exit 1
fi
G=$(grep -cE '^gen ' "$LOG"); G=${G:-0}
K_KEEP=$(grep 'DIRECT' "$LOG" | grep -c 'KEEP')
echo "  $G generations in ${EL}s; $D DIRECT decisions, $K_KEEP KEEP"
grep 'DIRECT' "$LOG" | tail -6 | sed 's/^/    /'
echo "  control arm for comparison (bk5.log): $(grep -c 'batch gate' bk5.log) decisions, $(grep -c KEEP bk5.log) KEEP"

[ -s "$OUT" ] || { echo "  NO OUTPUT NET: every batch rejected. That is the verdict, not a failure."; exit 0; }
nice -n 19 taskset -c 6-11 "$NM" "$OUT" bd_start.net "$PAIRS" > bd_vs_start.log 2>&1
echo "  DIRECT gen $G vs its own start: $(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+ +\(interval \[0\.[0-9]+, 0\.[0-9]+\]\)' bd_vs_start.log | head -1)"
echo "    compare  K=inf gen 100: 0.366 +/- 0.035    K=1 gen 37: 0.520 +/- 0.038"
echo "BATCHDIRECTDONE"
