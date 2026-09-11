#!/usr/bin/env bash
# CAPACITY, ASKED THE ONLY WAY IT CAN BE ASKED FROM A CHAMPION.
#
# `depth5_vs_depth3_RESULT.md` closed the datagen-depth lever: 1 -> 3 was +128 +/- 72 Elo, 3 -> 5 is
# a regression at equal wall clock, so the knee is at or below 3 and there is no direction left to
# push in label depth. `depth5_vs_depth3_PREREG.md` names what follows: capacity or search.
#
# WHY NOT `--rung 2`. That is what `w64_from_champion.sh` tried, and it cannot work: `--init` ADOPTS
# the saved net's rung and OVERRIDES `--rung`, so resuming the width-16 champion into a width-64
# slot runs the whole experiment at width 16 and reports a w16-vs-w16 A/A as a capacity result
# (`w64_misspecified_RESULT.md`, confirmed in the engine's own output). Resuming a w16 net into a
# w64 slot is simply not what resume means.
#
# THE DESIGNED PATH IS ARCH. `--arch-every N` lets the ARCH arm PROPOSE a step along the width menu
# [16,32,64,128,256,512] and judges it under its own gates. That reframes the question into the
# decision-relevant one:
#
#     not "is a width-64 net better in the abstract"
#     but "does widening THIS champion pass the gates it has to pass"
#
# and a NEGATIVE answer is a real answer: if no widening step ever passes, capacity is not the lever
# here, which is exactly what the project needs to know before spending days on it.
#
# ARCH also alternates direction (it proposes narrowing as well as widening) precisely so the arm
# cannot only ever grow -- "bigger is better" is the thing that has to be MEASURED, not assumed.
#
# PRE-REGISTERED READING, written before the run:
#   * A widening step PASSES  -> capacity is live from this champion; follow it up the menu.
#   * No step passes, and the run still beats its start -> training works, capacity is not the
#     lever, and the remaining direction is SEARCH (the P2 track). This is what I expect: every
#     capacity result in this project so far has been negative or unresolved, and the held-out fit
#     gain for w64 (12% on the SF column) never converted to strength.
#   * The run LOSES to its own start -> something regressed; read that before reading ARCH at all,
#     because a widening judged inside a failing run is not interpretable.
#
# The run is ALSO ordinary production training at the proven setting (depth 3), so the cores are not
# spent purely on a question -- whatever ARCH decides, the champion keeps improving. It is judged by
# netmatch against its own start, the paired instrument, exactly like both A/B arms.
set -uo pipefail
cd "$(dirname "$0")"
SCR=/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad
LEARN=$SCR/xt_cap/release/learn
NM=$SCR/xt_cap/release/examples/netmatch
SECS=${SECS:-3600}
CORES=${CORES:-6-11}
ARCH_EVERY=${ARCH_EVERY:-5}
[ -x "$LEARN" ] || { echo "no learn at $LEARN"; exit 1; }
[ -x "$NM" ]    || { echo "no netmatch at $NM"; exit 1; }

cp -f p1_champion.net archw_start.net
echo "$(date '+%H:%M') arch-widen: start $(md5sum archw_start.net | cut -c1-12), ${SECS}s, arch-every $ARCH_EVERY"

timeout "$SECS" taskset -c "$CORES" nice -n 19 ionice -c 3 "$LEARN" \
  --init archw_start.net --gens 1000000 --games 8 --threads 4 --depth 3 --epochs 3 \
  --gate-every 1000000 --arch-every "$ARCH_EVERY" --control-every 0 \
  --seed 20260910 --out archw.net --ledger ledger_archw.jsonl > archw.log 2>&1

G=$(grep -cE '^gen ' archw.log)
# Read the width the run ACTUALLY ran at, from the override line -- never from the line above it,
# which states the intent and not the outcome (the defect in w64_from_champion.sh).
START_W=$(grep -oE 'RESUMED (at rung [0-9]+ )?\(?width [0-9]+\)?' archw.log | head -1 | grep -oE '[0-9]+$')
[ -n "$START_W" ] || START_W=$(grep -oE 'RESUMED champion from [^ ]+ \(width [0-9]+\)' archw.log | grep -oE 'width [0-9]+' | awk '{print $2}')
echo "  $G generations, started at width ${START_W:-unknown}"
echo "  ARCH: $(grep -E '^ARCH:' archw.log | tail -1)"
grep -E 'ARCH .*(ACCEPT|REJECT|propos)' archw.log | tail -5 | sed 's/^/    /'

nice -n 19 taskset -c "$CORES" "$NM" archw.net archw_start.net 224 > archw_vs_start.log 2>&1
line=$(grep -oE 'scores 0\.[0-9]+ \+/- 0\.[0-9]+' archw_vs_start.log | head -1)
if [ -z "$line" ]; then
  echo "$(date '+%H:%M') archw gen $G: netmatch produced NO RATE -- read archw_vs_start.log. Not a verdict."
  exit 1
fi
r=$(echo "$line" | grep -oE '0\.[0-9]+' | head -1); c=$(echo "$line" | grep -oE '0\.[0-9]+' | tail -1)
v=$(python3 -c "
r,c=$r,$c
print('WINS vs its own start' if r-c>=0.5 else ('LOSES vs its own start' if r+c<0.5 else 'UNRESOLVED'))")
echo "$(date '+%H:%M') archw gen $G width ${START_W:-?}: $v   $r +/- $c" | tee -a ab_verdicts.out
