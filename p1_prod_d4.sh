#!/usr/bin/env bash
# THE ONE PERMITTED EXISTENCE RUN (CLAUDE.md, p1_prod_d4_PREREG.md): P1 production, datagen depth 4,
# 300 generations from the current champion, one pinned core, a ruler rung every 50 generations.
# Launched ONLY via ~/ops/manifest.txt. Writes its PID to $PIDFILE; exits 0 at once if the planned-N
# result already exists (so a watchdog relaunch of a finished run is a no-op).
set -uo pipefail
cd /home/maswabe/existence || exit 1
PIDFILE=/home/maswabe/ops/pids/existence-p1d4.pid
RESULT=/home/maswabe/existence/p1_prod_d4_RESULT.md
mkdir -p /home/maswabe/ops/pids; echo $$ > "$PIDFILE"
[ -f "$RESULT" ] && { echo "$(date '+%F %T') planned-N result exists -- nothing to do"; exit 0; }
LEARN=/home/maswabe/existence/bin/learn_prod
[ -x "$LEARN" ] || { echo "no $LEARN"; exit 1; }
[ -s p1_champion.net ] || { echo "no champion"; exit 1; }
GENS=300; EVERY=50
# Each launch gets its own tag so rung files never collide (the generation counter restarts per launch).
LT="d4_$(date +%m%d%H%M)"; OUT="prod_${LT}.net"; LOG="prod_${LT}.log"
cp -f p1_champion.net "prod_${LT}_start.net"
echo "$(date '+%F %T') launch $LT: start $(md5sum "prod_${LT}_start.net" | cut -c1-12), depth 4, $GENS gens, rung every $EVERY, 1 thread" | tee -a p1_prod_d4.out

ruler_one() {  # $1 = net file, $2 = generation label, $3 = tag for the ledger line
  local net=$1 g=$2 tag=$3 tmp
  tmp="/home/maswabe/existence/ckpt/ruler_${tag}_gen${g}.net"; mkdir -p ckpt; cp -f "$net" "$tmp" || return 1
  nice -n 19 python3 sf_ruler.py --net "$tmp" --depth 4 --sf-elo 1320 --sf-nodes 10000 --games 120 > "prod_${LT}_ruler_gen${g}.log" 2>&1
  local line; line=$(grep -oE 'Elo vs this opponent: .*' "prod_${LT}_ruler_gen${g}.log" | head -1)
  [ -n "$line" ] || { echo "$(date '+%F %T') RULER FAILED for gen $g (see prod_${LT}_ruler_gen${g}.log)" | tee -a p1_prod_d4.out; return 1; }
  echo "$(date '+%H:%M') $tag gen $g $line" >> live_ruler.out
  echo "$(date '+%F %T') rung gen $g: $line" | tee -a p1_prod_d4.out
}

nice -n 19 ionice -c 3 "$LEARN" \
  --init "prod_${LT}_start.net" --gens $GENS --games 8 --threads 1 --depth 4 --epochs 3 \
  --lr 0.0002 --lr-decay 1.0 --blend 0.85 \
  --gate-every 1000000 --arch-every 0 --control-every 0 --rung-every $EVERY --run-tag "$LT" \
  --seed 20260910 --out "$OUT" --ledger "ledger_prod_${LT}.jsonl" > "$LOG" 2>&1 &
LPID=$!
# control: the start net on the same ruler (gen 0), measured while training runs
ruler_one "prod_${LT}_start.net" 0 "prod_${LT}"
declare -A done_g
while :; do
  for g in $(seq $EVERY $EVERY $GENS); do
    f="$OUT.$LT.gen$g.net"
    [ -s "$f" ] && [ -z "${done_g[$g]:-}" ] && { done_g[$g]=1; ruler_one "$f" "$g" "prod_${LT}"; }
  done
  kill -0 $LPID 2>/dev/null || break
  sleep 60
done
wait $LPID; rc=$?
for g in $(seq $EVERY $EVERY $GENS); do f="$OUT.$LT.gen$g.net"; [ -s "$f" ] && [ -z "${done_g[$g]:-}" ] && { done_g[$g]=1; ruler_one "$f" "$g" "prod_${LT}"; }; done
G=$(grep -cE '^gen ' "$LOG")
echo "$(date '+%F %T') learn exited rc=$rc at $G generations" | tee -a p1_prod_d4.out
if [ "$G" -ge "$GENS" ] && [ "${#done_g[@]}" -eq $((GENS/EVERY)) ]; then
  {
    echo "# RESULT — P1 production, datagen depth 4, 300 generations (planned N reached $(date '+%F %T'))"
    echo
    echo "Prereg: p1_prod_d4_PREREG.md. Start net (control): prod_${LT}_start.net md5 $(md5sum "prod_${LT}_start.net" | cut -c1-32) = p1_champion.net at launch. Binary bin/learn_prod md5 $(md5sum bin/learn_prod | cut -c1-32). Ruler engine bin/engine md5 $(md5sum bin/engine | cut -c1-32), SF-1320 @10k nodes, our depth 4, 120 games per rung, one sample per rung. Final net $OUT md5 $(md5sum "$OUT" | cut -c1-32)."
    echo
    echo '```'
    grep -E "^[0-9:]+ prod_${LT} gen " live_ruler.out
    echo '```'
    echo
    echo "Pooled per rung / trend (ruler_pool.py, filtered to this run):"
    echo '```'
    python3 ruler_pool.py 2>/dev/null | grep -E "prod_${LT}|rung|pool|slope" | head -40
    echo '```'
    echo
    echo "Reading rule (fixed in the prereg): a rung differing from the control by less than the combined SE is not movement; FLAT unless the slope over rungs is significant at z >= 2. Verdict to be written by the session from the numbers above, not by this script."
  } > "$RESULT"
  echo "$(date '+%F %T') RESULT written: $RESULT" | tee -a p1_prod_d4.out
else
  echo "$(date '+%F %T') INCOMPLETE: $G/$GENS gens, ${#done_g[@]} rungs rulered -- no result file (a relaunch starts over)" | tee -a p1_prod_d4.out
fi
exit $rc
