#!/usr/bin/env bash
# Discriminator for resume_dip_PREREG.md: WHEN does a resumed run lose to the champion it resumed
# from? Snapshots at generations 5 / 25 / 100, each netmatched against the champion.
#
# The snapshot loop polls the trainer's --out file, which the trainer rewrites every generation.
# Copy, never read in place: a net being written while it is played is a net that changed partway
# through its own match.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
LEARN=$SCR/xt_cap/release/learn
NM=$SCR/xt_cap/release/examples/netmatch
PAIRS=${PAIRS:-160}
OUT=rd.net; LOG=rd.log
[ -x "$LEARN" ] && [ -x "$NM" ] || { echo "missing binaries"; exit 1; }

cp -f p1_champion.net rd_start.net
echo "$(date '+%H:%M') resume-dip: start $(md5sum rd_start.net | cut -c1-12)"

taskset -c 6-11 nice -n 19 ionice -c 3 "$LEARN" \
  --init rd_start.net --gens 120 --games 8 --threads 4 --depth 3 --epochs 3 \
  --gate-every 1000000 --arch-every 0 --control-every 0 \
  --seed 20260910 --out "$OUT" --ledger rd.jsonl > "$LOG" 2>&1 &
TR=$!

for target in 5 25 100; do
  while [ -d "/proc/$TR" ]; do
    # `grep -c` PRINTS 0 and EXITS 1 when it matches nothing, so `|| echo 0` emits TWO zeros and
    # `[ "0\n0" -ge N ]` dies with "integer expected". Let grep's own 0 stand; default only when
    # the file does not exist yet.
    g=$(grep -cE '^gen ' "$LOG" 2>/dev/null); g=${g:-0}
    [ "${g:-0}" -ge "$target" ] && break
    sleep 1
  done
  [ -s "$OUT" ] && cp -f "$OUT" "rd_g${target}.net" && echo "  snapshot gen $target taken"
done
wait $TR 2>/dev/null

echo "  trainer finished at $(grep -cE '^gen ' $LOG) generations"
echo
# CONTROL FIRST. A harness that reads systematically low would make real damage indistinguishable
# from an artefact, and this repo has retracted a finding where every arm silently used one net.
nice -n 19 taskset -c 6-11 "$NM" rd_start.net p1_champion.net "$PAIRS" > rd_control.log 2>&1
echo "  CONTROL champion vs itself: $(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+' rd_control.log | head -1)  (must be ~0.500)"

for target in 5 25 100; do
  [ -s "rd_g${target}.net" ] || { echo "  gen $target: no snapshot"; continue; }
  nice -n 19 taskset -c 6-11 "$NM" "rd_g${target}.net" p1_champion.net "$PAIRS" > "rd_g${target}_vs_champ.log" 2>&1
  echo "  gen ${target} vs champion: $(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+' rd_g${target}_vs_champ.log | head -1)"
done
echo "RESUMEDIPDONE"
