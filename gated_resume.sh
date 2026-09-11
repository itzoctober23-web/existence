#!/usr/bin/env bash
# Discriminator for gated_resume_PREREG.md: does the ~95 Elo resume dip survive a LIVE strength gate?
#
# Identical to resume_dip.sh except `--gate-every 1`, which is what turns the real per-generation
# match back on (main.rs:856: K > 1 sets batch_mode and "No match is played ... the Accept
# short-circuits `better`"). Same champion, same seed, same snapshots, same instrument.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
LEARN=$SCR/xt_cap/release/learn
NM=$SCR/xt_cap/release/examples/netmatch
PAIRS=${PAIRS:-160}
CAP=${CAP:-2700}          # wall-clock cap: the gate makes generations far slower, so this is bounded
OUT=gr.net; LOG=gr.log
[ -x "$LEARN" ] && [ -x "$NM" ] || { echo "missing binaries"; exit 1; }

cp -f p1_champion.net gr_start.net
echo "$(date '+%H:%M') gated-resume: start $(md5sum gr_start.net | cut -c1-12), gate ON (--gate-every 1)"

timeout "$CAP" taskset -c 6-11 nice -n 19 ionice -c 3 "$LEARN" \
  --init gr_start.net --gens 120 --games 8 --threads 4 --depth 3 --epochs 3 \
  --gate-every 1 --gate-pairs 224 --arch-every 0 --control-every 0 \
  --seed 20260910 --out "$OUT" --ledger gr.jsonl > "$LOG" 2>&1 &
TR=$!

# MEASURE the generation rate rather than assuming it -- the whole point of the gate is that it
# costs games, and "slow" is a number, not an adjective.
t0=$(date +%s)
for target in 5 25 100; do
  while kill -0 "$TR" 2>/dev/null; do
    # `grep -c` PRINTS 0 and EXITS 1 on no match, so `|| echo 0` would emit TWO zeros and the
    # comparison dies with "integer expected". Let grep's own 0 stand.
    g=$(grep -cE '^gen ' "$LOG" 2>/dev/null); g=${g:-0}
    [ "$g" -ge "$target" ] && break
    sleep 2
  done
  kill -0 "$TR" 2>/dev/null || { echo "  trainer exited before gen $target"; break; }
  [ -s "$OUT" ] && cp -f "$OUT" "gr_g${target}.net" \
    && echo "  snapshot gen $target at $(( $(date +%s) - t0 ))s"
done
wait "$TR" 2>/dev/null

G=$(grep -cE '^gen ' "$LOG"); EL=$(( $(date +%s) - t0 ))
A=$(grep -c 'ACCEPT' "$LOG"); R=$(grep -cE ' reject' "$LOG")
echo "  finished: $G generations in ${EL}s ($(python3 -c "print(f'{60*$G/max($EL,1):.1f}')") gen/min)"
echo "  gate actually RAN: $A accept / $R reject  (ungated runs show 100% ACCEPT and 0 reject)"

# CONTROL FIRST, always: a harness reading low would be indistinguishable from real damage.
nice -n 19 taskset -c 6-11 "$NM" gr_start.net p1_champion.net "$PAIRS" > gr_control.log 2>&1
echo "  CONTROL champion vs itself: $(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+' gr_control.log | head -1)  (must be ~0.500)"

for target in 5 25 100; do
  [ -s "gr_g${target}.net" ] || { echo "  gen $target: no snapshot"; continue; }
  nice -n 19 taskset -c 6-11 "$NM" "gr_g${target}.net" p1_champion.net "$PAIRS" > "gr_g${target}_vs_champ.log" 2>&1
  echo "  gen ${target} vs champion: $(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+ +\(interval \[0\.[0-9]+, 0\.[0-9]+\]\)' "gr_g${target}_vs_champ.log" | head -1)"
done
echo "GATEDRESUMEDONE"
